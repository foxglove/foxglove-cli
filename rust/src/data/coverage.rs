//! Coverage listing.

use serde::{Deserialize, Serialize};

use crate::cli::CoverageListArgs;
use crate::output::Format;
use crate::records::{
    fetch_list, is_false, is_zero, parse_timestamp, DeviceSummary, ProjectFallback, Record,
};
use crate::runtime::Runtime;
use crate::Outcome;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
struct Coverage {
    #[serde(rename = "deviceId")]
    device_id: String,
    #[serde(default)]
    device: DeviceSummary,
    start: String,
    end: String,
    status: String,
}

impl Record for Coverage {
    fn headers() -> &'static [&'static str] {
        &["Device ID", "Device Name", "Start", "End", "Status"]
    }

    fn fields(&self) -> Vec<String> {
        vec![
            self.device.id.clone(),
            self.device.name.clone(),
            self.start.clone(),
            self.end.clone(),
            self.status.clone(),
        ]
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CoverageListQuery {
    #[serde(rename = "device.id", skip_serializing_if = "String::is_empty")]
    device_id: String,
    #[serde(rename = "device.name", skip_serializing_if = "String::is_empty")]
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
    session_id: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    session_key: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    start: String,
    #[serde(skip_serializing_if = "is_zero")]
    tolerance: i64,
}

pub(crate) async fn list(runtime: &Runtime, args: &CoverageListArgs, format: Format) -> Outcome {
    let project_id = args
        .project_id
        .clone()
        .unwrap_or_default()
        .or_project(&runtime.project_id);
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
    )
    .await
}
