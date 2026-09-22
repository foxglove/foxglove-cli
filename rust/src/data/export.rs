//! Streaming, resumable data export.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs::{self, File};
use std::io::{self, IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::{Duration, Instant};
use time::OffsetDateTime;

use crate::api::{self, StreamRequest};
use crate::cli::DataExportArgs;
use crate::format::{
    Attachment, Channel, Error as FormatError, McapWriter, Message, ProtobufDecoder, RecordSink,
    Ros1DecoderCache, RosbagConnection, RosbagMessage, RosbagSink, RosbagWriter, Schema,
};
use crate::records::parse_timestamp_value;
use crate::runtime::Runtime;
use crate::Outcome;
use tokio::io::{AsyncRead, AsyncWriteExt, DuplexStream, ReadBuf};

const BINARY_OUTPUT_TERMINAL_ERROR: &str =
    "Binary output may screw up your terminal. Please redirect to a pipe or file.";
const PROGRESS_REPORT_INTERVAL: Duration = Duration::from_millis(100);

/// Export recording data as MCAP, ROS bag, or JSON.
///
/// Binary exports use recovery, reindexing, and atomic destination handling.
/// JSON exports are likewise staged before replacing their destination.
pub(crate) async fn export_data(
    runtime: &Runtime,
    args: &DataExportArgs,
    stdout: &mut dyn Write,
) -> Outcome {
    let request = match stream_request(args) {
        Ok(request) => request,
        Err(error) => return Outcome::failure(format!("Failed to build request: {error}\n")),
    };
    if !matches!(request.output_format.as_str(), "mcap0" | "bag1" | "json") {
        return Outcome::failure("Export failed: invalid format: supply mcap0, bag1, or json\n");
    }
    let destination = args
        .output_file
        .as_deref()
        .filter(|path| !path.is_empty())
        .map(PathBuf::from);
    if destination.is_none() && request.output_format != "json" && std::io::stdout().is_terminal() {
        return Outcome::failure(format!("{BINARY_OUTPUT_TERMINAL_ERROR}\n"));
    }

    let cancellation = api::ctrl_c_cancellation_token();
    let binary_file = destination.is_some() && request.output_format != "json";
    let result = match (request.output_format.as_str(), destination.as_deref()) {
        ("json", Some(path)) => staged_json_export(runtime, &request, path, &cancellation).await,
        (_, Some(path)) => resumable_export(runtime, request, path, &cancellation).await,
        (_, None) => stream_to_stdout(runtime, &request, stdout, &cancellation).await,
    };
    match result {
        Ok(()) if binary_file => Outcome {
            stderr: b"\n".to_vec(),
            ..Outcome::default()
        },
        Ok(()) => Outcome::default(),
        Err(api::ApiError::Cancelled) => Outcome {
            exit_code: 130,
            ..Outcome::default()
        },
        Err(error) => Outcome::failure(format!("Export failed: {error}\n")),
    }
}

async fn stream_to_stdout(
    runtime: &Runtime,
    request: &StreamRequest,
    stdout: &mut dyn Write,
    cancellation: &tokio_util::sync::CancellationToken,
) -> Result<(), api::ApiError> {
    let mut stream_request = request.clone();
    if stream_request.output_format == "json" {
        stream_request.output_format = "mcap0".into();
    }
    let mut stream = runtime
        .client
        .stream_with_cancellation(&stream_request, cancellation)
        .await?;
    if request.output_format == "json" {
        if std::io::stdout().is_terminal() {
            render_mcap_json_stream(&mut stream, stdout, &mut NoopProgress).await
        } else {
            render_mcap_json_stream(&mut stream, stdout, &mut ExportProgress::new()).await
        }
    } else {
        let mut progress = ExportProgress::new();
        while let Some(chunk) = stream.next_chunk().await? {
            stdout.write_all(&chunk).map_err(api::ApiError::Write)?;
            progress.advance(chunk.len());
        }
        Ok(())
    }
}

/// Render JSON to a private sibling staging directory. The old destination is
/// left untouched when the request, conversion, cancellation, or local write
/// fails.
async fn staged_json_export(
    runtime: &Runtime,
    request: &StreamRequest,
    destination: &Path,
    cancellation: &tokio_util::sync::CancellationToken,
) -> Result<(), api::ApiError> {
    let staging = create_export_staging(destination).map_err(api::ApiError::Write)?;
    let staged = staging.join("complete");
    let result = async {
        let mut output = create_export_file(&staged).map_err(api::ApiError::Write)?;
        let mut stream_request = request.clone();
        stream_request.output_format = "mcap0".into();
        let mut stream = runtime
            .client
            .stream_with_cancellation(&stream_request, cancellation)
            .await?;
        render_mcap_json_stream(&mut stream, &mut output, &mut NoopProgress).await?;
        output.flush().map_err(api::ApiError::Write)?;
        drop(output);
        crate::config::replace_file(&staged, destination).map_err(api::ApiError::Write)
    }
    .await;
    let _ = fs::remove_dir_all(staging);
    result
}

/// Render an opt-in export request diagnostic while keeping normal command
/// stderr unchanged.
pub(crate) fn export_debug_request(args: &DataExportArgs) -> Option<String> {
    stream_request(args)
        .ok()
        .map(|request| format!("[DEBUG] exporting with request: {request:#?}\n"))
}

fn stream_request(args: &DataExportArgs) -> Result<StreamRequest, String> {
    let output_format_value = args.output_format.clone().unwrap_or_default();
    let output_format = if output_format_value.is_empty() {
        "mcap0".to_owned()
    } else {
        output_format_value
    };
    let request = StreamRequest {
        episode_id: String::new(),
        recording_id: args.recording_id.clone().unwrap_or_default(),
        key: args.key.clone().unwrap_or_default(),
        import_id: args.import_id.clone().unwrap_or_default(),
        project_id: args.project_id.clone().unwrap_or_default(),
        device_id: args.device_id.clone().unwrap_or_default(),
        device_name: args.device_name.clone().unwrap_or_default(),
        start: parse_timestamp_value(args.start.as_deref().unwrap_or_default(), "start")?,
        end: parse_timestamp_value(args.end.as_deref().unwrap_or_default(), "end")?,
        output_format,
        compression_format: args.compression.clone(),
        include_attachments: args.include_attachments,
        replay_policy: args.replay_policy.clone().unwrap_or_default(),
        replay_lookback_seconds: args.replay_lookback_seconds.unwrap_or_default(),
        topics: args
            .topics
            .as_deref()
            .unwrap_or_default()
            .split(',')
            .filter(|topic| !topic.is_empty())
            .map(str::to_owned)
            .collect(),
        session_id: args.session_id.clone().unwrap_or_default(),
        session_key: args.session_key.clone().unwrap_or_default(),
    };
    request.validate()?;
    Ok(request)
}

#[derive(Clone, Copy, Debug, Default)]
struct ExportInfo {
    max_time: u64,
    message_count: u64,
}

impl ExportInfo {
    fn record(&mut self, time: u64) {
        self.message_count += 1;
        self.max_time = self.max_time.max(time);
    }
}

#[derive(Clone)]
struct PartialExport {
    path: PathBuf,
    info: ExportInfo,
}

async fn resumable_export(
    runtime: &Runtime,
    request: StreamRequest,
    destination: &Path,
    cancellation: &tokio_util::sync::CancellationToken,
) -> Result<(), api::ApiError> {
    resumable_download(
        runtime,
        request,
        destination,
        cancellation,
        &mut PartialExportProgress::default(),
        CompletionCheck::Reindex,
    )
    .await
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum CompletionCheck {
    Reindex,
    EndMagic,
}

/// Download into a private sibling directory, repair each interrupted response,
/// and replace the requested destination only after the complete result exists.
pub(crate) async fn resumable_download(
    runtime: &Runtime,
    mut request: StreamRequest,
    destination: &Path,
    cancellation: &tokio_util::sync::CancellationToken,
    progress: &mut dyn DownloadProgress,
    check: CompletionCheck,
) -> Result<(), api::ApiError> {
    let staging = create_export_staging(destination).map_err(api::ApiError::Write)?;
    let result = resumable_export_inner(
        runtime,
        &mut request,
        destination,
        &staging,
        cancellation,
        progress,
        check,
    )
    .await;
    let _ = fs::remove_dir_all(&staging);
    result
}

async fn resumable_export_inner(
    runtime: &Runtime,
    request: &mut StreamRequest,
    destination: &Path,
    staging: &Path,
    cancellation: &tokio_util::sync::CancellationToken,
    progress: &mut dyn DownloadProgress,
    check: CompletionCheck,
) -> Result<(), api::ApiError> {
    let mut partials = Vec::new();
    let mut complete_found = false;
    let mut empty_downloads = 0_u8;
    let mut repeated_starts = 0_u8;
    loop {
        let path = staging.join(format!("export-{}", partials.len()));
        let ended_cleanly =
            download_response(runtime, request, &path, cancellation, progress).await?;
        if check == CompletionCheck::EndMagic
            && ended_cleanly
            && partials.is_empty()
            && ends_with_mcap_magic(&path).map_err(api::ApiError::Write)?
        {
            partials.push(PartialExport {
                path,
                info: ExportInfo::default(),
            });
            complete_found = true;
            break;
        }
        let reindex_path = path.clone();
        let reindex_format = request.output_format.clone();
        let (complete, info) =
            tokio::task::spawn_blocking(move || reindex_partial(&reindex_path, &reindex_format))
                .await
                .map_err(|error| {
                    api::ApiError::Conversion(format!("failed to join reindex task: {error}"))
                })?
                .map_err(|error| {
                    api::ApiError::Conversion(format!("failed to reindex partial export: {error}"))
                })?;
        if cancellation.is_cancelled() {
            return Err(api::ApiError::Cancelled);
        }
        partials.push(PartialExport { path, info });
        if complete {
            complete_found = true;
            break;
        }
        if info.message_count == 0 {
            empty_downloads += 1;
            if empty_downloads > 1 {
                break;
            }
            continue;
        }
        empty_downloads = 0;
        let start = OffsetDateTime::from_unix_timestamp_nanos(i128::from(info.max_time)).map_err(
            |error| api::ApiError::Conversion(format!("invalid recovered timestamp: {error}")),
        )?;
        if request.start == Some(start) {
            repeated_starts += 1;
            if repeated_starts > 1 {
                break;
            }
        } else {
            repeated_starts = 0;
        }
        request.start = Some(start);
        if request.end.is_none() {
            request.end = Some(OffsetDateTime::now_utc());
        }
    }
    if check == CompletionCheck::EndMagic && !complete_found {
        return Err(api::ApiError::Conversion(
            "the stream ended before the download was complete".into(),
        ));
    }
    let merged = staging.join("complete");
    if partials.len() == 1 {
        fs::rename(&partials[0].path, &merged).map_err(api::ApiError::Write)?;
    } else {
        let partials_to_merge = partials.clone();
        let merge_format = request.output_format.clone();
        let merge_output = merged.clone();
        tokio::task::spawn_blocking(move || {
            merge_partials(&partials_to_merge, &merge_output, &merge_format)
        })
        .await
        .map_err(|error| api::ApiError::Conversion(format!("failed to join merge task: {error}")))?
        .map_err(|error| {
            api::ApiError::Conversion(format!("failed to merge partial exports: {error}"))
        })?;
        if cancellation.is_cancelled() {
            return Err(api::ApiError::Cancelled);
        }
    }
    crate::config::replace_file(&merged, destination).map_err(api::ApiError::Write)
}

async fn download_response(
    runtime: &Runtime,
    request: &StreamRequest,
    path: &Path,
    cancellation: &tokio_util::sync::CancellationToken,
    progress: &mut dyn DownloadProgress,
) -> Result<bool, api::ApiError> {
    let mut output =
        tokio::fs::File::from_std(create_export_file(path).map_err(api::ApiError::Write)?);
    let mut stream = runtime
        .client
        .stream_with_cancellation(request, cancellation)
        .await?;
    let mut bytes = 0_u64;
    let download = async {
        while let Some(chunk) = stream.next_chunk().await? {
            bytes += u64::try_from(chunk.len()).expect("chunk length fits u64");
            output
                .write_all(&chunk)
                .await
                .map_err(api::ApiError::Write)?;
            progress.advance(chunk.len());
        }
        output.flush().await.map_err(api::ApiError::Write)
    }
    .await;
    drop(output);
    progress.partial_finished();
    // A transport error after receiving bytes is a recoverable truncated
    // download. Local write failures and cancellation must preserve the
    // destination rather than being mistaken for a partial response.
    match download {
        Ok(()) => Ok(true),
        Err(api::ApiError::Transport(_)) if bytes > 0 => Ok(false),
        Err(error) => Err(error),
    }
}

fn create_export_staging(destination: &Path) -> std::io::Result<PathBuf> {
    let parent = destination
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    for attempt in 0..100_u32 {
        let path = parent.join(format!(".foxglove-export-{}-{attempt}", std::process::id()));
        let mut builder = fs::DirBuilder::new();
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            builder.mode(0o700);
        }
        match builder.create(&path) {
            Ok(()) => return Ok(path),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error),
        }
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::AlreadyExists,
        "could not create export staging directory",
    ))
}

