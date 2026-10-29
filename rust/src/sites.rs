//! Site commands.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Number, Value};

use crate::api::encode_path_segment;
use crate::cli::{SiteAddArgs, SiteEditArgs, SiteGetArgs, SiteIdArgs, SitePolicySetArgs};
use crate::output::{self, Format};
use crate::records::{fetch_list, format_record, Record};
use crate::runtime::Runtime;
use crate::Outcome;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
struct Site {
    id: String,
    name: String,
    #[serde(rename = "type")]
    kind: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    url: String,
    #[serde(
        rename = "retainRecordingsSeconds",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    retain_recordings_seconds: Option<Number>,
}

impl Record for Site {
    fn headers() -> &'static [&'static str] {
        &["ID", "Name", "Type", "URL", "Retain Recordings Seconds"]
    }

    fn fields(&self) -> Vec<String> {
        vec![
            self.id.clone(),
            self.name.clone(),
            self.kind.clone(),
            self.url.clone(),
            self.retain_recordings_seconds
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_default(),
        ]
    }
}

fn site_endpoint(id: &str) -> String {
    format!("/v1/sites/{}", encode_path_segment(id))
}

fn site_not_found(id: &str) -> Outcome {
    Outcome::failure(format!("Site not found: {id}\n"))
}

pub(crate) async fn list_sites(runtime: &Runtime, format: Format) -> Outcome {
    fetch_list::<Site, _>(
        runtime,
        format,
        "Failed to list sites",
        "/v1/sites",
        &(),
        None,
    )
    .await
}

