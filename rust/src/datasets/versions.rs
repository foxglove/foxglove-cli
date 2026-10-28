//! Dataset versions, and committing or discarding a dataset's draft.

use std::fmt::Write as _;

use serde::{Deserialize, Serialize};

use super::{
    commit_hint, dataset_endpoint, dataset_not_found, dataset_or_version_not_found,
    version_not_found, DatasetEpisode,
};
use crate::api::ApiError;
use crate::cli::{
    DatasetIdArgs, DatasetVersionCompareArgs, DatasetVersionGetArgs, DatasetVersionListArgs,
    DatasetVersionRestoreArgs,
};
use crate::episodes::{episode_include, Episode};
use crate::output::Format;
use crate::records::{
    format_list_output, format_record, is_false, plural, warn_if_has_next_cursor, EmptyRequest,
    NextCursor, Record, DEFAULT_LIST_LIMIT,
};
use crate::runtime::Runtime;
use crate::Outcome;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(super) struct DatasetVersion {
    #[serde(rename = "versionNumber")]
    pub(super) version_number: i64,
    #[serde(
        rename = "committedAt",
        skip_serializing_if = "Option::is_none",
        default
    )]
    pub(super) committed_at: Option<String>,
    #[serde(rename = "createdAt", default)]
    created_at: String,
    #[serde(rename = "episodeCount", default)]
    episode_count: usize,
    #[serde(rename = "addedEpisodeCount", default)]
    added_episode_count: usize,
    #[serde(rename = "removedEpisodeCount", default)]
    removed_episode_count: usize,
    #[serde(
        rename = "changedEpisodeCount",
        skip_serializing_if = "Option::is_none",
        default
    )]
    changed_episode_count: Option<usize>,
}

impl Record for DatasetVersion {
    fn headers() -> &'static [&'static str] {
        &[
            "Version",
            "Status",
            "Committed At",
            "Episode Count",
            "Added",
            "Removed",
            "Changed",
            "Created At",
        ]
    }

    fn fields(&self) -> Vec<String> {
        vec![
            self.version_number.to_string(),
            if self.committed_at.is_some() {
                "committed"
            } else {
                "draft"
            }
            .to_owned(),
            self.committed_at.clone().unwrap_or_default(),
            self.episode_count.to_string(),
            self.added_episode_count.to_string(),
            self.removed_episode_count.to_string(),
            self.changed_episode_count
                .map(|count| count.to_string())
                .unwrap_or_default(),
            self.created_at.clone(),
        ]
    }
}

#[derive(Deserialize)]
pub(super) struct DatasetVersionListResponse {
    #[serde(default)]
    pub(super) versions: Vec<DatasetVersion>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
struct DatasetVersionDetail {
    #[serde(flatten)]
    version: DatasetVersion,
    #[serde(rename = "hasMissingRecordings", default)]
    has_missing_recordings: bool,
}

impl Record for DatasetVersionDetail {
    fn headers() -> &'static [&'static str] {
        &[
            "Version",
            "Status",
            "Committed At",
            "Episode Count",
            "Added",
            "Removed",
            "Changed",
            "Created At",
            "Missing Recordings",
        ]
    }

    fn fields(&self) -> Vec<String> {
        let mut fields = self.version.fields();
        fields.push(self.has_missing_recordings.to_string());
        fields
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
struct DatasetEpisodeChange {
    change: String,
    #[serde(
        rename = "previousVersion",
        skip_serializing_if = "Option::is_none",
        default
    )]
    previous_version: Option<i32>,
    #[serde(
        rename = "previousEpisode",
        skip_serializing_if = "Option::is_none",
        default
    )]
    previous_episode: Option<Episode>,
    #[serde(flatten)]
    entry: DatasetEpisode,
}

