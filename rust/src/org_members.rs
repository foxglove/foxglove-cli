//! Organization member commands.

use serde::{Deserialize, Serialize};

use crate::cli::OrgMemberListArgs;
use crate::output::Format;
use crate::records::{fetch_list, is_zero, Record, DEFAULT_LIST_LIMIT};
use crate::runtime::Runtime;
use crate::Outcome;

/// An organization member. `role` and `status` stay strings so values added
/// to the API later still decode.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct OrgMember {
    id: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    avatar_image_url: Option<String>,
    created_at: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    display_name: Option<String>,
    email: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    last_seen_at: Option<String>,
    org_id: String,
    role: String,
    updated_at: String,
    status: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    project_ids: Option<Vec<String>>,
}

impl Record for OrgMember {
    fn headers() -> &'static [&'static str] {
        &[
            "ID",
            "Email",
            "Display Name",
            "Role",
            "Status",
            "Project IDs",
            "Last Seen At",
            "Created At",
            "Updated At",
        ]
    }

    fn fields(&self) -> Vec<String> {
        vec![
            self.id.clone(),
            self.email.clone(),
            self.display_name.clone().unwrap_or_default(),
            self.role.clone(),
            self.status.clone(),
            self.project_ids
                .as_deref()
                .map(|ids| ids.join(","))
                .unwrap_or_default(),
            self.last_seen_at.clone().unwrap_or_default(),
            self.created_at.clone(),
            self.updated_at.clone(),
        ]
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct OrgMemberListQuery {
    limit: i64,
    #[serde(skip_serializing_if = "is_zero")]
    offset: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    project_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    search: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    sort_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    sort_order: Option<String>,
}

pub(crate) async fn list_org_members(
    runtime: &Runtime,
    args: &OrgMemberListArgs,
    format: Format,
) -> Outcome {
    let query = OrgMemberListQuery {
        limit: args.limit.unwrap_or(DEFAULT_LIST_LIMIT),
        offset: args.offset.unwrap_or_default(),
        // Members belong to the organization, so the default project does not
        // narrow this list; only an explicit --project-id filters it.
        project_id: args.project_id.clone(),
        search: args.search.clone(),
        role: args.role.clone(),
        status: args.status.clone(),
        sort_by: args.sort_by.clone(),
        sort_order: args.sort_order.clone(),
    };
    fetch_list::<OrgMember, _>(
        runtime,
        format,
        "Failed to list organization members",
        "/v1/org-members",
        &query,
        Some(query.limit),
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::OrgMember;
    use crate::records::Record;

    #[test]
    fn org_members_round_trip_and_render() {
        let full = serde_json::json!({
            "id": "om_1",
            "avatarImageUrl": "https://example.com/a.png",
            "createdAt": "2024-01-02T03:04:05.000Z",
            "displayName": "Ada",
            "email": "ada@example.com",
            "lastSeenAt": "2024-02-03T04:05:06.000Z",
            "orgId": "org_1",
            "role": "admin",
            "updatedAt": "2024-01-03T03:04:05.000Z",
            "status": "active",
            "projectIds": ["prj_1", "prj_2"]
        });
        let member: OrgMember = serde_json::from_value(full.clone()).unwrap();
        assert_eq!(serde_json::to_value(&member).unwrap(), full);
        assert_eq!(
            member.fields(),
            [
                "om_1",
                "ada@example.com",
                "Ada",
                "admin",
                "active",
                "prj_1,prj_2",
                "2024-02-03T04:05:06.000Z",
                "2024-01-02T03:04:05.000Z",
                "2024-01-03T03:04:05.000Z",
            ]
        );

        let minimal = serde_json::json!({
            "id": "om_2",
            "createdAt": "2024-01-02T03:04:05.000Z",
            "email": "new@example.com",
            "orgId": "org_1",
            "role": "owner",
            "updatedAt": "2024-01-02T03:04:05.000Z",
            "status": "suspended"
        });
        let member: OrgMember = serde_json::from_value(minimal.clone()).unwrap();
        assert_eq!(serde_json::to_value(&member).unwrap(), minimal);
        assert_eq!(
            member.fields(),
            [
                "om_2",
                "new@example.com",
                "",
                "owner",
                "suspended",
                "",
                "",
                "2024-01-02T03:04:05.000Z",
                "2024-01-02T03:04:05.000Z",
            ]
        );

        let empty: OrgMember = serde_json::from_value(serde_json::json!({
            "id": "om_3",
            "createdAt": "2024-01-02T03:04:05.000Z",
            "email": "solo@example.com",
            "orgId": "org_1",
            "role": "user",
            "updatedAt": "2024-01-02T03:04:05.000Z",
            "status": "invited",
            "displayName": null,
            "lastSeenAt": null,
            "projectIds": []
        }))
        .unwrap();
        let value = serde_json::to_value(&empty).unwrap();
        assert_eq!(value["projectIds"], serde_json::json!([]));
        assert!(value.get("displayName").is_none());
    }
}
