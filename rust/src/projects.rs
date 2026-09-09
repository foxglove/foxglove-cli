//! Project commands.

use serde::{Deserialize, Serialize};

use crate::output::Format;
use crate::records::{fetch_list, Record};
use crate::runtime::Runtime;
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
            self.last_seen_at.clone().unwrap_or_default(),
        ]
    }
}

pub(crate) async fn list_projects(runtime: &Runtime, format: Format) -> Outcome {
    fetch_list::<Project, _>(
        runtime,
        format,
        "Failed to list projects",
        "/v1/projects",
        &(),
    )
    .await
}