/// Staged exports may contain private recording data, so their mode must not
/// depend on the caller's umask.
fn create_export_file(path: &Path) -> io::Result<File> {
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options.open(path)
}

const MCAP_MAGIC: &[u8; 8] = b"\x89MCAP0\r\n";

fn ends_with_mcap_magic(path: &Path) -> io::Result<bool> {
    use std::io::{Read, Seek, SeekFrom};

    let mut file = File::open(path)?;
    if file.metadata()?.len() < 2 * MCAP_MAGIC.len() as u64 {
        return Ok(false);
    }
    file.seek(SeekFrom::End(-8))?;
    let mut tail = [0_u8; 8];
    file.read_exact(&mut tail)?;
    Ok(&tail == MCAP_MAGIC)
}

fn reindex_partial(path: &Path, format: &str) -> Result<(bool, ExportInfo), FormatError> {
    match format {
        "mcap" | "mcap0" => reindex_mcap(path),
        "bag1" => reindex_bag(path),
        other => Err(FormatError::Invalid(format!(
            "unrecognized export format: {other}"
        ))),
    }
}

fn reindex_mcap(path: &Path) -> Result<(bool, ExportInfo), FormatError> {
    let recovered = path.with_extension("reindexed");
    let mut input = File::open(path)?;
    let output = create_export_file(&recovered)?;
    let mut sink = McapFileSink {
        writer: McapWriter::new(output)?,
        info: ExportInfo::default(),
    };
    let complete = crate::format::read_mcap_recover(&mut input, &mut sink)?;
    sink.writer.finish()?;
    let info = sink.info;
    drop(sink);
    drop(input);
    if complete {
        fs::remove_file(recovered)?;
    } else {
        crate::config::replace_file(&recovered, path)?;
    }
    Ok((complete, info))
}

