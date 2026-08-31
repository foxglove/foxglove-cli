//! Pending import commands.
#![allow(clippy::struct_field_names)]

use clap::ArgMatches;
use serde::{Deserialize, Serialize};

use crate::output::Format;
use crate::read_helpers::{
    add, add_str, finish_list, parse_timestamp, query, session_key_error, sort_query, value,
    ProjectFallback, Record, Runtime,
};
use crate::Outcome;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
struct PendingImport {
    #[serde(rename = "createdAt")]
    created_at: String,
    #[serde(rename = "updatedAt")]
    updated_at: String,
    #[serde(rename = "orgId")]
    org_id: String,
    filename: String,
    #[serde(rename = "pipelineStage")]
    pipeline_stage: String,
    #[serde(rename = "requestId")]
    request_id: String,
    #[serde(rename = "deviceId")]
    device_id: String,
    #[serde(rename = "deviceName")]
    device_name: String,
    #[serde(rename = "importId")]
    import_id: String,
    #[serde(rename = "siteId")]
    site_id: String,
    #[serde(rename = "projectId")]
    project_id: String,
    status: String,
    error: String,
}

impl Record for PendingImport {
    fn headers() -> &'static [&'static str] {
        &[
            "Created at",
            "Updated at",
            "Org ID",
            "Filename",
            "Pipeline stage",
            "Request ID",
            "Device ID",
            "Device name",
            "Import ID",
            "Site ID",
            "Project ID",
            "Status",
            "Error",
        ]
    }

    fn fields(&self) -> Vec<String> {
        vec![
            self.created_at.clone(),
            self.updated_at.clone(),
            self.org_id.clone(),
            self.filename.clone(),
            self.pipeline_stage.clone(),
            self.request_id.clone(),
            self.device_id.clone(),
            self.device_name.clone(),
            self.import_id.clone(),
            self.site_id.clone(),
            self.project_id.clone(),
            self.status.clone(),
            self.error.clone(),
        ]
    }
}

pub(crate) fn list_pending_imports(
    runtime: &Runtime,
    matches: &ArgMatches,
    format: Format,
) -> Outcome {
    let project_id = value(matches, "project-id").or_project(&runtime.project_id);
    if let Some(error) = session_key_error(matches, &project_id) {
        return Outcome::failure(error);
    }
    let updated_since = match parse_timestamp(&value(matches, "updated-since"), "updated since") {
        Ok(value) => value,
        Err(error) => {
            return Outcome::failure(format!(
                "Failed to parse value of --updated-since: {error}\n"
            ))
        }
    };
    let mut query = query();
    add_str(&mut query, "device.id", &value(matches, "device-id"));
    add_str(&mut query, "device.name", &value(matches, "device-name"));
    add_str(&mut query, "error", &value(matches, "error"));
    add_str(&mut query, "filename", &value(matches, "filename"));
    add(
        &mut query,
        "hasProjectId",
        "false",
        matches.get_flag("without-project"),
    );
    add_str(&mut query, "key", &value(matches, "key"));
    add_str(&mut query, "projectId", &project_id);
    add_str(&mut query, "requestId", &value(matches, "request-id"));
    add_str(&mut query, "sessionId", &value(matches, "session-id"));
    add_str(&mut query, "sessionKey", &value(matches, "session-key"));
    add_str(&mut query, "siteId", &value(matches, "site-id"));
    add(
        &mut query,
        "showCompleted",
        "true",
        matches.get_flag("show-completed"),
    );
    add(
        &mut query,
        "showQuarantined",
        "true",
        matches.get_flag("show-quarantined"),
    );
    add_str(&mut query, "updatedSince", &updated_since);
    sort_query(&mut query);
    finish_list(
        runtime,
        format,
        "Failed to list pending imports",
        move |client| async move {
            client
                .get::<_, Vec<PendingImport>>("/v1/data/pending-imports", &query)
                .await
        },
    )
}
