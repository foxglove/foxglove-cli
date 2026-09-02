//! Extension commands.
#![allow(clippy::struct_field_names)]

use clap::ArgMatches;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, ReadBuf};

use crate::output::Format;
use crate::read_helpers::{finish_list, Record, Runtime};
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
pub(crate) fn list_extensions(runtime: &Runtime, format: Format) -> Outcome {
    finish_list(
        runtime,
        format,
        "Failed to list extensions",
        |client| async move {
            client
                .get::<_, Vec<Extension>>("/v1/extensions", &Vec::<(String, String)>::new())
                .await
        },
    )
}

pub(crate) fn unpublish_extension(runtime: &Runtime, matches: &ArgMatches) -> Outcome {
    let id = crate::read_helpers::positional(matches, 0);
    match crate::read_helpers::block_on(runtime.client.delete(&format!("/v1/extensions/{id}"))) {
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

pub(crate) fn publish_extension(runtime: &Runtime, matches: &ArgMatches) -> Outcome {
    let filename = crate::read_helpers::positional(matches, 0);
    let path = Path::new(&filename);
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
    let reader = ProgressReader::new(file, metadata.len());
    let result = crate::read_helpers::block_on(async {
        let cancellation = crate::api::ctrl_c_cancellation_token();
        runtime
            .client
            .upload_extension_with_cancellation(reader, &cancellation)
            .await
    });
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

/// An asynchronous reader that reports extension-upload progress to stderr.
struct ProgressReader<R> {
    inner: R,
    total: u64,
    uploaded: u64,
    complete: bool,
}

impl<R> ProgressReader<R> {
    const fn new(inner: R, total: u64) -> Self {
        Self {
            inner,
            total,
            uploaded: 0,
            complete: false,
        }
    }

    fn report(&self, finished: bool) {
        let _ = write!(
            io::stderr(),
            "\ruploading {}/{} bytes",
            self.uploaded,
            self.total
        );
        if finished {
            let _ = writeln!(io::stderr());
        }
    }
}

impl<R: AsyncRead + Unpin> AsyncRead for ProgressReader<R> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        context: &mut Context<'_>,
        buffer: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let before = buffer.filled().len();
        match Pin::new(&mut self.inner).poll_read(context, buffer) {
            Poll::Ready(Ok(())) => {
                let read = buffer.filled().len().saturating_sub(before);
                self.uploaded = self.uploaded.saturating_add(read as u64);
                if read > 0 {
                    self.report(false);
                } else if !self.complete {
                    self.complete = true;
                    self.report(true);
                }
                Poll::Ready(Ok(()))
            }
            pending => pending,
        }
    }
}
