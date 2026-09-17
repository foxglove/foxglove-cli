//! Dataset version download.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use tokio_util::sync::CancellationToken;

use super::{DatasetEpisode, DatasetEpisodeListResponse};
use crate::api::{encode_path_segment, ApiError, StreamRequest};
use crate::cli::DatasetDownloadArgs;
use crate::records::DEFAULT_LIST_LIMIT;
use crate::runtime::Runtime;
use crate::Outcome;

const MANIFEST_FORMAT_VERSION: u32 = 1;
const MANIFEST_FILE_NAME: &str = "manifest.json";
const SLUG_MAX_CHARS: usize = 64;

const EPISODE_ATTEMPTS: u32 = 3;
const RETRY_BACKOFF: Duration = Duration::from_secs(1);

#[derive(Clone, Debug, Default, Deserialize)]
struct DatasetSummary {
    id: String,
    name: String,
    #[serde(rename = "projectId")]
    project_id: String,
}

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

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ManifestDataset {
    id: String,
    name: String,
    project_id: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ManifestVersion {
    version_number: i64,
    committed_at: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ManifestSelection {
    #[serde(skip_serializing_if = "Option::is_none")]
    topics: Option<Vec<String>>,
    episode_count: usize,
}

#[derive(Serialize)]
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
    status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    reason: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Manifest {
    format_version: u32,
    generated_at: String,
    dataset: ManifestDataset,
    version: ManifestVersion,
    selection: ManifestSelection,
    episodes: Vec<ManifestEpisode>,
}

fn archive_root_name(dataset_name: &str, version_number: i64) -> String {
    let mut slug = String::new();
    let mut pending_dash = false;
    for character in dataset_name.trim().chars() {
        if character.is_alphanumeric() || matches!(character, '.' | '_' | '-') {
            if pending_dash && !slug.is_empty() {
                slug.push('-');
            }
            pending_dash = false;
            slug.push(character);
        } else {
            pending_dash = true;
        }
        if slug.chars().count() >= SLUG_MAX_CHARS {
            break;
        }
    }
    let slug = slug.trim_matches('-');
    let slug = if slug.is_empty() { "dataset" } else { slug };
    format!("{slug}-v{version_number}")
}

fn episode_file_name(index: usize, episode_id: &str) -> String {
    let mut safe = String::new();
    let mut pending_dash = false;
    for character in episode_id.chars() {
        if character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-') {
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
    format!("episode_{index:04}_{safe}.mcap")
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

async fn fetch_dataset(runtime: &Runtime, id: &str) -> Result<DatasetSummary, ApiError> {
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
    let response: DatasetVersionListResponse = runtime
        .client
        .get(
            &format!("/v1/datasets/{}/versions", encode_path_segment(id)),
            &(),
        )
        .await
        .map_err(|error| format!("Failed to list dataset versions: {error}"))?;
    let mut committed = response
        .versions
        .into_iter()
        .filter(|version| version.committed_at.is_some());
    match requested {
        Some(number) => committed
            .find(|version| version.version_number == number)
            .ok_or_else(|| format!("Version {number} is not a committed version of this dataset")),
        None => committed
            .max_by_key(|version| version.version_number)
            .ok_or_else(|| "This dataset has no committed version to download".to_owned()),
    }
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
    cancellation: &CancellationToken,
) -> Result<u64, ApiError> {
    let mut attempt = 1;
    loop {
        let result = write_episode_once(runtime, request, path, cancellation).await;
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

async fn write_episode_once(
    runtime: &Runtime,
    request: &StreamRequest,
    path: &Path,
    cancellation: &CancellationToken,
) -> Result<u64, ApiError> {
    let mut stream = runtime
        .client
        .stream_with_cancellation(request, cancellation)
        .await?;
    let mut file = tokio::fs::File::create(path)
        .await
        .map_err(ApiError::Write)?;
    let result = match stream.copy_to(&mut file).await {
        Ok(written) => tokio::io::AsyncWriteExt::flush(&mut file)
            .await
            .map(|()| written)
            .map_err(ApiError::Write),
        Err(error) => Err(error),
    };
    if result.is_err() {
        drop(file);
        let _ = tokio::fs::remove_file(path).await;
    }
    result
}

struct DownloadTally {
    episodes: Vec<ManifestEpisode>,
    downloaded: usize,
    failed: usize,
    skipped: usize,
}

async fn download_episodes(
    runtime: &Runtime,
    args: &DatasetDownloadArgs,
    episodes: &[DatasetEpisode],
    topics: Option<&[String]>,
    directory: &Path,
    prefix: &str,
) -> Option<DownloadTally> {
    let cancellation = crate::api::ctrl_c_cancellation_token();
    let mut tally = DownloadTally {
        episodes: Vec::with_capacity(episodes.len()),
        downloaded: 0,
        failed: 0,
        skipped: 0,
    };
    for (index, entry) in episodes.iter().enumerate() {
        let base = |status, file, byte_size, reason| ManifestEpisode {
            index,
            id: entry.episode.id.clone(),
            file,
            start_time: entry.episode.start_time.clone(),
            end_time: entry.episode.end_time.clone(),
            metadata: entry.episode.metadata.clone(),
            byte_size,
            status,
            reason,
        };
        let note = if entry.has_missing_recordings == Some(true) {
            let reason = "Recordings for this episode are no longer available".to_owned();
            tally.skipped += 1;
            tally
                .episodes
                .push(base("skipped", None, None, Some(reason.clone())));
            format!("skipped: {reason}")
        } else {
            let name = episode_file_name(index, &entry.episode.id);
            let request = StreamRequest {
                episode_id: entry.episode.id.clone(),
                output_format: "mcap".to_owned(),
                include_attachments: args.include_attachments,
                topics: topics.map(<[String]>::to_vec).unwrap_or_default(),
                ..StreamRequest::default()
            };
            match write_episode(runtime, &request, &directory.join(&name), &cancellation).await {
                Ok(written) => {
                    tally.downloaded += 1;
                    tally.episodes.push(base(
                        "downloaded",
                        Some(format!("{prefix}/{name}")),
                        Some(written),
                        None,
                    ));
                    format!("{written} bytes written")
                }
                Err(error) if error.is_cancelled() => return None,
                Err(error) => {
                    let reason = error.to_string();
                    tally.failed += 1;
                    tally
                        .episodes
                        .push(base("failed", None, None, Some(reason.clone())));
                    format!("failed: {reason}")
                }
            }
        };
        let _ = writeln!(
            std::io::stderr(),
            "Episode {} of {} \u{2014} {note}",
            index + 1,
            episodes.len()
        );
    }
    Some(tally)
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
    let committed_at = version.committed_at.clone().unwrap_or_default();
    let episodes = match fetch_all_episodes(runtime, &args.dataset_id, version.version_number).await
    {
        Ok(episodes) => episodes,
        Err(error) => {
            return Outcome::failure(format!("Failed to list dataset episodes: {error}\n"))
        }
    };

    let root = archive_root_name(&dataset.name, version.version_number);
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

    let topics = topic_list(args.topics.as_deref());
    let prefix = manifest_prefix(&directory, &root);
    let tally = download_episodes(
        runtime,
        args,
        &episodes,
        topics.as_deref(),
        &directory,
        &prefix,
    )
    .await;
    let Some(tally) = tally else {
        return Outcome {
            exit_code: 130,
            ..Outcome::default()
        };
    };
    let manifest_episodes = tally.episodes;
    let (downloaded, failed, skipped) = (tally.downloaded, tally.failed, tally.skipped);

    let manifest = Manifest {
        format_version: MANIFEST_FORMAT_VERSION,
        generated_at: OffsetDateTime::now_utc()
            .format(&Rfc3339)
            .unwrap_or_default(),
        dataset: ManifestDataset {
            id: dataset.id,
            name: dataset.name,
            project_id: dataset.project_id,
        },
        version: ManifestVersion {
            version_number: version.version_number,
            committed_at,
        },
        selection: ManifestSelection {
            topics,
            episode_count: episodes.len(),
        },
        episodes: manifest_episodes,
    };
    let manifest_path = directory.join(MANIFEST_FILE_NAME);
    let encoded = match serde_json::to_vec_pretty(&manifest) {
        Ok(encoded) => encoded,
        Err(error) => return Outcome::failure(format!("Failed to build the manifest: {error}\n")),
    };
    if let Err(error) = std::fs::write(&manifest_path, encoded) {
        return Outcome::failure(format!(
            "Failed to write {}: {error}\n",
            manifest_path.display()
        ));
    }

    let summary = format!(
        "Downloaded {downloaded} of {} episodes to {} ({failed} failed, {skipped} skipped)\n",
        episodes.len(),
        directory.display()
    );
    if failed > 0 {
        return Outcome::failure(summary);
    }
    Outcome {
        stderr: summary.into_bytes(),
        ..Outcome::default()
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{archive_root_name, episode_file_name, manifest_prefix, topic_list};

    #[test]
    fn the_directory_name_matches_the_app_archive_root() {
        for (name, expected) in [
            ("Highway merges", "Highway-merges-v4"),
            ("  spaced  ", "spaced-v4"),
            ("a/b\\c", "a-b-c-v4"),
            ("!!!", "dataset-v4"),
            ("", "dataset-v4"),
            ("keep.dots_and-dashes", "keep.dots_and-dashes-v4"),
        ] {
            assert_eq!(archive_root_name(name, 4), expected, "{name:?}");
        }
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
