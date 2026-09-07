//! Event commands.
#![allow(clippy::struct_field_names)]

use clap::ArgMatches;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::output::Format;
use crate::read_helpers::{
    add, add_str, compact_json, finish_list, last_value, parse_i64, query, value, DeviceSummary,
    Record, Runtime,
};
use crate::Outcome;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
struct Event {
    #[serde(rename = "createdAt")]
    created_at: String,
    #[serde(default)]
    device: DeviceSummary,
    end: String,
    #[serde(rename = "eventTypeId")]
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
    matches: &ArgMatches,
    format: Format,
) -> Outcome {
    if let Some(values) = matches.get_many::<String>("query-field") {
        for field in values {
            if field != "metadata" && field != "properties" {
                return Outcome::failure(format!("Invalid --query-field value \"{field}\": must be \"metadata\" or \"properties\"\n"));
            }
        }
    }
    let mut query = query();
    add_str(&mut query, "device.id", &value(matches, "device-id"));
    add_str(&mut query, "device.name", &value(matches, "device-name"));
    add_str(&mut query, "end", &value(matches, "end"));
    add_str(&mut query, "eventTypeId", &value(matches, "event-type-id"));
    let limit = match parse_i64(matches, "limit", 100) {
        Ok(value) => value,
        Err(error) => return Outcome::failure(format!("{error}\n")),
    };
    let offset = match parse_i64(matches, "offset", 0) {
        Ok(value) => value,
        Err(error) => return Outcome::failure(format!("{error}\n")),
    };
    add(&mut query, "limit", limit, limit != 0);
    add(&mut query, "offset", offset, offset != 0);
    add_str(&mut query, "query", &value(matches, "query"));
    if let Some(values) = matches.get_many::<String>("query-field") {
        for field in values {
            query.push(("queryFields".to_owned(), field.clone()));
        }
    }
    add_str(&mut query, "sortBy", &value(matches, "sort-by"));
    let sort_order = last_value(matches, "sort-order");
    add_str(
        &mut query,
        "sortOrder",
        if sort_order.is_none() {
            "asc"
        } else {
            sort_order.as_deref().unwrap_or_default()
        },
    );
    add_str(&mut query, "start", &value(matches, "start"));
    finish_list(
        runtime,
        format,
        "Failed to list events",
        move |client| async move { client.get::<_, Vec<Event>>("/v1/events", &query).await },
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

pub(crate) async fn add_event(runtime: &Runtime, matches: &ArgMatches) -> Outcome {
    let mut metadata = std::collections::BTreeMap::new();
    if let Some(values) = matches.get_many::<String>("metadata") {
        for pair in values {
            let Some((key, value)) = pair.split_once(':') else {
                return Outcome::failure(format!("Invalid metadata key/value pair: {pair}\n"));
            };
            if key.is_empty() {
                return Outcome::failure(format!("Invalid metadata key/value pair: {pair}\n"));
            }
            metadata.insert(key.to_owned(), value.to_owned());
        }
    }
    let request = CreateEventRequest {
        device_id: value(matches, "device-id"),
        end: value(matches, "end"),
        event_type_id: value(matches, "event-type-id"),
        metadata,
        start: value(matches, "start"),
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
