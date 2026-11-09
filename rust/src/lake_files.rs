//! Lake file listing.

use serde::{Deserialize, Serialize};
use time::Duration;

use crate::cli::LakeFileListArgs;
use crate::episodes::parse_time_range;
use crate::output::Format;
use crate::records::{fetch_list, parse_timestamp_value, Record};
use crate::runtime::Runtime;
use crate::Outcome;

/// Longest `--start` to `--end` range the API accepts.
const MAX_RANGE: Duration = Duration::hours(24);

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
struct LakeFile {
    path: String,
    topics: Vec<String>,
}

impl Record for LakeFile {
    fn headers() -> &'static [&'static str] {
        &["Path", "Topics"]
    }

    fn fields(&self) -> Vec<String> {
        vec![self.path.clone(), self.topics.join(", ")]
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct LakeFileListQuery {
    #[serde(skip_serializing_if = "String::is_empty")]
    device_id: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    device_name: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    end: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    recording_id: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    recording_key: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    session_id: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    session_key: String,
    site_id: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    start: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    topic: String,
}

pub(crate) async fn list(runtime: &Runtime, args: &LakeFileListArgs, format: Format) -> Outcome {
    // A recording or session selector bounds the query on its own; only a device-wide query needs
    // a time range.
    let has_recording_or_session = args.recording_id.is_some()
        || args.recording_key.is_some()
        || args.session_id.is_some()
        || args.session_key.is_some();
    let (start, end) = match parse_time_range(args.start.as_deref(), args.end.as_deref()) {
        Ok(range) => range,
        Err(error) => return Outcome::failure(format!("{error}\n")),
    };
    if start.is_empty() {
        if !has_recording_or_session {
            return Outcome::failure(
                "--start and --end are required when listing by device without a recording or session\n",
            );
        }
    } else if exceeds_max_range(args) {
        return Outcome::failure("--start and --end must be at most 24 hours apart\n");
    }
    let query = LakeFileListQuery {
        device_id: args.device_id.clone().unwrap_or_default(),
        device_name: args.device_name.clone().unwrap_or_default(),
        end,
        recording_id: args.recording_id.clone().unwrap_or_default(),
        recording_key: args.recording_key.clone().unwrap_or_default(),
        session_id: args.session_id.clone().unwrap_or_default(),
        session_key: args.session_key.clone().unwrap_or_default(),
        site_id: args.site_id.clone(),
        start,
        topic: args.topic.clone().unwrap_or_default(),
    };
    fetch_list::<LakeFile, _>(
        runtime,
        format,
        "Failed to list lake files",
        "/v1/lake-files",
        &query,
        None,
    )
    .await
}

/// Report whether a validated `--start`/`--end` pair spans more than 24 hours.
fn exceeds_max_range(args: &LakeFileListArgs) -> bool {
    let parse = |raw: Option<&str>, label| parse_timestamp_value(raw.unwrap_or_default(), label);
    matches!(
        (parse(args.start.as_deref(), "start"), parse(args.end.as_deref(), "end")),
        (Ok(Some(start)), Ok(Some(end))) if end - start > MAX_RANGE
    )
}
