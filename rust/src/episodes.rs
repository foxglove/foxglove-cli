//! Episode commands.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::api::{encode_path_segment, ApiError};
use crate::cli::{
    EpisodeAddArgs, EpisodeBulkEditArgs, EpisodeEditArgs, EpisodeGetArgs, EpisodeIdArgs,
    EpisodeListArgs,
};
use crate::output::Format;
use crate::records::{
    compact_json, format_list_output, format_record, parse_timestamp, parse_timestamp_millis,
    warn_if_has_next_cursor, NextCursor, ProjectFallback, Record, DEFAULT_LIST_LIMIT,
};
use crate::runtime::Runtime;
use crate::Outcome;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(crate) struct RecordingObjectLocation {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub(crate) scheme: Option<String>,
    pub(crate) bucket: String,
    pub(crate) path: String,
    #[serde(
        rename = "azureStorageAccountName",
        skip_serializing_if = "Option::is_none",
        default
    )]
    pub(crate) azure_storage_account_name: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(crate) struct EpisodeRecording {
    pub(crate) id: String,
    #[serde(default)]
    pub(crate) path: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub(crate) location: Option<RecordingObjectLocation>,
    #[serde(default)]
    pub(crate) start: String,
    #[serde(default)]
    pub(crate) end: String,
    #[serde(rename = "deviceId", skip_serializing_if = "String::is_empty", default)]
    pub(crate) device_id: String,
    #[serde(default)]
    pub(crate) available: bool,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub(crate) resolvable: Option<bool>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(crate) struct Episode {
    pub(crate) id: String,
    #[serde(rename = "projectId")]
    pub(crate) project_id: String,
    #[serde(rename = "startTime")]
    pub(crate) start_time: String,
    #[serde(rename = "endTime")]
    pub(crate) end_time: String,
    #[serde(default)]
    pub(crate) metadata: Value,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub(crate) recordings: Option<Vec<EpisodeRecording>>,
    #[serde(
        rename = "hasMissingRecordings",
        skip_serializing_if = "Option::is_none",
        default
    )]
    pub(crate) has_missing_recordings: Option<bool>,
    #[serde(rename = "createdAt")]
    pub(crate) created_at: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub(crate) version: Option<i32>,
    #[serde(
        rename = "latestVersion",
        skip_serializing_if = "Option::is_none",
        default
    )]
    pub(crate) latest_version: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub(crate) labels: Option<Value>,
}

impl Episode {
    pub(crate) fn recording_ids(&self) -> String {
        self.recordings
            .as_ref()
            .map_or_else(String::new, |recordings| {
                recordings
                    .iter()
                    .map(|recording| recording.id.as_str())
                    .collect::<Vec<_>>()
                    .join(",")
            })
    }
}

impl Record for Episode {
    fn headers() -> &'static [&'static str] {
        &[
            "ID",
            "Project ID",
            "Start Time",
            "End Time",
            "Recordings",
            "Metadata",
            "Created At",
            "Version",
            "Latest Version",
        ]
    }

    fn fields(&self) -> Vec<String> {
        vec![
            self.id.clone(),
            self.project_id.clone(),
            self.start_time.clone(),
            self.end_time.clone(),
            self.recording_ids(),
            compact_json(&self.metadata),
            self.created_at.clone(),
            self.version
                .map(|version| version.to_string())
                .unwrap_or_default(),
            self.latest_version
                .map(|version| version.to_string())
                .unwrap_or_default(),
        ]
    }
}

#[derive(Deserialize)]
pub(crate) struct EpisodeListResponse {
    #[serde(default)]
    pub(crate) episodes: Vec<Episode>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct EpisodeListQuery<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    cursor: Option<&'a str>,
    #[serde(skip_serializing_if = "String::is_empty")]
    end: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    has_missing_recordings: Option<bool>,
    #[serde(skip_serializing_if = "String::is_empty")]
    include: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    label_query: Option<&'a str>,
    limit: i64,
    #[serde(skip_serializing_if = "String::is_empty")]
    project_id: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    recording_id: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    sort_by: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    sort_order: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    start: String,
}