impl Record for DatasetEpisodeChange {
    fn headers() -> &'static [&'static str] {
        &[
            "Change",
            "Episode ID",
            "Project ID",
            "Start Time",
            "End Time",
            "Recordings",
            "Metadata",
            "Added At",
            "Added In Version",
            "Created At",
            "Version",
            "Latest Version",
            "Custom Properties",
            "Previous Version",
            "Previous Custom Properties",
        ]
    }

    fn fields(&self) -> Vec<String> {
        let mut fields = vec![self.change.clone()];
        fields.extend(self.entry.fields());
        fields.push(
            self.previous_version
                .map_or_else(String::new, |version| version.to_string()),
        );
        fields.push(
            self.previous_episode
                .as_ref()
                .map_or_else(String::new, Episode::property_cell),
        );
        fields
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DatasetChangesetResponse {
    #[serde(default)]
    changes: Vec<DatasetEpisodeChange>,
    #[serde(default)]
    added_count: usize,
    #[serde(default)]
    removed_count: usize,
    #[serde(default)]
    changed_count: usize,
    #[serde(default)]
    next_cursor: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct VersionListQuery<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    cursor: Option<&'a str>,
    limit: i64,
    #[serde(skip_serializing_if = "String::is_empty")]
    sort_order: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CompareQuery<'a> {
    version: i64,
    limit: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    cursor: Option<&'a str>,
    #[serde(skip_serializing_if = "String::is_empty")]
    include: String,
}

#[derive(Serialize)]
struct RestoreQuery {
    #[serde(skip_serializing_if = "is_false")]
    force: bool,
}

#[derive(Deserialize)]
struct CommitResponse {
    committed: DatasetVersion,
}

#[derive(Deserialize)]
struct DiscardResponse {
    #[serde(default, rename = "discardedAdds")]
    adds: usize,
    #[serde(default, rename = "discardedRemoves")]
    removes: usize,
    #[serde(default, rename = "discardedUpdates")]
    updates: usize,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RestoreResponse {
    #[serde(default)]
    added: usize,
    #[serde(default)]
    removed: usize,
    #[serde(default)]
    discarded_adds: usize,
    #[serde(default)]
    discarded_removes: usize,
}

pub(crate) async fn list_versions(
    runtime: &Runtime,
    args: &DatasetVersionListArgs,
    format: Format,
) -> Outcome {
    let limit = args.limit.unwrap_or(DEFAULT_LIST_LIMIT);
    let query = VersionListQuery {
        cursor: args.cursor.as_deref().filter(|cursor| !cursor.is_empty()),
        limit,
        sort_order: args.sort_order.clone().unwrap_or_default(),
    };
    match runtime
        .client
        .get_page::<_, DatasetVersionListResponse>(
            &format!("{}/versions", dataset_endpoint(&args.dataset_id)),
            &query,
        )
        .await
    {
        Ok(page) => warn_if_has_next_cursor(
            format_list_output(
                &page.data.versions,
                format,
                NextCursor::Page(page.next_cursor.as_deref()),
            ),
            page.next_cursor.as_deref(),
        ),
        Err(error) if error.is_not_found_for("dataset") => dataset_not_found(&args.dataset_id),
        Err(error) => Outcome::failure(format!("Failed to list dataset versions: {error}\n")),
    }
}

pub(super) async fn draft_version(runtime: &Runtime, dataset_id: &str) -> Result<i64, Outcome> {
    let query = VersionListQuery {
        cursor: None,
        limit: 1,
        sort_order: "desc".to_owned(),
    };
    match runtime
        .client
        .get::<_, DatasetVersionListResponse>(
            &format!("{}/versions", dataset_endpoint(dataset_id)),
            &query,
        )
        .await
    {
        Ok(response) => match response.versions.first() {
            Some(version) if version.committed_at.is_none() => Ok(version.version_number),
            _ => Err(Outcome::failure(format!(
                "Dataset {dataset_id} has no draft\n"
            ))),
        },
        Err(error) if error.is_not_found_for("dataset") => Err(dataset_not_found(dataset_id)),
        Err(error) => Err(Outcome::failure(format!(
            "Failed to find the draft of dataset {dataset_id}: {error}\n"
        ))),
    }
}

pub(crate) async fn get_version(
    runtime: &Runtime,
    args: &DatasetVersionGetArgs,
    format: Format,
) -> Outcome {
    match runtime
        .client
        .get::<_, DatasetVersionDetail>(
            &format!(
                "{}/versions/{}",
                dataset_endpoint(&args.dataset_id),
                args.version
            ),
            &(),
        )
        .await
    {
        Ok(version) => format_record(&version, format),
        Err(error) if error.is_bare_not_found() => {
            dataset_or_version_not_found(&args.dataset_id, args.version)
        }
        Err(error) if error.is_not_found_for("version") => {
            version_not_found(&args.dataset_id, args.version)
        }
        Err(error) if error.is_not_found_for("dataset") => dataset_not_found(&args.dataset_id),
        Err(error) => Outcome::failure(format!("Failed to get dataset version: {error}\n")),
    }
}

pub(crate) async fn compare_versions(
    runtime: &Runtime,
    args: &DatasetVersionCompareArgs,
    format: Format,
) -> Outcome {
    let endpoint = format!(
        "{}/versions/{}/compare",
        dataset_endpoint(&args.dataset_id),
        args.target_version
    );
    let query = CompareQuery {
        version: args.base_version,
        limit: args.limit.unwrap_or(DEFAULT_LIST_LIMIT),
        cursor: args.cursor.as_deref().filter(|cursor| !cursor.is_empty()),
        include: episode_include(args.include_recordings, args.include_properties),
    };
    let page = match runtime
        .client
        .get::<_, DatasetChangesetResponse>(&endpoint, &query)
        .await
    {
        Ok(page) => page,
        Err(error) if error.is_bare_not_found() => {
            return Outcome::failure(format!(
                "Dataset {} not found, or it has no version {} or {}\n",
                args.dataset_id, args.base_version, args.target_version
            ))
        }
        Err(error) if error.is_not_found_for("version") => {
            return Outcome::failure(format!(
                "Version {} or {} of dataset {} not found\n",
                args.base_version, args.target_version, args.dataset_id
            ))
        }
        Err(error) if error.is_not_found_for("dataset") => {
            return dataset_not_found(&args.dataset_id)
        }
        Err(error) => {
            return Outcome::failure(format!("Failed to compare dataset versions: {error}\n"))
        }
    };
    let next_cursor = page
        .next_cursor
        .as_deref()
        .filter(|next| !next.is_empty() && !page.changes.is_empty());
    let mut outcome = format_list_output(&page.changes, format, NextCursor::Page(next_cursor));
    if outcome.exit_code == 0 {
        outcome.stderr.extend_from_slice(
            format!(
                "{} added, {} removed, {} changed\n",
                plural(page.added_count, "episode"),
                page.removed_count,
                page.changed_count
            )
            .as_bytes(),
        );
        if let Some(next) = next_cursor {
            outcome.stderr.extend_from_slice(
                format!("More changes exist; rerun with --cursor {next} to fetch the next page.\n")
                    .as_bytes(),
            );
        }
    }
    outcome
}

pub(crate) async fn restore_version(
    runtime: &Runtime,
    args: &DatasetVersionRestoreArgs,
) -> Outcome {
    match runtime
        .client
        .post_with_query::<_, _, RestoreResponse>(
            &format!(
                "{}/versions/{}/restore",
                dataset_endpoint(&args.dataset_id),
                args.version
            ),
            &RestoreQuery { force: args.force },
            &EmptyRequest {},
        )
        .await
    {
        Ok(response) => Outcome::notice(restore_summary(&response, args)),
        Err(error) if error.is_bare_not_found() => Outcome::failure(format!(
            "Dataset {} not found, or it has no committed version {}\n",
            args.dataset_id, args.version
        )),
        Err(error) if error.is_not_found_for("version") => Outcome::failure(format!(
            "Committed version {} of dataset {} not found\n",
            args.version, args.dataset_id
        )),
        Err(error) if error.is_not_found_for("dataset") => dataset_not_found(&args.dataset_id),
        Err(ApiError::Response { status: 409, .. }) if !args.force => Outcome::failure(format!(
            "Dataset {} has pending changes. Commit them first, or pass --force to discard them.\n",
            args.dataset_id
        )),
        Err(error) => Outcome::failure(format!(
            "Failed to restore version {}: {error}\n",
            args.version
        )),
    }
}

fn restore_summary(response: &RestoreResponse, args: &DatasetVersionRestoreArgs) -> String {
    let mut summary = String::new();
    if response.discarded_adds > 0 || response.discarded_removes > 0 {
        let _ = writeln!(
            summary,
            "Discarded {}",
            pending_changes(response.discarded_adds, response.discarded_removes, 0)
        );
    }
    if response.added == 0 && response.removed == 0 {
        let _ = writeln!(summary, "The draft matches version {}", args.version);
    } else {
        let _ = writeln!(
            summary,
            "Restored version {} into the draft: {} added, {} removed",
            args.version,
            plural(response.added, "episode"),
            response.removed
        );
        summary.push_str(&commit_hint(&args.dataset_id));
    }
    summary
}

fn pending_changes(additions: usize, removals: usize, updates: usize) -> String {
    let mut parts: Vec<String> = [
        (additions, "pending addition"),
        (removals, "pending removal"),
        (updates, "pending episode version update"),
    ]
    .into_iter()
    .filter(|(count, _)| *count > 0)
    .map(|(count, noun)| plural(count, noun))
    .collect();
    let last = parts.pop().unwrap_or_default();
    if parts.is_empty() {
        last
    } else {
        format!("{} and {last}", parts.join(", "))
    }
}

pub(crate) async fn commit_dataset(runtime: &Runtime, args: &DatasetIdArgs) -> Outcome {
    match runtime
        .client
        .post::<_, CommitResponse>(
            &format!("{}/commit", dataset_endpoint(&args.dataset_id)),
            &EmptyRequest {},
        )
        .await
    {
        Ok(CommitResponse { committed }) => Outcome::notice(format!(
            "Committed version {} with {} ({} added, {} removed{})\n",
            committed.version_number,
            plural(committed.episode_count, "episode"),
            committed.added_episode_count,
            committed.removed_episode_count,
            committed
                .changed_episode_count
                .map(|count| format!(", {count} changed"))
                .unwrap_or_default()
        )),
        Err(error) if error.is_not_found_for("dataset") => dataset_not_found(&args.dataset_id),
        Err(error) => Outcome::failure(format!("Failed to commit dataset: {error}\n")),
    }
}

pub(crate) async fn discard_dataset(runtime: &Runtime, args: &DatasetIdArgs) -> Outcome {
    match runtime
        .client
        .post::<_, DiscardResponse>(
            &format!("{}/discard", dataset_endpoint(&args.dataset_id)),
            &EmptyRequest {},
        )
        .await
    {
        Ok(response) if response.adds == 0 && response.removes == 0 && response.updates == 0 => {
            Outcome::notice("No pending changes to discard\n")
        }
        Ok(response) => Outcome::notice(format!(
            "Discarded {}\n",
            pending_changes(response.adds, response.removes, response.updates)
        )),
        Err(error) if error.is_not_found_for("dataset") => dataset_not_found(&args.dataset_id),
        Err(error) => Outcome::failure(format!("Failed to discard dataset changes: {error}\n")),
    }
}

#[cfg(test)]
mod tests {
    use super::{DatasetEpisode, DatasetEpisodeChange, DatasetVersion, DatasetVersionDetail};
    use crate::records::Record;

    #[test]
    fn the_uncommitted_version_is_listed_as_the_draft() {
        let versions: Vec<DatasetVersion> = serde_json::from_value(serde_json::json!([
            {"versionNumber": 5, "createdAt": "2024-01-03T00:00:00Z", "episodeCount": 12, "addedEpisodeCount": 3, "removedEpisodeCount": 1},
            {"versionNumber": 4, "committedAt": "2024-01-02T00:00:00Z", "createdAt": "2024-01-01T00:00:00Z", "episodeCount": 10, "addedEpisodeCount": 10, "removedEpisodeCount": 0},
        ]))
        .unwrap();
        assert_eq!(
            versions[0].fields(),
            ["5", "draft", "", "12", "3", "1", "", "2024-01-03T00:00:00Z"]
        );
        assert_eq!(
            versions[1].fields()[1..3],
            ["committed", "2024-01-02T00:00:00Z"]
        );
        let output = serde_json::to_value(&versions[0]).unwrap();
        assert!(output.get("committedAt").is_none());
    }

    #[test]
    fn a_version_reports_missing_recordings_after_its_counts() {
        let detail: DatasetVersionDetail = serde_json::from_value(serde_json::json!({
            "versionNumber": 4,
            "committedAt": "2024-01-02T00:00:00Z",
            "createdAt": "2024-01-01T00:00:00Z",
            "episodeCount": 10,
            "addedEpisodeCount": 10,
            "removedEpisodeCount": 0,
            "hasMissingRecordings": true,
            "changedEpisodeCount": 3,
        }))
        .unwrap();
        assert_eq!(DatasetVersionDetail::headers().len(), detail.fields().len());
        assert_eq!(detail.fields().last().unwrap(), "true");
        let output = serde_json::to_value(&detail).unwrap();
        assert_eq!(output["versionNumber"], 4);
        assert_eq!(output["hasMissingRecordings"], true);
        assert_eq!(output["changedEpisodeCount"], 3);
        assert_eq!(detail.fields()[6], "3");
    }

    #[test]
    fn a_change_leads_with_its_side_and_keeps_the_membership_columns() {
        assert_eq!(
            DatasetEpisodeChange::headers()[1..=DatasetEpisode::headers().len()],
            *DatasetEpisode::headers()
        );
        let change: DatasetEpisodeChange = serde_json::from_value(serde_json::json!({
            "change": "removed",
            "addedAt": "2024-01-02T03:04:08Z",
            "addedInVersion": 2,
            "episode": {
                "id": "ep_fixture",
                "version": 1,
                "latestVersion": 1,
                "projectId": "prj_default",
                "startTime": "2024-01-02T03:04:05Z",
                "endTime": "2024-01-02T03:04:06Z",
                "metadata": {},
                "createdAt": "2024-01-02T03:04:07Z",
            },
        }))
        .unwrap();
        let fields = change.fields();
        assert_eq!(fields[..2], ["removed", "ep_fixture"]);
        let output = serde_json::to_value(&change).unwrap();
        assert_eq!(output["change"], "removed");
        assert_eq!(output["episode"]["id"], "ep_fixture");
    }
}
