//! Extension commands.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

use crate::cli::{ExtensionIdArgs, FileArgs};
use crate::output::Format;
use crate::records::{fetch_list, Record};
use crate::runtime::Runtime;
use crate::Outcome;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
struct Extension {
    id: String,
    name: String,
    publisher: String,
    #[serde(rename = "displayName")]
    display_name: String,
    description: Option<String>,
    #[serde(rename = "activeVersion")]
    active_version: Option<String>,
    #[serde(rename = "sha256Sum")]
    sha256_sum: Option<String>,
}

impl Record for Extension {
    fn headers() -> &'static [&'static str] {
        &[
            "ID",
            "Name",
            "Publisher",
            "Display Name",
            "Description",
            "Active Version",
            "SHA-256 Sum",
        ]
    }

    fn fields(&self) -> Vec<String> {
        vec![
            self.id.clone(),
            self.name.clone(),
            self.publisher.clone(),
            self.display_name.clone(),
            self.description.clone().unwrap_or_default(),
            self.active_version.clone().unwrap_or_default(),
            self.sha256_sum.clone().unwrap_or_default(),
        ]
    }
}

pub(crate) async fn list_extensions(runtime: &Runtime, format: Format) -> Outcome {
    fetch_list::<Extension, _>(
        runtime,
        format,
        "Failed to list extensions",
        "/v1/extensions",
        &(),
    )
    .await
}

pub(crate) async fn unpublish_extension(runtime: &Runtime, args: &ExtensionIdArgs) -> Outcome {
    match runtime
        .client
        .delete(&format!("/v1/extensions/{}", args.extension_id))
        .await
    {
        Ok(()) => Outcome {
            stderr: b"Extension deleted\n".to_vec(),
            ..Outcome::default()
        },
        Err(error) if error.is_not_found() => Outcome {
            stderr: b"Not found. The resource may have already been deleted.\nExtension deleted\n"
                .to_vec(),
            ..Outcome::default()
        },
        Err(error) => Outcome::failure(format!("Failed to delete extension: {error}\n")),
    }
}

pub(crate) async fn publish_extension(runtime: &Runtime, args: &FileArgs) -> Outcome {
    let path = Path::new(&args.file);
    let metadata = match fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(error) => {
            return Outcome::failure(format!(
                "Extension upload failed: failed to open input file: {error}\n"
            ));
        }
    };
    if path.extension().and_then(|extension| extension.to_str()) != Some("foxe") {
        return Outcome::failure("Extension upload failed: file should have a '.foxe' extension\n");
    }
    if metadata.len() > 30 * 1024 * 1024 {
        return Outcome::failure("Extension upload failed: file size may not exceed 30mb\n");
    }
    let file = match std::fs::File::open(path) {
        Ok(file) => file,
        Err(error) => {
            return Outcome::failure(format!(
                "Extension upload failed: failed to open input file: {error}\n"
            ));
        }
    };
    let file = tokio::fs::File::from_std(file);
    let reader = crate::data::UploadProgressReader::new(file, metadata.len());
    let result = async {
        let cancellation = crate::api::ctrl_c_cancellation_token();
        runtime
            .client
            .upload_extension_with_cancellation(reader, &cancellation)
            .await
    }
    .await;
    match result {
        Ok(()) => Outcome {
            stderr: b"Extension published\n".to_vec(),
            ..Outcome::default()
        },
        Err(error) if error.is_cancelled() => Outcome {
            exit_code: 130,
            ..Outcome::default()
        },
        Err(error) => Outcome::failure(format!("Extension upload failed: {error}\n")),
    }
}
