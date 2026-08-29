//! YAML-backed CLI configuration with Viper-compatible environment precedence.

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
    /// GO-001 is intentionally preserved: the parsed `--config` flag does not
    /// affect the file loaded for the current invocation.
    ///
    /// # Errors
    ///
    /// This delegates to [`Self::load`].
    pub fn load_default() -> Result<Self, String> {
        let home = env::var_os("HOME")
            .or_else(|| env::var_os("USERPROFILE"))
            .unwrap_or_default();
        Self::load(PathBuf::from(home).join(".foxgloverc"))
    }

    /// Load a configuration from an explicit path.
    ///
    /// # Errors
    ///
    /// This signature preserves room for future configuration-load errors.
    pub fn load(path: PathBuf) -> Result<Self, String> {
        let permissions = fs::metadata(&path)
            .ok()
            .map(|metadata| metadata.permissions());
        // Execute ignores Viper's ReadInConfig error, including missing,
        // unreadable, and malformed files. Preserve that behavior here.
        let source = fs::read_to_string(&path).unwrap_or_default();
        // The Go oracle deliberately ignores Viper's ReadInConfig error. Treat
        // malformed YAML as an empty configuration for the same invocation.
        let values = match serde_yaml_ng::from_str::<Value>(&source) {
            Ok(Value::Mapping(values)) => values,
            Ok(_) | Err(_) => Mapping::new(),
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
        if let Ok(value) = env::var(environment_name(key)) {
            return Some(value);
        }
        self.values
            .get(Value::String(key.to_owned()))
            .map(value_as_string)
    }

    /// Whether a value is supplied by either the environment or the file.
    #[must_use]
    pub fn is_set(&self, key: &str) -> bool {
        env::var_os(environment_name(key)).is_some()
            || self.values.contains_key(Value::String(key.to_owned()))
    }

    /// Set a persisted value.
    pub fn set(&mut self, key: &str, value: Value) {
        self.values.insert(Value::String(key.to_owned()), value);
    }

    /// Remove a persisted value, returning false when no source supplied it.
    pub fn remove(&mut self, key: &str) -> bool {
        if !self.is_set(key) {
            return false;
        }
        self.values.remove(Value::String(key.to_owned()));
        true
    }

    /// Atomically save a deterministic YAML mapping and preserve existing mode bits.
    ///
    /// # Errors
    ///
    /// Returns a Go-compatible error string when serialization or filesystem
    /// operations fail.
    pub fn save(&self) -> Result<(), String> {
        let source = serde_yaml_ng::to_string(&Value::Mapping(sorted_mapping(&self.values)))
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
        fs::rename(&temporary_path, &self.path)
            .map_err(|error| format!("failed to write config: {error}\n"))?;
        cleanup.disarm();
        Ok(())
    }

    fn create_temporary(&self) -> Result<(PathBuf, File), String> {
        for attempt in 0..100_u8 {
            let suffix = format!("tmp-{}-{attempt}", std::process::id());
            let path = self.path.with_extension(suffix);
            match OpenOptions::new().write(true).create_new(true).open(&path) {
                Ok(file) => return Ok((path, file)),
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
                Err(error) => return Err(format!("failed to write config: {error}\n")),
            }
        }
        Err("failed to write config: could not create a temporary file\n".to_owned())
    }
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

fn sorted_mapping(values: &Mapping) -> Mapping {
    let mut strings = values
        .iter()
        .filter_map(|(key, value)| match key {
            Value::String(key) => Some((key.clone(), value.clone())),
            _ => None,
        })
        .collect::<Vec<_>>();
    strings.sort_by(|left, right| left.0.cmp(&right.0));
    let mut sorted = Mapping::new();
    for (key, value) in strings {
        sorted.insert(Value::String(key), value);
    }
    for (key, value) in values {
        if !matches!(key, Value::String(_)) {
            sorted.insert(key.clone(), value.clone());
        }
    }
    sorted
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::{sorted_mapping, Config};
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
    fn mappings_are_written_in_viper_order() {
        let mut values = Mapping::new();
        values.insert(Value::String("z".into()), Value::String("last".into()));
        values.insert(Value::String("a".into()), Value::String("first".into()));
        let keys = sorted_mapping(&values)
            .into_iter()
            .map(|(key, _)| key.as_str().unwrap().to_owned())
            .collect::<Vec<_>>();
        assert_eq!(keys, ["a", "z"]);
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
}