fn reindex_bag(path: &Path) -> Result<(bool, ExportInfo), FormatError> {
    let recovered = path.with_extension("reindexed");
    let mut input = File::open(path)?;
    let output = create_export_file(&recovered)?;
    let mut sink = BagFileSink {
        writer: RosbagWriter::new(output)?,
        info: ExportInfo::default(),
        connections: HashSet::new(),
    };
    let complete = crate::format::read_rosbag_recover(&mut input, &mut sink)?;
    sink.writer.finish()?;
    let info = sink.info;
    drop(sink);
    drop(input);
    crate::config::replace_file(&recovered, path)?;
    Ok((complete, info))
}

struct McapFileSink {
    writer: McapWriter<File>,
    info: ExportInfo,
}

impl RecordSink for McapFileSink {
    fn schema(&mut self, schema: Schema) -> Result<(), FormatError> {
        self.writer.schema(&schema)
    }
    fn channel(&mut self, channel: Channel) -> Result<(), FormatError> {
        self.writer.channel(&channel)
    }
    fn message(&mut self, message: Message) -> Result<(), FormatError> {
        self.info.record(message.log_time);
        self.writer.message(&message)
    }
    fn metadata(
        &mut self,
        name: String,
        metadata: BTreeMap<String, String>,
    ) -> Result<(), FormatError> {
        self.writer.metadata(name, metadata)
    }
    fn attachment(&mut self, attachment: Attachment) -> Result<(), FormatError> {
        self.writer.attachment(&attachment)
    }
}

