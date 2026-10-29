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
