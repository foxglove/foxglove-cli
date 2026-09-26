//! YAML-backed CLI configuration with environment-over-file precedence.

use std::env;
use std::fs::{self, File, OpenOptions, Permissions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use serde_yaml_ng::{Mapping, Value};

/// The persisted Foxglove CLI configuration.
pub struct Config {
    path: PathBuf,
    values: Mapping,
    permissions: Option<Permissions>,
}

impl Config {
    /// Load the default configuration file.
    ///
    /// # Errors
    ///
    /// This delegates to [`Self::load`].
    pub fn load_default() -> Result<Self, String> {
        Self::load_from_path(None)
    }

    /// Load a configuration from a caller-selected path, or the default path.
    ///
    /// This lets command dispatch honor `--config` without making config
    /// consumers duplicate the platform-specific default-path logic.
    ///
    /// # Errors
    ///
    /// Returns an error when the selected configuration is unreadable or is
    /// not a valid YAML mapping.
    pub fn load_from_path(path: Option<&Path>) -> Result<Self, String> {
        if let Some(path) = path {
            return Self::load(path.to_owned());
        }
        let home = env::var_os("HOME")
            .or_else(|| env::var_os("USERPROFILE"))
            .unwrap_or_default();
        Self::load(PathBuf::from(home).join(".foxgloverc"))
    }

    /// Load a configuration from an explicit path.
    ///
    /// # Errors
    ///
    /// A missing file is an empty configuration so first-run commands can
    /// create it. Other I/O errors and malformed YAML are reported instead of
    /// being mistaken for a valid empty configuration.
    pub fn load(path: PathBuf) -> Result<Self, String> {
        let permissions = match fs::metadata(&path) {
            Ok(metadata) => Some(metadata.permissions()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => None,
            Err(error) => {
                return Err(format!(
                    "failed to read config {}: {error}\n",
                    path.display()
                ))
            }
        };
        let source = match fs::read_to_string(&path) {
            Ok(source) => source,
            Err(error) if error.kind() == io::ErrorKind::NotFound => String::new(),
            Err(error) => {
                return Err(format!(
                    "failed to read config {}: {error}\n",
                    path.display()
                ))
            }
        };
        let values = match serde_yaml_ng::from_str::<Value>(&source) {
            Ok(Value::Mapping(values)) => values,
            Ok(Value::Null) => Mapping::new(),
            Ok(_) => {
                return Err(format!(
                    "failed to parse config {}: expected a YAML mapping\n",
                    path.display()
                ))
            }
            Err(error) => {
                return Err(format!(
                    "failed to parse config {}: {error}\n",
                    path.display()
                ))
            }
        };
        Ok(Self {
            path,
            values,
            permissions,
        })
    }

    /// Return the selected path, primarily for the interactive auth prompt.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Return a string value using Viper's environment-over-file precedence.
    pub fn get_string(&self, key: &str) -> Option<String> {
        if let Some(value) = environment_value(key) {
            return Some(value);
        }
        self.values
            .get(Value::String(key.to_owned()))
            .map(value_as_string)
    }

    /// Whether a value is supplied by either the environment or the file.
    #[must_use]
    pub fn is_set(&self, key: &str) -> bool {
        environment_value(key).is_some() || self.values.contains_key(Value::String(key.to_owned()))
    }

    /// Whether a nonempty environment override supplies this value.
    #[must_use]
    pub fn is_env_set(&self, key: &str) -> bool {
        environment_value(key).is_some()
    }

    /// Set a persisted value.
    pub fn set(&mut self, key: &str, value: Value) {
        self.values.insert(Value::String(key.to_owned()), value);
    }

    /// Remove a persisted value, returning false when the file did not contain
    /// it. Environment overrides are external to the configuration file and
    /// are never reported as removed.
    pub fn remove(&mut self, key: &str) -> bool {
        self.values.remove(Value::String(key.to_owned())).is_some()
    }

    /// Atomically save the YAML mapping and preserve existing mode bits.
    ///
    /// # Errors
    ///
    /// Returns a descriptive error when serialization or filesystem operations
    /// fail.
    pub fn save(&self) -> Result<(), String> {
        let source = serde_yaml_ng::to_string(&Value::Mapping(self.values.clone()))
            .map_err(|error| format!("failed to write config: {error}\n"))?;
        let (temporary_path, mut temporary) = self.create_temporary()?;
        let cleanup = TemporaryGuard(&temporary_path);
        temporary
            .write_all(source.as_bytes())
            .and_then(|()| temporary.flush())
            .map_err(|error| format!("failed to write config: {error}\n"))?;
        if let Some(permissions) = &self.permissions {
            fs::set_permissions(&temporary_path, permissions.clone())
                .map_err(|error| format!("failed to write config: {error}\n"))?;
        }
        drop(temporary);
        replace_file(&temporary_path, &self.path)
            .map_err(|error| format!("failed to write config: {error}\n"))?;
        cleanup.disarm();
        Ok(())
    }

    fn create_temporary(&self) -> Result<(PathBuf, File), String> {
        for attempt in 0..100_u8 {
            let suffix = format!("tmp-{}-{attempt}", std::process::id());
            let path = self.path.with_extension(suffix);
            match OpenOptions::new().write(true).create_new(true).open(&path) {
                Ok(file) => {
                    // A newly-created config can contain a bearer token. Do
                    // this before its contents are written; existing files
                    // retain their permissions in `save` above.
                    if let Err(error) = restrict_new_config_permissions(&file) {
                        drop(file);
                        let _ = fs::remove_file(&path);
                        return Err(error);
                    }
                    return Ok((path, file));
                }
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
                Err(error) => return Err(format!("failed to write config: {error}\n")),
            }
        }
        Err("failed to write config: could not create a temporary file\n".to_owned())
    }
}

#[cfg(unix)]
fn restrict_new_config_permissions(file: &File) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;

    file.set_permissions(Permissions::from_mode(0o600))
        .map_err(|error| format!("failed to write config: {error}\n"))
}

#[cfg(not(unix))]
fn restrict_new_config_permissions(_file: &File) -> Result<(), String> {
    Ok(())
}

/// Replace the destination in one filesystem operation.
/// Both paths must be on the same filesystem; callers use sibling staging paths.
pub(crate) fn replace_file(temporary: &Path, destination: &Path) -> io::Result<()> {
    fs::rename(temporary, destination)
}

struct TemporaryGuard<'a>(&'a Path);

