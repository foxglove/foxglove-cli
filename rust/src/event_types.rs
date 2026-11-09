//! Event type commands.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::api::{encode_path_segment, ApiError};
use crate::cli::{
    EventTypeAddArgs, EventTypeCustomPropertyArg, EventTypeEditArgs, EventTypeGetArgs,
    EventTypeIdArgs,
};
use crate::output::Format;
use crate::records::{format_output, format_record, null_to_default, Record};
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

/// Fill in custom property keys for table and CSV output. JSON output keeps
/// the API's IDs and skips the lookup.
async fn resolve_custom_property_keys(
    runtime: &Runtime,
    event_types: &mut [EventType],
    format: Format,
) -> Option<ApiError> {
    if format == Format::Json
        || event_types
            .iter()
            .all(|event_type| event_type.custom_properties.is_empty())
    {
        return None;
    }
    match runtime
        .client
        .get::<_, Vec<CustomPropertyDefinition>>(
            "/v1/custom-properties",
            &[("resourceType", "event")],
        )
        .await
    {
        Ok(definitions) => {
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
            None
        }
        Err(error) => Some(error),
    }
}

fn with_lookup_warning(mut outcome: Outcome, lookup_error: Option<ApiError>) -> Outcome {
    if let Some(error) = lookup_error {
        outcome.stderr.extend_from_slice(
            format!("Showing custom property IDs; failed to load their keys: {error}\n").as_bytes(),
        );
    }
    outcome
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
    let lookup_error = resolve_custom_property_keys(runtime, &mut event_types, format).await;
    with_lookup_warning(format_output(&event_types, format), lookup_error)
}

fn event_type_endpoint(id: &str) -> String {
    format!("/v1/event-types/{}", encode_path_segment(id))
}

fn event_type_not_found(id: &str) -> Outcome {
    Outcome::failure(format!("Event type not found: {id}\n"))
}

pub(crate) async fn get_event_type(
    runtime: &Runtime,
    args: &EventTypeGetArgs,
    format: Format,
) -> Outcome {
    let event_type = match runtime
        .client
        .get::<_, EventType>(&event_type_endpoint(&args.event_type_id), &())
        .await
    {
        Ok(event_type) => event_type,
        Err(error) if error.is_not_found_for("event type") => {
            return event_type_not_found(&args.event_type_id)
        }
        Err(error) => return Outcome::failure(format!("Failed to get event type: {error}\n")),
    };
    let mut event_types = [event_type];
    let lookup_error = resolve_custom_property_keys(runtime, &mut event_types, format).await;
    with_lookup_warning(format_record(&event_types[0], format), lookup_error)
}

fn custom_properties(args: &[EventTypeCustomPropertyArg]) -> Vec<EventTypeCustomProperty> {
    args.iter()
        .map(|property| EventTypeCustomProperty {
            id: property.id.clone(),
            required: property.required,
            key: None,
        })
        .collect()
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CreateEventTypeRequest {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    color_name: Option<String>,
    custom_properties: Vec<EventTypeCustomProperty>,
}

#[derive(Deserialize)]
struct EventTypeIdResponse {
    id: String,
}

pub(crate) async fn add_event_type(runtime: &Runtime, args: &EventTypeAddArgs) -> Outcome {
    let request = CreateEventTypeRequest {
        name: args.name.clone(),
        color_name: args.color.clone(),
        custom_properties: custom_properties(&args.custom_property),
    };
    match runtime
        .client
        .post::<_, EventTypeIdResponse>("/v1/event-types", &request)
        .await
    {
        Ok(response) => Outcome::notice(format!("Event type created: {}\n", response.id)),
        Err(error) => Outcome::failure(format!("Failed to create event type: {error}\n")),
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct EditEventTypeRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    color_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    custom_properties: Option<Vec<EventTypeCustomProperty>>,
}

pub(crate) async fn edit_event_type(runtime: &Runtime, args: &EventTypeEditArgs) -> Outcome {
    let custom_properties = (args.clear_custom_properties || !args.custom_property.is_empty())
        .then(|| custom_properties(&args.custom_property));
    if args.name.is_none() && args.color.is_none() && custom_properties.is_none() {
        return Outcome::failure("Nothing to update\n");
    }
    let request = EditEventTypeRequest {
        name: args.name.clone(),
        color_name: args.color.clone(),
        custom_properties,
    };
    match runtime
        .client
        .patch::<_, _, EventTypeIdResponse>(
            &event_type_endpoint(&args.event_type_id),
            &(),
            &request,
        )
        .await
    {
        Ok(_) => Outcome::notice(format!("Event type updated: {}\n", args.event_type_id)),
        Err(error) if error.is_not_found_for("event type") => {
            event_type_not_found(&args.event_type_id)
        }
        Err(error) => Outcome::failure(format!("Failed to edit event type: {error}\n")),
    }
}

pub(crate) async fn delete_event_type(runtime: &Runtime, args: &EventTypeIdArgs) -> Outcome {
    match runtime
        .client
        .delete(&event_type_endpoint(&args.event_type_id))
        .await
    {
        Ok(()) => Outcome::notice(format!("Event type deleted: {}\n", args.event_type_id)),
        Err(error) if error.is_not_found_for("event type") => {
            event_type_not_found(&args.event_type_id)
        }
        Err(error) => Outcome::failure(format!("Failed to delete event type: {error}\n")),
    }
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
