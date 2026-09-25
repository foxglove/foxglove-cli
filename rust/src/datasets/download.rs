//! Dataset version download.

use std::collections::HashMap;
use std::fmt::Write as _;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use tokio_util::sync::CancellationToken;

use super::{Dataset, DatasetEpisode, DatasetEpisodeListResponse};
use crate::api::{encode_path_segment, ApiError, StreamRequest};
use crate::cli::DatasetDownloadArgs;
use crate::data::{resumable_download, CompletionCheck, ExportProgress};
use crate::records::DEFAULT_LIST_LIMIT;
use crate::runtime::Runtime;
use crate::Outcome;

const MANIFEST_FORMAT_VERSION: u32 = 1;
const MANIFEST_FILE_NAME: &str = "manifest.json";
const SLUG_MAX_CHARS: usize = 64;

const NO_STREAMABLE_RECORDINGS: &str = "NoStreamableRecordings";
const NO_DATA_LEFT_REASON: &str = "No Primary Site holds data for any recording in this episode. Import those recordings to include it.";

const EPISODE_ATTEMPTS: u32 = 3;
const RETRY_BACKOFF: Duration = Duration::from_secs(1);

#[derive(Clone, Debug, Default, Deserialize)]
struct DatasetVersion {
    #[serde(rename = "versionNumber")]
    version_number: i64,
    #[serde(rename = "committedAt", default)]
    committed_at: Option<String>,
}

