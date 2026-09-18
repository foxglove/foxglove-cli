//! Public site list, lookup, and mutation commands.

use serde::{Deserialize, Serialize};
use serde_json::Number;

use crate::api::encode_path_segment;
use crate::cli::{SiteAddArgs, SiteEditArgs};
use crate::output::{self, Format};
use crate::records::{fetch_list, format_output, Record};
use crate::runtime::Runtime;
use crate::Outcome;

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct Site {
    id: String,
    name: String,
    #[serde(rename = "type")]
    kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
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
            self.url.clone().unwrap_or_default(),
            self.retain_recordings_seconds
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_default(),
        ]
    }
}

pub(crate) fn parse_retention(raw: &str) -> Result<Number, String> {
    let number = raw
        .parse::<Number>()
        .map_err(|_| "expected a finite nonnegative number of seconds".to_owned())?;
    if number.as_f64().is_some_and(|value| value >= 0.0) {
        Ok(number)
    } else {
        Err("expected a finite nonnegative number of seconds".to_owned())
    }
}

pub(crate) async fn list_sites(runtime: &Runtime, format: Format) -> Outcome {
    fetch_list::<Site, _>(runtime, format, "Failed to list sites", "/v1/sites", &()).await
}

pub(crate) async fn get_site(runtime: &Runtime, id: &str, format: Format) -> Outcome {
    match runtime
        .client
        .get::<_, Site>(&format!("/v1/sites/{}", encode_path_segment(id)), &())
        .await
    {
        Ok(site) if format == Format::Json => {
            let mut stdout = Vec::new();
            match output::render_json(&mut stdout, &site) {
                Ok(()) => Outcome::success(stdout),
                Err(error) => Outcome::failure(format!("failed to render output: {error}\n")),
            }
        }
        Ok(site) => format_output(&[site], format),
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
    retain_recordings_seconds: Option<&'a Number>,
}

pub(crate) async fn add_site(runtime: &Runtime, args: &SiteAddArgs) -> Outcome {
    if args.retain_recordings_seconds.is_some() && args.site_type != "edge" {
        return Outcome::failure("--retain-recordings-seconds is only available for Edge Sites\n");
    }
    let request = CreateSiteRequest {
        name: &args.name,
        site_type: &args.site_type,
        retain_recordings_seconds: args.retain_recordings_seconds.as_ref(),
    };
    match runtime.client.post::<_, Site>("/v1/sites", &request).await {
        Ok(site) => mutation_outcome("created", &site.id),
        Err(error) => Outcome::failure(format!("Failed to create site: {error}\n")),
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct EditSiteRequest<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    retain_recordings_seconds: Option<&'a Number>,
    #[serde(skip_serializing_if = "Option::is_none")]
    url: Option<&'a str>,
}

pub(crate) async fn edit_site(runtime: &Runtime, args: &SiteEditArgs) -> Outcome {
    if args.name.is_none() && args.retain_recordings_seconds.is_none() && args.url.is_none() {
        return Outcome::failure("Nothing to update\n");
    }
    let request = EditSiteRequest {
        name: args.name.as_deref(),
        retain_recordings_seconds: args.retain_recordings_seconds.as_ref(),
        url: args.url.as_deref(),
    };
    match runtime
        .client
        .patch::<_, _, Site>(
            &format!("/v1/sites/{}", encode_path_segment(&args.site.id)),
            &(),
            &request,
        )
        .await
    {
        Ok(site) => mutation_outcome("updated", &site.id),
        Err(error) => Outcome::failure(format!("Failed to edit site: {error}\n")),
    }
}

pub(crate) async fn delete_site(runtime: &Runtime, id: &str) -> Outcome {
    match runtime
        .client
        .delete(&format!("/v1/sites/{}", encode_path_segment(id)))
        .await
    {
        Ok(()) => mutation_outcome("deleted", id),
        Err(error) => Outcome::failure(format!("Failed to delete site: {error}\n")),
    }
}

fn mutation_outcome(action: &str, id: &str) -> Outcome {
    Outcome {
        stderr: format!("Site {action}: {id}\n").into_bytes(),
        ..Outcome::default()
    }
}
