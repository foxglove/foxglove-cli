//! Coverage listing.

use serde::{Deserialize, Serialize};

use crate::cli::CoverageListArgs;
use crate::output::Format;
use crate::records::{
    fetch_list, is_false, is_zero, null_to_default, parse_timestamp, ProjectFallback, Record,
};
use crate::runtime::Runtime;
use crate::Outcome;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
struct Coverage {
    #[serde(rename = "deviceId")]
    #[serde(default, deserialize_with = "null_to_default")]
    device_id: String,
    #[serde(default, deserialize_with = "null_to_default")]
    device: CoverageDevice,
    start: String,
    end: String,
    status: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
struct CoverageDevice {
    name: String,
    id: String,
    #[serde(rename = "pendingCoverageUpdate", default)]
    pending_coverage_update: Option<PendingCoverageUpdate>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
struct PendingCoverageUpdate {
    #[serde(rename = "nextInvalidationRequestedAt")]
    #[serde(default, deserialize_with = "null_to_default")]
    next_invalidation_requested_at: String,
}

impl Record for Coverage {
    fn headers() -> &'static [&'static str] {
        &[
            "Device ID",
            "Device Name",
            "Start",
            "End",
            "Status",
            "Coverage Update Requested At",
        ]
    }

    fn fields(&self) -> Vec<String> {
        vec![
            self.device_id.clone(),
            self.device.name.clone(),
            self.start.clone(),
            self.end.clone(),
            self.status.clone(),
            self.device
                .pending_coverage_update
                .as_ref()
                .map(|update| update.next_invalidation_requested_at.clone())
                .unwrap_or_default(),
        ]
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CoverageListQuery {
    #[serde(skip_serializing_if = "String::is_empty")]
    device_id: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    device_name: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    end: String,
    #[serde(skip_serializing_if = "is_false")]
    include_edge_recordings: bool,
    #[serde(skip_serializing_if = "String::is_empty")]
    project_id: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    recording_id: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    recording_key: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    session_id: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    session_key: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    start: String,
    #[serde(skip_serializing_if = "is_zero")]
    tolerance: i64,
}

pub(crate) async fn list(runtime: &Runtime, args: &CoverageListArgs, format: Format) -> Outcome {
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
    let tolerance = args.tolerance.unwrap_or_default();
    let query = CoverageListQuery {
        device_id: args.device_id.clone().unwrap_or_default(),
        device_name: args.device_name.clone().unwrap_or_default(),
        end,
        include_edge_recordings: args.include_edge_recordings,
        project_id,
        recording_id: args.recording_id.clone().unwrap_or_default(),
        recording_key: args.recording_key.clone().unwrap_or_default(),
        session_id: args.session_id.clone().unwrap_or_default(),
        session_key,
        start,
        tolerance,
    };
    fetch_list::<Coverage, _>(
        runtime,
        format,
        "Failed to list coverage",
        "/v1/data/coverage",
        &query,
        None,
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::Coverage;

    #[test]
    fn missing_and_null_fields_render_explicit_defaults() {
        let original = serde_json::json!({"start":"2024-01-02T03:04:05Z","end":"2024-01-02T03:04:06Z","status":"imported"});
        for (field, expected) in [
            ("deviceId", serde_json::json!("")),
            (
                "device",
                serde_json::json!({"id": "", "name": "", "pendingCoverageUpdate": null}),
            ),
        ] {
            let mut with_null = original.clone();
            with_null[field] = serde_json::Value::Null;
            for response in [original.clone(), with_null] {
                let record: Coverage = serde_json::from_value(response).unwrap();
                let output = serde_json::to_value(record).unwrap();
                assert_eq!(output[field], expected, "{field}");
            }
        }
    }

    #[test]
    fn a_pending_coverage_update_is_kept_and_null_when_absent() {
        let pending = serde_json::json!({"nextInvalidationRequestedAt": "2024-01-02T03:04:07Z"});
        for (response_update, expected) in [
            (Some(pending.clone()), pending),
            (Some(serde_json::Value::Null), serde_json::Value::Null),
            (None, serde_json::Value::Null),
        ] {
            let mut response = serde_json::json!({
                "deviceId": "dev_one",
                "device": {"id": "dev_one", "name": "Robot A"},
                "start": "2024-01-02T03:04:05Z",
                "end": "2024-01-02T03:04:06Z",
                "status": "imported",
            });
            if let Some(update) = response_update {
                response["device"]["pendingCoverageUpdate"] = update;
            }
            let record: Coverage = serde_json::from_value(response).unwrap();
            let output = serde_json::to_value(record).unwrap();
            assert_eq!(output["device"]["pendingCoverageUpdate"], expected);
        }
    }
}