struct BagFileSink {
    writer: RosbagWriter<File>,
    info: ExportInfo,
    connections: HashSet<u32>,
}

impl RosbagSink for BagFileSink {
    fn connection(&mut self, connection: RosbagConnection) -> Result<(), FormatError> {
        if !self.connections.insert(connection.id) {
            return Ok(());
        }
        self.writer.connection(connection)
    }
    fn message(&mut self, message: RosbagMessage) -> Result<(), FormatError> {
        self.info.record(message.time);
        self.writer.message(&message)
    }
}

fn merge_partials(
    partials: &[PartialExport],
    output: &Path,
    format: &str,
) -> Result<(), FormatError> {
    match format {
        "mcap" | "mcap0" => merge_mcap_partials(partials, output),
        "bag1" => merge_bag_partials(partials, output),
        other => Err(FormatError::Invalid(format!(
            "unrecognized export format: {other}"
        ))),
    }
}

fn scan_through(index: usize, partials: &[PartialExport]) -> u64 {
    if partials[index + 1..]
        .iter()
        .all(|partial| partial.info.message_count == 0)
    {
        partials[index].info.max_time
    } else {
        partials[index].info.max_time.saturating_sub(1)
    }
}

fn merge_mcap_partials(partials: &[PartialExport], output: &Path) -> Result<(), FormatError> {
    let file = create_export_file(output)?;
    let mut sink = McapMergeSink {
        writer: McapWriter::new(file)?,
        schema_offset: 0,
        channel_offset: 0,
        max_schema: 0,
        max_channel: 0,
        scan_through: 0,
        earlier_records: HashSet::new(),
        current_records: HashSet::new(),
    };
    for (index, partial) in partials.iter().enumerate() {
        if partial.info.message_count == 0 {
            continue;
        }
        let current = std::mem::take(&mut sink.current_records);
        sink.earlier_records.extend(current);
        sink.schema_offset = sink.max_schema;
        sink.channel_offset = sink.max_channel.saturating_add(1);
        sink.scan_through = scan_through(index, partials);
        let mut input = File::open(&partial.path)?;
        crate::format::read_mcap(&mut input, &mut sink)?;
    }
    sink.writer.finish()
}

