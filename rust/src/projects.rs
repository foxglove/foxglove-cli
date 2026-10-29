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
    #[serde(
        rename = "isInviteOnly",
        skip_serializing_if = "Option::is_none",
        default
    )]
    is_invite_only: Option<bool>,
}

impl Record for Project {
    fn headers() -> &'static [&'static str] {
        &[
            "ID",
            "Name",
            "Member Count",
            "Last Recording Uploaded At",
            "Invite Only",
        ]
    }

    fn fields(&self) -> Vec<String> {
        vec![
            self.id.clone(),
            self.name.clone(),
            self.org_member_count.to_string(),
            self.last_seen_at.clone().unwrap_or_default(),
            self.is_invite_only
                .map(|invite_only| invite_only.to_string())
                .unwrap_or_default(),
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
        None,
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::Project;
    use crate::records::Record;

    #[test]
    fn invite_only_round_trips_and_renders() {
        let record: Project = serde_json::from_value(serde_json::json!({
            "id": "prj_1",
            "name": "Fleet",
            "orgMemberCount": 3,
            "isInviteOnly": true
        }))
        .unwrap();
        assert_eq!(serde_json::to_value(&record).unwrap()["isInviteOnly"], true);
        assert_eq!(record.fields()[4], "true");

        let older: Project = serde_json::from_value(serde_json::json!({
            "id": "prj_2",
            "orgMemberCount": 1
        }))
        .unwrap();
        assert!(serde_json::to_value(&older)
            .unwrap()
            .get("isInviteOnly")
            .is_none());
        assert_eq!(older.fields()[4], "");
    }
}
