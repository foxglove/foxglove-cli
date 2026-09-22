//! Dataset version download.

use std::collections::HashMap;
use std::fmt::Write as _;
use std::io::{IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use tokio_util::sync::CancellationToken;
use unicode_general_category::{get_general_category, GeneralCategory};

use super::{DatasetEpisode, DatasetEpisodeListResponse};
use crate::api::{encode_path_segment, ApiError, StreamRequest};
use crate::cli::DatasetDownloadArgs;
use crate::data::export::{resumable_download, CompletionCheck, DownloadProgress};
use crate::records::DEFAULT_LIST_LIMIT;
use crate::runtime::Runtime;
use crate::Outcome;

const MANIFEST_FORMAT_VERSION: u32 = 1;
const MANIFEST_FILE_NAME: &str = "manifest.json";
const SLUG_MAX_UTF16_UNITS: usize = 64;

const NO_STREAMABLE_RECORDINGS: &str = "NoStreamableRecordings";
const NO_DATA_LEFT_REASON: &str = "No Primary Site holds data for any recording in this episode. Import those recordings to include it.";

const EPISODE_ATTEMPTS: u32 = 3;
const RETRY_BACKOFF: Duration = Duration::from_secs(1);
const PROGRESS_INTERVAL: Duration = Duration::from_millis(200);

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
    #[serde(skip_serializing_if = "Option::is_none")]
    include_attachments: Option<bool>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    episode_has_missing_recordings: Option<bool>,
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

fn is_slug_character(character: char) -> bool {
    matches!(character, '.' | '_' | '-')
        || matches!(
            get_general_category(character),
            GeneralCategory::UppercaseLetter
                | GeneralCategory::LowercaseLetter
                | GeneralCategory::TitlecaseLetter
                | GeneralCategory::ModifierLetter
                | GeneralCategory::OtherLetter
                | GeneralCategory::DecimalNumber
                | GeneralCategory::LetterNumber
                | GeneralCategory::OtherNumber
        )
}

fn archive_root_name(dataset_name: &str, version_number: i64) -> String {
    let mut slug = String::new();
    let mut pending_dash = false;
    for character in dataset_name.trim().chars() {
        if is_slug_character(character) {
            if pending_dash {
                slug.push('-');
            }
            pending_dash = false;
            slug.push(character);
        } else {
            pending_dash = true;
        }
    }
    let slug = slug.trim_matches('-');
    let mut units = 0;
    let end = slug
        .char_indices()
        .find_map(|(offset, character)| {
            units += character.len_utf16();
            (units > SLUG_MAX_UTF16_UNITS).then_some(offset)
        })
        .unwrap_or(slug.len());
    let slug = &slug[..end];
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
        .filter(|version| version.committed_at.is_some())
        .max_by_key(|version| version.version_number)
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
    request: Result<StreamRequest, String>,
    path: &Path,
    progress: &mut EpisodeProgress,
    cancellation: &CancellationToken,
) -> Result<u64, ApiError> {
    let request = request.map_err(ApiError::Conversion)?;
    let mut attempt = 1;
    loop {
        let result = resumable_download(
            runtime,
            request.clone(),
            path,
            cancellation,
            progress,
            CompletionCheck::EndMagic,
        )
        .await
        .and_then(|()| {
            std::fs::metadata(path)
                .map(|metadata| metadata.len())
                .map_err(ApiError::Write)
        });
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
) -> Result<StreamRequest, String> {
    let bound = |raw: &str| {
        OffsetDateTime::parse(raw, &Rfc3339)
            .map_err(|error| format!("Invalid episode time {raw:?}: {error}"))
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

struct EpisodeProgress {
    label: String,
    received: u64,
    last_report: Instant,
    live: bool,
    shown: usize,
}

impl EpisodeProgress {
    fn new(index: usize, total: usize) -> Self {
        Self {
            label: format!("Episode {} of {total}", index + 1),
            received: 0,
            last_report: Instant::now(),
            live: std::io::stderr().is_terminal(),
            shown: 0,
        }
    }

    fn finish(&self, note: &str) {
        let line = format!("{} \u{2014} {note}", self.label);
        let carriage = if self.shown > 0 { "\r" } else { "" };
        let padding = " ".repeat(self.shown.saturating_sub(line.chars().count()));
        let _ = writeln!(std::io::stderr(), "{carriage}{line}{padding}");
    }
}

impl DownloadProgress for EpisodeProgress {
    fn advance(&mut self, bytes: usize) {
        self.received = self.received.saturating_add(bytes as u64);
        if !self.live || self.last_report.elapsed() < PROGRESS_INTERVAL {
            return;
        }
        let line = format!("{} \u{2014} {} bytes received", self.label, self.received);
        let _ = write!(std::io::stderr(), "\r{line}");
        self.shown = self.shown.max(line.chars().count());
        self.last_report = Instant::now();
    }
}

#[derive(Deserialize)]
struct PreviousManifest {
    dataset: PreviousDataset,
    version: PreviousVersion,
    selection: PreviousSelection,
    episodes: Vec<PreviousEpisode>,
}

#[derive(Deserialize)]
struct PreviousDataset {
    id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PreviousVersion {
    version_number: i64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PreviousSelection {
    #[serde(default)]
    topics: Option<Vec<String>>,
    #[serde(default)]
    include_attachments: Option<bool>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PreviousEpisode {
    index: usize,
    id: String,
    #[serde(default)]
    byte_size: Option<u64>,
    status: String,
    #[serde(default)]
    episode_has_missing_recordings: Option<bool>,
}

struct Selection<'a> {
    dataset_id: &'a str,
    version_number: i64,
    topics: Option<&'a [String]>,
    include_attachments: bool,
}

#[derive(Clone, Copy)]
struct EarlierFile {
    size: u64,
    partial: bool,
}

fn previously_downloaded(
    directory: &Path,
    selection: &Selection<'_>,
) -> HashMap<String, EarlierFile> {
    let Ok(encoded) = std::fs::read(directory.join(MANIFEST_FILE_NAME)) else {
        return HashMap::new();
    };
    let Ok(previous) = serde_json::from_slice::<PreviousManifest>(&encoded) else {
        return HashMap::new();
    };
    let same_selection = previous.dataset.id == selection.dataset_id
        && previous.version.version_number == selection.version_number
        && previous.selection.topics.as_deref() == selection.topics
        && previous.selection.include_attachments.unwrap_or(true) == selection.include_attachments;
    if !same_selection {
        return HashMap::new();
    }
    previous
        .episodes
        .into_iter()
        .filter(|episode| episode.status == "downloaded")
        .filter_map(|episode| {
            Some((
                episode_file_name(episode.index, &episode.id),
                EarlierFile {
                    size: episode.byte_size?,
                    partial: episode.episode_has_missing_recordings == Some(true),
                },
            ))
        })
        .collect()
}

struct DownloadTally {
    episodes: Vec<ManifestEpisode>,
    downloaded: usize,
    partial: usize,
    failed: usize,
    skipped: usize,
    cancelled: bool,
}

impl DownloadTally {
    fn record(&mut self, episode: ManifestEpisode) {
        match episode.status {
            "downloaded" => {
                self.downloaded += 1;
                self.partial += usize::from(episode.episode_has_missing_recordings == Some(true));
            }
            "skipped" => self.skipped += 1,
            _ => self.failed += 1,
        }
        self.episodes.push(episode);
    }
}

async fn download_episodes(
    runtime: &Runtime,
    args: &DatasetDownloadArgs,
    episodes: &[DatasetEpisode],
    topics: Option<&[String]>,
    directory: &Path,
    prefix: &str,
    reusable: &HashMap<String, EarlierFile>,
) -> DownloadTally {
    let cancellation = crate::api::ctrl_c_cancellation_token();
    let mut tally = DownloadTally {
        episodes: Vec::with_capacity(episodes.len()),
        downloaded: 0,
        partial: 0,
        failed: 0,
        skipped: 0,
        cancelled: false,
    };
    let mut stopped: Option<String> = None;
    let mut not_attempted = 0_usize;
    for (index, entry) in episodes.iter().enumerate() {
        let missing_recordings = entry.has_missing_recordings == Some(true);
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
            episode_has_missing_recordings: (status == "downloaded").then_some(missing_recordings),
        };
        let mut progress = EpisodeProgress::new(index, episodes.len());
        let name = episode_file_name(index, &entry.episode.id);
        let path = directory.join(&name);
        let earlier = reusable.get(&name).copied().filter(|earlier| {
            std::fs::metadata(&path).is_ok_and(|file| file.len() == earlier.size)
        });
        let partial_note = if missing_recordings {
            " (some of its recordings are no longer available)"
        } else {
            ""
        };
        let file = Some(format!("{prefix}/{name}"));
        let mut quiet = false;
        let (mut episode, mut note) = if let Some(earlier) = earlier.filter(|e| !e.partial) {
            (
                base("downloaded", file.clone(), Some(earlier.size), None),
                format!("{} bytes already downloaded{partial_note}", earlier.size),
            )
        } else if let Some(reason) = &stopped {
            not_attempted += 1;
            quiet = true;
            let reason = format!("Not attempted: {reason}");
            (base("failed", None, None, Some(reason.clone())), reason)
        } else {
            let request = episode_request(entry, args, topics);
            match write_episode(runtime, request, &path, &mut progress, &cancellation).await {
                Ok(written) => (
                    base("downloaded", file.clone(), Some(written), None),
                    format!("{written} bytes written{partial_note}"),
                ),
                Err(error) if error.code() == Some(NO_STREAMABLE_RECORDINGS) => (
                    base("skipped", None, None, Some(NO_DATA_LEFT_REASON.to_owned())),
                    format!("skipped: {NO_DATA_LEFT_REASON}"),
                ),
                Err(error) => {
                    let reason = error.to_string();
                    if error.is_cancelled() {
                        tally.cancelled = true;
                        stopped = Some("the download was cancelled".to_owned());
                    } else if matches!(error, ApiError::Write(_)) {
                        stopped =
                            Some(format!("an earlier episode could not be written: {reason}"));
                    }
                    (
                        base("failed", None, None, Some(reason.clone())),
                        format!("failed: {reason}"),
                    )
                }
            }
        };
        if let Some(earlier) = earlier.filter(|_| episode.status != "downloaded") {
            note = format!("{} bytes kept from an earlier run ({note})", earlier.size);
            episode = base("downloaded", file, Some(earlier.size), None);
            episode.episode_has_missing_recordings = Some(true);
        }
        tally.record(episode);
        if quiet {
            continue;
        }
        progress.finish(&note);
    }
    if let Some(reason) = stopped.filter(|_| not_attempted > 0) {
        report_not_attempted(not_attempted, &reason);
    }
    tally
}

fn report_not_attempted(count: usize, reason: &str) {
    let noun = if count == 1 { "episode" } else { "episodes" };
    let _ = writeln!(
        std::io::stderr(),
        "{count} more {noun} not attempted: {reason}"
    );
}

fn write_manifest(directory: &Path, manifest: &Manifest) -> Result<(), String> {
    let path = directory.join(MANIFEST_FILE_NAME);
    let encoded = serde_json::to_vec_pretty(manifest)
        .map_err(|error| format!("Failed to build the manifest: {error}\n"))?;
    std::fs::write(&path, encoded)
        .map_err(|error| format!("Failed to write {}: {error}\n", path.display()))
}

fn summary(tally: &DownloadTally, total: usize, directory: &Path) -> String {
    let mut summary = format!(
        "Downloaded {} of {total} episodes to {} ({} failed, {} skipped)\n",
        tally.downloaded,
        directory.display(),
        tally.failed,
        tally.skipped
    );
    if tally.partial > 0 {
        let noun = if tally.partial == 1 {
            "episode has"
        } else {
            "episodes have"
        };
        let _ = writeln!(
            summary,
            "{} downloaded {noun} missing recordings",
            tally.partial
        );
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
    let reusable = previously_downloaded(
        &directory,
        &Selection {
            dataset_id: &dataset.id,
            version_number: version.version_number,
            topics: topics.as_deref(),
            include_attachments: args.include_attachments,
        },
    );
    let tally = download_episodes(
        runtime,
        args,
        &episodes,
        topics.as_deref(),
        &directory,
        &prefix,
        &reusable,
    )
    .await;
    let summary = summary(&tally, episodes.len(), &directory);
    let (failed, cancelled) = (tally.failed, tally.cancelled);

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
            // The app always includes attachments and never writes this field, so a
            // default download writes the same manifest as the app.
            include_attachments: (!args.include_attachments).then_some(false),
            episode_count: episodes.len(),
        },
        episodes: tally.episodes,
    };
    if let Err(error) = write_manifest(&directory, &manifest) {
        return Outcome::failure(error);
    }

    if cancelled {
        return Outcome {
            exit_code: 130,
            stderr: summary.into_bytes(),
            ..Outcome::default()
        };
    }
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
            (
                "Left lane cut ins and highway merges on rainy nights with heavy traffic",
                "Left-lane-cut-ins-and-highway-merges-on-rainy-nights-with-heavy--v4",
            ),
            ("सड़क परीक्षण", "सड-क-पर-क-षण-v4"),
            ("ทดสอบการขับขี่", "ทดสอบการข-บข-v4"),
            ("Ⓐ test", "test-v4"),
        ] {
            assert_eq!(archive_root_name(name, 4), expected, "{name:?}");
        }
    }

    #[test]
    fn the_directory_name_is_cut_like_the_app_after_trimming() {
        assert_eq!(
            archive_root_name(&format!("--{}", "a".repeat(70)), 4),
            format!("{}-v4", "a".repeat(64))
        );
        assert_eq!(
            archive_root_name(&"\u{1D400}".repeat(40), 4),
            format!("{}-v4", "\u{1D400}".repeat(32))
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
