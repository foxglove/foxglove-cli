//! File and edge-recording imports.

use std::fs::File;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::UploadProgressReader;
use crate::api::UploadRequest;
use crate::cli::DataImportArgs;
use crate::records::ProjectFallback;
use crate::runtime::Runtime;
use crate::Outcome;

#[derive(Deserialize)]
struct ImportFromEdgeResponse {
    #[allow(dead_code)]
    id: String,
}

#[derive(Serialize)]
struct EmptyRequest {}

pub(crate) async fn from_edge(runtime: &Runtime, args: &DataImportArgs) -> Outcome {
    let id = args.edge_recording_id.as_deref().unwrap_or_default();
    match runtime
        .client
        .post::<_, ImportFromEdgeResponse>(&format!("/v1/recordings/{id}/import"), &EmptyRequest {})
        .await
    {
        Ok(_) => Outcome::default(),
        Err(error) => Outcome::failure(format!("Failed to import edge recording: {error}\n")),
    }
}

fn validate(path: &Path) -> Result<(), String> {
    let mut file = File::open(path).map_err(|error| error.to_string())?;
    crate::format::validate_import(&mut file).map_err(|error| error.to_string())
}

pub(crate) async fn file(runtime: &Runtime, args: &DataImportArgs) -> Outcome {
    let path = Path::new(&args.file);
    let project_id = args
        .project_id
        .clone()
        .unwrap_or_default()
        .or_project(&runtime.project_id);
    let session_key = args.session_key.clone().unwrap_or_default();
    if !session_key.is_empty() && project_id.is_empty() {
        return Outcome::failure("--project-id is required when using --session-key\n");
    }
    if let Err(error) = validate(path) {
        return Outcome::failure(format!("Failed to import {}: {error}\n", args.file));
    }
    let upload_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_owned();
    let request = UploadRequest {
        filename: upload_name,
        project_id,
        key: args.key.clone().unwrap_or_default(),
        device_id: args.device_id.clone().unwrap_or_default(),
        device_name: args.device_name.clone().unwrap_or_default(),
        session_id: args.session_id.clone().unwrap_or_default(),
        session_key,
    };
    let input = match File::open(path) {
        Ok(file) => file,
        Err(error) => {
            return Outcome::failure(format!("Failed to import {}: {error}\n", args.file))
        }
    };
    let total = match input.metadata() {
        Ok(metadata) => metadata.len(),
        Err(error) => {
            return Outcome::failure(format!("Failed to import {}: {error}\n", args.file))
        }
    };
    let result = async {
        let file = UploadProgressReader::new(tokio::fs::File::from_std(input), total);
        let cancellation = crate::api::ctrl_c_cancellation_token();
        runtime
            .client
            .upload_with_cancellation(file, &request, &cancellation)
            .await
    }
    .await;
    match result {
        Ok(()) => Outcome::default(),
        Err(error) if error.is_cancelled() => Outcome {
            exit_code: 130,
            ..Outcome::default()
        },
        Err(error) => Outcome::failure(format!("Failed to import {}: {error}\n", args.file)),
    }
}
