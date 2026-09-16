//! Dataset commands.

use serde::{Deserialize, Serialize};

use crate::api::encode_path_segment;
use crate::cli::{DatasetEpisodeListArgs, DatasetListArgs};
use crate::episodes::{include_recordings, parse_time_range, Episode};
use crate::output::Format;
use crate::records::{
    compact_json, creator_name, fetch_list, format_output, is_zero, null_to_default, optional_bool,
    Creator, ProjectFallback, Record,
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
    #[serde(skip_serializing_if = "Option::is_none", default)]
    creator: Option<Creator>,
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
            "Created By",
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
            creator_name(self.creator.as_ref()),
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
            "Missing Recordings",
            "Metadata",
            "Added At",
            "Added In Version",
            "Created By",
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
            optional_bool(self.has_missing_recordings),
            compact_json(&self.episode.metadata),
            self.added_at.clone(),
            self.added_in_version.to_string(),
            creator_name(self.episode.creator.as_ref()),
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
    #[serde(skip_serializing_if = "is_zero")]
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
    #[serde(skip_serializing_if = "is_zero")]
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
    let query = DatasetListQuery {
        limit: args.limit.unwrap_or_default(),
        offset: args.offset.unwrap_or_default(),
        project_id: args.project_id.clone().or_project(&runtime.project_id),
        sort_by: args.sort_by.clone().unwrap_or_default(),
        sort_order: args.sort_order.clone().unwrap_or_default(),
    };
    fetch_list::<Dataset, _>(
        runtime,
        format,
        "Failed to list datasets",
        "/v1/datasets",
        &query,
    )
    .await
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
    let query = DatasetEpisodeListQuery {
        end,
        has_missing_recordings: args.has_missing_recordings,
        include: include_recordings(args.include_recordings),
        limit: args.limit.unwrap_or_default(),
        offset: args.offset.unwrap_or_default(),
        recording_id: args.recording_id.clone().unwrap_or_default(),
        sort_by: args.sort_by.clone().unwrap_or_default(),
        sort_order: args.sort_order.clone().unwrap_or_default(),
        start,
    };
    let endpoint = format!(
        "/v1/datasets/{}/episodes",
        encode_path_segment(&args.dataset_id)
    );
    match runtime
        .client
        .get::<_, DatasetEpisodeListResponse>(&endpoint, &query)
        .await
    {
        Ok(response) => format_output(&response.episodes, format),
        Err(error) if error.is_not_found() => {
            Outcome::failure(format!("Dataset not found: {}\n", args.dataset_id))
        }
        Err(error) => Outcome::failure(format!("Failed to list dataset episodes: {error}\n")),
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
    fn the_creator_display_name_fills_the_created_by_cell() {
        let record: Dataset = serde_json::from_value(serde_json::json!({
            "id": "ds_fixture",
            "projectId": "prj_default",
            "name": "Fixture",
            "createdAt": "2024-01-02T03:04:05Z",
            "updatedAt": "2024-01-02T03:04:06Z",
            "creator": {"type": "api-key", "displayName": "CI key", "isDeleted": false},
        }))
        .unwrap();
        assert_eq!(record.fields()[5], "CI key");
    }

    #[test]
    fn membership_reports_missing_recordings_over_the_nested_episode() {
        let record: DatasetEpisode = serde_json::from_value(serde_json::json!({
            "addedAt": "2024-01-02T03:04:08Z",
            "addedInVersion": 3,
            "hasMissingRecordings": true,
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
        assert_eq!(fields[5], "true");
        assert_eq!(fields[6], r#"{"run":7}"#);
        assert_eq!(fields[8], "3");
    }
}
