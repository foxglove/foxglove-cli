//! Topic commands.
#![allow(clippy::struct_field_names)]

use clap::ArgMatches;
use serde::{Deserialize, Serialize};

use crate::output::Format;
use crate::read_helpers::{
    add, add_str, finish_list, parse_i64, parse_timestamp, query, session_key_error, sort_query,
    value, ProjectFallback, Record, Runtime,
};
use crate::Outcome;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
struct Topic {
    encoding: String,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    schema: String,
    #[serde(rename = "schemaEncoding")]
    schema_encoding: String,
    #[serde(rename = "schemaName")]
    schema_name: String,
    topic: String,
    version: String,
}

impl Record for Topic {
    fn headers() -> &'static [&'static str] {
        &[
            "Topic",
            "Schema Name",
            "Schema Encoding",
            "Encoding",
            "Version",
            "Schema",
        ]
    }

    fn fields(&self) -> Vec<String> {
        vec![
            self.topic.clone(),
            self.schema_name.clone(),
            self.schema_encoding.clone(),
            self.encoding.clone(),
            self.version.clone(),
            self.schema.clone(),
        ]
    }
}
pub(crate) fn list_topics(runtime: &Runtime, matches: &ArgMatches, format: Format) -> Outcome {
    let source_flags = [
        "device-id",
        "device-name",
        "recording-id",
        "recording-key",
        "session-id",
        "session-key",
    ];
    if source_flags.iter().all(|id| value(matches, id).is_empty()) {
        return Outcome::failure("provide one of --device-id, --device-name, --recording-id, --recording-key, --session-id, or --session-key\n");
    }
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
    add_str(&mut query, "deviceId", &value(matches, "device-id"));
    add_str(&mut query, "deviceName", &value(matches, "device-name"));
    add_str(&mut query, "end", &end);
    add(
        &mut query,
        "includeSchemas",
        "true",
        matches.get_flag("include-schemas"),
    );
    let limit = match parse_i64(matches, "limit", 0) {
        Ok(value) => value,
        Err(error) => return Outcome::failure(format!("{error}\n")),
    };
    add(&mut query, "limit", limit, limit != 0);
    let offset = match parse_i64(matches, "offset", 0) {
        Ok(value) => value,
        Err(error) => return Outcome::failure(format!("{error}\n")),
    };
    add(&mut query, "offset", offset, offset != 0);
    add_str(&mut query, "projectId", &project_id);
    add_str(&mut query, "recordingId", &value(matches, "recording-id"));
    add_str(&mut query, "recordingKey", &value(matches, "recording-key"));
    add_str(&mut query, "sessionId", &value(matches, "session-id"));
    add_str(&mut query, "sessionKey", &value(matches, "session-key"));
    add_str(&mut query, "sortBy", &value(matches, "sort-by"));
    add_str(&mut query, "sortOrder", &value(matches, "sort-order"));
    add_str(&mut query, "start", &start);
    sort_query(&mut query);
    finish_list(
        runtime,
        format,
        "Failed to list topics",
        move |client| async move { client.get::<_, Vec<Topic>>("/v1/data/topics", &query).await },
    )
}
