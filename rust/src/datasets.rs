//! Dataset commands.

mod download;
mod versions;

pub(crate) use download::download_dataset;
pub(crate) use versions::{
    commit_dataset, compare_versions, discard_dataset, get_version, list_versions, restore_version,
};

use std::collections::HashSet;
use std::fmt::Write as _;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::api::encode_path_segment;
use crate::cli::{
    DatasetAddArgs, DatasetEditArgs, DatasetEpisodeListArgs, DatasetEpisodeMutationArgs,
    DatasetGetArgs, DatasetIdArgs, DatasetListArgs,
};
use crate::episodes::{include_recordings, parse_time_range, Episode};
use crate::output::Format;
use crate::records::{
    compact_json, format_output, format_record, is_zero, null_to_default, plural,
    warn_if_truncated, ProjectFallback, Record, DEFAULT_LIST_LIMIT,
};
use crate::runtime::Runtime;
use crate::Outcome;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
struct Dataset {
    id: String,
    #[serde(rename = "projectId")]
    project_id: String,
    name: String,
    #[serde(default, deserialize_with = "null_to_default")]
    description: String,
    #[serde(rename = "episodeCount", default, deserialize_with = "null_to_default")]
    episode_count: i64,
    #[serde(rename = "createdAt")]
    created_at: String,
    #[serde(rename = "updatedAt")]
    updated_at: String,
}

impl Record for Dataset {
    fn headers() -> &'static [&'static str] {
        &[
            "ID",
            "Name",
            "Project ID",
            "Description",
            "Episode Count",
            "Created At",
            "Updated At",
        ]
    }

    fn fields(&self) -> Vec<String> {
        vec![
            self.id.clone(),
            self.name.clone(),
            self.project_id.clone(),
            self.description.clone(),
            self.episode_count.to_string(),
            self.created_at.clone(),
            self.updated_at.clone(),
        ]
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
struct DatasetEpisode {
    #[serde(rename = "addedAt")]
    added_at: String,
    #[serde(rename = "addedInVersion")]
    added_in_version: i64,
    #[serde(
        rename = "hasMissingRecordings",
        skip_serializing_if = "Option::is_none",
        default
    )]
    has_missing_recordings: Option<bool>,
    episode: Episode,
}

impl Record for DatasetEpisode {
    fn headers() -> &'static [&'static str] {
        &[
            "Episode ID",
            "Project ID",
            "Start Time",
            "End Time",
            "Recordings",
            "Metadata",
            "Added At",
            "Added In Version",
            "Created At",
        ]
    }

    fn fields(&self) -> Vec<String> {
        vec![
            self.episode.id.clone(),
            self.episode.project_id.clone(),
            self.episode.start_time.clone(),
            self.episode.end_time.clone(),
            self.episode.recording_ids(),
            compact_json(&self.episode.metadata),
            self.added_at.clone(),
            self.added_in_version.to_string(),
            self.episode.created_at.clone(),
        ]
    }
}

