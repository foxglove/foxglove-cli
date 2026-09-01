//! Project commands.
#![allow(clippy::struct_field_names)]

use serde::{Deserialize, Serialize};

use crate::output::Format;
use crate::read_helpers::{
    finish_list, format_go_timestamp, serialize_optional_go_timestamp, Record, Runtime,
};
use crate::Outcome;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
struct Project {
    id: String,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    name: String,
    #[serde(rename = "orgMemberCount")]
    org_member_count: i64,
    #[serde(
        rename = "lastSeenAt",
        skip_serializing_if = "Option::is_none",
        serialize_with = "serialize_optional_go_timestamp",
        default
    )]
    last_seen_at: Option<String>,
}

impl Record for Project {
    fn headers() -> &'static [&'static str] {
        &["ID", "Name", "Member Count", "Last Recording Uploaded At"]
    }

    fn fields(&self) -> Vec<String> {
        vec![
            self.id.clone(),
            self.name.clone(),
            self.org_member_count.to_string(),
            self.last_seen_at
                .as_deref()
                .map(format_go_timestamp)
                .unwrap_or_default(),
        ]
    }
}
pub(crate) fn list_projects(runtime: &Runtime, format: Format) -> Outcome {
    finish_list(
        runtime,
        format,
        "Failed to list projects",
        |client| async move {
            client
                .get::<_, Vec<Project>>("/v1/projects", &Vec::<(String, String)>::new())
                .await
        },
    )
}
