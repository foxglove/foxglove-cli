//! Recording commands.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::api::encode_path_segment;
use crate::cli::{RecordingDeleteArgs, RecordingListArgs, RecordingTransferArgs};
use crate::output::Format;
use crate::records::{
    compact_json, fetch_list, is_zero, null_to_default, parse_timestamp, DeviceSummary,
    ProjectFallback, Record,
};
use crate::runtime::Runtime;
use crate::Outcome;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
struct Site {
    name: String,
    id: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
struct MetadataRecord {
    name: String,
    metadata: Value,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
struct Recording {
    id: String,
    path: String,
    size: i64,
    #[serde(rename = "messageCount")]
    #[serde(default, deserialize_with = "null_to_default")]
    message_count: i64,
    #[serde(rename = "createdAt")]
    created_at: String,
    #[serde(rename = "importedAt")]
    #[serde(default, deserialize_with = "null_to_default")]
    imported_at: String,
    start: String,
    end: String,
    #[serde(rename = "importStatus")]
    import_status: String,
    #[serde(default, deserialize_with = "null_to_default")]
    site: Site,
    #[serde(rename = "edgeSite")]
    #[serde(default, deserialize_with = "null_to_default")]
    edge_site: Site,
    #[serde(default, deserialize_with = "null_to_default")]
    device: DeviceSummary,
    #[serde(default)]
    metadata: Option<Vec<MetadataRecord>>,
    #[serde(default, deserialize_with = "null_to_default")]
    key: String,
    #[serde(rename = "projectId")]
    project_id: String,
}

impl Record for Recording {
    fn headers() -> &'static [&'static str] {
        &[
            "Recording ID",
            "Path",
            "Size",
            "Message Count",
            "Created At",
            "Imported At",
            "Start",
            "End",
            "Import Status",
            "Site ID",
            "Site Name",
            "Edge Site ID",
            "Edge Site Name",
            "Device ID",
            "Device Name",
            "Metadata",
            "Key",
            "Project ID",
        ]
    }

