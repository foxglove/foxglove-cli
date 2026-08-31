//! Extension commands.
#![allow(clippy::struct_field_names)]

use serde::{Deserialize, Serialize};

use crate::output::Format;
use crate::read_helpers::{finish_list, Record, Runtime};
use crate::Outcome;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
struct Extension {
    id: String,
    name: String,
    publisher: String,
    #[serde(rename = "displayName")]
    display_name: String,
    description: Option<String>,
    #[serde(rename = "activeVersion")]
    active_version: Option<String>,
    #[serde(rename = "sha256Sum")]
    sha256_sum: Option<String>,
}

impl Record for Extension {
    fn headers() -> &'static [&'static str] {
        &[
            "ID",
            "Name",
            "Publisher",
            "Display Name",
            "Description",
            "Active Version",
            "SHA-256 Sum",
        ]
    }

    fn fields(&self) -> Vec<String> {
        vec![
            self.id.clone(),
            self.name.clone(),
            self.publisher.clone(),
            self.display_name.clone(),
            self.description.clone().unwrap_or_default(),
            self.active_version.clone().unwrap_or_default(),
            self.sha256_sum.clone().unwrap_or_default(),
        ]
    }
}
pub(crate) fn list_extensions(runtime: &Runtime, format: Format) -> Outcome {
    finish_list(
        runtime,
        format,
        "Failed to list extensions",
        |client| async move {
            client
                .get::<_, Vec<Extension>>("/v1/extensions", &Vec::<(String, String)>::new())
                .await
        },
    )
}
