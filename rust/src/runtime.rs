//! Runtime configuration shared by network-backed commands.

use std::path::Path;

use crate::api::{ApiError, FoxgloveClient};
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

pub(crate) fn load(
    config_path: Option<&Path>,
    client_id: Option<&str>,
    debug: bool,
) -> Result<Runtime, String> {
    let config = Config::load_from_path(config_path)?;
    crate::config::warn_legacy_environment();
    let project_id = config.get_string("default_project_id").unwrap_or_default();
    let project_source = if project_id.is_empty() {
        "none"
    } else {
        crate::config::environment_name("default_project_id").unwrap_or("default_project_id")
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
    .map_err(|error| match error {
        ApiError::InvalidUrl(message) => format!(
            "{message} (run `foxglove auth login --base-url https://...` or set FOXGLOVE_BASE_URL)\n"
        ),
        error => format!("{error}\n"),
    })?
    .with_debug(debug);
    Ok(Runtime {
        client,
        project_id,
        project_source,
        config,
    })
}

pub(crate) struct ProjectScope {
    pub(crate) id: String,
    pub(crate) source: &'static str,
}

impl Runtime {
    pub(crate) fn project_scope(&self, explicit: Option<&str>) -> ProjectScope {
        if let Some(id) = explicit {
            return ProjectScope {
                id: id.to_owned(),
                source: "--project-id",
            };
        }
        ProjectScope {
            id: self.project_id.clone(),
            source: self.project_source,
        }
    }
}