pub(crate) async fn get_site(runtime: &Runtime, args: &SiteGetArgs, format: Format) -> Outcome {
    match runtime
        .client
        .get::<_, Site>(&site_endpoint(&args.site_id), &())
        .await
    {
        Ok(site) => format_record(&site, format),
        Err(error) if error.is_not_found_for("site") => site_not_found(&args.site_id),
        Err(error) => Outcome::failure(format!("Failed to get site: {error}\n")),
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CreateSiteRequest<'a> {
    name: &'a str,
    #[serde(rename = "type")]
    site_type: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    retain_recordings_seconds: Option<u64>,
}

pub(crate) async fn add_site(runtime: &Runtime, args: &SiteAddArgs) -> Outcome {
    let request = CreateSiteRequest {
        name: &args.name,
        site_type: &args.site_type,
        retain_recordings_seconds: args.retain_recordings_seconds,
    };
    match runtime.client.post::<_, Site>("/v1/sites", &request).await {
        Ok(site) => Outcome::notice(format!("Site created: {}\n", site.id)),
        Err(error) => Outcome::failure(format!("Failed to create site: {error}\n")),
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct EditSiteRequest<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    retain_recordings_seconds: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    url: Option<&'a str>,
}

pub(crate) async fn edit_site(runtime: &Runtime, args: &SiteEditArgs) -> Outcome {
    if args.name.is_none() && args.retain_recordings_seconds.is_none() && args.url.is_none() {
        return Outcome::failure("Nothing to update\n");
    }
    let request = EditSiteRequest {
        name: args.name.as_deref(),
        retain_recordings_seconds: args.retain_recordings_seconds,
        url: args.url.as_deref(),
    };
    match runtime
        .client
        .patch::<_, _, Value>(&site_endpoint(&args.site_id), &(), &request)
        .await
    {
        Ok(_) => Outcome::notice(format!("Site updated: {}\n", args.site_id)),
        Err(error) if error.is_not_found_for("site") => site_not_found(&args.site_id),
        Err(error) => Outcome::failure(format!("Failed to edit site: {error}\n")),
    }
}

pub(crate) async fn delete_site(runtime: &Runtime, args: &SiteIdArgs) -> Outcome {
    match runtime.client.delete(&site_endpoint(&args.site_id)).await {
        Ok(()) => Outcome::notice(format!("Site deleted: {}\n", args.site_id)),
        Err(error) if error.is_not_found_for("site") => site_not_found(&args.site_id),
        Err(error) => Outcome::failure(format!("Failed to delete site: {error}\n")),
    }
}

/// One saved revision of a site's ingest path policy. The policy document is
/// passed through unchanged so new rule fields survive a round trip.
#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct PolicyRevision {
    revision: i64,
    created_at: String,
    policy: Value,
}

fn policy_endpoint(id: &str) -> String {
    format!("{}/ingest-path-policy", site_endpoint(id))
}

fn json_outcome(value: &impl Serialize) -> Outcome {
    let mut stdout = Vec::new();
    match output::render_json(&mut stdout, value) {
        Ok(()) => Outcome::success(stdout),
        Err(error) => Outcome::failure(format!("failed to render output: {error}\n")),
    }
}

pub(crate) async fn get_policy(runtime: &Runtime, args: &SiteIdArgs) -> Outcome {
    match runtime
        .client
        .get_optional::<_, PolicyRevision>(&policy_endpoint(&args.site_id), &())
        .await
    {
        Ok(Some(revision)) => json_outcome(&revision),
        Ok(None) => Outcome::notice(format!(
            "Site {} has no saved ingest path policy\n",
            args.site_id
        )),
        Err(error) if error.is_not_found_for("site") => site_not_found(&args.site_id),
        Err(error) => Outcome::failure(format!("Failed to get ingest path policy: {error}\n")),
    }
}

/// Read a policy document, `{"schemaVersion": 1, "rules": [...]}`, and attach
/// the optional previous revision.
fn policy_request(
    raw: &[u8],
    previous_revision: Option<u64>,
) -> Result<Map<String, Value>, String> {
    let mut policy = match serde_json::from_slice::<Value>(raw) {
        Ok(Value::Object(policy)) => policy,
        Ok(_) => {
            return Err(
                "Invalid policy JSON: expected an object with schemaVersion and a rules array"
                    .to_owned(),
            )
        }
        Err(error) => return Err(format!("Invalid policy JSON: {error}")),
    };
    if !policy.contains_key("schemaVersion") || !policy.get("rules").is_some_and(Value::is_array) {
        return Err(
            "Invalid policy JSON: expected an object with schemaVersion and a rules array"
                .to_owned(),
        );
    }
    if let Some(previous_revision) = previous_revision {
        if policy.contains_key("previousRevision") {
            return Err(
                "Set previousRevision in the file or with --previous-revision, not both".to_owned(),
            );
        }
        policy.insert("previousRevision".to_owned(), previous_revision.into());
    }
    Ok(policy)
}

pub(crate) async fn set_policy(runtime: &Runtime, args: &SitePolicySetArgs) -> Outcome {
    let raw = match tokio::fs::read(&args.file).await {
        Ok(raw) => raw,
        Err(error) => {
            return Outcome::failure(format!("Failed to read {}: {error}\n", args.file.display()))
        }
    };
    let request = match policy_request(&raw, args.previous_revision) {
        Ok(request) => request,
        Err(error) => return Outcome::failure(format!("{error}\n")),
    };
    match runtime
        .client
        .post::<_, PolicyRevision>(&policy_endpoint(&args.site_id), &request)
        .await
    {
        Ok(revision) => Outcome::notice(format!(
            "Ingest path policy for site {} is at revision {}\n",
            args.site_id, revision.revision
        )),
        Err(error) if error.is_not_found_for("site") => site_not_found(&args.site_id),
        Err(error) => Outcome::failure(format!("Failed to save ingest path policy: {error}\n")),
    }
}

#[cfg(test)]
mod tests {
    use super::{policy_request, Site};
    use crate::records::Record;

    #[test]
    fn sites_render_optional_fields_only_when_present() {
        let site: Site = serde_json::from_value(serde_json::json!({
            "id": "site_1",
            "name": "Edge",
            "type": "edge",
            "retainRecordingsSeconds": 604_800,
        }))
        .unwrap();
        assert_eq!(site.fields(), ["site_1", "Edge", "edge", "", "604800"]);
        let json = serde_json::to_value(&site).unwrap();
        assert!(json.get("url").is_none(), "{json}");
        assert_eq!(json["retainRecordingsSeconds"], 604_800);
    }

    #[test]
    fn invalid_policy_documents_are_rejected_before_sending() {
        for (raw, previous, expected) in [
            ("[", None, "Invalid policy JSON:"),
            ("[]", None, "expected an object"),
            (r#"{"rules":[]}"#, None, "expected an object"),
            (
                r#"{"schemaVersion":1,"rules":{}}"#,
                None,
                "expected an object",
            ),
            (
                r#"{"schemaVersion":1,"rules":[],"previousRevision":2}"#,
                Some(3),
                "not both",
            ),
        ] {
            assert!(
                policy_request(raw.as_bytes(), previous)
                    .unwrap_err()
                    .contains(expected),
                "{raw}"
            );
        }
        assert_eq!(
            serde_json::Value::Object(
                policy_request(br#"{"schemaVersion":1,"rules":[]}"#, Some(0)).unwrap()
            ),
            serde_json::json!({"schemaVersion": 1, "rules": [], "previousRevision": 0})
        );
    }
}