struct McapMergeSink {
    writer: McapWriter<File>,
    schema_offset: u16,
    channel_offset: u16,
    max_schema: u16,
    max_channel: u16,
    scan_through: u64,
    earlier_records: HashSet<u64>,
    current_records: HashSet<u64>,
}

impl McapMergeSink {
    fn repeats_earlier_partial(&mut self, fingerprint: impl std::hash::Hash) -> bool {
        use std::hash::{BuildHasher, BuildHasherDefault, DefaultHasher};

        let fingerprint = BuildHasherDefault::<DefaultHasher>::default().hash_one(fingerprint);
        self.current_records.insert(fingerprint);
        self.earlier_records.contains(&fingerprint)
    }
}

impl RecordSink for McapMergeSink {
    fn schema(&mut self, mut schema: Schema) -> Result<(), FormatError> {
        schema.id = schema
            .id
            .checked_add(self.schema_offset)
            .ok_or_else(|| FormatError::Invalid("too many MCAP schemas while merging".into()))?;
        self.max_schema = self.max_schema.max(schema.id);
        self.writer.schema(&schema)
    }
    fn channel(&mut self, mut channel: Channel) -> Result<(), FormatError> {
        channel.id = channel
            .id
            .checked_add(self.channel_offset)
            .ok_or_else(|| FormatError::Invalid("too many MCAP channels while merging".into()))?;
        // Schema ID zero means no schema and is not part of the ID namespace.
        if channel.schema_id != 0 {
            channel.schema_id = channel
                .schema_id
                .checked_add(self.schema_offset)
                .ok_or_else(|| {
                    FormatError::Invalid("too many MCAP schemas while merging".into())
                })?;
        }
        self.max_channel = self.max_channel.max(channel.id);
        self.writer.channel(&channel)
    }
    fn message(&mut self, mut message: Message) -> Result<(), FormatError> {
        if message.log_time > self.scan_through {
            return Ok(());
        }
        message.channel_id = message
            .channel_id
            .checked_add(self.channel_offset)
            .ok_or_else(|| FormatError::Invalid("too many MCAP channels while merging".into()))?;
        self.writer.message(&message)
    }
    fn metadata(
        &mut self,
        name: String,
        metadata: BTreeMap<String, String>,
    ) -> Result<(), FormatError> {
        if self.repeats_earlier_partial(("metadata", &name, &metadata)) {
            return Ok(());
        }
        self.writer.metadata(name, metadata)
    }
    fn attachment(&mut self, attachment: Attachment) -> Result<(), FormatError> {
        if self.repeats_earlier_partial((
            "attachment",
            attachment.log_time,
            attachment.create_time,
            &attachment.name,
            &attachment.media_type,
            &attachment.data,
        )) {
            return Ok(());
        }
        self.writer.attachment(&attachment)
    }
}

fn merge_bag_partials(partials: &[PartialExport], output: &Path) -> Result<(), FormatError> {
    let file = create_export_file(output)?;
    let mut sink = BagMergeSink {
        writer: RosbagWriter::new(file)?,
        connection_offset: 0,
        max_connection: 0,
        scan_through: 0,
        connections: HashSet::new(),
    };
    for (index, partial) in partials.iter().enumerate() {
        if partial.info.message_count == 0 {
            continue;
        }
        sink.connection_offset = sink.max_connection.saturating_add(1);
        sink.scan_through = scan_through(index, partials);
        sink.connections.clear();
        let mut input = File::open(&partial.path)?;
        let _ = crate::format::read_rosbag_recover(&mut input, &mut sink)?;
    }
    sink.writer.finish()
}

struct BagMergeSink {
    writer: RosbagWriter<File>,
    connection_offset: u32,
    max_connection: u32,
    scan_through: u64,
    connections: HashSet<u32>,
}

impl RosbagSink for BagMergeSink {
    fn connection(&mut self, mut connection: RosbagConnection) -> Result<(), FormatError> {
        if !self.connections.insert(connection.id) {
            return Ok(());
        }
        connection.id = connection
            .id
            .checked_add(self.connection_offset)
            .ok_or_else(|| {
                FormatError::Invalid("too many ROS bag connections while merging".into())
            })?;
        self.max_connection = self.max_connection.max(connection.id);
        self.writer.connection(connection)
    }
    fn message(&mut self, mut message: RosbagMessage) -> Result<(), FormatError> {
        if message.time > self.scan_through {
            return Ok(());
        }
        message.connection_id = message
            .connection_id
            .checked_add(self.connection_offset)
            .ok_or_else(|| {
                FormatError::Invalid("too many ROS bag connections while merging".into())
            })?;
        self.writer.message(&message)
    }
}