#[derive(Deserialize)]
struct DatasetEpisodeListResponse {
    #[serde(default)]
    episodes: Vec<DatasetEpisode>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DatasetListQuery {
    limit: i64,
    #[serde(skip_serializing_if = "is_zero")]
    offset: i64,
    #[serde(skip_serializing_if = "String::is_empty")]
    project_id: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    sort_by: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    sort_order: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DatasetEpisodeListQuery {
    #[serde(skip_serializing_if = "String::is_empty")]
    end: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    has_missing_recordings: Option<bool>,
    #[serde(skip_serializing_if = "String::is_empty")]
    include: String,
    limit: i64,
    #[serde(skip_serializing_if = "is_zero")]
    offset: i64,
    #[serde(skip_serializing_if = "String::is_empty")]
    recording_id: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    sort_by: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    sort_order: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    start: String,
}

pub(crate) async fn list_datasets(
    runtime: &Runtime,
    args: &DatasetListArgs,
    format: Format,
) -> Outcome {
    let limit = args.limit.unwrap_or(DEFAULT_LIST_LIMIT);
    let query = DatasetListQuery {
        limit,
        offset: args.offset.unwrap_or_default(),
        project_id: args.project_id.clone().or_project(&runtime.project_id),
        sort_by: args.sort_by.clone().unwrap_or_default(),
        sort_order: args.sort_order.clone().unwrap_or_default(),
    };
    match runtime
        .client
        .get::<_, Vec<Dataset>>("/v1/datasets", &query)
        .await
    {
        Ok(datasets) => warn_if_truncated(format_output(&datasets, format), datasets.len(), limit),
        Err(error) => Outcome::failure(format!("Failed to list datasets: {error}\n")),
    }
}

pub(crate) async fn list_dataset_episodes(
    runtime: &Runtime,
    args: &DatasetEpisodeListArgs,
    format: Format,
) -> Outcome {
    let (start, end) = match parse_time_range(args.start.as_deref(), args.end.as_deref()) {
        Ok(range) => range,
        Err(error) => return Outcome::failure(format!("{error}\n")),
    };
    let limit = args.limit.unwrap_or(DEFAULT_LIST_LIMIT);
    let query = DatasetEpisodeListQuery {
        end,
        has_missing_recordings: args.has_missing_recordings,
        include: include_recordings(args.include_recordings),
        limit,
        offset: args.offset.unwrap_or_default(),
        recording_id: args.recording_id.clone().unwrap_or_default(),
        sort_by: args.sort_by.clone().unwrap_or_default(),
        sort_order: args.sort_order.clone().unwrap_or_default(),
        start,
    };
    let endpoint = match args.version {
        Some(version) => format!(
            "{}/versions/{version}/episodes",
            dataset_endpoint(&args.dataset_id)
        ),
        None => format!("{}/episodes", dataset_endpoint(&args.dataset_id)),
    };
    match runtime
        .client
        .get::<_, DatasetEpisodeListResponse>(&endpoint, &query)
        .await
    {
        Ok(response) => {
            let count = response.episodes.len();
            warn_if_truncated(format_output(&response.episodes, format), count, limit)
        }
        Err(error) if error.is_not_found() => match args.version {
            Some(version) => version_not_found(&args.dataset_id, version),
            None => dataset_not_found(&args.dataset_id),
        },
        Err(error) => Outcome::failure(format!("Failed to list dataset episodes: {error}\n")),
    }
}

fn dataset_endpoint(id: &str) -> String {
    format!("/v1/datasets/{}", encode_path_segment(id))
}

fn dataset_not_found(id: &str) -> Outcome {
    Outcome::failure(format!("Dataset not found: {id}\n"))
}

fn version_not_found(id: &str, version: i64) -> Outcome {
    Outcome::failure(format!("Version {version} of dataset {id} not found\n"))
}

fn commit_hint(id: &str) -> String {
    format!("Run foxglove datasets commit {id} to commit the draft as a new version.\n")
}

pub(crate) async fn get_dataset(
    runtime: &Runtime,
    args: &DatasetGetArgs,
    format: Format,
) -> Outcome {
    match runtime
        .client
        .get::<_, Dataset>(&dataset_endpoint(&args.dataset_id), &())
        .await
    {
        Ok(dataset) => format_record(&dataset, format),
        Err(error) if error.is_not_found() => dataset_not_found(&args.dataset_id),
        Err(error) => Outcome::failure(format!("Failed to get dataset: {error}\n")),
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CreateDatasetRequest {
    project_id: String,
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    episode_ids: Vec<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct EpisodesPatchResponse {
    #[serde(default)]
    added: usize,
    #[serde(default)]
    removed: usize,
    #[serde(default)]
    already_present: usize,
}

#[derive(Deserialize)]
struct CreateDatasetResponse {
    id: String,
    #[serde(flatten)]
    episodes: EpisodesPatchResponse,
}

fn additions(response: &EpisodesPatchResponse) -> String {
    let mut summary = format!("Added {}", plural(response.added, "episode"));
    if response.already_present > 0 {
        let _ = write!(summary, " ({} already present)", response.already_present);
    }
    summary.push('\n');
    summary
}

fn removals(response: &EpisodesPatchResponse, requested: &[String]) -> String {
    let mut summary = format!("Removed {}", plural(response.removed, "episode"));
    let absent = requested
        .iter()
        .collect::<HashSet<_>>()
        .len()
        .saturating_sub(response.removed);
    if absent > 0 {
        let _ = write!(summary, " ({absent} not in the dataset)");
    }
    summary.push('\n');
    summary
}

pub(crate) async fn add_dataset(runtime: &Runtime, args: &DatasetAddArgs) -> Outcome {
    if args.name.trim().is_empty() {
        return Outcome::failure("--name cannot be empty\n");
    }
    let project_id = args.project_id.clone().or_project(&runtime.project_id);
    if project_id.is_empty() {
        return Outcome::failure("--project-id is required when creating a dataset\n");
    }
    let request = CreateDatasetRequest {
        project_id,
        name: args.name.clone(),
        description: args
            .description
            .clone()
            .filter(|description| !description.is_empty()),
        episode_ids: args.episode_id.clone(),
    };
    match runtime
        .client
        .post::<_, CreateDatasetResponse>("/v1/datasets", &request)
        .await
    {
        Ok(response) => {
            let mut stderr = format!("Dataset created: {}\n", response.id);
            if !request.episode_ids.is_empty() {
                stderr.push_str(&additions(&response.episodes));
            }
            if response.episodes.added > 0 {
                stderr.push_str(&commit_hint(&response.id));
            }
            Outcome::notice(stderr)
        }
        Err(error) if error.is_not_found() => {
            Outcome::failure(format!("Project not found: {}\n", request.project_id))
        }
        Err(error) => Outcome::failure(format!("Failed to create dataset: {error}\n")),
    }
}

#[derive(Serialize)]
struct EditDatasetRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<Value>,
}

pub(crate) async fn edit_dataset(runtime: &Runtime, args: &DatasetEditArgs) -> Outcome {
    if args.name.is_none() && args.description.is_none() {
        return Outcome::failure("Nothing to update\n");
    }
    if args
        .name
        .as_deref()
        .is_some_and(|name| name.trim().is_empty())
    {
        return Outcome::failure("--name cannot be empty\n");
    }
    let request = EditDatasetRequest {
        name: args.name.clone(),
        description: args.description.clone().map(|description| {
            if description.is_empty() {
                Value::Null
            } else {
                Value::String(description)
            }
        }),
    };
    match runtime
        .client
        .patch::<_, _, Value>(&dataset_endpoint(&args.dataset_id), &(), &request)
        .await
    {
        Ok(_) => Outcome::notice(format!("Dataset updated: {}\n", args.dataset_id)),
        Err(error) if error.is_not_found() => dataset_not_found(&args.dataset_id),
        Err(error) => Outcome::failure(format!("Failed to edit dataset: {error}\n")),
    }
}

pub(crate) async fn delete_dataset(runtime: &Runtime, args: &DatasetIdArgs) -> Outcome {
    match runtime
        .client
        .delete(&dataset_endpoint(&args.dataset_id))
        .await
    {
        Ok(()) => Outcome::notice(format!("Dataset deleted: {}\n", args.dataset_id)),
        Err(error) if error.is_not_found() => Outcome::notice(format!(
            "Not found. The resource may have already been deleted.\nDataset deleted: {}\n",
            args.dataset_id
        )),
        Err(error) => Outcome::failure(format!("Failed to delete dataset: {error}\n")),
    }
}

#[derive(Serialize)]
struct PatchDatasetEpisodesRequest {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    add: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    remove: Vec<String>,
}

pub(crate) async fn patch_dataset_episodes(
    runtime: &Runtime,
    args: &DatasetEpisodeMutationArgs,
    add: bool,
) -> Outcome {
    let episode_ids = args.episode_ids.clone();
    let request = if add {
        PatchDatasetEpisodesRequest {
            add: episode_ids,
            remove: Vec::new(),
        }
    } else {
        PatchDatasetEpisodesRequest {
            add: Vec::new(),
            remove: episode_ids,
        }
    };
    match runtime
        .client
        .patch::<_, _, EpisodesPatchResponse>(
            &format!("{}/episodes", dataset_endpoint(&args.dataset_id)),
            &(),
            &request,
        )
        .await
    {
        Ok(response) => {
            let mut stderr = if add {
                additions(&response)
            } else {
                removals(&response, &args.episode_ids)
            };
            if response.added > 0 || response.removed > 0 {
                stderr.push_str(&commit_hint(&args.dataset_id));
            }
            Outcome::notice(stderr)
        }
        Err(error) if error.is_not_found() => dataset_not_found(&args.dataset_id),
        Err(error) => Outcome::failure(format!(
            "Failed to {} episodes: {error}\n",
            if add { "add" } else { "remove" }
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::{Dataset, DatasetEpisode};
    use crate::records::Record;

    #[test]
    fn missing_and_null_fields_render_explicit_defaults() {
        let original = serde_json::json!({"id":"ds_fixture","projectId":"prj_default","name":"Fixture","createdAt":"2024-01-02T03:04:05Z","updatedAt":"2024-01-02T03:04:06Z"});
        for (field, expected) in [
            ("description", serde_json::json!("")),
            ("episodeCount", serde_json::json!(0)),
        ] {
            let mut with_null = original.clone();
            with_null[field] = serde_json::Value::Null;
            for response in [original.clone(), with_null] {
                let record: Dataset = serde_json::from_value(response).unwrap();
                let output = serde_json::to_value(record).unwrap();
                assert_eq!(output[field], expected, "{field}");
            }
        }
    }

    #[test]
    fn membership_fields_render_around_the_nested_episode() {
        let record: DatasetEpisode = serde_json::from_value(serde_json::json!({
            "addedAt": "2024-01-02T03:04:08Z",
            "addedInVersion": 3,
            "episode": {
                "id": "ep_fixture",
                "projectId": "prj_default",
                "startTime": "2024-01-02T03:04:05Z",
                "endTime": "2024-01-02T03:04:06Z",
                "metadata": {"run": 7},
                "createdAt": "2024-01-02T03:04:07Z",
            },
        }))
        .unwrap();
        let fields = record.fields();
        assert_eq!(fields[0], "ep_fixture");
        assert_eq!(fields[5], r#"{"run":7}"#);
        assert_eq!(fields[6], "2024-01-02T03:04:08Z");
        assert_eq!(fields[7], "3");
    }
}
