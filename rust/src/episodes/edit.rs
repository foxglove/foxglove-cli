//! Episode updates, one at a time or atomically from a JSON file, with optional version preconditions.

use std::collections::{BTreeMap, HashMap};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{episode_endpoint, Episode};
use crate::api::ApiError;
use crate::cli::{EpisodeBulkEditArgs, EpisodeEditArgs};
use crate::custom_properties::properties;
use crate::output::Format;
use crate::records::{format_list_output, parse_timestamp_millis, NextCursor, Record};
use crate::runtime::Runtime;
use crate::Outcome;

/// Most episodes one bulk edit request may change.
const MAX_BULK_EDITS: usize = 2000;

#[derive(Serialize)]
struct RecordingChanges<'a> {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    add: Vec<&'a str>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    remove: Vec<&'a str>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct EpisodePatch<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    applies_to_version: Option<i32>,
    #[serde(skip_serializing_if = "String::is_empty")]
    start_time: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    end_time: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    recordings: Option<RecordingChanges<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    properties: Option<BTreeMap<String, Value>>,
}

fn episode_patch(args: &EpisodeEditArgs) -> Result<EpisodePatch<'_>, String> {
    if args.property.is_empty()
        && args.remove_property.is_empty()
        && args.start.is_none()
        && args.end.is_none()
        && args.add_recording_id.is_empty()
        && args.remove_recording_id.is_empty()
    {
        return Err("Nothing to update".to_owned());
    }
    for property in &args.property {
        let Some((key, _)) = property.split_once(':') else {
            return Err(format!("invalid key/value pair: {property}"));
        };
        if args.remove_property.iter().any(|removed| removed == key) {
            return Err(format!(
                "cannot set and remove property {key} in the same edit"
            ));
        }
    }
    if let Some(id) = args
        .add_recording_id
        .iter()
        .find(|id| args.remove_recording_id.contains(id))
    {
        return Err(format!(
            "cannot add and remove recording {id} in the same edit"
        ));
    }
    let recordings = if args.add_recording_id.is_empty() && args.remove_recording_id.is_empty() {
        None
    } else {
        Some(RecordingChanges {
            add: args.add_recording_id.iter().map(String::as_str).collect(),
            remove: args
                .remove_recording_id
                .iter()
                .map(String::as_str)
                .collect(),
        })
    };
    Ok(EpisodePatch {
        applies_to_version: args.applies_to_version,
        start_time: parse_timestamp_millis(args.start.as_deref().unwrap_or_default(), "start")?,
        end_time: parse_timestamp_millis(args.end.as_deref().unwrap_or_default(), "end")?,
        recordings,
        properties: None,
    })
}

pub(crate) async fn edit_episode(runtime: &Runtime, args: &EpisodeEditArgs) -> Outcome {
    let mut patch = match episode_patch(args) {
        Ok(patch) => patch,
        Err(error) => return Outcome::failure(format!("{error}\n")),
    };
    patch.properties = match properties(runtime, "episode", &args.property).await {
        Ok(properties) => properties,
        Err(error) => return Outcome::failure(format!("Failed to edit episode: {error}\n")),
    };
    if !args.remove_property.is_empty() {
        let properties = patch.properties.get_or_insert_with(BTreeMap::new);
        for key in &args.remove_property {
            properties.insert(key.clone(), Value::Null);
        }
    }
    match runtime
        .client
        .patch::<_, _, Episode>(&episode_endpoint(&args.episode_id), &(), &patch)
        .await
    {
        Ok(episode) => Outcome::notice(format!(
            "Episode updated: {} (version {})\n",
            episode.id, episode.version
        )),
        // A 404 can also name missing recordings, so match the episode message exactly.
        Err(error)
            if error.is_bare_not_found()
                || error
                    .to_string()
                    .eq_ignore_ascii_case("One or more episodes were not found") =>
        {
            Outcome::failure(format!("Episode not found: {}\n", args.episode_id))
        }
        // A 409 can also mean the episode cannot take another version, so only point
        // at the latest version when the edit named one.
        Err(error @ ApiError::Response { status: 409, .. }) if args.applies_to_version.is_some() => {
            Outcome::failure(format!(
                "Failed to edit episode: {error}\nRun foxglove episodes get {} to see its latest version.\n",
                args.episode_id
            ))
        }
        Err(error) => Outcome::failure(format!("Failed to edit episode: {error}\n")),
    }
}