async fn render_mcap_json_stream(
    stream: &mut api::ResponseStream,
    stdout: &mut dyn Write,
    progress: &mut dyn DownloadProgress,
) -> Result<(), api::ApiError> {
    // The duplex buffer bounds memory use and provides backpressure: the
    // network reader only advances as the MCAP decoder consumes records.
    let (mut reader, mut writer) = tokio::io::duplex(64 * 1024);
    let mut output = ProgressWriter::new(stdout, progress);
    let mut sink = JsonSink::new(&mut output);
    let download = async { copy_stream_to_duplex(stream, &mut writer).await };
    let convert = async {
        crate::format::read_mcap_async(&mut reader, &mut sink)
            .await
            .map_err(|error| api::ApiError::Conversion(format!("JSON conversion error: {error}")))
    };
    tokio::try_join!(download, convert).map(|_| ())
}

async fn copy_stream_to_duplex(
    stream: &mut api::ResponseStream,
    writer: &mut DuplexStream,
) -> Result<(), api::ApiError> {
    while let Some(chunk) = stream.next_chunk().await? {
        writer
            .write_all(&chunk)
            .await
            .map_err(api::ApiError::Write)?;
    }
    writer.shutdown().await.map_err(api::ApiError::Write)
}

/// JSON progress measures emitted NDJSON bytes rather than the larger
/// downloaded MCAP stream.
struct ProgressWriter<'a> {
    inner: &'a mut dyn Write,
    progress: &'a mut dyn DownloadProgress,
}

impl<'a> ProgressWriter<'a> {
    fn new(inner: &'a mut dyn Write, progress: &'a mut dyn DownloadProgress) -> Self {
        Self { inner, progress }
    }
}

impl Write for ProgressWriter<'_> {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        let written = self.inner.write(buffer)?;
        self.progress.advance(written);
        Ok(written)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

#[cfg(test)]
fn render_mcap_json(bytes: &[u8], stdout: &mut dyn Write) -> Result<(), FormatError> {
    let mut sink = JsonSink::new(stdout);
    crate::format::read_mcap(&mut std::io::Cursor::new(bytes), &mut sink)
}

struct JsonSink<'a> {
    stdout: &'a mut dyn Write,
    schemas: HashMap<u16, Schema>,
    channels: HashMap<u16, Channel>,
    ros1: Ros1DecoderCache,
    protobuf: ProtobufDecoder,
}

impl<'a> JsonSink<'a> {
    fn new(stdout: &'a mut dyn Write) -> Self {
        Self {
            stdout,
            schemas: HashMap::new(),
            channels: HashMap::new(),
            ros1: Ros1DecoderCache::default(),
            protobuf: ProtobufDecoder::default(),
        }
    }
}

impl RecordSink for JsonSink<'_> {
    fn schema(&mut self, schema: Schema) -> Result<(), FormatError> {
        self.schemas.insert(schema.id, schema);
        Ok(())
    }

    fn channel(&mut self, channel: Channel) -> Result<(), FormatError> {
        self.channels.insert(channel.id, channel);
        Ok(())
    }

    fn message(&mut self, message: Message) -> Result<(), FormatError> {
        let channel = self.channels.get(&message.channel_id).ok_or_else(|| {
            FormatError::Invalid(format!("unknown MCAP channel: {}", message.channel_id))
        })?;
        let schema = self.schemas.get(&channel.schema_id).ok_or_else(|| {
            FormatError::Invalid(format!("unknown MCAP schema: {}", channel.schema_id))
        })?;
        let data = match schema.encoding.as_str() {
            "ros1msg" => self.ros1.decode_json(schema, &message.data)?,
            "protobuf" => serde_json::to_vec(&self.protobuf.decode_json(schema, &message.data)?)
                .map_err(|error| {
                    FormatError::Invalid(format!("failed to marshal message: {error}"))
                })?,
            _ => {
                return Err(FormatError::Invalid(
                    "JSON output only supported for ros1msg and protobuf schemas".into(),
                ))
            }
        };
        self.stdout.write_all(b"{\"topic\":")?;
        serde_json::to_writer(&mut *self.stdout, &channel.topic)
            .map_err(|error| FormatError::Invalid(format!("failed to write JSON: {error}")))?;
        self.stdout.write_all(b",\"sequence\":")?;
        self.stdout
            .write_all(message.sequence.to_string().as_bytes())?;
        self.stdout.write_all(b",\"log_time\":")?;
        write_decimal_time(self.stdout, message.log_time)?;
        self.stdout.write_all(b",\"publish_time\":")?;
        write_decimal_time(self.stdout, message.publish_time)?;
        self.stdout.write_all(b",\"data\":")?;
        self.stdout.write_all(&data)?;
        self.stdout.write_all(b"}\n")?;
        Ok(())
    }

    fn metadata(&mut self, _: String, _: BTreeMap<String, String>) -> Result<(), FormatError> {
        Ok(())
    }

    fn attachment(&mut self, _: Attachment) -> Result<(), FormatError> {
        Ok(())
    }
}

