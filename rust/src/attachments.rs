//! Attachment commands.

use std::io::Write;

use serde::{Deserialize, Serialize};

use crate::cli::{AttachmentDownloadArgs, AttachmentListArgs};
use crate::output::Format;
use crate::records::{fetch_list, ProjectFallback, Record};
use crate::runtime::Runtime;
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

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AttachmentListQuery {
    #[serde(skip_serializing_if = "String::is_empty")]
    import_id: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    project_id: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    recording_id: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    session_id: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    session_key: String,
}

pub(crate) async fn list_attachments(
    runtime: &Runtime,
    args: &AttachmentListArgs,
    format: Format,
) -> Outcome {
    let query = AttachmentListQuery {
        import_id: args.import_id.clone().unwrap_or_default(),
        project_id: args.project_id.clone().or_project(&runtime.project_id),
        recording_id: args.recording_id.clone().unwrap_or_default(),
        session_id: args.session_id.clone().unwrap_or_default(),
        session_key: args.session_key.clone().unwrap_or_default(),
    };
    if !query.session_key.is_empty() && query.project_id.is_empty() {
        return Outcome::failure("--project-id is required when using --session-key\n");
    }
    fetch_list::<Attachment, _>(
        runtime,
        format,
        "Failed to list attachments",
        "/v1/recording-attachments",
        &query,
    )
    .await
}

pub(crate) async fn download_attachment(
    runtime: &Runtime,
    args: &AttachmentDownloadArgs,
    stdout_writer: &mut dyn Write,
) -> Outcome {
    let result = async {
        let cancellation = crate::api::ctrl_c_cancellation_token();
        let mut response = runtime
            .client
            .attachment_with_cancellation(&args.attachment_id, &cancellation)
            .await?;
        while let Some(chunk) = response.next_chunk().await? {
            stdout_writer
                .write_all(&chunk)
                .map_err(crate::api::ApiError::Write)?;
        }
        Ok::<(), crate::api::ApiError>(())
    }
    .await;
    match result {
        Ok(()) => Outcome::default(),
        Err(error) if error.is_cancelled() => Outcome {
            exit_code: 130,
            ..Outcome::default()
        },
        Err(error) => Outcome::failure(format!("Failed to fetch attachment: {error}\n")),
    }
}
