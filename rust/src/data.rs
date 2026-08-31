//! Data commands.
#![allow(clippy::struct_field_names)]

use clap::ArgMatches;
use serde::{Deserialize, Serialize};

use crate::output::Format;
use crate::read_helpers::{
    add, add_str, finish_list, parse_i64, parse_timestamp, query, session_key_error, sort_query,
    value, DeviceSummary, ProjectFallback, Record, Runtime,
};
use crate::Outcome;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
struct Import {
    id: String,
    #[serde(rename = "deviceId")]
    device_id: String,
    filename: String,
    #[serde(rename = "importTime")]
    import_time: String,
    start: String,
    end: String,
    #[serde(rename = "inputType")]
    input_type: String,
    #[serde(rename = "outputType")]
    output_type: String,
    #[serde(rename = "inputSize")]
    input_size: i64,
    #[serde(rename = "totalOutputSize")]
    total_output_size: i64,
}

impl Record for Import {
    fn headers() -> &'static [&'static str] {
        &[
            "Import ID",
            "Device ID",
            "Filename",
            "Import Time",
            "Start",
            "End",
            "Input Type",
            "Output Type",
            "Input Size",
            "Total Output Size",
        ]
    }

    fn fields(&self) -> Vec<String> {
        vec![
            self.id.clone(),
            self.device_id.clone(),
            self.filename.clone(),
            self.import_time.clone(),
            self.start.clone(),
            self.end.clone(),
            self.input_type.clone(),
            self.output_type.clone(),
            self.input_size.to_string(),
            self.total_output_size.to_string(),
        ]
    }
}
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
pub(crate) fn list_imports(runtime: &Runtime, matches: &ArgMatches, format: Format) -> Outcome {
    let mut query = query();
    let start = match parse_timestamp(&value(matches, "start"), "start") {
        Ok(value) => value,
        Err(error) => return Outcome::failure(format!("{error}\n")),
    };
    let end = match parse_timestamp(&value(matches, "end"), "end") {
        Ok(value) => value,
        Err(error) => return Outcome::failure(format!("{error}\n")),
    };
    let data_start = match parse_timestamp(&value(matches, "data-start"), "data start") {
        Ok(value) => value,
        Err(error) => return Outcome::failure(format!("{error}\n")),
    };
    let data_end = match parse_timestamp(&value(matches, "data-end"), "data end") {
        Ok(value) => value,
        Err(error) => return Outcome::failure(format!("{error}\n")),
    };
    add_str(&mut query, "dataEnd", &data_end);
    add_str(&mut query, "dataStart", &data_start);
    add_str(&mut query, "deviceId", &value(matches, "device-id"));
    add(
        &mut query,
        "includeDeleted",
        "true",
        matches.get_flag("include-deleted"),
    );
    add_str(&mut query, "end", &end);
    add_str(&mut query, "start", &start);
    sort_query(&mut query);
    finish_list(
        runtime,
        format,
        "Failed to list imports",
        move |client| async move {
            client
                .get::<_, Vec<Import>>("/v1/data/imports", &query)
                .await
        },
    )
}

pub(crate) fn list_coverage(runtime: &Runtime, matches: &ArgMatches, format: Format) -> Outcome {
    let project_id = value(matches, "project-id").or_project(&runtime.project_id);
    if let Some(error) = session_key_error(matches, &project_id) {
        return Outcome::failure(error);
    }
    let start = match parse_timestamp(&value(matches, "start"), "start") {
        Ok(value) => value,
        Err(error) => return Outcome::failure(format!("{error}\n")),
    };
    let end = match parse_timestamp(&value(matches, "end"), "end") {
        Ok(value) => value,
        Err(error) => return Outcome::failure(format!("{error}\n")),
    };
    let mut query = query();
    add_str(&mut query, "device.id", &value(matches, "device-id"));
    add_str(&mut query, "device.name", &value(matches, "device-name"));
    add_str(&mut query, "end", &end);
    add(
        &mut query,
        "includeEdgeRecordings",
        "true",
        matches.get_flag("include-edge-recordings"),
    );
    add_str(&mut query, "projectId", &project_id);
    add_str(&mut query, "recordingId", &value(matches, "recording-id"));
    add_str(&mut query, "sessionId", &value(matches, "session-id"));
    add_str(&mut query, "sessionKey", &value(matches, "session-key"));
    add_str(&mut query, "start", &start);
    let tolerance = match parse_i64(matches, "tolerance", 0) {
        Ok(value) => value,
        Err(error) => return Outcome::failure(format!("{error}\n")),
    };
    add(&mut query, "tolerance", tolerance, tolerance != 0);
    sort_query(&mut query);
    finish_list(
        runtime,
        format,
        "Failed to list coverage",
        move |client| async move {
            client
                .get::<_, Vec<Coverage>>("/v1/data/coverage", &query)
                .await
        },
    )
}
