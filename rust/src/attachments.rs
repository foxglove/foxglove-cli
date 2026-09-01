//! Attachment commands.
#![allow(clippy::struct_field_names)]

use std::io::Write;

use clap::ArgMatches;
use serde::{Deserialize, Serialize};

use crate::output::Format;
use crate::read_helpers::{
    add_str, finish_list, query, session_key_error, sort_query, value, Record, Runtime,
};
use crate::Outcome;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
struct Attachment {
    id: String,
    #[serde(rename = "recordingId")]
    recording_id: String,
    #[serde(rename = "siteId")]
    site_id: String,
    name: String,
    #[serde(rename = "mediaType")]
    media_type: String,
    #[serde(rename = "logTime")]
    log_time: String,
    #[serde(rename = "createTime")]
    create_time: String,
    crc: u64,
    size: i64,
    fingerprint: String,
}

impl Record for Attachment {
    fn headers() -> &'static [&'static str] {
        &[
            "ID",
            "Recording ID",
            "Site ID",
            "Name",
            "Media Type",
            "Log Time",
            "Create Time",
            "CRC",
            "Size",
            "Fingerprint",
        ]
    }

    fn fields(&self) -> Vec<String> {
        vec![
            self.id.clone(),
            self.recording_id.clone(),
            self.site_id.clone(),
            self.name.clone(),
            self.media_type.clone(),
            self.log_time.clone(),
            self.create_time.clone(),
            self.crc.to_string(),
            self.size.to_string(),
            self.fingerprint.clone(),
        ]
    }
}
pub(crate) fn list_attachments(runtime: &Runtime, matches: &ArgMatches, format: Format) -> Outcome {
    let session_key = value(matches, "session-key");
    let project_id = value(matches, "project-id");
    if let Some(error) = session_key_error(matches, &project_id) {
        return Outcome::failure(error);
    }
    let mut query = query();
    add_str(&mut query, "importId", &value(matches, "import-id"));
    add_str(&mut query, "projectId", &project_id);
    add_str(&mut query, "recordingId", &value(matches, "recording-id"));
    add_str(&mut query, "sessionId", &value(matches, "session-id"));
    add_str(&mut query, "sessionKey", &session_key);
    sort_query(&mut query);
    finish_list(
        runtime,
        format,
        "Failed to list attachments",
        move |client| async move {
            client
                .get::<_, Vec<Attachment>>("/v1/recording-attachments", &query)
                .await
        },
    )
}

pub(crate) fn download_attachment(
    runtime: &Runtime,
    matches: &ArgMatches,
    stdout_writer: &mut dyn Write,
) -> Outcome {
    let id = crate::read_helpers::positional(matches, 0);
    let result = crate::read_helpers::block_on(async {
        let mut response = runtime.client.attachment(&id).await?;
        while let Some(chunk) = response.next_chunk().await? {
            stdout_writer
                .write_all(&chunk)
                .map_err(crate::api::ApiError::Write)?;
        }
        Ok::<(), crate::api::ApiError>(())
    });
    match result {
        Ok(()) => Outcome::default(),
        Err(error) => Outcome::failure(format!("Failed to fetch attachment: {error}\n")),
    }
}
