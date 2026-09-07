//! Event type commands.
#![allow(clippy::struct_field_names)]

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::output::Format;
use crate::read_helpers::{compact_json, finish_list, Record, Runtime};
use crate::Outcome;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
struct EventTypeProperty {
    key: String,
    label: String,
    required: bool,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    values: Vec<String>,
    #[serde(rename = "valueType")]
    value_type: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
struct EventType {
    #[serde(rename = "colorName")]
    color_name: String,
    #[serde(rename = "createdAt")]
    created_at: String,
    id: String,
    name: String,
    #[serde(default)]
    properties: Option<Vec<EventTypeProperty>>,
    #[serde(rename = "updatedAt")]
    updated_at: String,
}

impl Record for EventType {
    fn headers() -> &'static [&'static str] {
        &[
            "ID",
            "Name",
            "Color",
            "Properties",
            "Created At",
            "Updated At",
        ]
    }

    fn fields(&self) -> Vec<String> {
        vec![
            self.id.clone(),
            self.name.clone(),
            self.color_name.clone(),
            compact_json(&serde_json::to_value(&self.properties).unwrap_or(Value::Null)),
            self.created_at.clone(),
            self.updated_at.clone(),
        ]
    }
}
pub(crate) async fn list_event_types(runtime: &Runtime, format: Format) -> Outcome {
    finish_list(
        runtime,
        format,
        "Failed to list event types",
        |client| async move {
            client
                .get::<_, Vec<EventType>>("/v1/event-types", &Vec::<(String, String)>::new())
                .await
        },
    )
    .await
}