#[derive(Deserialize)]
struct DatasetVersionListResponse {
    #[serde(default)]
    versions: Vec<DatasetVersion>,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ManifestDataset {
    id: String,
    name: String,
    project_id: String,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ManifestVersion {
    version_number: i64,
    committed_at: String,
}

#[derive(Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct ManifestSelection {
    #[serde(skip_serializing_if = "Option::is_none")]
    topics: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    include_attachments: Option<bool>,
    episode_count: usize,
}

#[derive(Clone, Copy, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
enum Status {
    Downloaded,
    Skipped,
    Failed,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ManifestEpisode {
    index: usize,
    id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    file: Option<String>,
    start_time: String,
    end_time: String,
    metadata: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    byte_size: Option<u64>,
    status: Status,
    #[serde(skip_serializing_if = "Option::is_none")]
    reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    episode_has_missing_recordings: Option<bool>,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct Manifest {
    format_version: u32,
    generated_at: String,
    dataset: ManifestDataset,
    version: ManifestVersion,
    selection: ManifestSelection,
    episodes: Vec<ManifestEpisode>,
}

fn safe_name(text: &str) -> String {
    let mut safe = String::new();
    let mut pending_dash = false;
    for character in text.chars() {
        if character.is_alphanumeric() || matches!(character, '.' | '_' | '-') {
            if pending_dash {
                safe.push('-');
                pending_dash = false;
            }
            safe.push(character);
        } else {
            pending_dash = true;
        }
    }
    if pending_dash {
        safe.push('-');
    }
    safe
}

fn default_directory_name(dataset_name: &str, version_number: i64) -> String {
    let slug: String = safe_name(dataset_name)
        .trim_matches('-')
        .chars()
        .take(SLUG_MAX_CHARS)
        .collect();
    let slug = if slug.is_empty() { "dataset" } else { &slug };
    format!("{slug}-v{version_number}")
}

fn episode_file_name(index: usize, episode_id: &str) -> String {
    format!("episode_{index:04}_{}.mcap", safe_name(episode_id))
}

fn manifest_prefix(directory: &Path, root: &str) -> String {
    std::fs::canonicalize(directory)
        .as_deref()
        .unwrap_or(directory)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(root)
        .to_owned()
}

fn topic_list(raw: Option<&str>) -> Option<Vec<String>> {
    let topics: Vec<String> = raw
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|topic| !topic.is_empty())
        .map(ToOwned::to_owned)
        .collect();
    (!topics.is_empty()).then_some(topics)
}

async fn fetch_dataset(runtime: &Runtime, id: &str) -> Result<Dataset, ApiError> {
    runtime
        .client
        .get(&format!("/v1/datasets/{}", encode_path_segment(id)), &())
        .await
}

async fn resolve_version(
    runtime: &Runtime,
    id: &str,
    requested: Option<i64>,
) -> Result<DatasetVersion, String> {
    let versions = format!("/v1/datasets/{}/versions", encode_path_segment(id));
    if let Some(number) = requested {
        let version: DatasetVersion = match runtime
            .client
            .get(&format!("{versions}/{number}"), &())
            .await
        {
            Ok(version) => version,
            Err(error) if error.is_not_found() => {
                return Err(format!("Version {number} is not a version of this dataset"))
            }
            Err(error) => return Err(format!("Failed to get dataset version: {error}")),
        };
        return if version.committed_at.is_some() {
            Ok(version)
        } else {
            Err(format!(
                "Version {number} is not a committed version of this dataset"
            ))
        };
    }
    let response: DatasetVersionListResponse = runtime
        .client
        .get(&versions, &[("sortOrder", "desc")])
        .await
        .map_err(|error| format!("Failed to list dataset versions: {error}"))?;
    response
        .versions
        .into_iter()
        .find(|version| version.committed_at.is_some())
        .ok_or_else(|| "This dataset has no committed version to download".to_owned())
}

async fn fetch_all_episodes(
    runtime: &Runtime,
    id: &str,
    version_number: i64,
) -> Result<Vec<DatasetEpisode>, ApiError> {
    let mut episodes = Vec::new();
    loop {
        let query = [
            ("limit".to_owned(), DEFAULT_LIST_LIMIT.to_string()),
            ("offset".to_owned(), episodes.len().to_string()),
            ("sortBy".to_owned(), "startTime".to_owned()),
            ("sortOrder".to_owned(), "asc".to_owned()),
        ];
        let page: DatasetEpisodeListResponse = runtime
            .client
            .get(
                &format!(
                    "/v1/datasets/{}/versions/{version_number}/episodes",
                    encode_path_segment(id)
                ),
                &query,
            )
            .await?;
        let received = page.episodes.len();
        episodes.extend(page.episodes);
        if i64::try_from(received).is_ok_and(|received| received < DEFAULT_LIST_LIMIT) {
            return Ok(episodes);
        }
    }
}

async fn write_episode(
    runtime: &Runtime,
    request: &StreamRequest,
    path: &Path,
    label: &str,
    cancellation: &CancellationToken,
) -> Result<u64, ApiError> {
    let mut progress = ExportProgress::labeled(label);
    let mut attempt = 1;
    loop {
        progress.restart();
        let result = resumable_download(
            runtime,
            request.clone(),
            path,
            cancellation,
            &mut progress,
            CompletionCheck::EndMagic,
        )
        .await
        .and_then(|()| {
            std::fs::metadata(path)
                .map(|metadata| metadata.len())
                .map_err(ApiError::Write)
        });
        if let Ok(written) = &result {
            progress.finish_at(*written);
        }
        let Err(error) = &result else { return result };
        if attempt >= EPISODE_ATTEMPTS || !error.is_retryable() {
            return result;
        }
        tokio::select! {
            () = cancellation.cancelled() => return Err(ApiError::Cancelled),
            () = tokio::time::sleep(RETRY_BACKOFF * attempt) => {}
        }
        attempt += 1;
    }
}

fn episode_request(
    entry: &DatasetEpisode,
    args: &DatasetDownloadArgs,
    topics: Option<&[String]>,
) -> Result<StreamRequest, ApiError> {
    let bound = |raw: &str| {
        OffsetDateTime::parse(raw, &Rfc3339)
            .map_err(|error| ApiError::Conversion(format!("Invalid episode time {raw:?}: {error}")))
    };
    Ok(StreamRequest {
        episode_id: entry.episode.id.clone(),
        start: Some(bound(&entry.episode.start_time)?),
        end: Some(bound(&entry.episode.end_time)?),
        output_format: "mcap".to_owned(),
        include_attachments: args.include_attachments,
        topics: topics.map(<[String]>::to_vec).unwrap_or_default(),
        ..StreamRequest::default()
    })
}

fn previously_downloaded(directory: &Path, current: &Manifest) -> HashMap<String, ManifestEpisode> {
    let Ok(encoded) = std::fs::read(directory.join(MANIFEST_FILE_NAME)) else {
        return HashMap::new();
    };
    let Ok(previous) = serde_json::from_slice::<Manifest>(&encoded) else {
        return HashMap::new();
    };
    if previous.dataset.id != current.dataset.id
        || previous.version.version_number != current.version.version_number
        || previous.selection != current.selection
    {
        return HashMap::new();
    }
    previous
        .episodes
        .into_iter()
        .filter(|episode| episode.file.is_some() && episode.byte_size.is_some())
        .map(|episode| (episode_file_name(episode.index, &episode.id), episode))
        .collect()
}

fn is_complete(earlier: &ManifestEpisode) -> bool {
    earlier.status == Status::Downloaded && earlier.episode_has_missing_recordings != Some(true)
}

enum EpisodeResult {
    Reused(u64),
    Downloaded(u64),
    Skipped,
    Failed(String),
    NotAttempted(String),
}

/// Builds the manifest entry and progress note for one episode. `kept` is a file an
/// earlier run left that this run did not replace.
fn manifest_entry(
    index: usize,
    entry: &DatasetEpisode,
    result: &EpisodeResult,
    kept: Option<u64>,
    file: String,
) -> (ManifestEpisode, Option<String>) {
    let missing_recordings = entry.has_missing_recordings == Some(true);
    let (status, byte_size, reason, missing, note) = match result {
        EpisodeResult::Reused(size) => (
            Status::Downloaded,
            Some(*size),
            None,
            Some(false),
            Some(format!("{size} bytes already downloaded")),
        ),
        EpisodeResult::Downloaded(size) => (
            Status::Downloaded,
            Some(*size),
            None,
            Some(missing_recordings),
            missing_recordings.then(|| "some of its recordings are no longer available".to_owned()),
        ),
        EpisodeResult::Skipped => (
            Status::Skipped,
            None,
            Some(NO_DATA_LEFT_REASON.to_owned()),
            None,
            Some(format!("skipped: {NO_DATA_LEFT_REASON}")),
        ),
        EpisodeResult::Failed(reason) => (
            Status::Failed,
            None,
            Some(reason.clone()),
            None,
            Some(format!("failed: {reason}")),
        ),
        EpisodeResult::NotAttempted(reason) => (
            Status::Failed,
            None,
            Some(format!("Not attempted: {reason}")),
            None,
            Some(format!("not attempted: {reason}")),
        ),
    };
    let mut episode = ManifestEpisode {
        index,
        id: entry.episode.id.clone(),
        file: byte_size.map(|_| file.clone()),
        start_time: entry.episode.start_time.clone(),
        end_time: entry.episode.end_time.clone(),
        metadata: entry.episode.metadata.clone(),
        byte_size,
        status,
        reason,
        episode_has_missing_recordings: missing,
    };
    let Some(size) = kept else {
        return (episode, note);
    };
    // A skip means no data is left to download, so the earlier file is the download. A
    // failure stays a failure, with the earlier file listed so the next run can find it.
    if status == Status::Skipped {
        episode.status = Status::Downloaded;
        episode.reason = None;
    }
    episode.file = Some(file);
    episode.byte_size = Some(size);
    episode.episode_has_missing_recordings = Some(true);
    let note = note.map(|note| format!("{size} bytes kept from an earlier run ({note})"));
    (episode, note)
}

/// Writes the manifest with the episodes done so far, plus the files an earlier run
/// left for the episodes still to come, so an interrupted run keeps the earlier files.
fn checkpoint(
    directory: &Path,
    manifest: &mut Manifest,
    remaining: &[DatasetEpisode],
    earlier: &HashMap<String, ManifestEpisode>,
    prefix: &str,
) -> std::io::Result<()> {
    let done = manifest.episodes.len();
    for (offset, entry) in remaining.iter().enumerate() {
        let name = episode_file_name(done + offset, &entry.episode.id);
        if let Some(episode) = earlier.get(&name) {
            manifest.episodes.push(ManifestEpisode {
                file: Some(format!("{prefix}/{name}")),
                ..episode.clone()
            });
        }
    }
    let result = write_manifest(directory, manifest);
    manifest.episodes.truncate(done);
    result
}

async fn download_episodes(
    runtime: &Runtime,
    args: &DatasetDownloadArgs,
    episodes: &[DatasetEpisode],
    manifest: &mut Manifest,
    directory: &Path,
    prefix: &str,
    earlier: &HashMap<String, ManifestEpisode>,
) -> bool {
    let cancellation = crate::api::ctrl_c_cancellation_token();
    let mut cancelled = false;
    let mut stopped: Option<String> = None;
    let mut not_attempted = 0_usize;
    for (index, entry) in episodes.iter().enumerate() {
        let label = format!("Episode {} of {}", index + 1, episodes.len());
        let name = episode_file_name(index, &entry.episode.id);
        let path = directory.join(&name);
        let on_disk = earlier.get(&name).filter(|episode| {
            std::fs::metadata(&path).is_ok_and(|file| Some(file.len()) == episode.byte_size)
        });
        let result = if let Some(complete) = on_disk.filter(|episode| is_complete(episode)) {
            EpisodeResult::Reused(complete.byte_size.unwrap_or_default())
        } else if let Some(reason) = &stopped {
            EpisodeResult::NotAttempted(reason.clone())
        } else {
            let result = match episode_request(entry, args, manifest.selection.topics.as_deref()) {
                Ok(request) => write_episode(runtime, &request, &path, &label, &cancellation).await,
                Err(error) => Err(error),
            };
            match result {
                Ok(written) => EpisodeResult::Downloaded(written),
                Err(error) if error.code() == Some(NO_STREAMABLE_RECORDINGS) => {
                    EpisodeResult::Skipped
                }
                Err(error) => {
                    let reason = error.to_string();
                    if error.is_cancelled() {
                        cancelled = true;
                        stopped = Some("the download was cancelled".to_owned());
                    } else if matches!(error, ApiError::Write(_)) {
                        stopped =
                            Some(format!("an earlier episode could not be written: {reason}"));
                    }
                    EpisodeResult::Failed(reason)
                }
            }
        };
        let kept = on_disk
            .filter(|_| {
                !matches!(
                    result,
                    EpisodeResult::Reused(_) | EpisodeResult::Downloaded(_)
                )
            })
            .and_then(|episode| episode.byte_size);
        let (episode, note) =
            manifest_entry(index, entry, &result, kept, format!("{prefix}/{name}"));
        manifest.episodes.push(episode);
        if matches!(result, EpisodeResult::Downloaded(_)) && stopped.is_none() {
            if let Err(error) =
                checkpoint(directory, manifest, &episodes[index + 1..], earlier, prefix)
            {
                stopped = Some(format!("the manifest could not be written: {error}"));
            }
        }
        if matches!(result, EpisodeResult::NotAttempted(_)) && kept.is_none() {
            not_attempted += 1;
        } else if let Some(note) = note {
            let _ = writeln!(std::io::stderr(), "{label}: {note}");
        }
    }
    if let Some(reason) = stopped.filter(|_| not_attempted > 0) {
        report_not_attempted(not_attempted, &reason);
    }
    cancelled
}

fn report_not_attempted(count: usize, reason: &str) {
    let noun = if count == 1 { "episode" } else { "episodes" };
    let _ = writeln!(
        std::io::stderr(),
        "{count} more {noun} not attempted: {reason}"
    );
}

/// Replaces the manifest atomically, so a failed write never truncates the last one.
fn write_manifest(directory: &Path, manifest: &mut Manifest) -> std::io::Result<()> {
    manifest.generated_at = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_default();
    let staging = directory.join(format!(".{MANIFEST_FILE_NAME}.{}.tmp", std::process::id()));
    let encoded = serde_json::to_vec_pretty(manifest).map_err(std::io::Error::other)?;
    std::fs::write(&staging, encoded)
        .and_then(|()| std::fs::rename(&staging, directory.join(MANIFEST_FILE_NAME)))
        .inspect_err(|_| {
            let _ = std::fs::remove_file(&staging);
        })
}

fn summary(episodes: &[ManifestEpisode], directory: &Path) -> String {
    let count = |status: Status| {
        episodes
            .iter()
            .filter(|episode| episode.status == status)
            .count()
    };
    let mut summary = format!(
        "Downloaded {} of {} episodes to {} ({} failed, {} skipped)\n",
        count(Status::Downloaded),
        episodes.len(),
        directory.display(),
        count(Status::Failed),
        count(Status::Skipped)
    );
    let partial = episodes
        .iter()
        .filter(|episode| {
            episode.status == Status::Downloaded
                && episode.episode_has_missing_recordings == Some(true)
        })
        .count();
    if partial > 0 {
        let noun = if partial == 1 {
            "episode has"
        } else {
            "episodes have"
        };
        let _ = writeln!(summary, "{partial} downloaded {noun} missing recordings");
    }
    summary
}

pub(crate) async fn download_dataset(runtime: &Runtime, args: &DatasetDownloadArgs) -> Outcome {
    let dataset = match fetch_dataset(runtime, &args.dataset_id).await {
        Ok(dataset) => dataset,
        Err(error) if error.is_not_found() => {
            return Outcome::failure(format!("Dataset not found: {}\n", args.dataset_id))
        }
        Err(error) => return Outcome::failure(format!("Failed to get dataset: {error}\n")),
    };
    let version = match resolve_version(runtime, &args.dataset_id, args.version).await {
        Ok(version) => version,
        Err(error) => return Outcome::failure(format!("{error}\n")),
    };
    let episodes = match fetch_all_episodes(runtime, &args.dataset_id, version.version_number).await
    {
        Ok(episodes) => episodes,
        Err(error) => {
            return Outcome::failure(format!("Failed to list dataset episodes: {error}\n"))
        }
    };

    let root = default_directory_name(&dataset.name, version.version_number);
    let directory = args
        .output
        .as_deref()
        .filter(|path| !path.is_empty())
        .map_or_else(|| PathBuf::from(&root), PathBuf::from);
    if let Err(error) = std::fs::create_dir_all(&directory) {
        return Outcome::failure(format!(
            "Failed to create {}: {error}\n",
            directory.display()
        ));
    }

    let prefix = manifest_prefix(&directory, &root);
    let mut manifest = Manifest {
        format_version: MANIFEST_FORMAT_VERSION,
        generated_at: String::new(),
        dataset: ManifestDataset {
            id: dataset.id,
            name: dataset.name,
            project_id: dataset.project_id,
        },
        version: ManifestVersion {
            version_number: version.version_number,
            committed_at: version.committed_at.unwrap_or_default(),
        },
        selection: ManifestSelection {
            topics: topic_list(args.topics.as_deref()),
            // The app always includes attachments and never writes this field, so a
            // default download writes the same manifest as the app.
            include_attachments: (!args.include_attachments).then_some(false),
            episode_count: episodes.len(),
        },
        episodes: Vec::with_capacity(episodes.len()),
    };
    let earlier = previously_downloaded(&directory, &manifest);
    let cancelled = download_episodes(
        runtime,
        args,
        &episodes,
        &mut manifest,
        &directory,
        &prefix,
        &earlier,
    )
    .await;
    if let Err(error) = write_manifest(&directory, &mut manifest) {
        return Outcome::failure(format!(
            "Failed to write {}: {error}\n",
            directory.join(MANIFEST_FILE_NAME).display()
        ));
    }

    let failed = manifest.episodes.iter().any(|episode| {
        episode.status == Status::Failed
            || args.strict
                && (episode.status == Status::Skipped
                    || episode.episode_has_missing_recordings == Some(true))
    });
    Outcome {
        stderr: summary(&manifest.episodes, &directory).into_bytes(),
        exit_code: if cancelled { 130 } else { u8::from(failed) },
        ..Outcome::default()
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{default_directory_name, episode_file_name, manifest_prefix, topic_list};

    #[test]
    fn the_directory_name_is_a_safe_slug_of_the_dataset_name() {
        for (name, expected) in [
            ("Highway merges", "Highway-merges-v4"),
            ("  spaced  ", "spaced-v4"),
            ("a/b\\c", "a-b-c-v4"),
            ("!!!", "dataset-v4"),
            ("", "dataset-v4"),
            ("keep.dots_and-dashes", "keep.dots_and-dashes-v4"),
            (
                "Left lane cut ins and highway merges on rainy nights with heavy traffic",
                "Left-lane-cut-ins-and-highway-merges-on-rainy-nights-with-heavy--v4",
            ),
            ("東京 drive", "東京-drive-v4"),
        ] {
            assert_eq!(default_directory_name(name, 4), expected, "{name:?}");
        }
        assert_eq!(
            default_directory_name(&format!("--{}", "a".repeat(70)), 4),
            format!("{}-v4", "a".repeat(64))
        );
    }

    #[test]
    fn episode_files_are_ordered_and_unambiguous() {
        assert_eq!(
            episode_file_name(0, "ep_01HXYZ"),
            "episode_0000_ep_01HXYZ.mcap"
        );
        assert_eq!(
            episode_file_name(12, "ep/../escape"),
            "episode_0012_ep-..-escape.mcap"
        );
    }

    #[test]
    fn manifest_paths_name_the_directory_the_download_landed_in() {
        assert_eq!(
            manifest_prefix(Path::new("Highway-merges-v4"), "root"),
            "Highway-merges-v4"
        );
        assert_eq!(manifest_prefix(Path::new("out/data/"), "root"), "data");
        let here = manifest_prefix(Path::new("."), "root");
        assert!(!here.contains(['/', '\\']), "{here}");
    }

    #[test]
    fn topics_are_split_trimmed_and_optional() {
        assert_eq!(topic_list(None), None);
        assert_eq!(topic_list(Some("")), None);
        assert_eq!(topic_list(Some(" , ")), None);
        assert_eq!(
            topic_list(Some("/a, /b ,,/c")),
            Some(vec!["/a".to_owned(), "/b".to_owned(), "/c".to_owned()])
        );
    }
}