fn write_decimal_time(stdout: &mut dyn Write, value: u64) -> Result<(), FormatError> {
    write!(
        stdout,
        "{}.{:09}",
        value / 1_000_000_000,
        value % 1_000_000_000
    )?;
    Ok(())
}

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

/// Stderr progress for downloads with no known total.
struct ExportProgress<W: Write> {
    downloaded: u64,
    last_report: Instant,
    writer: Option<W>,
}

pub(crate) trait DownloadProgress {
    fn advance(&mut self, bytes: usize);
    fn partial_finished(&mut self) {}
}

#[derive(Default)]
struct PartialExportProgress(Option<ExportProgress<io::Stderr>>);

impl DownloadProgress for PartialExportProgress {
    fn advance(&mut self, bytes: usize) {
        self.0
            .get_or_insert_with(ExportProgress::new)
            .advance(bytes);
    }

    fn partial_finished(&mut self) {
        self.0 = None;
    }
}

struct NoopProgress;

impl DownloadProgress for NoopProgress {
    fn advance(&mut self, _bytes: usize) {}
}

impl ExportProgress<io::Stderr> {
    fn new() -> Self {
        Self::new_with_writer(io::stderr())
    }
}

impl<W: Write> ExportProgress<W> {
    fn new_with_writer(writer: W) -> Self {
        Self {
            downloaded: 0,
            last_report: Instant::now(),
            writer: Some(writer),
        }
    }

    fn advance(&mut self, bytes: usize) {
        self.downloaded = self.downloaded.saturating_add(bytes as u64);
        if Instant::now().saturating_duration_since(self.last_report) >= PROGRESS_REPORT_INTERVAL {
            self.report();
        }
    }

    fn report(&mut self) {
        if let Some(writer) = self.writer.as_mut() {
            let _ = write!(writer, "\rexporting: {} bytes", self.downloaded);
        }
        self.last_report = Instant::now();
    }

    fn finish(&mut self) {
        if self.downloaded == 0 {
            return;
        }
        if let Some(writer) = self.writer.as_mut() {
            let _ = writeln!(writer, "\rexporting: {} bytes", self.downloaded);
        }
        self.downloaded = 0;
    }

    #[cfg(test)]
    fn into_writer(mut self) -> W {
        self.finish();
        self.writer.take().expect("progress writer is present")
    }
}

impl<W: Write> Drop for ExportProgress<W> {
    fn drop(&mut self) {
        self.finish();
    }
}

