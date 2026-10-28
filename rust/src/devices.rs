//! Device commands.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

use crate::api::encode_path_segment;
use crate::cli::{DeviceAddArgs, DeviceEditArgs, DeviceListArgs};
use crate::custom_properties::properties;
use crate::output::Format;
use crate::records::{
    compact_json, fetch_list, is_zero, ProjectFallback, Record, DEFAULT_LIST_LIMIT,
};
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
            if self.properties.is_null() {
                String::new()
            } else {
                compact_json(&self.properties)
            },
            self.created_at.clone(),
            self.updated_at.clone(),
            self.project_id.clone(),
        ]
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DeviceListQuery {
    limit: i64,
    #[serde(skip_serializing_if = "is_zero")]
    offset: i64,
    #[serde(skip_serializing_if = "String::is_empty")]
    project_id: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ProjectQuery {
    #[serde(skip_serializing_if = "String::is_empty")]
    project_id: String,
}

pub(crate) async fn list_devices(
    runtime: &Runtime,
    args: &DeviceListArgs,
    format: Format,
) -> Outcome {
    let query = DeviceListQuery {
        limit: args.limit.unwrap_or(DEFAULT_LIST_LIMIT),
        offset: args.offset.unwrap_or_default(),
        project_id: args.project_id.clone().or_project(&runtime.project_id),
    };
    fetch_list::<Device, _>(
        runtime,
        format,
        "Failed to list devices",
        "/v1/devices",
        &query,
        Some(query.limit),
    )
    .await
}

#[derive(Serialize)]
struct DeviceRequest {
    #[serde(skip_serializing_if = "String::is_empty")]
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

pub(crate) async fn add_device(runtime: &Runtime, args: &DeviceAddArgs) -> Outcome {
    if args.name.trim().is_empty() {
        return Outcome::failure("--name cannot be empty\n");
    }
    let properties = match properties(runtime, "device", &args.property).await {
        Ok(properties) => properties,
        Err(error) => return Outcome::failure(format!("Failed to create device: {error}\n")),
    };
    let request = DeviceRequest {
        name: args.name.clone(),
        project_id: args.project_id.clone().or_project(&runtime.project_id),
        properties,
    };
    match runtime
        .client
        .post::<_, DeviceResponse>("/v1/devices", &request)
        .await
    {
        Ok(response) => Outcome::notice(format!("Device created: {}\n", response.id)),
        Err(error) => Outcome::failure(format!("Failed to create device: {error}\n")),
    }
}

pub(crate) async fn edit_device(runtime: &Runtime, args: &DeviceEditArgs) -> Outcome {
    if args
        .update
        .name
        .as_deref()
        .is_some_and(|name| name.trim().is_empty())
    {
        return Outcome::failure("--name cannot be empty\n");
    }
    let properties = match properties(runtime, "device", &args.update.property).await {
        Ok(properties) => properties,
        Err(error) => return Outcome::failure(format!("Failed to edit device: {error}\n")),
    };
    let name = args.update.name.clone().unwrap_or_default();
    if name.is_empty() && properties.is_none() {
        return Outcome::failure("Nothing to update\n");
    }
    let query = ProjectQuery {
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
        Ok(response) => Outcome::notice(format!("Device updated: {}\n", response.name)),
        Err(error) => Outcome::failure(format!("Failed to edit device: {error}\n")),
    }
}
