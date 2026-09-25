//! Runtime configuration shared by network-backed commands.

use std::path::Path;

use crate::api::FoxgloveClient;
use crate::config::Config;

pub(crate) const DEFAULT_CLIENT_ID: &str = "d51173be08ed4cf7a734aed9ac30afd0";
pub(crate) const DEFAULT_BASE_URL: &str = "https://api.foxglove.dev";

pub(crate) fn version() -> &'static str {
    env!("FOXGLOVE_VERSION")
}

pub(crate) fn user_agent() -> String {
    format!("foxglove-cli/{}", version())
}

pub(crate) struct Runtime {
    pub(crate) client: FoxgloveClient,
    pub(crate) project_id: String,
    project_source: &'static str,
    pub(crate) config: Config,
}

pub(crate) fn load(config_path: Option<&Path>, client_id: Option<&str>) -> Result<Runtime, String> {
    let config = Config::load_from_path(config_path)?;
    let project_id = config.get_string("default_project_id").unwrap_or_default();
    let project_source = if project_id.is_empty() {
        "unscoped"
    } else if std::env::var("DEFAULT_PROJECT_ID").is_ok_and(|value| !value.is_empty()) {
        "environment"
    } else {
        "config"
    };
    let base_url = config
        .get_string("base_url")
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| DEFAULT_BASE_URL.to_owned());
    let token = config.get_string("bearer_token").unwrap_or_default();
    let client = FoxgloveClient::new(
        &base_url,
        client_id.unwrap_or(DEFAULT_CLIENT_ID),
        token,
        user_agent(),
    )
    .map_err(|error| error.to_string())?;
    Ok(Runtime {
        client,
        project_id,
        project_source,
        config,
    })
}

/// Effective project scope for command diagnostics; credentials are never included.
pub(crate) struct ProjectScope {
    pub(crate) id: String,
    pub(crate) source: &'static str,
    pub(crate) inherited: bool,
}

impl Runtime {
    pub(crate) fn project_scope(&self, explicit: Option<&str>) -> ProjectScope {
        if let Some(id) = explicit {
            return ProjectScope {
                id: id.to_owned(),
                source: "flag",
                inherited: false,
            };
        }
        ProjectScope {
            id: self.project_id.clone(),
            source: self.project_source,
            inherited: !self.project_id.is_empty(),
        }
    }
}
