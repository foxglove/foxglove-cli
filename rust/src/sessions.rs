//! Session commands.
#![allow(clippy::struct_field_names)]

use clap::ArgMatches;
use serde::{Deserialize, Serialize};

use crate::output::Format;
use crate::read_helpers::{
    add_str, block_on, finish_list, format_output, positional, query, sort_query, value,
    DeviceSummary, ProjectFallback, Record, Runtime,
};
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
pub(crate) fn list_sessions(runtime: &Runtime, matches: &ArgMatches, format: Format) -> Outcome {
    let mut query = query();
    add_str(&mut query, "deviceId", &value(matches, "device-id"));
    add_str(&mut query, "deviceName", &value(matches, "device-name"));
    add_str(
        &mut query,
        "projectId",
        &value(matches, "project-id").or_project(&runtime.project_id),
    );
    sort_query(&mut query);
    finish_list(
        runtime,
        format,
        "Failed to list sessions",
        move |client| async move { client.get::<_, Vec<Session>>("/v1/sessions", &query).await },
    )
}

pub(crate) fn get_session(runtime: &Runtime, matches: &ArgMatches) -> Outcome {
    let key = positional(matches, 0);
    let project_id = value(matches, "project-id").or_project(&runtime.project_id);
    let mut query = query();
    add_str(&mut query, "projectId", &project_id);
    sort_query(&mut query);
    let result = block_on(
        runtime
            .client
            .get::<_, Session>(&format!("/v1/sessions/{key}"), &query),
    );
    match result {
        Ok(session) => session_outcome(&session),
        Err(error) if error.is_forbidden() => {
            Outcome::failure("Not authenticated. Run foxglove auth login.\n")
        }
        Err(error) if error.is_not_found() => {
            Outcome::failure(format!("Session not found: {key}\n"))
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
        session.id, session.name, session.key, session.project_id, device, session.created_at, session.updated_at, recordings
    ))
}

pub(crate) fn list_session_recordings(runtime: &Runtime, matches: &ArgMatches) -> Outcome {
    let key = positional(matches, 0);
    let project_id = value(matches, "project-id").or_project(&runtime.project_id);
    let mut query = query();
    add_str(&mut query, "projectId", &project_id);
    sort_query(&mut query);
    let result = block_on(
        runtime
            .client
            .get::<_, Session>(&format!("/v1/sessions/{key}"), &query),
    );
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
