//! Device commands.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

use crate::api::encode_path_segment;
use crate::cli::{DeviceEditArgs, DeviceListArgs, DeviceWriteArgs};
use crate::output::Format;
use crate::records::{compact_json, fetch_list, ProjectFallback, Record};
use crate::runtime::Runtime;
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

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DeviceListQuery {
    #[serde(skip_serializing_if = "String::is_empty")]
    project_id: String,
}

pub(crate) async fn list_devices(
    runtime: &Runtime,
    args: &DeviceListArgs,
    format: Format,
) -> Outcome {
    let query = DeviceListQuery {
        project_id: args.project_id.clone().or_project(&runtime.project_id),
    };
    fetch_list::<Device, _>(
        runtime,
        format,
        "Failed to list devices",
        "/v1/devices",
        &query,
    )
    .await
}

#[derive(Deserialize)]
struct CustomPropertyDefinition {
    key: String,
    #[serde(rename = "valueType")]
    value_type: String,
    #[serde(default)]
    values: Vec<String>,
}

#[derive(Serialize)]
struct DeviceRequest {
    name: String,
    #[serde(rename = "projectId", skip_serializing_if = "String::is_empty")]
    project_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    properties: Option<BTreeMap<String, Value>>,
}

#[derive(Deserialize)]
struct DeviceResponse {
    id: String,
    name: String,
}

async fn device_properties(
    runtime: &Runtime,
    pairs: &[String],
) -> Result<Option<BTreeMap<String, Value>>, String> {
    if pairs.is_empty() {
        return Ok(None);
    }
    // The Go form encoder uses the exported Go field name here because this
    // request type has no form tag; preserve that wire spelling.
    let query = vec![("ResourceType".to_owned(), "device".to_owned())];
    let definitions = runtime
        .client
        .get::<_, Vec<CustomPropertyDefinition>>("/v1/custom-properties", &query)
        .await
        .map_err(|error| format!("failed to load custom properties: {error}"))?;
    let definitions = definitions
        .into_iter()
        .map(|definition| (definition.key.clone(), definition))
        .collect::<BTreeMap<_, _>>();
    let mut properties = BTreeMap::new();
    for pair in pairs {
        let Some((key, raw_value)) = pair.split_once(':') else {
            return Err(format!("invalid key/value pair: {pair}"));
        };
        if key.is_empty() {
            return Err(format!("invalid key/value pair: {pair}"));
        }
        let Some(definition) = definitions.get(key) else {
            return Err(format!("unknown key: {key}"));
        };
        let value = match definition.value_type.as_str() {
            "string" => Value::String(raw_value.to_owned()),
            "number" => {
                let number = raw_value
                    .parse::<f64>()
                    .map_err(|_| format!("invalid value for number: {raw_value}"))?;
                serde_json::from_str(&number.to_string())
                    .map_err(|_| format!("invalid value for number: {raw_value}"))?
            }
            "boolean" => match raw_value {
                "1" | "t" | "T" | "TRUE" | "true" | "True" => Value::Bool(true),
                "0" | "f" | "F" | "FALSE" | "false" | "False" => Value::Bool(false),
                _ => return Err(format!("invalid value for boolean: {raw_value}")),
            },
            "enum" if definition.values.iter().any(|value| value == raw_value) => {
                Value::String(raw_value.to_owned())
            }
            "enum" => return Err(format!("invalid enum value: {raw_value}")),
            other => return Err(format!("unsupported type: {other}")),
        };
        properties.insert(key.to_owned(), value);
    }
    Ok(Some(properties))
}

pub(crate) async fn add_device(runtime: &Runtime, args: &DeviceWriteArgs) -> Outcome {
    let properties = match device_properties(runtime, &args.property).await {
        Ok(properties) => properties,
        Err(error) => return Outcome::failure(format!("Failed to create device: {error}\n")),
    };
    let request = DeviceRequest {
        name: args.name.clone().unwrap_or_default(),
        project_id: args.project_id.clone().or_project(&runtime.project_id),
        properties,
    };
    match runtime
        .client
        .post::<_, DeviceResponse>("/v1/devices", &request)
        .await
    {
        Ok(response) => Outcome {
            stderr: format!("Device created: {}\n", response.id).into_bytes(),
            ..Outcome::default()
        },
        Err(error) => Outcome::failure(format!("Failed to create device: {error}\n")),
    }
}

pub(crate) async fn edit_device(runtime: &Runtime, args: &DeviceEditArgs) -> Outcome {
    let properties = match device_properties(runtime, &args.update.property).await {
        Ok(properties) => properties,
        Err(error) => return Outcome::failure(format!("Failed to edit device: {error}\n")),
    };
    let name = args.update.name.clone().unwrap_or_default();
    if name.is_empty() && properties.is_none() {
        return Outcome::failure("Nothing to update\n");
    }
    let query = DeviceListQuery {
        project_id: args
            .update
            .project_id
            .clone()
            .or_project(&runtime.project_id),
    };
    let request = DeviceRequest {
        name,
        project_id: String::new(),
        properties,
    };
    match runtime
        .client
        .patch::<_, _, DeviceResponse>(
            &format!("/v1/devices/{}", encode_path_segment(&args.id)),
            &query,
            &request,
        )
        .await
    {
        Ok(response) => Outcome {
            stderr: format!("Device updated: {}\n", response.name).into_bytes(),
            ..Outcome::default()
        },
        Err(error) => Outcome::failure(format!("Failed to edit device: {error}\n")),
    }
}