pub(crate) fn episode_includes(recordings: bool, labels: bool) -> String {
    let mut include = Vec::new();
    if recordings {
        include.push("recordings");
    }
    if labels {
        include.push("labels");
    }
    include.join(",")
}

pub(crate) fn parse_time_range(
    start: Option<&str>,
    end: Option<&str>,
) -> Result<(String, String), String> {
    let start = parse_timestamp(start.unwrap_or_default(), "start")?;
    let end = parse_timestamp(end.unwrap_or_default(), "end")?;
    if start.is_empty() != end.is_empty() {
        return Err("both --start and --end must be specified, or neither".to_owned());
    }
    Ok((start, end))
}

pub(crate) async fn list_episodes(
    runtime: &Runtime,
    args: &EpisodeListArgs,
    format: Format,
) -> Outcome {
    let (start, end) = match parse_time_range(args.start.as_deref(), args.end.as_deref()) {
        Ok(range) => range,
        Err(error) => return Outcome::failure(format!("{error}\n")),
    };
    let limit = args.limit.unwrap_or(DEFAULT_LIST_LIMIT);
    let query = EpisodeListQuery {
        cursor: args.cursor.as_deref().filter(|cursor| !cursor.is_empty()),
        end,
        has_missing_recordings: args.has_missing_recordings,
        include: episode_includes(args.include_recordings, args.include_labels),
        label_query: args.label_query.as_deref(),
        limit,
        project_id: args.project_id.clone().or_project(&runtime.project_id),
        recording_id: args.recording_id.clone().unwrap_or_default(),
        sort_by: args.sort_by.clone().unwrap_or_default(),
        sort_order: args.sort_order.clone().unwrap_or_default(),
        start,
    };
    match runtime
        .client
        .get_page::<_, EpisodeListResponse>("/v1/episodes", &query)
        .await
    {
        Ok(page) => warn_if_has_next_cursor(
            format_list_output(
                &page.data.episodes,
                format,
                NextCursor::Page(page.next_cursor.as_deref()),
            ),
            page.next_cursor.as_deref(),
        ),
        Err(error) => Outcome::failure(format!("Failed to list episodes: {error}\n")),
    }
}

fn episode_endpoint(id: &str) -> String {
    format!("/v1/episodes/{}", encode_path_segment(id))
}

#[derive(Serialize)]
struct EpisodeGetQuery {
    #[serde(skip_serializing_if = "Option::is_none")]
    version: Option<i32>,

    #[serde(skip_serializing_if = "String::is_empty")]
    include: String,
}

