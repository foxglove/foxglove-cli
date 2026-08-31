//! Device commands.
#![allow(clippy::struct_field_names)]

use clap::ArgMatches;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::output::Format;
use crate::read_helpers::{
    add_str, compact_json, finish_list, query, sort_query, value, ProjectFallback, Record, Runtime,
};
use crate::Outcome;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
struct Device {
    id: String,
    name: String,
    #[serde(default)]
    properties: Value,
    #[serde(rename = "createdAt")]
    created_at: String,
    #[serde(rename = "updatedAt")]
    updated_at: String,
    #[serde(rename = "projectId")]
    project_id: String,
}

impl Record for Device {
    fn headers() -> &'static [&'static str] {
        &[
            "ID",
            "Name",
            "Custom Properties",
            "Created At",
            "Updated At",
            "Project ID",
        ]
    }

    fn fields(&self) -> Vec<String> {
        vec![
            self.id.clone(),
            self.name.clone(),
            compact_json(&self.properties),
            self.created_at.clone(),
            self.updated_at.clone(),
            self.project_id.clone(),
        ]
    }
}
pub(crate) fn list_devices(runtime: &Runtime, matches: &ArgMatches, format: Format) -> Outcome {
    let mut query = query();
    add_str(
        &mut query,
        "projectId",
        &value(matches, "project-id").or_project(&runtime.project_id),
    );
    sort_query(&mut query);
    finish_list(
        runtime,
        format,
        "Failed to list devices",
        move |client| async move { client.get::<_, Vec<Device>>("/v1/devices", &query).await },
    )
}
