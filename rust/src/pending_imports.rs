//! Pending import commands.

use serde::{Deserialize, Serialize};

use crate::cli::PendingImportListArgs;
use crate::output::Format;
use crate::records::{fetch_list, is_false, parse_timestamp, ProjectFallback, Record};
use crate::runtime::Runtime;
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

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PendingImportListQuery {
    #[serde(rename = "device.id", skip_serializing_if = "String::is_empty")]
    device_id: String,
    #[serde(rename = "device.name", skip_serializing_if = "String::is_empty")]
    device_name: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    error: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    filename: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    has_project_id: Option<bool>,
    #[serde(skip_serializing_if = "String::is_empty")]
    key: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    project_id: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    request_id: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    session_id: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    session_key: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    site_id: String,
    #[serde(skip_serializing_if = "is_false")]
    show_completed: bool,
    #[serde(skip_serializing_if = "is_false")]
    show_quarantined: bool,
    #[serde(skip_serializing_if = "String::is_empty")]
    updated_since: String,
}

pub(crate) async fn list_pending_imports(
    runtime: &Runtime,
    args: &PendingImportListArgs,
    format: Format,
) -> Outcome {
    let project_id = args
        .project_id
        .clone()
        .unwrap_or_default()
        .or_project(&runtime.project_id);
    let session_key = args.session_key.clone().unwrap_or_default();
    if !session_key.is_empty() && project_id.is_empty() {
        return Outcome::failure("--project-id is required when using --session-key\n");
    }
    let updated_since = match parse_timestamp(
        args.updated_since.as_deref().unwrap_or_default(),
        "updated since",
    ) {
        Ok(value) => value,
        Err(error) => {
            return Outcome::failure(format!(
                "Failed to parse value of --updated-since: {error}\n"
            ))
        }
    };
    let query = PendingImportListQuery {
        device_id: args.device_id.clone().unwrap_or_default(),
        device_name: args.device_name.clone().unwrap_or_default(),
        error: args.error.clone().unwrap_or_default(),
        filename: args.filename.clone().unwrap_or_default(),
        has_project_id: args.without_project.then_some(false),
        key: args.key.clone().unwrap_or_default(),
        project_id,
        request_id: args.request_id.clone().unwrap_or_default(),
        session_id: args.session_id.clone().unwrap_or_default(),
        session_key,
        site_id: args.site_id.clone().unwrap_or_default(),
        show_completed: args.show_completed,
        show_quarantined: args.show_quarantined,
        updated_since,
    };
    fetch_list::<PendingImport, _>(
        runtime,
        format,
        "Failed to list pending imports",
        "/v1/data/pending-imports",
        &query,
    )
    .await
}
