//! Attachment commands.

use std::io::Write;

use serde::{Deserialize, Serialize};

use crate::api::encode_path_segment;
use crate::cli::{AttachmentDownloadArgs, AttachmentGetArgs, AttachmentListArgs};
use crate::export::BINARY_OUTPUT_TERMINAL_ERROR;
use crate::output::Format;
use crate::records::{
    fetch_list, format_record, is_zero, ProjectFallback, Record, DEFAULT_LIST_LIMIT,
};
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
    #[serde(
        rename = "dataOffset",
        skip_serializing_if = "Option::is_none",
        default
    )]
    data_offset: Option<i64>,
    #[serde(rename = "lakePath", skip_serializing_if = "Option::is_none", default)]
    lake_path: Option<String>,
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
            "Data Offset",
            "Lake Path",
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
            self.data_offset
                .map(|offset| offset.to_string())
                .unwrap_or_default(),
            self.lake_path.clone().unwrap_or_default(),
        ]
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AttachmentListQuery {
    #[serde(skip_serializing_if = "String::is_empty")]
    device_id: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    device_name: String,
    limit: i64,
    #[serde(skip_serializing_if = "is_zero")]
    offset: i64,
    #[serde(skip_serializing_if = "String::is_empty")]
    project_id: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    recording_id: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    site_id: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    sort_by: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    sort_order: String,
}

pub(crate) async fn list_attachments(
    runtime: &Runtime,
    args: &AttachmentListArgs,
    format: Format,
) -> Outcome {
    let query = AttachmentListQuery {
        device_id: args.device_id.clone().unwrap_or_default(),
        device_name: args.device_name.clone().unwrap_or_default(),
        limit: args.limit.unwrap_or(DEFAULT_LIST_LIMIT),
        offset: args.offset.unwrap_or_default(),
        project_id: args.project_id.clone().or_project(&runtime.project_id),
        recording_id: args.recording_id.clone().unwrap_or_default(),
        site_id: args.site_id.clone().unwrap_or_default(),
        sort_by: args.sort_by.clone().unwrap_or_default(),
        sort_order: args.sort_order.clone().unwrap_or_default(),
    };
    fetch_list::<Attachment, _>(
        runtime,
        format,
        "Failed to list attachments",
        "/v1/recording-attachments",
        &query,
        Some(query.limit),
    )
    .await
}

pub(crate) async fn get_attachment(
    runtime: &Runtime,
    args: &AttachmentGetArgs,
    format: Format,
) -> Outcome {
    let endpoint = format!(
        "/v1/recording-attachments/{}",
        encode_path_segment(&args.attachment_id)
    );
    match runtime.client.get::<_, Attachment>(&endpoint, &()).await {
        Ok(attachment) => format_record(&attachment, format),
        Err(error) if error.is_not_found_for("attachment") => {
            Outcome::failure(format!("Attachment not found: {}\n", args.attachment_id))
        }
        Err(error) => Outcome::failure(format!("Failed to get attachment: {error}\n")),
    }
}

pub(crate) async fn download_attachment(
    runtime: &Runtime,
    args: &AttachmentDownloadArgs,
    stdout_writer: &mut dyn Write,
    stdout_is_terminal: bool,
) -> Outcome {
    if stdout_is_terminal {
        return Outcome::failure(format!("{BINARY_OUTPUT_TERMINAL_ERROR}\n"));
    }
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

#[cfg(test)]
mod tests {
    use std::fs;

    use super::{download_attachment, Attachment, BINARY_OUTPUT_TERMINAL_ERROR};
    use crate::cli::AttachmentDownloadArgs;
    use crate::records::Record;

    #[test]
    fn data_offset_and_lake_path_are_optional() {
        let base = serde_json::json!({
            "id": "att_1",
            "recordingId": "rec_1",
            "siteId": "site_1",
            "name": "a.yaml",
            "mediaType": "application/yaml",
            "logTime": "",
            "createTime": "",
            "crc": 0,
            "size": 1,
            "fingerprint": "f",
        });
        let attachment: Attachment = serde_json::from_value(base.clone()).unwrap();
        assert_eq!(attachment.fields()[10..], ["", ""]);
        let json = serde_json::to_value(&attachment).unwrap();
        assert!(json.get("dataOffset").is_none(), "{json}");
        assert!(json.get("lakePath").is_none(), "{json}");

        let mut with_fields = base;
        with_fields["dataOffset"] = serde_json::json!(4096);
        with_fields["lakePath"] = serde_json::json!("lake/rec_1/a.yaml");
        let attachment: Attachment = serde_json::from_value(with_fields).unwrap();
        assert_eq!(attachment.fields()[10..], ["4096", "lake/rec_1/a.yaml"]);
        let json = serde_json::to_value(&attachment).unwrap();
        assert_eq!(json["dataOffset"], 4096);
        assert_eq!(json["lakePath"], "lake/rec_1/a.yaml");
    }

    #[tokio::test]
    async fn downloads_refuse_a_terminal_before_requesting() {
        let directory = std::env::temp_dir().join(format!(
            "foxglove-rust-attachments-test-{}",
            std::process::id()
        ));
        fs::create_dir_all(&directory).unwrap();
        let config = directory.join(".foxgloverc");
        fs::write(&config, "base_url: http://127.0.0.1:1\n").unwrap();
        let runtime = crate::runtime::load(Some(&config), None, false).unwrap();
        fs::remove_dir_all(&directory).unwrap();
        let args = AttachmentDownloadArgs {
            attachment_id: "att_1".to_owned(),
        };

        let outcome = download_attachment(&runtime, &args, &mut Vec::new(), true).await;
        assert_eq!(outcome.exit_code, 1);
        assert_eq!(
            outcome.stderr,
            format!("{BINARY_OUTPUT_TERMINAL_ERROR}\n").as_bytes()
        );
    }
}
