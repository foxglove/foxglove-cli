//! Topic commands.

use serde::{Deserialize, Serialize};

use crate::cli::TopicListArgs;
use crate::output::Format;
use crate::records::{
    fetch_list, is_false, is_zero, parse_timestamp, ProjectFallback, Record, DEFAULT_LIST_LIMIT,
};
use crate::runtime::Runtime;
use crate::Outcome;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[allow(clippy::struct_field_names)]
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

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TopicListQuery {
    #[serde(skip_serializing_if = "String::is_empty")]
    device_id: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    device_name: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    end: String,
    #[serde(skip_serializing_if = "is_false")]
    include_schemas: bool,
    limit: i64,
    #[serde(skip_serializing_if = "is_zero")]
    offset: i64,
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
    sort_by: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    sort_order: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    start: String,
}

pub(crate) async fn list_topics(
    runtime: &Runtime,
    args: &TopicListArgs,
    format: Format,
) -> Outcome {
    if [
        &args.device_id,
        &args.device_name,
        &args.recording_id,
        &args.recording_key,
        &args.session_id,
        &args.session_key,
    ]
    .into_iter()
    .all(|value| value.as_deref().unwrap_or_default().is_empty())
    {
        return Outcome::failure("provide one of --device-id, --device-name, --recording-id, --recording-key, --session-id, or --session-key\n");
    }
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
    let limit = args.limit.unwrap_or(DEFAULT_LIST_LIMIT);
    let offset = args.offset.unwrap_or_default();
    let query = TopicListQuery {
        device_id: args.device_id.clone().unwrap_or_default(),
        device_name: args.device_name.clone().unwrap_or_default(),
        end,
        include_schemas: args.include_schemas,
        limit,
        offset,
        project_id,
        recording_id: args.recording_id.clone().unwrap_or_default(),
        recording_key: args.recording_key.clone().unwrap_or_default(),
        session_id: args.session_id.clone().unwrap_or_default(),
        session_key,
        sort_by: args.sort_by.clone().unwrap_or_default(),
        sort_order: args.sort_order.clone().unwrap_or_default(),
        start,
    };
    fetch_list::<Topic, _>(
        runtime,
        format,
        "Failed to list topics",
        "/v1/data/topics",
        &query,
    )
    .await
}
