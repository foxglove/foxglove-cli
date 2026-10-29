//! Device custom property history commands.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::api::encode_path_segment;
use crate::cli::{
    DevicePropertyIntervalGetArgs, DevicePropertyIntervalListArgs, DevicePropertyIntervalUpdateArgs,
};
use crate::custom_properties::properties;
use crate::output::Format;
use crate::records::{
    compact_json, format_output, format_record, is_zero, parse_timestamp, warn_if_truncated,
    ProjectFallback, Record, DEFAULT_LIST_LIMIT,
};
use crate::runtime::Runtime;
use crate::Outcome;

#[derive(Clone, Debug, Deserialize, Serialize)]
struct PropertyTimeInterval {
    id: String,
    #[serde(rename = "deviceId")]
    device_id: String,
    key: String,
    #[serde(default)]
    value: Value,
    start: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    end: Option<String>,
}

impl Record for PropertyTimeInterval {
    fn headers() -> &'static [&'static str] {
        &["ID", "Device ID", "Key", "Value", "Start", "End"]
    }

    fn fields(&self) -> Vec<String> {
        vec![
            self.id.clone(),
            self.device_id.clone(),
            self.key.clone(),
            match &self.value {
                Value::String(value) => value.clone(),
                Value::Null => String::new(),
                value => compact_json(value),
            },
            self.start.clone(),
            self.end.clone().unwrap_or_default(),
        ]
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ListQuery {
    #[serde(skip_serializing_if = "String::is_empty")]
    end: String,
    limit: i64,
    #[serde(skip_serializing_if = "is_zero")]
    offset: i64,
    #[serde(skip_serializing_if = "String::is_empty")]
    project_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    query: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    sort_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    sort_order: Option<String>,
    #[serde(skip_serializing_if = "String::is_empty")]
    start: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ProjectQuery {
    #[serde(skip_serializing_if = "String::is_empty")]
    project_id: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct UpdateRequest {
    #[serde(skip_serializing_if = "String::is_empty")]
    project_id: String,
    key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    value: Option<Value>,
    start: String,
    end: String,
}

fn intervals_path(device: &str) -> String {
    format!(
        "/v1/devices/{}/property-time-intervals",
        encode_path_segment(device)
    )
}

pub(crate) async fn list_property_intervals(
    runtime: &Runtime,
    args: &DevicePropertyIntervalListArgs,
    format: Format,
) -> Outcome {
    let start = match parse_timestamp(args.start.as_deref().unwrap_or_default(), "start") {
        Ok(start) => start,
        Err(error) => return Outcome::failure(format!("{error}\n")),
    };
    let end = match parse_timestamp(args.end.as_deref().unwrap_or_default(), "end") {
        Ok(end) => end,
        Err(error) => return Outcome::failure(format!("{error}\n")),
    };
    let query = ListQuery {
        end,
        limit: args.limit.unwrap_or(DEFAULT_LIST_LIMIT),
        offset: args.offset.unwrap_or_default(),
        project_id: args.project_id.clone().or_project(&runtime.project_id),
        query: args.query.clone(),
        sort_by: args.sort_by.clone(),
        sort_order: args.sort_order.clone(),
        start,
    };
    let result = runtime
        .client
        .get::<_, Vec<PropertyTimeInterval>>(&intervals_path(&args.device), &query)
        .await;
    match result {
        Ok(intervals) => warn_if_truncated(
            format_output(&intervals, format),
            intervals.len(),
            query.limit,
        ),
        Err(error) if error.is_not_found_for("device") => {
            Outcome::failure(format!("Device not found: {}\n", args.device))
        }
        Err(error) => {
            Outcome::failure(format!("Failed to list property time intervals: {error}\n"))
        }
    }
}

pub(crate) async fn get_property_interval(
    runtime: &Runtime,
    args: &DevicePropertyIntervalGetArgs,
    format: Format,
) -> Outcome {
    let query = ProjectQuery {
        project_id: args.project_id.clone().or_project(&runtime.project_id),
    };
    let result = runtime
        .client
        .get::<_, PropertyTimeInterval>(
            &format!(
                "{}/{}",
                intervals_path(&args.device),
                encode_path_segment(&args.id)
            ),
            &query,
        )
        .await;
    match result {
        Ok(interval) => format_record(&interval, format),
        // A 404 that names nothing is reported as a missing interval.
        Err(error) if !error.is_bare_not_found() && error.is_not_found_for("device") => {
            Outcome::failure(format!("Device not found: {}\n", args.device))
        }
        Err(error) if error.is_not_found_for("property time interval") => {
            Outcome::failure(format!("Property time interval not found: {}\n", args.id))
        }
        Err(error) => Outcome::failure(format!("Failed to get property time interval: {error}\n")),
    }
}

pub(crate) async fn update_property_interval(
    runtime: &Runtime,
    args: &DevicePropertyIntervalUpdateArgs,
) -> Outcome {
    let start = match parse_timestamp(&args.start, "start") {
        Ok(start) => start,
        Err(error) => return Outcome::failure(format!("{error}\n")),
    };
    let end = match parse_timestamp(&args.end, "end") {
        Ok(end) => end,
        Err(error) => return Outcome::failure(format!("{error}\n")),
    };
    let value = if args.unset {
        None
    } else {
        match typed_value(runtime, &args.key, &args.value).await {
            Ok(value) => Some(value),
            Err(error) => {
                return Outcome::failure(format!(
                    "Failed to update property time interval: {error}\n"
                ))
            }
        }
    };
    let request = UpdateRequest {
        project_id: args.project_id.clone().or_project(&runtime.project_id),
        key: args.key.clone(),
        value,
        start,
        end,
    };
    let result = runtime
        .client
        .post_without_response(
            &format!(
                "/v1/actions/devices/{}/update-property-time-interval",
                encode_path_segment(&args.device)
            ),
            &request,
        )
        .await;
    match result {
        Ok(()) => Outcome::notice(format!(
            "Property time interval updated: {} from {} to {}\n",
            request.key, request.start, request.end
        )),
        Err(error) if error.is_not_found_for("device") => {
            Outcome::failure(format!("Device not found: {}\n", args.device))
        }
        Err(error) => Outcome::failure(format!(
            "Failed to update property time interval: {error}\n"
        )),
    }
}

/// Converts `--value` arguments to the JSON type the key's definition declares.
async fn typed_value(runtime: &Runtime, key: &str, values: &[String]) -> Result<Value, String> {
    let pairs = values
        .iter()
        .map(|value| format!("{key}:{value}"))
        .collect::<Vec<_>>();
    let value = properties(runtime, "device", &pairs)
        .await?
        .and_then(|mut properties| properties.remove(key))
        .ok_or_else(|| format!("missing value for {key}"))?;
    if values.len() > 1 && !value.is_array() {
        return Err(format!(
            "--value can be repeated only for multi-enum properties; {key} takes one value"
        ));
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::PropertyTimeInterval;
    use crate::records::Record;

    #[test]
    fn intervals_render_values_by_type_and_keep_open_ends() {
        let intervals: Vec<PropertyTimeInterval> = serde_json::from_value(serde_json::json!([
            {"id": "dpi_1", "deviceId": "dev_1", "key": "location", "value": "sf",
             "start": "2026-01-01T00:00:00Z", "end": "2026-02-01T00:00:00Z"},
            {"id": "dpi_2", "deviceId": "dev_1", "key": "tags", "value": ["night", "rain"],
             "start": "2026-02-01T00:00:00Z"},
            {"id": "dpi_3", "deviceId": "dev_1", "key": "speed", "value": 3.5,
             "start": "2026-02-01T00:00:00Z"},
        ]))
        .unwrap();
        assert_eq!(
            intervals[0].fields()[3..],
            ["sf", "2026-01-01T00:00:00Z", "2026-02-01T00:00:00Z"]
        );
        assert_eq!(
            intervals[1].fields()[3..],
            [r#"["night","rain"]"#, "2026-02-01T00:00:00Z", ""]
        );
        assert_eq!(intervals[2].fields()[3], "3.5");
        let json = serde_json::to_value(&intervals[1]).unwrap();
        assert!(json.get("end").is_none(), "{json}");
        assert_eq!(json["value"], serde_json::json!(["night", "rain"]));
    }
}
