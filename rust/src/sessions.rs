//! Session commands.

use serde::{Deserialize, Serialize};
use std::fmt::Write as _;

use crate::api::encode_path_segment;
use crate::cli::{
    SessionAddArgs, SessionEditArgs, SessionListArgs, SessionLookupArgs,
    SessionRecordingMutationArgs,
};
use crate::output::Format;
use crate::records::{fetch_list, format_output, DeviceSummary, ProjectFallback, Record};
use crate::runtime::Runtime;
use crate::Outcome;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
struct SessionRecording {
    id: String,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    path: String,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    start: String,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    end: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
struct Session {
    id: String,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    name: String,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    key: String,
    #[serde(
        rename = "projectId",
        skip_serializing_if = "String::is_empty",
        default
    )]
    project_id: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    device: Option<DeviceSummary>,
    #[serde(rename = "createdAt")]
    created_at: String,
    #[serde(rename = "updatedAt")]
    updated_at: String,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    recordings: Vec<SessionRecording>,
}

impl Record for SessionRecording {
    fn headers() -> &'static [&'static str] {
        &["ID", "Start", "End", "Path"]
    }

    fn fields(&self) -> Vec<String> {
        vec![
            self.id.clone(),
            self.start.clone(),
            self.end.clone(),
            self.path.clone(),
        ]
    }
}