#[derive(Deserialize, Serialize)]
struct EpisodeEditResult {
    id: String,
    version: i32,
}

impl Record for EpisodeEditResult {
    fn headers() -> &'static [&'static str] {
        &["ID", "Version"]
    }

    fn fields(&self) -> Vec<String> {
        vec![self.id.clone(), self.version.to_string()]
    }
}

#[derive(Deserialize)]
struct EpisodeEditsResponse {
    episodes: Vec<EpisodeEditResult>,
}

/// Checks the edit count and IDs locally and returns the file's JSON unchanged, so
/// omitted fields and explicit property removals reach the API as written.
fn bulk_edits(raw: &[u8]) -> Result<Value, String> {
    let request: Value =
        serde_json::from_slice(raw).map_err(|error| format!("Invalid edit JSON: {error}"))?;
    let episodes = request
        .get("episodes")
        .and_then(Value::as_array)
        .ok_or("The file must contain an episodes array")?;
    if !(1..=MAX_BULK_EDITS).contains(&episodes.len()) {
        return Err(format!(
            "The file contains {} edits; it must contain 1-{MAX_BULK_EDITS}",
            episodes.len()
        ));
    }
    let mut ids = HashMap::new();
    for (index, episode) in episodes.iter().enumerate() {
        let id = episode
            .get("id")
            .and_then(Value::as_str)
            .filter(|id| !id.trim().is_empty())
            .ok_or_else(|| format!("Edit {} needs a nonempty episode id", index + 1))?;
        if let Some(first) = ids.insert(id, index + 1) {
            return Err(format!(
                "Edits {first} and {} have the same episode ID: {id}",
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
    let request = match bulk_edits(&raw) {
        Ok(request) => request,
        Err(error) => return Outcome::failure(format!("{error}\n")),
    };
    match runtime
        .client
        .patch::<_, _, EpisodeEditsResponse>("/v1/episodes", &(), &request)
        .await
    {
        Ok(response) => format_list_output(&response.episodes, format, NextCursor::NotPaginated),
        Err(error @ ApiError::Response { status: 409, .. }) => Outcome::failure(format!(
            "Failed to edit episodes: {error}\nNo edits were applied. Get the conflicting episodes again and review their changes, then update appliesToVersion in the file before retrying.\n"
        )),
        Err(error) => Outcome::failure(format!("Failed to edit episodes: {error}\n")),
    }
}

#[cfg(test)]
mod tests {
    use super::bulk_edits;

    #[test]
    fn invalid_bulk_edits_are_rejected_before_sending() {
        for (raw, expected) in [
            (r#"{"episodes":[]}"#, "it must contain 1-2000"),
            (
                r#"{"episodes":[{"id":"ep_one"},{"id":"ep_one"}]}"#,
                "Edits 1 and 2 have the same episode ID: ep_one",
            ),
            (r#"{"episodes":[{}]}"#, "Edit 1 needs a nonempty episode id"),
            (
                r#"{"episodes":[{"id":"  "}]}"#,
                "Edit 1 needs a nonempty episode id",
            ),
            (r#"{"episodes":{}}"#, "episodes array"),
            ("[", "Invalid edit JSON"),
        ] {
            assert!(
                bulk_edits(raw.as_bytes()).unwrap_err().contains(expected),
                "{raw}"
            );
        }
        assert!(bulk_edits(br#"{"episodes":[{"id":"ep_one"}]}"#).is_ok());
    }
}
