//! Event commands.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::cli::{EventAddArgs, EventListArgs};
use crate::output::Format;
use crate::records::{compact_json, fetch_list, null_to_default, DeviceSummary, Record};
use crate::runtime::Runtime;
use crate::Outcome;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[allow(clippy::struct_field_names)]
struct Event {
    #[serde(rename = "createdAt")]
    created_at: String,
    #[serde(default)]
    device: DeviceSummary,
    end: String,
    #[serde(rename = "eventTypeId")]
    #[serde(default, deserialize_with = "null_to_default")]
    event_type_id: String,
    id: String,
    #[serde(default)]
    metadata: Value,
    #[serde(default)]
    properties: Value,
    start: String,
    #[serde(rename = "updatedAt")]
    updated_at: String,
}

impl Record for Event {
    fn headers() -> &'static [&'static str] {
        &[
            "ID",
            "Device ID",
            "Device Name",
            "Start",
            "End",
            "Event Type ID",
            "Created At",
            "Updated At",
            "Metadata",
            "Properties",
        ]
    }

    fn fields(&self) -> Vec<String> {
        vec![
            self.id.clone(),
            self.device.id.clone(),
            self.device.name.clone(),
            self.start.clone(),
            self.end.clone(),
            self.event_type_id.clone(),
            self.created_at.clone(),
            self.updated_at.clone(),
            compact_json(&self.metadata),
            compact_json(&self.properties),
        ]
    }
}

pub(crate) async fn list_events(
    runtime: &Runtime,
    args: &EventListArgs,
    format: Format,
) -> Outcome {
    for field in &args.query_field {
        if field != "metadata" && field != "properties" {
            return Outcome::failure(format!(
                "Invalid --query-field value \"{field}\": must be \"metadata\" or \"properties\"\n"
            ));
        }
    }
    let limit = args.limit.unwrap_or(100);
    let offset = args.offset.unwrap_or_default();
    let query: Vec<(String, String)> = [
        ("device.id", args.device_id.clone().unwrap_or_default()),
        ("device.name", args.device_name.clone().unwrap_or_default()),
        ("end", args.end.clone().unwrap_or_default()),
        (
            "eventTypeId",
            args.event_type_id.clone().unwrap_or_default(),
        ),
        (
            "limit",
            if limit == 0 {
                String::new()
            } else {
                limit.to_string()
            },
        ),
        (
            "offset",
            if offset == 0 {
                String::new()
            } else {
                offset.to_string()
            },
        ),
        ("query", args.query.clone().unwrap_or_default()),
        ("sortBy", args.sort_by.clone().unwrap_or_default()),
        (
            "sortOrder",
            args.sort_order.clone().unwrap_or_else(|| "asc".to_owned()),
        ),
        ("start", args.start.clone().unwrap_or_default()),
    ]
    .into_iter()
    .filter(|(_, value)| !value.is_empty())
    .map(|(key, value)| (key.to_owned(), value))
    .chain(
        args.query_field
            .iter()
            .map(|field| ("queryFields".to_owned(), field.clone())),
    )
    .collect();
    fetch_list::<Event, _>(
        runtime,
        format,
        "Failed to list events",
        "/v1/events",
        &query,
    )
    .await
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CreateEventRequest {
    device_id: String,
    end: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    event_type_id: String,
    metadata: std::collections::BTreeMap<String, String>,
    start: String,
}

#[derive(Deserialize)]
struct CreateEventResponse {
    id: String,
}

pub(crate) async fn add_event(runtime: &Runtime, args: &EventAddArgs) -> Outcome {
    let mut metadata = std::collections::BTreeMap::new();
    for pair in &args.metadata {
        let Some((key, value)) = pair.split_once(':') else {
            return Outcome::failure(format!("Invalid metadata key/value pair: {pair}\n"));
        };
        if key.is_empty() {
            return Outcome::failure(format!("Invalid metadata key/value pair: {pair}\n"));
        }
        metadata.insert(key.to_owned(), value.to_owned());
    }
    let request = CreateEventRequest {
        device_id: args.device_id.clone().unwrap_or_default(),
        end: args.end.clone().unwrap_or_default(),
        event_type_id: args.event_type_id.clone().unwrap_or_default(),
        metadata,
        start: args.start.clone().unwrap_or_default(),
    };
    match runtime
        .client
        .post::<_, CreateEventResponse>("/v1/events", &request)
        .await
    {
        Ok(response) => Outcome {
            stderr: format!("Created event: {}\n", response.id).into_bytes(),
            ..Outcome::default()
        },
        Err(error) => Outcome::failure(format!("Failed to add event: {error}\n")),
    }
}

#[cfg(test)]
mod tests {
    use super::Event;

    #[test]
    fn missing_and_null_fields_render_explicit_defaults() {
        let original = serde_json::json!({"id":"evt_fixture","device":{"id":"dev_fixture","name":"Fixture"},"start":"2024-01-02T03:04:05Z","end":"2024-01-02T03:04:06Z","createdAt":"2024-01-02T03:04:05Z","updatedAt":"2024-01-02T03:04:06Z","metadata":{},"properties":{}});
        for (field, expected) in [("eventTypeId", serde_json::json!(""))] {
            let mut with_null = original.clone();
            with_null[field] = serde_json::Value::Null;
            for response in [original.clone(), with_null] {
                let record: Event = serde_json::from_value(response).unwrap();
                let output = serde_json::to_value(record).unwrap();
                assert_eq!(output[field], expected, "{field}");
            }
        }
    }
}
