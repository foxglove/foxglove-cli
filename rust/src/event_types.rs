//! Event type commands.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::output::Format;
use crate::records::{format_output, null_to_default, Record};
use crate::runtime::Runtime;
use crate::Outcome;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
struct EventTypeCustomProperty {
    id: String,
    required: bool,
    #[serde(skip)]
    key: Option<String>,
}

#[derive(Deserialize)]
struct CustomPropertyDefinition {
    id: String,
    key: String,
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
    #[serde(rename = "customProperties")]
    #[serde(default, deserialize_with = "null_to_default")]
    custom_properties: Vec<EventTypeCustomProperty>,
    #[serde(rename = "updatedAt")]
    updated_at: String,
}

impl Record for EventType {
    fn headers() -> &'static [&'static str] {
        &[
            "ID",
            "Name",
            "Color",
            "Custom Properties",
            "Created At",
            "Updated At",
        ]
    }

    fn fields(&self) -> Vec<String> {
        vec![
            self.id.clone(),
            self.name.clone(),
            self.color_name.clone(),
            self.custom_properties
                .iter()
                .map(|property| {
                    let name = property.key.as_deref().unwrap_or(&property.id);
                    if property.required {
                        format!("{name} (required)")
                    } else {
                        name.to_owned()
                    }
                })
                .collect::<Vec<_>>()
                .join(", "),
            self.created_at.clone(),
            self.updated_at.clone(),
        ]
    }
}

pub(crate) async fn list_event_types(runtime: &Runtime, format: Format) -> Outcome {
    let mut event_types = match runtime
        .client
        .get::<_, Vec<EventType>>("/v1/event-types", &())
        .await
    {
        Ok(event_types) => event_types,
        Err(error) => return Outcome::failure(format!("Failed to list event types: {error}\n")),
    };
    if format != Format::Json
        && event_types
            .iter()
            .any(|event_type| !event_type.custom_properties.is_empty())
    {
        if let Ok(definitions) = runtime
            .client
            .get::<_, Vec<CustomPropertyDefinition>>(
                "/v1/custom-properties",
                &[("resourceType", "event")],
            )
            .await
        {
            let keys = definitions
                .into_iter()
                .map(|definition| (definition.id, definition.key))
                .collect::<HashMap<_, _>>();
            for property in event_types
                .iter_mut()
                .flat_map(|event_type| &mut event_type.custom_properties)
            {
                property.key = keys.get(&property.id).cloned();
            }
        }
    }
    format_output(&event_types, format)
}

#[cfg(test)]
mod tests {
    use super::EventType;

    #[test]
    fn missing_and_null_fields_render_explicit_defaults() {
        let original = serde_json::json!({"id":"evtt_fixture","name":"Fixture","createdAt":"2024-01-02T03:04:05Z","updatedAt":"2024-01-02T03:04:06Z"});
        for (field, expected) in [
            ("colorName", serde_json::json!("")),
            ("customProperties", serde_json::json!([])),
        ] {
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
