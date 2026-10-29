//! Device token commands.

use serde::{Deserialize, Serialize};

use crate::api::encode_path_segment;
use crate::cli::{
    DeviceTokenAddArgs, DeviceTokenEditArgs, DeviceTokenGetArgs, DeviceTokenIdArgs,
    DeviceTokenListArgs,
};
use crate::output::Format;
use crate::records::{fetch_list, format_record, ProjectFallback, Record};
use crate::runtime::Runtime;
use crate::Outcome;

const DEVICE_TOKEN_HEADERS: [&str; 6] = [
    "ID",
    "Device ID",
    "Project ID",
    "Enabled",
    "Preapproved",
    "Created At",
];

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct DeviceToken {
    id: String,
    #[serde(default)]
    device_id: Option<String>,
    project_id: String,
    enabled: bool,
    preapproved: bool,
    created_at: String,
}

impl DeviceToken {
    fn cells(&self) -> Vec<String> {
        vec![
            self.id.clone(),
            self.device_id.clone().unwrap_or_default(),
            self.project_id.clone(),
            self.enabled.to_string(),
            self.preapproved.to_string(),
            self.created_at.clone(),
        ]
    }
}

impl Record for DeviceToken {
    fn headers() -> &'static [&'static str] {
        &DEVICE_TOKEN_HEADERS
    }

    fn fields(&self) -> Vec<String> {
        self.cells()
    }
}

/// A newly created token, which is the only response that carries its secret.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
struct CreatedDeviceToken {
    #[serde(flatten)]
    device_token: DeviceToken,
    token: String,
}

impl Record for CreatedDeviceToken {
    fn headers() -> &'static [&'static str] {
        &[
            "ID",
            "Device ID",
            "Project ID",
            "Enabled",
            "Preapproved",
            "Created At",
            "Token",
        ]
    }

    fn fields(&self) -> Vec<String> {
        let mut fields = self.device_token.cells();
        fields.push(self.token.clone());
        fields
    }
}

fn device_token_endpoint(id: &str) -> String {
    format!("/v1/device-tokens/{}", encode_path_segment(id))
}

