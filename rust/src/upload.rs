//! Local file uploads.

use std::fs::File;
use std::path::Path;

use std::io::{self, Write};
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::{Duration, Instant};

use crate::api::UploadRequest;
use crate::cli::UploadArgs;
use crate::records::ProjectFallback;
use crate::runtime::Runtime;
use crate::Outcome;
use tokio::io::{AsyncRead, ReadBuf};

fn validate(path: &Path) -> Result<(), String> {
    let mut file = File::open(path).map_err(|error| error.to_string())?;
    crate::format::validate_import(&mut file).map_err(|error| error.to_string())
}

pub(crate) async fn upload_file(runtime: &Runtime, args: &UploadArgs) -> Outcome {
    let path = Path::new(&args.file);
    let project_id = args.project_id.clone().or_project(&runtime.project_id);
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

const PROGRESS_REPORT_INTERVAL: Duration = Duration::from_millis(100);

/// A small stderr-only progress reader. Because it wraps the HTTP body, its
/// byte count is exactly the amount the client has requested from the file.
pub(crate) struct UploadProgressReader<R, W: Write> {
    inner: R,
    total: u64,
    uploaded: u64,
    finished: bool,
    last_report: Instant,
    writer: Option<W>,
}

impl<R> UploadProgressReader<R, io::Stderr> {
    pub(crate) fn new(inner: R, total: u64) -> Self {
        Self::new_with_writer(inner, total, io::stderr())
    }
}

impl<R, W: Write> UploadProgressReader<R, W> {
    fn new_with_writer(inner: R, total: u64, writer: W) -> Self {
        Self {
            inner,
            total,
            uploaded: 0,
            finished: false,
            last_report: Instant::now(),
            writer: Some(writer),
        }
    }

    fn report(&mut self, newline: bool) {
        if let Some(writer) = self.writer.as_mut() {
            let percent = self.uploaded.saturating_mul(100) / self.total.max(1);
            let _ = write!(
                writer,
                "\ruploading: {}/{} bytes ({percent}%)",
                self.uploaded, self.total
            );
            if newline {
                let _ = writeln!(writer);
            }
        }
        self.last_report = Instant::now();
    }

    fn finish(&mut self) {
        if !self.finished {
            self.finished = true;
            if self.total != 0 {
                self.report(true);
            }
        }
    }

    #[cfg(test)]
    fn into_writer(mut self) -> W {
        self.finish();
        self.writer.take().expect("progress writer is present")
    }
}

impl<R: AsyncRead + Unpin, W: Write + Unpin> AsyncRead for UploadProgressReader<R, W> {
    fn poll_read(
        self: Pin<&mut Self>,
        context: &mut Context<'_>,
        buffer: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        let before = buffer.filled().len();
        match Pin::new(&mut this.inner).poll_read(context, buffer) {
            Poll::Ready(Ok(())) => {
                let read = buffer.filled().len().saturating_sub(before);
                this.uploaded = this.uploaded.saturating_add(read as u64);
                if this.uploaded >= this.total || read == 0 {
                    this.finish();
                } else if Instant::now().saturating_duration_since(this.last_report)
                    >= PROGRESS_REPORT_INTERVAL
                {
                    this.report(false);
                }
                Poll::Ready(Ok(()))
            }
            pending => pending,
        }
    }
}

impl<R, W: Write> Drop for UploadProgressReader<R, W> {
    fn drop(&mut self) {
        // Close the progress line on every return path, including failures and
        // cancellation where reqwest stops reading before EOF.
        self.finish();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn upload_progress_reports_actual_bytes_and_completes_once() {
        let mut reader = UploadProgressReader::new_with_writer((), 1_024, Vec::new());
        assert!(reader.writer.as_ref().expect("writer").is_empty());
        reader.uploaded = 512;
        reader.last_report = Instant::now()
            .checked_sub(PROGRESS_REPORT_INTERVAL)
            .expect("test instant supports 100ms subtraction");
        reader.report(false);
        reader.finish();
        reader.finish();

        let output = String::from_utf8(reader.into_writer()).expect("utf8 progress");
        assert!(output.contains("uploading: 512/1024 bytes (50%)"));
        assert_eq!(output.matches("uploading: 512/1024 bytes (50%)").count(), 2);
        assert_eq!(output.matches('\n').count(), 1);
    }

    #[test]
    fn upload_progress_finishes_on_final_read_without_duplicate_drop_line() {
        use tokio::io::AsyncReadExt;

        let runtime = tokio::runtime::Runtime::new().expect("runtime");
        let mut reader =
            UploadProgressReader::new_with_writer(Cursor::new(vec![1_u8, 2, 3]), 3, Vec::new());
        let mut data = Vec::new();
        runtime
            .block_on(reader.read_to_end(&mut data))
            .expect("read input");
        assert_eq!(data, vec![1, 2, 3]);

        let output = String::from_utf8(reader.into_writer()).expect("utf8 progress");
        assert!(output.contains("uploading: 3/3 bytes (100%)"));
        assert_eq!(output.matches('\n').count(), 1);
    }

    #[test]
    fn upload_progress_zero_size_is_quiet() {
        let reader = UploadProgressReader::new_with_writer((), 0, Vec::new());
        let output = String::from_utf8(reader.into_writer()).expect("utf8 progress");
        assert!(output.is_empty());
    }
}
