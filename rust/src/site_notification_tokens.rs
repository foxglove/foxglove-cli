//! Site inbox notification token commands.

use serde::{Deserialize, Serialize};

use crate::api::encode_path_segment;
use crate::cli::{
    SiteNotificationTokenAddArgs, SiteNotificationTokenIdArgs, SiteNotificationTokenListArgs,
};
use crate::output::Format;
use crate::records::{fetch_list, format_record, null_to_default, Record};
use crate::runtime::Runtime;
use crate::Outcome;

const ENDPOINT: &str = "/v1/site-inbox-notification-tokens";

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
struct SiteNotificationToken {
    id: String,
    // Optional so that a creation response lacking it still yields the token value.
    #[serde(rename = "orgId", default, skip_serializing_if = "Option::is_none")]
    org_id: Option<String>,
    #[serde(rename = "siteId")]
    site_id: String,
    #[serde(rename = "createdAt")]
    created_at: String,
}

impl Record for SiteNotificationToken {
    fn headers() -> &'static [&'static str] {
        &["ID", "Site ID", "Created At"]
    }

    fn fields(&self) -> Vec<String> {
        vec![
            self.id.clone(),
            self.site_id.clone(),
            self.created_at.clone(),
        ]
    }
}

/// A newly created notification token, which is the only response that carries
/// the token value.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
struct CreatedSiteNotificationToken {
    #[serde(flatten)]
    notification_token: SiteNotificationToken,
    #[serde(default, deserialize_with = "null_to_default")]
    token: String,
}

impl Record for CreatedSiteNotificationToken {
    fn headers() -> &'static [&'static str] {
        &["ID", "Site ID", "Created At", "Token"]
    }

    fn fields(&self) -> Vec<String> {
        let mut fields = self.notification_token.fields();
        fields.push(self.token.clone());
        fields
    }
}

#[derive(Serialize)]
struct SiteNotificationTokenListQuery<'a> {
    #[serde(rename = "siteId", skip_serializing_if = "Option::is_none")]
    site_id: Option<&'a str>,
}

#[derive(Serialize)]
struct SiteNotificationTokenRequest<'a> {
    #[serde(rename = "siteId")]
    site_id: &'a str,
}

pub(crate) async fn list_site_notification_tokens(
    runtime: &Runtime,
    args: &SiteNotificationTokenListArgs,
    format: Format,
) -> Outcome {
    fetch_list::<SiteNotificationToken, _>(
        runtime,
        format,
        "Failed to list site notification tokens",
        ENDPOINT,
        &SiteNotificationTokenListQuery {
            site_id: args.site_id.as_deref(),
        },
        None,
    )
    .await
}

pub(crate) async fn add_site_notification_token(
    runtime: &Runtime,
    args: &SiteNotificationTokenAddArgs,
    format: Format,
) -> Outcome {
    let request = SiteNotificationTokenRequest {
        site_id: &args.site_id,
    };
    match runtime
        .client
        .post::<_, CreatedSiteNotificationToken>(ENDPOINT, &request)
        .await
    {
        Ok(created) if created.token.is_empty() => Outcome::failure(format!(
            "Site notification token {id} was created, but the response did not include its value. Delete it with `foxglove site-notification-tokens delete {id}` and try again.\n",
            id = created.notification_token.id
        )),
        Ok(created) => {
            let mut outcome = format_record(&created, format);
            if outcome.exit_code == 0 {
                outcome.stderr =
                    b"Site notification token created. Save the token now; it is not shown again.\n"
                        .to_vec();
            }
            outcome
        }
        Err(error) => Outcome::failure(format!(
            "Failed to create site notification token: {error}\n"
        )),
    }
}

pub(crate) async fn delete_site_notification_token(
    runtime: &Runtime,
    args: &SiteNotificationTokenIdArgs,
) -> Outcome {
    match runtime
        .client
        .delete(&format!(
            "{ENDPOINT}/{}",
            encode_path_segment(&args.token_id)
        ))
        .await
    {
        Ok(()) => Outcome::notice("Site notification token deleted\n"),
        Err(error) if error.is_not_found_for("inbox notification token") => Outcome::failure(
            format!("Site notification token not found: {}\n", args.token_id),
        ),
        Err(error) => Outcome::failure(format!(
            "Failed to delete site notification token: {error}\n"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::{CreatedSiteNotificationToken, SiteNotificationToken};
    use crate::records::Record;

    #[test]
    fn the_org_id_is_kept_in_json_output_but_not_in_the_table() {
        let token: SiteNotificationToken = serde_json::from_value(serde_json::json!({"id":"sint_1","orgId":"org_1","siteId":"site_1","createdAt":"2026-01-02T03:04:05Z"}))
            .unwrap();
        assert_eq!(serde_json::to_value(&token).unwrap()["orgId"], "org_1");
        assert_eq!(token.fields(), ["sint_1", "site_1", "2026-01-02T03:04:05Z"]);

        let created: CreatedSiteNotificationToken = serde_json::from_value(serde_json::json!({"id":"sint_2","orgId":"org_1","siteId":"site_1","createdAt":"2026-01-02T03:04:05Z","token":"fox_snt_fake"}))
            .unwrap();
        let output = serde_json::to_value(&created).unwrap();
        assert_eq!(output["orgId"], "org_1");
        assert_eq!(output["token"], "fox_snt_fake");
    }

    #[test]
    fn a_response_without_an_org_id_omits_it_from_json_output() {
        let created: CreatedSiteNotificationToken = serde_json::from_value(serde_json::json!({"id":"sint_2","siteId":"site_1","createdAt":"2026-01-02T03:04:05Z","token":"fox_snt_fake"}))
            .unwrap();
        let output = serde_json::to_value(&created).unwrap();
        assert!(output.get("orgId").is_none());
        assert_eq!(output["token"], "fox_snt_fake");
    }
}