fn device_token_not_found(id: &str) -> Outcome {
    Outcome::failure(format!("Device token not found: {id}\n"))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DeviceTokenListQuery {
    #[serde(skip_serializing_if = "Option::is_none")]
    device_id: Option<String>,
}

pub(crate) async fn list_device_tokens(
    runtime: &Runtime,
    args: &DeviceTokenListArgs,
    format: Format,
) -> Outcome {
    let query = DeviceTokenListQuery {
        device_id: args.device_id.clone(),
    };
    fetch_list::<DeviceToken, _>(
        runtime,
        format,
        "Failed to list device tokens",
        "/v1/device-tokens",
        &query,
        None,
    )
    .await
}

pub(crate) async fn get_device_token(
    runtime: &Runtime,
    args: &DeviceTokenGetArgs,
    format: Format,
) -> Outcome {
    match runtime
        .client
        .get::<_, DeviceToken>(&device_token_endpoint(&args.id), &())
        .await
    {
        Ok(device_token) => format_record(&device_token, format),
        Err(error) if error.is_not_found_for("device token") => device_token_not_found(&args.id),
        Err(error) => Outcome::failure(format!("Failed to get device token: {error}\n")),
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CreateDeviceTokenRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    device_id: Option<String>,
    #[serde(skip_serializing_if = "String::is_empty")]
    project_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    preapproved: Option<bool>,
}

/// The project to send when creating a token. A token for a device belongs to
/// that device's project, so the default project applies only to tokens
/// created without a device.
fn add_project_id(args: &DeviceTokenAddArgs, default_project_id: &str) -> String {
    if args.device_id.is_some() {
        args.project_id.clone().unwrap_or_default()
    } else {
        args.project_id.clone().or_project(default_project_id)
    }
}

pub(crate) async fn add_device_token(
    runtime: &Runtime,
    args: &DeviceTokenAddArgs,
    format: Format,
) -> Outcome {
    let request = CreateDeviceTokenRequest {
        device_id: args.device_id.clone(),
        project_id: add_project_id(args, &runtime.project_id),
        enabled: args.enabled,
        preapproved: args.preapproved,
    };
    match runtime
        .client
        .post::<_, CreatedDeviceToken>("/v1/device-tokens", &request)
        .await
    {
        Ok(created) => {
            let mut outcome = format_record(&created, format);
            if outcome.exit_code == 0 {
                outcome.stderr = format!(
                    "Device token created: {}\nStore the token now; it is not shown again.\n",
                    created.device_token.id
                )
                .into_bytes();
            }
            outcome
        }
        Err(error)
            if args.device_id.is_none()
                && !request.project_id.is_empty()
                && error.is_not_found_for("project") =>
        {
            Outcome::failure(format!("Project not found: {}\n", request.project_id))
        }
        Err(error) => Outcome::failure(format!("Failed to create device token: {error}\n")),
    }
}

#[derive(Serialize)]
struct EditDeviceTokenRequest {
    enabled: bool,
}

pub(crate) async fn edit_device_token(runtime: &Runtime, args: &DeviceTokenEditArgs) -> Outcome {
    let request = EditDeviceTokenRequest {
        enabled: args.enabled,
    };
    match runtime
        .client
        .patch::<_, _, DeviceToken>(&device_token_endpoint(&args.id), &(), &request)
        .await
    {
        Ok(device_token) => Outcome::notice(format!(
            "Device token {}: {}\n",
            if device_token.enabled {
                "enabled"
            } else {
                "disabled"
            },
            device_token.id
        )),
        Err(error) if error.is_not_found_for("device token") => device_token_not_found(&args.id),
        Err(error) => Outcome::failure(format!("Failed to edit device token: {error}\n")),
    }
}

pub(crate) async fn delete_device_token(runtime: &Runtime, args: &DeviceTokenIdArgs) -> Outcome {
    match runtime
        .client
        .delete(&device_token_endpoint(&args.id))
        .await
    {
        Ok(()) => Outcome::notice(format!("Device token deleted: {}\n", args.id)),
        Err(error) if error.is_not_found_for("device token") => device_token_not_found(&args.id),
        Err(error) => Outcome::failure(format!("Failed to delete device token: {error}\n")),
    }
}

#[cfg(test)]
mod tests {
    use super::{CreatedDeviceToken, DeviceToken};
    use crate::records::Record;

    #[test]
    fn device_tokens_keep_a_null_device_and_render_the_secret_only_on_creation() {
        let token: DeviceToken = serde_json::from_value(serde_json::json!({
            "id": "dt_1",
            "deviceId": null,
            "projectId": "prj_1",
            "enabled": true,
            "preapproved": false,
            "createdAt": "2024-01-02T03:04:05Z",
        }))
        .unwrap();
        assert_eq!(
            token.fields(),
            ["dt_1", "", "prj_1", "true", "false", "2024-01-02T03:04:05Z"]
        );
        assert_eq!(
            serde_json::to_value(&token).unwrap()["deviceId"],
            serde_json::Value::Null
        );

        let created: CreatedDeviceToken = serde_json::from_value(serde_json::json!({
            "id": "dt_1",
            "deviceId": "dev_1",
            "projectId": "prj_1",
            "enabled": true,
            "preapproved": true,
            "createdAt": "2024-01-02T03:04:05Z",
            "token": "fake-device-token",
        }))
        .unwrap();
        assert_eq!(CreatedDeviceToken::headers().last(), Some(&"Token"));
        assert_eq!(created.fields().last().unwrap(), "fake-device-token");
        assert_eq!(
            serde_json::to_value(&created).unwrap()["token"],
            "fake-device-token"
        );
    }
}