impl<W: Write> DownloadProgress for ExportProgress<W> {
    fn advance(&mut self, bytes: usize) {
        Self::advance(self, bytes);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn export_progress_reports_downloaded_bytes() {
        let mut progress = ExportProgress::new_with_writer(Vec::new());
        progress.advance(1_024);
        assert!(progress.writer.as_ref().expect("writer").is_empty());
        progress.last_report = Instant::now()
            .checked_sub(PROGRESS_REPORT_INTERVAL)
            .expect("test instant supports 100ms subtraction");
        progress.advance(1);

        let output = String::from_utf8(progress.into_writer()).expect("utf8 progress");
        assert!(output.contains("exporting: 1025 bytes"));
        assert_eq!(output.matches('\n').count(), 1);
    }
    use std::io::Cursor;

    use crate::format::McapWriter;

    #[test]
    fn renders_ros1_mcap_as_ndjson() {
        let mut writer = McapWriter::new(Cursor::new(Vec::new())).expect("writer");
        writer
            .schema(&Schema {
                id: 1,
                name: "example/Message".into(),
                encoding: "ros1msg".into(),
                data: b"uint8 value\n".to_vec(),
            })
            .expect("schema");
        writer
            .channel(&Channel {
                id: 1,
                schema_id: 1,
                topic: "/example".into(),
                message_encoding: "ros1".into(),
                metadata: BTreeMap::new(),
            })
            .expect("channel");
        writer
            .message(&Message {
                channel_id: 1,
                sequence: 3,
                log_time: 4_000_000_005,
                publish_time: 6_000_000_007,
                data: vec![9],
            })
            .expect("message");
        let input = writer.finish_into().expect("finish").into_inner();

        let mut output = Vec::new();
        render_mcap_json(&input, &mut output).expect("render");
        assert_eq!(
            output,
            b"{\"topic\":\"/example\",\"sequence\":3,\"log_time\":4.000000005,\"publish_time\":6.000000007,\"data\":{\"value\":9}}\n"
        );
    }

    #[test]
    fn export_timestamp_matches_go_second_precision() {
        let value = parse_timestamp_value("2024-03-01T01:02:03.123456789Z", "start")
            .expect("timestamp")
            .expect("value");
        assert_eq!(value.nanosecond(), 0);
    }

    #[test]
    fn merge_sink_preserves_schema_zero_and_remaps_nonzero_schemas() {
        let directory = std::env::temp_dir().join(format!(
            "foxglove-rust-merge-schema-test-{}",
            std::process::id()
        ));
        fs::create_dir(&directory).unwrap();
        let path = directory.join("merged.mcap");
        let schema = Schema {
            id: 1,
            name: "example/Message".into(),
            encoding: "ros1msg".into(),
            data: b"uint8 value\n".to_vec(),
        };
        let mut writer = McapWriter::new(create_export_file(&path).unwrap()).unwrap();
        writer.schema(&schema).unwrap();
        let mut sink = McapMergeSink {
            writer,
            schema_offset: 1,
            channel_offset: 2,
            max_schema: 1,
            max_channel: 1,
            scan_through: u64::MAX,
            earlier_records: HashSet::new(),
            current_records: HashSet::new(),
        };
        sink.schema(schema).unwrap();
        for (id, schema_id) in [(1, 0), (2, 1)] {
            sink.channel(Channel {
                id,
                schema_id,
                topic: format!("/channel{id}"),
                message_encoding: "json".into(),
                metadata: BTreeMap::new(),
            })
            .unwrap();
        }
        sink.writer.finish().unwrap();
        drop(sink);
        let bytes = fs::read(&path).unwrap();
        let summary = mcap::Summary::read(&bytes).unwrap().unwrap();
        assert!(summary.channels[&3].schema.is_none());
        let remapped = summary.channels[&4].schema.as_ref().unwrap();
        assert_eq!(remapped.id, 2);
        assert_eq!(remapped.data.as_ref(), b"uint8 value\n");
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn scan_through_keeps_the_last_non_empty_partial_boundary() {
        let partials = vec![
            PartialExport {
                path: PathBuf::new(),
                info: ExportInfo {
                    max_time: 42,
                    message_count: 1,
                },
            },
            PartialExport {
                path: PathBuf::new(),
                info: ExportInfo::default(),
            },
        ];

        assert_eq!(scan_through(0, &partials), 42);
    }

    #[cfg(unix)]
    #[test]
    fn staged_export_file_is_private() {
        use std::os::unix::fs::PermissionsExt;

        let directory =
            std::env::temp_dir().join(format!("foxglove-rust-export-test-{}", std::process::id()));
        fs::create_dir_all(&directory).expect("temp directory");
        let path = directory.join("export");
        let file = create_export_file(&path).expect("staged file");
        drop(file);
        assert_eq!(
            fs::metadata(path).expect("metadata").permissions().mode() & 0o777,
            0o600
        );
        fs::remove_dir_all(directory).expect("remove temp directory");
    }

    #[cfg(unix)]
    #[test]
    fn staging_directory_and_reindexed_export_are_private() {
        use std::os::unix::fs::PermissionsExt;

        let directory = std::env::temp_dir().join(format!(
            "foxglove-rust-reindex-permissions-test-{}",
            std::process::id()
        ));
        fs::create_dir_all(&directory).expect("test directory");
        let destination = directory.join("output.bag");
        let staging = create_export_staging(&destination).expect("staging directory");
        assert_eq!(
            fs::metadata(&staging)
                .expect("staging metadata")
                .permissions()
                .mode()
                & 0o777,
            0o700
        );

        let partial = staging.join("partial.bag");
        let mut writer = RosbagWriter::new(Cursor::new(Vec::new())).expect("bag writer");
        writer
            .connection(RosbagConnection {
                id: 0,
                topic: "/example".into(),
                type_name: "example/Message".into(),
                md5sum: "fixture".into(),
                message_definition: Vec::new(),
                caller_id: None,
                latching: None,
            })
            .expect("connection");
        writer
            .message(&RosbagMessage {
                connection_id: 0,
                time: 1,
                data: vec![1],
            })
            .expect("message");
        let bytes = writer.finish_into().expect("finish bag").into_inner();
        fs::write(&partial, bytes).expect("partial bag");
        fs::set_permissions(&partial, fs::Permissions::from_mode(0o644))
            .expect("make regression observable");

        reindex_bag(&partial).expect("reindex bag");
        assert_eq!(
            fs::metadata(&partial)
                .expect("reindexed metadata")
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
        fs::remove_dir_all(directory).expect("remove test directory");
    }
}