pub(crate) async fn get_episode(
    runtime: &Runtime,
    args: &EpisodeGetArgs,
    format: Format,
) -> Outcome {
    let query = EpisodeGetQuery {
        version: args.version,
        include: episode_includes(args.include_recordings, args.include_labels),
    };
    match runtime
        .client
        .get::<_, Episode>(&episode_endpoint(&args.episode_id), &query)
        .await
    {
        Ok(episode) => format_record(&episode, format),
        Err(error) if error.is_not_found_for("episode") => {
            Outcome::failure(format!("Episode not found: {}\n", args.episode_id))
        }
        Err(error) => Outcome::failure(format!("Failed to get episode: {error}\n")),
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CreateEpisodesRequest {
    project_id: String,
    episodes: Vec<EpisodeInput>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct EpisodeInput {
    #[serde(skip_serializing_if = "Option::is_none")]
    event_id: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    recordings: Vec<String>,
    #[serde(skip_serializing_if = "String::is_empty")]
    start_time: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    end_time: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    metadata: Option<Value>,
}

#[derive(Deserialize)]
struct CreateEpisodesResponse {
    #[serde(default)]
    episodes: Vec<CreatedEpisode>,
}

#[derive(Deserialize)]
struct CreatedEpisode {
    id: String,
    created: bool,
}

fn episode_input(args: &EpisodeAddArgs) -> Result<EpisodeInput, String> {
    let start_time = parse_timestamp_millis(args.start.as_deref().unwrap_or_default(), "start")?;
    let end_time = parse_timestamp_millis(args.end.as_deref().unwrap_or_default(), "end")?;
    if start_time.is_empty() != end_time.is_empty() {
        return Err("both --start and --end must be specified, or neither".to_owned());
    }
    let metadata = args
        .metadata
        .as_deref()
        .map(|raw| match serde_json::from_str::<Value>(raw) {
            Ok(value @ Value::Object(_)) => Ok(value),
            _ => Err(format!("--metadata must be a JSON object: {raw}")),
        })
        .transpose()?;
    Ok(EpisodeInput {
        event_id: args.event_id.clone(),
        recordings: args.recording_id.clone(),
        start_time,
        end_time,
        metadata,
    })
}

pub(crate) async fn add_episode(runtime: &Runtime, args: &EpisodeAddArgs) -> Outcome {
    let episode = match episode_input(args) {
        Ok(episode) => episode,
        Err(error) => return Outcome::failure(format!("{error}\n")),
    };
    let project_id = args.project_id.clone().or_project(&runtime.project_id);
    if project_id.is_empty() {
        return Outcome::failure("--project-id is required when creating an episode\n");
    }
    let request = CreateEpisodesRequest {
        project_id,
        episodes: vec![episode],
    };
    match runtime
        .client
        .post::<_, CreateEpisodesResponse>("/v1/episodes", &request)
        .await
    {
        Ok(response) => match response.episodes.first() {
            Some(episode) if episode.created => {
                Outcome::notice(format!("Episode created: {}\n", episode.id))
            }
            Some(episode) if args.metadata.is_some() => Outcome::notice(format!(
                "Episode already exists: {} (its metadata is unchanged)\n",
                episode.id
            )),
            Some(episode) => Outcome::notice(format!("Episode already exists: {}\n", episode.id)),
            None => Outcome::failure("Failed to create episode: the API returned no episode\n"),
        },
        Err(error) if error.is_not_found_for("project") => {
            Outcome::failure(format!("Project not found: {}\n", request.project_id))
        }
        Err(error) => Outcome::failure(format!("Failed to create episode: {error}\n")),
    }
}

pub(crate) async fn delete_episode(runtime: &Runtime, args: &EpisodeIdArgs) -> Outcome {
    match runtime
        .client
        .delete(&episode_endpoint(&args.episode_id))
        .await
    {
        Ok(()) => Outcome::notice(format!("Episode deleted: {}\n", args.episode_id)),
        Err(error) if error.is_not_found_for("episode") => {
            Outcome::failure(format!("Episode not found: {}\n", args.episode_id))
        }
        Err(error) => Outcome::failure(format!("Failed to delete episode: {error}\n")),
    }
}

pub(crate) async fn edit_episode(
    runtime: &Runtime,
    args: &EpisodeEditArgs,
    format: Format,
) -> Outcome {
    let request = match episode_patch(args) {
        Ok(request) => request,
        Err(error) => return Outcome::failure(format!("{error}\n")),
    };
    match runtime
        .client
        .patch::<_, _, Episode>(&episode_endpoint(&args.episode_id), &(), &request)
        .await
    {
        Ok(episode) => format_record(&episode, format),
        Err(error) if error.is_not_found_for("episode") => {
            Outcome::failure(format!("Episode not found: {}\n", args.episode_id))
        }
        Err(error @ ApiError::Response { status: 409, .. }) => Outcome::failure(format!(
            "Failed to edit episode: {error}\nGet the episode again and review its current state before retrying with --applies-to-version set to its current version.\n"
        )),
        Err(error) => Outcome::failure(format!("Failed to edit episode: {error}\n")),
    }
}

fn episode_patch(args: &EpisodeEditArgs) -> Result<Value, String> {
    let mut patch = serde_json::Map::new();
    for (key, value, flag) in [
        ("startTime", args.start.as_deref(), "start"),
        ("endTime", args.end.as_deref(), "end"),
    ] {
        if let Some(value) = value {
            if value.trim().is_empty() {
                return Err(format!("--{flag} cannot be empty"));
            }
            patch.insert(key.to_owned(), parse_timestamp_millis(value, flag)?.into());
        }
    }
    // A precondition-only patch checks the current version without changing it.
    if let Some(version) = args.applies_to_version {
        patch.insert("appliesToVersion".to_owned(), version.into());
    }
    let mut recordings = serde_json::Map::new();
    for (key, ids) in [
        ("add", &args.add_recording),
        ("remove", &args.remove_recording),
    ] {
        if !ids.is_empty() {
            recordings.insert(key.to_owned(), serde_json::json!(ids));
        }
    }
    if !recordings.is_empty() {
        patch.insert("recordings".to_owned(), recordings.into());
    }
    if let Some(raw) = &args.labels {
        let labels: Value =
            serde_json::from_str(raw).map_err(|error| format!("Invalid --labels JSON: {error}"))?;
        if !labels.is_object() {
            return Err("--labels must be a JSON object".to_owned());
        }
        patch.insert("labels".to_owned(), labels);
    }
    if patch.is_empty() {
        return Err("Nothing to update".to_owned());
    }
    Ok(patch.into())
}

#[derive(Deserialize, Serialize)]
struct EpisodeUpdateResult {
    id: String,
    version: i32,
}

impl Record for EpisodeUpdateResult {
    fn headers() -> &'static [&'static str] {
        &["ID", "Version"]
    }
    fn fields(&self) -> Vec<String> {
        vec![self.id.clone(), self.version.to_string()]
    }
}

#[derive(Deserialize)]
struct EpisodeUpdatesResponse {
    episodes: Vec<EpisodeUpdateResult>,
}

fn bulk_patch(raw: &[u8]) -> Result<Value, String> {
    let request: Value =
        serde_json::from_slice(raw).map_err(|error| format!("Invalid patch JSON: {error}"))?;
    let episodes = request
        .get("episodes")
        .and_then(Value::as_array)
        .ok_or("Patch file must contain an episodes array")?;
    if !(1..=2000).contains(&episodes.len()) {
        return Err(format!(
            "Patch file contains {} episodes; it must contain 1-2000",
            episodes.len()
        ));
    }
    let mut ids = std::collections::HashMap::new();
    for (index, episode) in episodes.iter().enumerate() {
        let id = episode
            .get("id")
            .and_then(Value::as_str)
            .filter(|id| !id.trim().is_empty())
            .ok_or_else(|| format!("Patch {} needs a nonempty episode id", index + 1))?;
        if let Some(first) = ids.insert(id, index + 1) {
            return Err(format!(
                "Patches {first} and {} have the same episode ID: {id}",
                index + 1
            ));
        }
    }
    Ok(request)
}

pub(crate) async fn edit_episodes(
    runtime: &Runtime,
    args: &EpisodeBulkEditArgs,
    format: Format,
) -> Outcome {
    let raw = match tokio::fs::read(&args.file).await {
        Ok(raw) => raw,
        Err(error) => {
            return Outcome::failure(format!("Failed to read {}: {error}\n", args.file.display()))
        }
    };
    let request = match bulk_patch(&raw) {
        Ok(request) => request,
        Err(error) => return Outcome::failure(format!("{error}\n")),
    };
    match runtime
        .client
        .patch::<_, _, EpisodeUpdatesResponse>("/v1/episodes", &(), &request)
        .await
    {
        Ok(response) => format_list_output(&response.episodes, format, NextCursor::NotPaginated),
        Err(error @ ApiError::Response { status: 409, .. }) => Outcome::failure(format!(
            "Failed to edit episodes: {error}\nNo patches were applied. Get the conflicting episodes again and review their changes, then update appliesToVersion in the file before retrying.\n"
        )),
        Err(error) => Outcome::failure(format!("Failed to edit episodes: {error}\n")),
    }
}

#[cfg(test)]
mod tests {
    use super::{bulk_patch, parse_time_range, Episode};

    #[test]
    fn invalid_bulk_patches_are_rejected() {
        for (raw, expected) in [
            (r#"{"episodes":[]}"#, "1-2000"),
            (
                r#"{"episodes":[{"id":"ep_one"},{"id":"ep_one"}]}"#,
                "Patches 1 and 2 have the same episode ID: ep_one",
            ),
            (
                r#"{"episodes":[{}]}"#,
                "Patch 1 needs a nonempty episode id",
            ),
            (r#"{"episodes":{}}"#, "episodes array"),
            (
                r#"{"episodes":[{"id":"  "}]}"#,
                "Patch 1 needs a nonempty episode id",
            ),
        ] {
            assert!(
                bulk_patch(raw.as_bytes()).unwrap_err().contains(expected),
                "{raw}"
            );
        }
        assert!(bulk_patch(br#"{"episodes":[{"id":"ep_one"}]}"#).is_ok());
    }

    #[test]
    fn an_absent_recordings_list_is_omitted_from_json_output() {
        let record: Episode = serde_json::from_value(serde_json::json!({
            "id": "ep_fixture",
            "projectId": "prj_default",
            "startTime": "2024-01-02T03:04:05Z",
            "endTime": "2024-01-02T03:04:06Z",
            "metadata": {},
            "createdAt": "2024-01-02T03:04:07Z",
        }))
        .unwrap();
        let output = serde_json::to_value(&record).unwrap();
        assert!(output.get("recordings").is_none());
        assert_eq!(record.recording_ids(), "");
    }

    #[test]
    fn included_recordings_render_as_a_comma_separated_cell() {
        let record: Episode = serde_json::from_value(serde_json::json!({
            "id": "ep_fixture",
            "projectId": "prj_default",
            "startTime": "2024-01-02T03:04:05Z",
            "endTime": "2024-01-02T03:04:06Z",
            "metadata": {},
            "createdAt": "2024-01-02T03:04:07Z",
            "recordings": [
                {"id": "rec_one", "path": "one.mcap", "start": "", "end": "", "available": true, "resolvable": true},
                {"id": "rec_two", "path": "two.mcap", "start": "", "end": "", "available": false},
            ],
        }))
        .unwrap();
        assert_eq!(record.recording_ids(), "rec_one,rec_two");
        assert_eq!(
            record.recordings.as_ref().unwrap()[0].resolvable,
            Some(true)
        );
    }

    #[test]
    fn a_recording_location_is_kept_in_json_output() {
        let record: Episode = serde_json::from_value(serde_json::json!({
            "id": "ep_fixture",
            "projectId": "prj_default",
            "startTime": "2024-01-02T03:04:05Z",
            "endTime": "2024-01-02T03:04:06Z",
            "metadata": {},
            "createdAt": "2024-01-02T03:04:07Z",
            "recordings": [
                {
                    "id": "rec_s3",
                    "path": "one.mcap",
                    "location": {"bucket": "robot-logs", "path": "fleet/one.mcap"},
                    "start": "",
                    "end": "",
                    "available": true,
                },
                {
                    "id": "rec_azure",
                    "path": "two.mcap",
                    "location": {
                        "scheme": "az",
                        "bucket": "robot-logs",
                        "path": "fleet/two.mcap",
                        "azureStorageAccountName": "fleetstorage",
                    },
                    "start": "",
                    "end": "",
                    "available": true,
                },
                {"id": "rec_imported", "path": "three.mcap", "start": "", "end": "", "available": true},
            ],
        }))
        .unwrap();
        let output = serde_json::to_value(&record).unwrap();
        let recordings = &output["recordings"];
        assert_eq!(
            recordings[0]["location"],
            serde_json::json!({"bucket": "robot-logs", "path": "fleet/one.mcap"})
        );
        assert_eq!(
            recordings[1]["location"],
            serde_json::json!({
                "scheme": "az",
                "bucket": "robot-logs",
                "path": "fleet/two.mcap",
                "azureStorageAccountName": "fleetstorage",
            })
        );
        assert!(recordings[2].get("location").is_none());
    }

    #[test]
    fn a_half_open_time_range_is_rejected_before_sending_a_request() {
        assert_eq!(
            parse_time_range(Some("2024-01-02"), None).unwrap_err(),
            "both --start and --end must be specified, or neither"
        );
        assert_eq!(
            parse_time_range(None, Some("2024-01-02")).unwrap_err(),
            "both --start and --end must be specified, or neither"
        );
        assert_eq!(
            parse_time_range(Some("2024-01-02"), Some("2024-01-03")).unwrap(),
            (
                "2024-01-02T00:00:00Z".to_owned(),
                "2024-01-03T00:00:00Z".to_owned()
            )
        );
        assert_eq!(
            parse_time_range(None, None).unwrap(),
            (String::new(), String::new())
        );
    }
}
