//! Single episode updates with optional version preconditions.

use std::collections::BTreeMap;

use serde::Serialize;
use serde_json::Value;

use super::{episode_endpoint, Episode};
use crate::api::ApiError;
use crate::cli::EpisodeEditArgs;
use crate::custom_properties::properties;
use crate::records::parse_timestamp_millis;
use crate::runtime::Runtime;
use crate::Outcome;

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
