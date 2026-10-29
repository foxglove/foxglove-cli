//! Site token commands.

use serde::{Deserialize, Serialize};

use crate::api::encode_path_segment;
use crate::cli::{SiteTokenAddArgs, SiteTokenIdArgs, SiteTokenListArgs};
use crate::output::Format;
use crate::records::{fetch_list, format_record, null_to_default, Record};
use crate::runtime::Runtime;
use crate::Outcome;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
struct SiteToken {
    id: String,
    #[serde(rename = "siteId")]
    site_id: String,
    #[serde(rename = "createdAt")]
    created_at: String,
}

impl Record for SiteToken {
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

/// A newly created site token, which is the only response that carries the
/// token value.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
struct CreatedSiteToken {
    #[serde(flatten)]
    site_token: SiteToken,
    #[serde(default, deserialize_with = "null_to_default")]
    token: String,
}

impl Record for CreatedSiteToken {
    fn headers() -> &'static [&'static str] {
        &["ID", "Site ID", "Created At", "Token"]
    }

    fn fields(&self) -> Vec<String> {
        let mut fields = self.site_token.fields();
        fields.push(self.token.clone());
        fields
    }
}

#[derive(Serialize)]
struct SiteTokenListQuery<'a> {
    #[serde(rename = "siteId", skip_serializing_if = "Option::is_none")]
    site_id: Option<&'a str>,
}

#[derive(Serialize)]
struct SiteTokenRequest<'a> {
    #[serde(rename = "siteId")]
    site_id: &'a str,
}

pub(crate) async fn list_site_tokens(
    runtime: &Runtime,
    args: &SiteTokenListArgs,
    format: Format,
) -> Outcome {
    fetch_list::<SiteToken, _>(
        runtime,
        format,
        "Failed to list site tokens",
        "/v1/site-tokens",
        &SiteTokenListQuery {
            site_id: args.site_id.as_deref(),
        },
        None,
    )
    .await
}

pub(crate) async fn add_site_token(
    runtime: &Runtime,
    args: &SiteTokenAddArgs,
    format: Format,
) -> Outcome {
    let request = SiteTokenRequest {
        site_id: &args.site_id,
    };
    match runtime
        .client
        .post::<_, CreatedSiteToken>("/v1/site-tokens", &request)
        .await
    {
        Ok(created) if created.token.is_empty() => Outcome::failure(format!(
            "Site token {id} was created, but the response did not include its value. Delete it with `foxglove site-tokens delete {id}` and try again.\n",
            id = created.site_token.id
        )),
        Ok(created) => {
            let mut outcome = format_record(&created, format);
            if outcome.exit_code == 0 {
                outcome.stderr =
                    b"Site token created. Save the token now; it is not shown again.\n".to_vec();
            }
            outcome
        }
        Err(error) => Outcome::failure(format!("Failed to create site token: {error}\n")),
    }
}

pub(crate) async fn delete_site_token(runtime: &Runtime, args: &SiteTokenIdArgs) -> Outcome {
    match runtime
        .client
        .delete(&format!(
            "/v1/site-tokens/{}",
            encode_path_segment(&args.site_token_id)
        ))
        .await
    {
        Ok(()) => Outcome::notice("Site token deleted\n"),
        Err(error) if error.is_not_found_for("site token") => {
            Outcome::failure(format!("Site token not found: {}\n", args.site_token_id))
        }
        Err(error) => Outcome::failure(format!("Failed to delete site token: {error}\n")),
    }
}
