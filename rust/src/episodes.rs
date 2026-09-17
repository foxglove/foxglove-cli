//! Episode commands.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::cli::EpisodeListArgs;
use crate::output::Format;
use crate::records::{
    compact_json, format_output, is_zero, optional_bool, parse_timestamp, warn_if_truncated,
    ProjectFallback, Record, DEFAULT_LIST_LIMIT,
};
use crate::runtime::Runtime;
use crate::Outcome;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(crate) struct EpisodeRecording {
    pub(crate) id: String,
    #[serde(default)]
    pub(crate) path: String,
    #[serde(default)]
    pub(crate) start: String,
    #[serde(default)]
    pub(crate) end: String,
    #[serde(rename = "deviceId", skip_serializing_if = "String::is_empty", default)]
    pub(crate) device_id: String,
    #[serde(default)]
    pub(crate) available: bool,
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
            "Missing Recordings",
            "Metadata",
            "Created At",
        ]
    }

    fn fields(&self) -> Vec<String> {
        vec![
            self.id.clone(),
            self.project_id.clone(),
            self.start_time.clone(),
            self.end_time.clone(),
            self.recording_ids(),
            optional_bool(self.has_missing_recordings),
            compact_json(&self.metadata),
            self.created_at.clone(),
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
struct EpisodeListQuery {
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

pub(crate) fn include_recordings(requested: bool) -> String {
    if requested {
        "recordings".to_owned()
    } else {
        String::new()
    }
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
        end,
        has_missing_recordings: args.has_missing_recordings,
        include: include_recordings(args.include_recordings),
        limit,
        offset: args.offset.unwrap_or_default(),
        project_id: args.project_id.clone().or_project(&runtime.project_id),
        recording_id: args.recording_id.clone().unwrap_or_default(),
        sort_by: args.sort_by.clone().unwrap_or_default(),
        sort_order: args.sort_order.clone().unwrap_or_default(),
        start,
    };
    match runtime
        .client
        .get::<_, EpisodeListResponse>("/v1/episodes", &query)
        .await
    {
        Ok(mut response) => {
            for episode in &mut response.episodes {
                episode.has_missing_recordings = episode
                    .has_missing_recordings
                    .or(args.has_missing_recordings);
            }
            let count = response.episodes.len();
            warn_if_truncated(format_output(&response.episodes, format), count, limit)
        }
        Err(error) => Outcome::failure(format!("Failed to list episodes: {error}\n")),
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_time_range, Episode};

    #[test]
    fn missing_optional_fields_are_omitted_from_json_output() {
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
        for field in ["recordings", "hasMissingRecordings"] {
            assert!(output.get(field).is_none(), "{field}");
        }
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
            "hasMissingRecordings": true,
            "recordings": [
                {"id": "rec_one", "path": "one.mcap", "start": "", "end": "", "available": true},
                {"id": "rec_two", "path": "two.mcap", "start": "", "end": "", "available": false},
            ],
        }))
        .unwrap();
        assert_eq!(record.recording_ids(), "rec_one,rec_two");
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
