//! Pending import commands.

use serde::{Deserialize, Serialize};

use crate::cli::PendingImportListArgs;
use crate::output::Format;
use crate::records::{
    fetch_list, is_false, null_to_default, parse_timestamp, ProjectFallback, Record,
};
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
    #[serde(default, deserialize_with = "null_to_default")]
    device_id: String,
    #[serde(rename = "deviceName")]
    #[serde(default, deserialize_with = "null_to_default")]
    device_name: String,
    #[serde(rename = "importId")]
    #[serde(default, deserialize_with = "null_to_default")]
    import_id: String,
    #[serde(rename = "siteId")]
    site_id: String,
    #[serde(rename = "projectId")]
    #[serde(default, deserialize_with = "null_to_default")]
    project_id: String,
    #[serde(default, deserialize_with = "null_to_default")]
    status: String,
    #[serde(default, deserialize_with = "null_to_default")]
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
    if args.without_project
        && (args.project_id.as_deref().is_some_and(|id| !id.is_empty())
            || args
                .session_key
                .as_deref()
                .is_some_and(|key| !key.is_empty()))
    {
        return Outcome::failure(
            "--without-project cannot be combined with a nonempty --project-id or --session-key\n",
        );
    }
    let project_id = if args.without_project {
        String::new()
    } else {
        args.project_id.clone().or_project(&runtime.project_id)
    };
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

#[cfg(test)]
mod tests {
    use super::PendingImport;

    #[test]
    fn missing_and_null_fields_render_explicit_defaults() {
        let original = serde_json::json!({"createdAt":"2024-01-02T03:04:05Z","updatedAt":"2024-01-02T03:04:06Z","orgId":"org_fixture","filename":"fixture.mcap","pipelineStage":"parse","requestId":"req_fixture","siteId":"site_fixture"});
        for (field, expected) in [
            ("deviceId", serde_json::json!("")),
            ("deviceName", serde_json::json!("")),
            ("importId", serde_json::json!("")),
            ("projectId", serde_json::json!("")),
            ("status", serde_json::json!("")),
            ("error", serde_json::json!("")),
        ] {
            let mut with_null = original.clone();
            with_null[field] = serde_json::Value::Null;
            for response in [original.clone(), with_null] {
                let record: PendingImport = serde_json::from_value(response).unwrap();
                let output = serde_json::to_value(record).unwrap();
                assert_eq!(output[field], expected, "{field}");
            }
        }
    }
}
