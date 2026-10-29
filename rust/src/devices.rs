//! Device commands.

use serde::{Deserialize, Serialize};
use serde_json::{Number, Value};
use std::collections::BTreeMap;

use crate::api::encode_path_segment;
use crate::cli::{DeviceAddArgs, DeviceDeleteArgs, DeviceEditArgs, DeviceGetArgs, DeviceListArgs};
use crate::custom_properties::properties;
use crate::output::Format;
use crate::records::{
    compact_json, fetch_list, format_record, is_zero, ProjectFallback, Record, DEFAULT_LIST_LIMIT,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    enabled: Option<bool>,
    #[serde(
        rename = "remoteAccessEnabled",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    remote_access_enabled: Option<bool>,
    #[serde(
        rename = "retainRecordingsSeconds",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    retain_recordings_seconds: Option<Number>,
    #[serde(
        rename = "autoDeleteAfterImport",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    auto_delete_after_import: Option<bool>,
}

fn optional_cell(value: Option<impl ToString>) -> String {
    value.map(|value| value.to_string()).unwrap_or_default()
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
            "Enabled",
            "Remote Access Enabled",
            "Retain Recordings Seconds",
            "Auto Delete After Import",
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
            optional_cell(self.enabled),
            optional_cell(self.remote_access_enabled),
            optional_cell(self.retain_recordings_seconds.as_ref()),
            optional_cell(self.auto_delete_after_import),
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
    #[serde(skip_serializing_if = "Option::is_none")]
    query: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    sort_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    sort_order: Option<String>,
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
        query: args.query.clone(),
        sort_by: args.sort_by.clone(),
        sort_order: args.sort_order.clone(),
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

pub(crate) async fn get_device(runtime: &Runtime, args: &DeviceGetArgs, format: Format) -> Outcome {
    let query = ProjectQuery {
        project_id: args.project_id.clone().or_project(&runtime.project_id),
    };
    match runtime
        .client
        .get::<_, Device>(
            &format!("/v1/devices/{}", encode_path_segment(&args.device)),
            &query,
        )
        .await
    {
        Ok(device) => format_record(&device, format),
        Err(error) if error.is_not_found_for("device") => {
            Outcome::failure(format!("Device not found: {}\n", args.device))
        }
        Err(error) => Outcome::failure(format!("Failed to get device: {error}\n")),
    }
}

pub(crate) async fn delete_device(runtime: &Runtime, args: &DeviceDeleteArgs) -> Outcome {
    let query = ProjectQuery {
        project_id: args.project_id.clone().or_project(&runtime.project_id),
    };
    match runtime
        .client
        .delete_with_query(
            &format!("/v1/devices/{}", encode_path_segment(&args.device)),
            &query,
        )
        .await
    {
        Ok(()) => Outcome::notice(format!("Device deleted: {}\n", args.device)),
        Err(error) if error.is_not_found_for("device") => {
            Outcome::failure(format!("Device not found: {}\n", args.device))
        }
        Err(error) => Outcome::failure(format!("Failed to delete device: {error}\n")),
    }
}

#[derive(Default, Serialize)]
#[serde(rename_all = "camelCase")]
struct DeviceRequest {
    #[serde(skip_serializing_if = "String::is_empty")]
    name: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    project_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    properties: Option<BTreeMap<String, Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    remote_access_enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    retain_recordings_seconds: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    auto_delete_after_import: Option<bool>,
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
        ..DeviceRequest::default()
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
    let update = &args.update;
    let request = DeviceRequest {
        name: update.name.clone().unwrap_or_default(),
        project_id: String::new(),
        properties,
        enabled: update.enabled,
        remote_access_enabled: update.remote_access_enabled,
        retain_recordings_seconds: update.retain_recordings_seconds,
        auto_delete_after_import: update.auto_delete_after_import,
    };
    if request.name.is_empty()
        && request.properties.is_none()
        && request.enabled.is_none()
        && request.remote_access_enabled.is_none()
        && request.retain_recordings_seconds.is_none()
        && request.auto_delete_after_import.is_none()
    {
        return Outcome::failure("Nothing to update\n");
    }
    let query = ProjectQuery {
        project_id: args
            .update
            .project_id
            .clone()
            .or_project(&runtime.project_id),
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
