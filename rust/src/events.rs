//! Event commands.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::api::encode_path_segment;
use crate::cli::{EventAddArgs, EventEditArgs, EventGetArgs, EventIdArgs, EventListArgs};
use crate::custom_properties::{check_property_edits, properties, remove_properties};
use crate::output::Format;
use crate::records::{
    compact_json, fetch_list, format_record, null_to_default, parse_timestamp, DeviceSummary,
    ProjectFallback, Record, DEFAULT_LIST_LIMIT,
};
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
            if self.properties.is_null() {
                String::new()
            } else {
                compact_json(&self.properties)
            },
        ]
    }
}

/// Normalizes an optional time flag the same way as other commands, keeping nanoseconds.
fn optional_time(raw: Option<&str>, label: &str) -> Result<String, String> {
    parse_timestamp(raw.unwrap_or_default(), label)
}

/// The `createdAfter`, `end`, `start`, and `updatedAfter` list filters.
fn list_times(args: &EventListArgs) -> Result<[String; 4], String> {
    Ok([
        optional_time(args.created_after.as_deref(), "created after")?,
        optional_time(args.end.as_deref(), "end")?,
        optional_time(args.start.as_deref(), "start")?,
        optional_time(args.updated_after.as_deref(), "updated after")?,
    ])
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
    let [created_after, end, start, updated_after] = match list_times(args) {
        Ok(times) => times,
        Err(error) => return Outcome::failure(format!("{error}\n")),
    };
    let limit = args.limit.unwrap_or(DEFAULT_LIST_LIMIT);
    let offset = args.offset.unwrap_or_default();
    let query: Vec<(String, String)> = [
        ("createdAfter", created_after),
        ("deviceId", args.device_id.clone().unwrap_or_default()),
        ("deviceName", args.device_name.clone().unwrap_or_default()),
        ("end", end),
        ("eventId", args.event_id.clone().unwrap_or_default()),
        (
            "eventTypeId",
            args.event_type_id.clone().unwrap_or_default(),
        ),
        ("limit", limit.to_string()),
        (
            "offset",
            if offset == 0 {
                String::new()
            } else {
                offset.to_string()
            },
        ),
        (
            "projectId",
            args.project_id.clone().or_project(&runtime.project_id),
        ),
        ("query", args.query.clone().unwrap_or_default()),
        ("queryFields", args.query_field.join(",")),
        ("sortBy", args.sort_by.clone().unwrap_or_default()),
        ("sortOrder", args.sort_order.clone().unwrap_or_default()),
        ("start", start),
        ("updatedAfter", updated_after),
    ]
    .into_iter()
    .filter(|(_, value)| !value.is_empty())
    .map(|(key, value)| (key.to_owned(), value))
    .collect();
    fetch_list::<Event, _>(
        runtime,
        format,
        "Failed to list events",
        "/v1/events",
        &query,
        Some(limit),
    )
    .await
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CreateEventRequest {
    #[serde(skip_serializing_if = "String::is_empty")]
    project_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    device_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    device_name: Option<String>,
    end: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    event_type_id: String,
    metadata: BTreeMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    properties: Option<BTreeMap<String, Value>>,
    start: String,
}

#[derive(Deserialize)]
struct CreateEventResponse {
    id: String,
}

/// Parses `key:value` metadata pairs; the value may itself contain colons.
fn metadata(pairs: &[String]) -> Result<BTreeMap<String, String>, String> {
    let mut metadata = BTreeMap::new();
    for pair in pairs {
        match pair.split_once(':') {
            Some((key, value)) if !key.is_empty() => {
                metadata.insert(key.to_owned(), value.to_owned());
            }
            _ => return Err(format!("Invalid metadata key/value pair: {pair}")),
        }
    }
    Ok(metadata)
}

fn create_event_request(
    args: &EventAddArgs,
    project_id: String,
) -> Result<CreateEventRequest, String> {
    Ok(CreateEventRequest {
        project_id,
        device_id: args.device_id.clone(),
        device_name: args.device_name.clone(),
        end: parse_timestamp(&args.end, "end")?,
        event_type_id: args.event_type_id.clone().unwrap_or_default(),
        metadata: metadata(&args.metadata)?,
        properties: None,
        start: parse_timestamp(&args.start, "start")?,
    })
}

pub(crate) async fn add_event(runtime: &Runtime, args: &EventAddArgs) -> Outcome {
    let project_id = args.project_id.clone().or_project(&runtime.project_id);
    let mut request = match create_event_request(args, project_id) {
        Ok(request) => request,
        Err(error) => return Outcome::failure(format!("{error}\n")),
    };
    request.properties = match properties(runtime, "event", &args.property).await {
        Ok(properties) => properties,
        Err(error) => return Outcome::failure(format!("Failed to add event: {error}\n")),
    };
    match runtime
        .client
        .post::<_, CreateEventResponse>("/v1/events", &request)
        .await
    {
        Ok(response) => Outcome::notice(format!("Created event: {}\n", response.id)),
        Err(error) => Outcome::failure(format!("Failed to add event: {error}\n")),
    }
}

fn event_endpoint(id: &str) -> String {
    format!("/v1/events/{}", encode_path_segment(id))
}

pub(crate) async fn get_event(runtime: &Runtime, args: &EventGetArgs, format: Format) -> Outcome {
    match runtime
        .client
        .get::<_, Event>(&event_endpoint(&args.event_id), &())
        .await
    {
        Ok(event) => format_record(&event, format),
        Err(error) if error.is_not_found_for("event") => {
            Outcome::failure(format!("Event not found: {}\n", args.event_id))
        }
        Err(error) => Outcome::failure(format!("Failed to get event: {error}\n")),
    }
}

pub(crate) async fn delete_event(runtime: &Runtime, args: &EventIdArgs) -> Outcome {
    match runtime.client.delete(&event_endpoint(&args.event_id)).await {
        Ok(()) => Outcome::notice(format!("Event deleted: {}\n", args.event_id)),
        Err(error) if error.is_not_found_for("event") => {
            Outcome::failure(format!("Event not found: {}\n", args.event_id))
        }
        Err(error) => Outcome::failure(format!("Failed to delete event: {error}\n")),
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct EventPatch {
    #[serde(skip_serializing_if = "Option::is_none")]
    start: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    end: Option<String>,
    /// An empty string removes the event type.
    #[serde(skip_serializing_if = "Option::is_none")]
    event_type_id: Option<String>,
    /// Replaces all of the event's metadata; an empty map removes it.
    #[serde(skip_serializing_if = "Option::is_none")]
    metadata: Option<BTreeMap<String, String>>,
    /// Changes only the listed keys; a `null` value removes that property.
    #[serde(skip_serializing_if = "Option::is_none")]
    properties: Option<BTreeMap<String, Value>>,
}

fn event_patch(args: &EventEditArgs) -> Result<EventPatch, String> {
    if args.start.is_none()
        && args.end.is_none()
        && args.event_type_id.is_none()
        && !args.remove_event_type
        && args.metadata.is_empty()
        && !args.clear_metadata
        && args.property.is_empty()
        && args.remove_property.is_empty()
    {
        return Err("Nothing to update".to_owned());
    }
    check_property_edits(&args.property, &args.remove_property)?;
    let metadata = if args.clear_metadata {
        Some(BTreeMap::new())
    } else if args.metadata.is_empty() {
        None
    } else {
        Some(metadata(&args.metadata)?)
    };
    Ok(EventPatch {
        start: args
            .start
            .as_deref()
            .map(|raw| parse_timestamp(raw, "start"))
            .transpose()?,
        end: args
            .end
            .as_deref()
            .map(|raw| parse_timestamp(raw, "end"))
            .transpose()?,
        event_type_id: if args.remove_event_type {
            Some(String::new())
        } else {
            args.event_type_id.clone()
        },
        metadata,
        properties: None,
    })
}

pub(crate) async fn edit_event(runtime: &Runtime, args: &EventEditArgs) -> Outcome {
    let mut patch = match event_patch(args) {
        Ok(patch) => patch,
        Err(error) => return Outcome::failure(format!("{error}\n")),
    };
    patch.properties = match properties(runtime, "event", &args.property).await {
        Ok(properties) => properties,
        Err(error) => return Outcome::failure(format!("Failed to edit event: {error}\n")),
    };
    remove_properties(&mut patch.properties, &args.remove_property);
    match runtime
        .client
        .patch::<_, _, Event>(&event_endpoint(&args.event_id), &(), &patch)
        .await
    {
        Ok(event) => Outcome::notice(format!("Event updated: {}\n", event.id)),
        Err(error) if error.is_not_found_for("event") => {
            Outcome::failure(format!("Event not found: {}\n", args.event_id))
        }
        Err(error) => Outcome::failure(format!("Failed to edit event: {error}\n")),
    }
}

#[cfg(test)]
mod tests {
    use super::{create_event_request, event_patch, Event};
    use crate::cli::{EventAddArgs, EventEditArgs};

    #[test]
    fn a_new_event_names_its_device_by_id_or_name() {
        let args = |device_id: Option<&str>, device_name: Option<&str>| EventAddArgs {
            project_id: None,
            device_id: device_id.map(str::to_owned),
            device_name: device_name.map(str::to_owned),
            end: "2024-01-02T03:04:06Z".to_owned(),
            event_type_id: None,
            metadata: Vec::new(),
            property: Vec::new(),
            start: "2024-01-02T03:04:05Z".to_owned(),
        };
        let body = |args: &EventAddArgs| {
            serde_json::to_value(create_event_request(args, String::new()).unwrap()).unwrap()
        };
        assert_eq!(
            body(&args(Some("dev_1"), None)),
            serde_json::json!({"deviceId": "dev_1", "start": "2024-01-02T03:04:05Z", "end": "2024-01-02T03:04:06Z", "metadata": {}})
        );
        assert_eq!(
            body(&args(None, Some("robot"))),
            serde_json::json!({"deviceName": "robot", "start": "2024-01-02T03:04:05Z", "end": "2024-01-02T03:04:06Z", "metadata": {}})
        );
    }

    #[test]
    fn event_times_are_normalized_with_nanosecond_precision() {
        let add = |start: &str, end: &str| EventAddArgs {
            project_id: None,
            device_id: Some("dev_1".to_owned()),
            device_name: None,
            end: end.to_owned(),
            event_type_id: None,
            metadata: Vec::new(),
            property: Vec::new(),
            start: start.to_owned(),
        };
        let request = create_event_request(
            &add("2024-01-02", "2024-01-02T03:04:05.123456789"),
            String::new(),
        )
        .unwrap();
        assert_eq!(request.start, "2024-01-02T00:00:00Z");
        assert_eq!(request.end, "2024-01-02T03:04:05.123456789Z");
        assert!(
            create_event_request(&add("2024-01-02", "not a date"), String::new())
                .err()
                .unwrap()
                .starts_with("failed to parse end time:")
        );

        let edit = |start: Option<&str>, end: Option<&str>| EventEditArgs {
            event_id: "evt_1".to_owned(),
            start: start.map(str::to_owned),
            end: end.map(str::to_owned),
            event_type_id: None,
            remove_event_type: false,
            metadata: Vec::new(),
            clear_metadata: false,
            property: Vec::new(),
            remove_property: Vec::new(),
        };
        let patch = event_patch(&edit(Some("2024-01-02T03:04:05.000000001+0100"), None)).unwrap();
        assert_eq!(
            patch.start.as_deref(),
            Some("2024-01-02T03:04:05.000000001+01:00")
        );
        assert_eq!(patch.end, None);
        assert!(event_patch(&edit(None, Some("2024-02-30")))
            .unwrap_err()
            .starts_with("failed to parse end time:"));
    }

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