impl TemporaryGuard<'_> {
    fn disarm(self) {
        std::mem::forget(self);
    }
}

impl Drop for TemporaryGuard<'_> {
    fn drop(&mut self) {
        let _ = fs::remove_file(self.0);
    }
}

fn environment_name(key: &str) -> String {
    key.to_ascii_uppercase()
}

fn environment_value(key: &str) -> Option<String> {
    // Viper ignores empty environment values unless AllowEmptyEnv is enabled.
    // Keep explicitly empty persisted values intact: only environment overrides
    // use this fallback rule.
    env::var(environment_name(key))
        .ok()
        .filter(|value| !value.is_empty())
}

fn value_as_string(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::String(value) => value.clone(),
        other => serde_yaml_ng::to_string(other)
            .unwrap_or_default()
            .trim()
            .to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::Config;
    use serde_yaml_ng::{Mapping, Value};

    static NEXT_DIRECTORY: AtomicUsize = AtomicUsize::new(0);

    fn test_path() -> std::path::PathBuf {
        let directory = std::env::temp_dir().join(format!(
            "foxglove-rust-config-test-{}-{}",
            std::process::id(),
            NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&directory).unwrap();
        directory.join(".foxgloverc")
    }

    #[test]
    fn missing_value_is_not_removed() {
        let mut config = Config {
            path: "unused".into(),
            values: Mapping::new(),
            permissions: None,
        };
        assert!(!config.remove("default_project_id"));
    }

    #[test]
    fn malformed_config_is_reported() {
        let path = test_path();
        fs::write(&path, "default_project_id: [\n").unwrap();
        let Err(error) = Config::load(path.clone()) else {
            panic!("malformed YAML should fail");
        };
        assert!(error.contains("failed to parse config"));
        fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn explicit_path_is_honored() {
        let path = test_path();
        fs::write(&path, "default_project_id: explicit\n").unwrap();
        let config = Config::load_from_path(Some(&path)).unwrap();
        assert_eq!(config.path(), path);
        assert_eq!(
            config.get_string("default_project_id").as_deref(),
            Some("explicit")
        );
        fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn unknown_yaml_values_survive_updates() {
        let path = test_path();
        fs::write(
            &path,
            "auth_type: 1\ncustom:\n  nested: true\ndefault_project_id: old\n",
        )
        .unwrap();
        let mut config = Config::load(path.clone()).unwrap();
        config.set("default_project_id", Value::String("new".into()));
        config.save().unwrap();
        let values: Value = serde_yaml_ng::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(
            values["custom"]["nested"],
            Value::Bool(true),
            "unknown nested configuration should be preserved"
        );
        assert_eq!(values["default_project_id"], Value::String("new".into()));
        fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn existing_permissions_survive_atomic_replacement() {
        use std::os::unix::fs::PermissionsExt;

        let path = test_path();
        fs::write(&path, "default_project_id: old\n").unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
        let mut config = Config::load(path.clone()).unwrap();
        config.set("default_project_id", Value::String("new".into()));
        config.save().unwrap();
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
        fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn new_config_is_private() {
        use std::os::unix::fs::PermissionsExt;

        let path = test_path();
        let mut config = Config::load(path.clone()).unwrap();
        config.set("bearer_token", Value::String("secret".into()));
        config.save().unwrap();
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
        fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn replacement_installs_new_contents() {
        use super::replace_file;

        let path = test_path();
        let temporary = path.with_extension("replacement");
        fs::write(&path, "bearer_token: old\n").unwrap();
        fs::write(&temporary, "bearer_token: new\n").unwrap();

        replace_file(&temporary, &path).unwrap();

        assert_eq!(fs::read_to_string(&path).unwrap(), "bearer_token: new\n");
        assert!(!temporary.exists());
        fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn failed_replacement_preserves_destination() {
        let path = test_path();
        let missing = path.with_extension("missing");
        fs::write(&path, "bearer_token: old\n").unwrap();

        assert!(super::replace_file(&missing, &path).is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), "bearer_token: old\n");
        assert_eq!(fs::read_dir(path.parent().unwrap()).unwrap().count(), 1);
        fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn locked_destination_preserves_both_files_on_failed_replacement() {
        use std::os::windows::fs::OpenOptionsExt;

        let path = test_path();
        let temporary = path.with_extension("replacement");
        fs::write(&path, "bearer_token: old\n").unwrap();
        fs::write(&temporary, "bearer_token: new\n").unwrap();
        let locked = fs::OpenOptions::new()
            .read(true)
            .share_mode(0)
            .open(&path)
            .unwrap();

        assert!(super::replace_file(&temporary, &path).is_err());
        drop(locked);
        assert_eq!(fs::read_to_string(&path).unwrap(), "bearer_token: old\n");
        assert_eq!(
            fs::read_to_string(&temporary).unwrap(),
            "bearer_token: new\n"
        );
        assert_eq!(fs::read_dir(path.parent().unwrap()).unwrap().count(), 2);
        fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }
}