    fn fields(&self) -> Vec<String> {
        vec![
            self.id.clone(),
            self.path.clone(),
            format!("{} B", self.size),
            self.message_count.to_string(),
            self.created_at.clone(),
            self.imported_at.clone(),
            self.start.clone(),
            self.end.clone(),
            self.import_status.clone(),
            self.site.id.clone(),
            self.site.name.clone(),
            self.edge_site.id.clone(),
            self.edge_site.name.clone(),
            self.device.id.clone(),
            self.device.name.clone(),
            compact_json(&serde_json::to_value(&self.metadata).unwrap_or(Value::Null)),
            self.key.clone(),
            self.project_id.clone(),
        ]
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RecordingListQuery {
    #[serde(rename = "device.id", skip_serializing_if = "String::is_empty")]
    device_id: String,
    #[serde(rename = "device.name", skip_serializing_if = "String::is_empty")]
    device_name: String,
    #[serde(rename = "edgeSite.id", skip_serializing_if = "String::is_empty")]
    edge_site_id: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    end: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    import_status: String,
    #[serde(skip_serializing_if = "is_zero")]
    limit: i64,
    #[serde(skip_serializing_if = "is_zero")]
    offset: i64,
    #[serde(skip_serializing_if = "String::is_empty")]
    path: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    project_id: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    session_id: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    session_key: String,
    #[serde(rename = "site.id", skip_serializing_if = "String::is_empty")]
    site_id: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    sort_by: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    sort_order: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    start: String,
}

pub(crate) async fn list_recordings(
    runtime: &Runtime,
    args: &RecordingListArgs,
    format: Format,
) -> Outcome {
    let project_id = args.project_id.clone().or_project(&runtime.project_id);
    let session_key = args.session_key.clone().unwrap_or_default();
    if !session_key.is_empty() && project_id.is_empty() {
        return Outcome::failure("--project-id is required when using --session-key\n");
    }
    let start = match parse_timestamp(args.start.as_deref().unwrap_or_default(), "start") {
        Ok(value) => value,
        Err(error) => return Outcome::failure(format!("{error}\n")),
    };
    let end = match parse_timestamp(args.end.as_deref().unwrap_or_default(), "end") {
        Ok(value) => value,
        Err(error) => return Outcome::failure(format!("{error}\n")),
    };
    let limit = args.limit.unwrap_or(2000);
    let offset = args.offset.unwrap_or_default();
    let query = RecordingListQuery {
        device_id: args.device_id.clone().unwrap_or_default(),
        device_name: args.device_name.clone().unwrap_or_default(),
        edge_site_id: args.edge_site_id.clone().unwrap_or_default(),
        end,
        import_status: args.import_status.clone().unwrap_or_default(),
        limit,
        offset,
        path: args.path.clone().unwrap_or_default(),
        project_id,
        session_id: args.session_id.clone().unwrap_or_default(),
        session_key,
        site_id: args.site_id.clone().unwrap_or_default(),
        sort_by: args.sort_by.clone().unwrap_or_default(),
        sort_order: args.sort_order.clone().unwrap_or_default(),
        start,
    };
    fetch_list::<Recording, _>(
        runtime,
        format,
        "Failed to list recordings",
        "/v1/recordings",
        &query,
    )
    .await
}

pub(crate) async fn delete_recording(runtime: &Runtime, args: &RecordingDeleteArgs) -> Outcome {
    match runtime
        .client
        .delete(&format!("/v1/recordings/{}", encode_path_segment(&args.id)))
        .await
    {
        Ok(()) => Outcome::default(),
        Err(error) if error.is_not_found() => Outcome {
            stderr: b"Not found. The resource may have already been deleted.\n".to_vec(),
            ..Outcome::default()
        },
        Err(error) => Outcome::failure(format!("Failed to delete recording: {error}\n")),
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TransferResponse {
    id: String,
    import_status: String,
}

pub(crate) async fn transfer_recording(runtime: &Runtime, args: &RecordingTransferArgs) -> Outcome {
    match runtime
        .client
        .post::<_, TransferResponse>(
            &format!("/v1/recordings/{}/import", encode_path_segment(&args.id)),
            &serde_json::json!({}),
        )
        .await
    {
        Ok(response) => {
            let message = if response.import_status == "complete" {
                "Recording already available at its Primary Site"
            } else {
                "Transfer request accepted"
            };
            Outcome {
                stderr: format!(
                    "{message}: {} (importStatus: {})\n",
                    response.id, response.import_status
                )
                .into_bytes(),
                ..Outcome::default()
            }
        }
        Err(error) => Outcome::failure(format!("Failed to transfer edge recording: {error}\n")),
    }
}

#[cfg(test)]
mod tests {
    use super::Recording;

    #[test]
    fn missing_and_null_fields_render_explicit_defaults() {
        let original = serde_json::json!({"id":"rec_fixture","path":"fixture.mcap","size":128,"createdAt":"2024-01-02T03:04:05Z","start":"2024-01-02T03:04:05Z","end":"2024-01-02T03:04:06Z","importStatus":"none","projectId":"prj_default"});
        for (field, expected) in [
            ("messageCount", serde_json::json!(0)),
            ("importedAt", serde_json::json!("")),
            ("site", serde_json::json!({"id": "", "name": ""})),
            ("edgeSite", serde_json::json!({"id": "", "name": ""})),
            ("device", serde_json::json!({"id": "", "name": ""})),
            ("key", serde_json::json!("")),
        ] {
            let mut with_null = original.clone();
            with_null[field] = serde_json::Value::Null;
            for response in [original.clone(), with_null] {
                let record: Recording = serde_json::from_value(response).unwrap();
                let output = serde_json::to_value(record).unwrap();
                assert_eq!(output[field], expected, "{field}");
            }
        }
    }
}
