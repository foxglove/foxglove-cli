//! Project commands.

use std::collections::HashSet;
use std::fmt::Write as _;

use serde::{Deserialize, Serialize};

use crate::api::encode_path_segment;
use crate::cli::ProjectMemberMutationArgs;
use crate::output::Format;
use crate::records::{fetch_list, plural, Record};
use crate::runtime::Runtime;
use crate::Outcome;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
struct Project {
    id: String,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    name: String,
    #[serde(
        rename = "orgMemberCount",
        skip_serializing_if = "Option::is_none",
        default
    )]
    org_member_count: Option<i64>,
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
            self.org_member_count
                .map(|count| count.to_string())
                .unwrap_or_default(),
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

/// The API accepts at most this many member IDs per change.
const MAX_MEMBER_CHANGES: usize = 50;

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum MemberChange {
    Add,
    Remove,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProjectMembersRequest<'a> {
    action: MemberChange,
    member_ids: &'a [String],
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProjectMembersResponse {
    affected_member_ids: Vec<String>,
}

/// Summarize a change, naming the members it affected. The API omits members
/// already in the requested state; repeated IDs count once.
fn member_change_summary(
    change: MemberChange,
    project_id: &str,
    requested: &[String],
    affected: &[String],
) -> String {
    let (verb, preposition) = match change {
        MemberChange::Add => ("Added", "to"),
        MemberChange::Remove => ("Removed", "from"),
    };
    let mut summary = format!(
        "{verb} {} {preposition} project {project_id}",
        plural(affected.len(), "member")
    );
    if !affected.is_empty() {
        let _ = write!(summary, ": {}", affected.join(", "));
    }
    let unchanged = requested
        .iter()
        .collect::<HashSet<_>>()
        .len()
        .saturating_sub(affected.len());
    if unchanged > 0 {
        let _ = write!(summary, " ({unchanged} unchanged)");
    }
    summary.push('\n');
    summary
}

pub(crate) async fn change_project_members(
    runtime: &Runtime,
    args: &ProjectMemberMutationArgs,
    change: MemberChange,
) -> Outcome {
    if args.member_ids.len() > MAX_MEMBER_CHANGES {
        return Outcome::failure(format!(
            "At most {MAX_MEMBER_CHANGES} member IDs can be changed at once (got {}). Split the IDs into more than one command.\n",
            args.member_ids.len()
        ));
    }
    let request = ProjectMembersRequest {
        action: change,
        member_ids: &args.member_ids,
    };
    match runtime
        .client
        .patch::<_, _, ProjectMembersResponse>(
            &format!(
                "/v1/projects/{}/members",
                encode_path_segment(&args.project_id)
            ),
            &(),
            &request,
        )
        .await
    {
        Ok(response) => Outcome::notice(member_change_summary(
            change,
            &args.project_id,
            &args.member_ids,
            &response.affected_member_ids,
        )),
        Err(error) => Outcome::failure(format!(
            "Failed to {} project members: {error}\n",
            match change {
                MemberChange::Add => "add",
                MemberChange::Remove => "remove",
            }
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::{member_change_summary, MemberChange, Project, ProjectMembersRequest};
    use crate::records::Record;

    #[test]
    fn member_changes_send_the_action_and_summarize_unchanged_members() {
        let ids = ["mem_1", "mem_2", "mem_1", "mem_3"].map(str::to_owned);
        let request = ProjectMembersRequest {
            action: MemberChange::Remove,
            member_ids: &ids,
        };
        assert_eq!(
            serde_json::to_value(&request).unwrap(),
            serde_json::json!({"action": "remove", "memberIds": ["mem_1", "mem_2", "mem_1", "mem_3"]})
        );
        assert_eq!(
            member_change_summary(MemberChange::Add, "prj_1", &ids, &ids[..2]),
            "Added 2 members to project prj_1: mem_1, mem_2 (1 unchanged)\n"
        );
        assert_eq!(
            member_change_summary(MemberChange::Remove, "prj_1", &ids[..1], &[]),
            "Removed 0 members from project prj_1 (1 unchanged)\n"
        );
    }

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

        let without_count: Project = serde_json::from_value(serde_json::json!({
            "id": "prj_3",
            "isInviteOnly": false
        }))
        .unwrap();
        assert_eq!(without_count.fields()[2], "");
        assert!(serde_json::to_value(&without_count)
            .unwrap()
            .get("orgMemberCount")
            .is_none());
    }
}
