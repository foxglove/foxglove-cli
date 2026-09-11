//! Event type commands.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::output::Format;
use crate::records::{compact_json, fetch_list, null_to_default, Record};
use crate::runtime::Runtime;
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
    #[serde(default, deserialize_with = "null_to_default")]
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
    fetch_list::<EventType, _>(
        runtime,
        format,
        "Failed to list event types",
        "/v1/event-types",
        &(),
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::EventType;

    #[test]
    fn missing_and_null_fields_render_explicit_defaults() {
        let original = serde_json::json!({"id":"evtt_fixture","name":"Fixture","createdAt":"2024-01-02T03:04:05Z","updatedAt":"2024-01-02T03:04:06Z"});
        for (field, expected) in [("colorName", serde_json::json!(""))] {
            let mut with_null = original.clone();
            with_null[field] = serde_json::Value::Null;
            for response in [original.clone(), with_null] {
                let record: EventType = serde_json::from_value(response).unwrap();
                let output = serde_json::to_value(record).unwrap();
                assert_eq!(output[field], expected, "{field}");
            }
        }
    }
}