impl Record for Session {
    fn headers() -> &'static [&'static str] {
        &[
            "ID",
            "Name",
            "Key",
            "Project ID",
            "Device",
            "Created At",
            "Updated At",
        ]
    }

    fn fields(&self) -> Vec<String> {
        let device = self.device.as_ref().map_or_else(
            || "-".to_owned(),
            |device| {
                if device.id.is_empty() {
                    device.name.clone()
                } else {
                    format!("{} ({})", device.name, device.id)
                }
            },
        );
        vec![
            self.id.clone(),
            self.name.clone(),
            self.key.clone(),
            self.project_id.clone(),
            device,
            self.created_at.clone(),
            self.updated_at.clone(),
        ]
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SessionListQuery {
    #[serde(skip_serializing_if = "String::is_empty")]
    device_id: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    device_name: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    project_id: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ProjectQuery {
    #[serde(skip_serializing_if = "String::is_empty")]
    project_id: String,
}

pub(crate) async fn list_sessions(
    runtime: &Runtime,
    args: &SessionListArgs,
    format: Format,
) -> Outcome {
    let query = SessionListQuery {
        device_id: args.device_id.clone().unwrap_or_default(),
        device_name: args.device_name.clone().unwrap_or_default(),
        project_id: args.project_id.clone().or_project(&runtime.project_id),
    };
    fetch_list::<Session, _>(
        runtime,
        format,
        "Failed to list sessions",
        "/v1/sessions",
        &query,
    )
    .await
}

pub(crate) async fn get_session(runtime: &Runtime, args: &SessionLookupArgs) -> Outcome {
    let query = ProjectQuery {
        project_id: args.project_id.clone().or_project(&runtime.project_id),
    };
    let result = runtime
        .client
        .get::<_, Session>(
            &format!("/v1/sessions/{}", encode_path_segment(&args.session)),
            &query,
        )
        .await;
    match result {
        Ok(session) => session_outcome(&session),
        Err(error) if error.is_forbidden() => {
            Outcome::failure("Not authenticated. Run foxglove auth login.\n")
        }
        Err(error) if error.is_not_found() => {
            Outcome::failure(format!("Session not found: {}\n", args.session))
        }
        Err(error) => Outcome::failure(format!("Failed to get session: {error}\n")),
    }
}

fn session_outcome(session: &Session) -> Outcome {
    let device = session
        .device
        .as_ref()
        .map(|device| format!("Device:     {} ({})\n", device.name, device.id))
        .unwrap_or_default();
    let recordings = if session.recordings.is_empty() {
        "(none)".to_owned()
    } else {
        session
            .recordings
            .iter()
            .map(|recording| recording.id.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    };
    Outcome::success(format!(
        "ID:         {}\nName:       {}\nKey:        {}\nProject ID: {}\n{}Created At: {}\nUpdated At: {}\nRecordings: {}\n",
        session.id,
        session.name,
        session.key,
        session.project_id,
        device,
        session.created_at,
        session.updated_at,
        recordings
    ))
}

pub(crate) async fn list_session_recordings(
    runtime: &Runtime,
    args: &SessionLookupArgs,
) -> Outcome {
    let query = ProjectQuery {
        project_id: args.project_id.clone().or_project(&runtime.project_id),
    };
    let result = runtime
        .client
        .get::<_, Session>(
            &format!("/v1/sessions/{}", encode_path_segment(&args.session)),
            &query,
        )
        .await;
    match result {
        Ok(session) if session.recordings.is_empty() => {
            Outcome::success("No recordings in this session.\n")
        }
        Ok(session) => format_output(&session.recordings, Format::Table),
        Err(error) if error.is_forbidden() => {
            Outcome::failure("Not authenticated. Run foxglove auth login.\n")
        }
        Err(error) => Outcome::failure(format!("Failed to list session recordings: {error}\n")),
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CreateSessionRequest {
    #[serde(skip_serializing_if = "String::is_empty")]
    name: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    project_id: String,
    device_id: String,
}

#[derive(Deserialize)]
struct CreateSessionResponse {
    id: String,
    #[serde(default)]
    key: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PatchSessionRecordingsRequest {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    add_recording_ids: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    remove_recording_ids: Vec<String>,
}

#[derive(Serialize)]
struct PatchSessionKeyRequest<'a> {
    key: Option<&'a str>,
}

pub(crate) async fn add_session(runtime: &Runtime, args: &SessionAddArgs) -> Outcome {
    let device_id = args.device_id.clone().unwrap_or_default();
    if device_id.is_empty() {
        return Outcome::failure("--device-id is required when creating a session\n");
    }
    let request = CreateSessionRequest {
        name: args.name.clone().unwrap_or_default(),
        project_id: args.project_id.clone().or_project(&runtime.project_id),
        device_id,
    };
    match runtime
        .client
        .post::<_, CreateSessionResponse>("/v1/sessions", &request)
        .await
    {
        Ok(response) => {
            let mut stderr = format!("Session created: {}\n", response.id);
            if !response.key.is_empty() {
                let _ = writeln!(stderr, "Session key: {}", response.key);
            }
            Outcome::notice(stderr)
        }
        Err(error) if error.is_forbidden() => {
            Outcome::failure("Not authenticated. Run foxglove auth login.\n")
        }
        Err(error) => Outcome::failure(format!("Failed to create session: {error}\n")),
    }
}

pub(crate) async fn delete_session(runtime: &Runtime, args: &SessionLookupArgs) -> Outcome {
    let query = ProjectQuery {
        project_id: args.project_id.clone().or_project(&runtime.project_id),
    };
    match runtime
        .client
        .delete_with_query(
            &format!("/v1/sessions/{}", encode_path_segment(&args.session)),
            &query,
        )
        .await
    {
        Ok(()) => Outcome::notice(format!("Session deleted: {}\n", args.session)),
        Err(error) if error.is_not_found() => Outcome::notice(format!(
            "Not found. The resource may have already been deleted.\nSession deleted: {}\n",
            args.session
        )),
        Err(error) if error.is_forbidden() => {
            Outcome::failure("Not authenticated. Run foxglove auth login.\n")
        }
        Err(error) => Outcome::failure(format!("Failed to delete session: {error}\n")),
    }
}

pub(crate) async fn edit_session_key(runtime: &Runtime, args: &SessionEditArgs) -> Outcome {
    let query = ProjectQuery {
        project_id: args.project_id.clone().or_project(&runtime.project_id),
    };
    let request = PatchSessionKeyRequest {
        key: if args.remove_key {
            None
        } else {
            args.key.as_deref()
        },
    };
    match runtime
        .client
        .patch::<_, _, serde_json::Value>(
            &format!("/v1/sessions/{}", encode_path_segment(&args.session)),
            &query,
            &request,
        )
        .await
    {
        Ok(_) => Outcome {
            stderr: if let Some(key) = &args.key {
                format!("Session key updated: {key}\n")
            } else {
                format!("Session key removed: {}\n", args.session)
            }
            .into_bytes(),
            ..Outcome::default()
        },
        Err(error) if error.is_forbidden() => {
            Outcome::failure("Not authenticated. Run foxglove auth login.\n")
        }
        Err(error) => Outcome::failure(format!("Failed to update session key: {error}\n")),
    }
}

pub(crate) async fn patch_session_recordings(
    runtime: &Runtime,
    args: &SessionRecordingMutationArgs,
    add: bool,
) -> Outcome {
    let query = ProjectQuery {
        project_id: args.project_id.clone().or_project(&runtime.project_id),
    };
    let request = PatchSessionRecordingsRequest {
        add_recording_ids: if add {
            vec![args.recording.clone()]
        } else {
            Vec::new()
        },
        remove_recording_ids: if add {
            Vec::new()
        } else {
            vec![args.recording.clone()]
        },
    };
    match runtime
        .client
        .patch::<_, _, serde_json::Value>(
            &format!("/v1/sessions/{}", encode_path_segment(&args.session)),
            &query,
            &request,
        )
        .await
    {
        Ok(_) => Outcome::notice(format!(
            "Recording {} {} session\n",
            args.recording,
            if add { "added to" } else { "removed from" }
        )),
        Err(error) if error.is_forbidden() => {
            Outcome::failure("Not authenticated. Run foxglove auth login.\n")
        }
        Err(error) => Outcome::failure(format!(
            "Failed to {} recording {} session: {error}\n",
            if add { "add" } else { "remove" },
            if add { "to" } else { "from" }
        )),
    }
}
