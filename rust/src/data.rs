//! Data commands.
#![allow(clippy::struct_field_names)]

use clap::ArgMatches;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs::{self, File};
use std::io::{IsTerminal, Write};
use std::path::{Path, PathBuf};
use time::format_description::well_known::Rfc3339;
use time::{Date, OffsetDateTime, PrimitiveDateTime, Time, UtcOffset};

use crate::api::{self, StreamRequest, UploadRequest};
use crate::format::{
    Attachment, Channel, Error as FormatError, McapWriter, Message, ProtobufDecoder, RecordSink,
    Ros1DecoderCache, RosbagConnection, RosbagMessage, RosbagSink, RosbagWriter, Schema,
};
use crate::output::Format;
use crate::read_helpers::{
    add, add_str, finish_list, parse_i64, parse_timestamp, query, session_key_error, sort_query,
    value, DeviceSummary, ProjectFallback, Record, Runtime,
};
use crate::Outcome;
use tokio::io::{AsyncWriteExt, DuplexStream};

const BINARY_OUTPUT_TERMINAL_ERROR: &str =
    "Binary output may screw up your terminal. Please redirect to a pipe or file.";

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
struct Import {
    id: String,
    #[serde(rename = "deviceId")]
    device_id: String,
    filename: String,
    #[serde(rename = "importTime")]
    import_time: String,
    start: String,
    end: String,
    #[serde(rename = "inputType")]
    input_type: String,
    #[serde(rename = "outputType")]
    output_type: String,
    #[serde(rename = "inputSize")]
    input_size: i64,
    #[serde(rename = "totalOutputSize")]
    total_output_size: i64,
}

impl Record for Import {
    fn headers() -> &'static [&'static str] {
        &[
            "Import ID",
            "Device ID",
            "Filename",
            "Import Time",
            "Start",
            "End",
            "Input Type",
            "Output Type",
            "Input Size",
            "Total Output Size",
        ]
    }

    fn fields(&self) -> Vec<String> {
        vec![
            self.id.clone(),
            self.device_id.clone(),
            self.filename.clone(),
            self.import_time.clone(),
            self.start.clone(),
            self.end.clone(),
            self.input_type.clone(),
            self.output_type.clone(),
            self.input_size.to_string(),
            self.total_output_size.to_string(),
        ]
    }
}
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
struct Coverage {
    #[serde(rename = "deviceId")]
    device_id: String,
    #[serde(default)]
    device: DeviceSummary,
    start: String,
    end: String,
    status: String,
}

impl Record for Coverage {
    fn headers() -> &'static [&'static str] {
        &["Device ID", "Device Name", "Start", "End", "Status"]
    }

    fn fields(&self) -> Vec<String> {
        vec![
            self.device.id.clone(),
            self.device.name.clone(),
            self.start.clone(),
            self.end.clone(),
            self.status.clone(),
        ]
    }
}

/// Execute the Phase 6 single-request export path.
///
/// Non-JSON `--output-file` exports deliberately remain in Phase 7 because
/// they require recovery, reindexing, and atomic destination handling.
pub(crate) fn export_data(
    runtime: &Runtime,
    matches: &ArgMatches,
    stdout: &mut dyn Write,
) -> Outcome {
    let request = match stream_request(matches) {
        Ok(request) => request,
        Err(error) => return Outcome::failure(format!("Failed to build request: {error}\n")),
    };
    if !matches!(request.output_format.as_str(), "mcap0" | "bag1" | "json") {
        return Outcome::failure("Export failed: invalid format: supply mcap0, bag1, or json\n");
    }
    if !value(matches, "output-file").is_empty() && request.output_format != "json" {
        let destination = PathBuf::from(value(matches, "output-file"));
        return match crate::read_helpers::block_on(async {
            let cancellation = api::ctrl_c_cancellation_token();
            resumable_export(runtime, request, &destination, &cancellation).await
        }) {
            Ok(()) => Outcome {
                stderr: b"\n".to_vec(),
                ..Outcome::default()
            },
            Err(api::ApiError::Cancelled) => Outcome {
                exit_code: 130,
                ..Outcome::default()
            },
            Err(error) => Outcome::failure(format!("Export failed: {error}\n")),
        };
    }
    if request.output_format != "json" && std::io::stdout().is_terminal() {
        return Outcome::failure(format!("{BINARY_OUTPUT_TERMINAL_ERROR}\n"));
    }

    let result = crate::read_helpers::block_on(async {
        let cancellation = api::ctrl_c_cancellation_token();
        let mut stream_request = request.clone();
        if stream_request.output_format == "json" {
            stream_request.output_format = "mcap0".into();
        }
        let mut stream = runtime
            .client
            .stream_with_cancellation(&stream_request, &cancellation)
            .await?;
        if request.output_format == "json" {
            render_mcap_json_stream(&mut stream, stdout).await
        } else {
            while let Some(chunk) = stream.next_chunk().await? {
                stdout.write_all(&chunk).map_err(api::ApiError::Write)?;
            }
            Ok(())
        }
    });
    match result {
        Ok(()) => Outcome::default(),
        Err(api::ApiError::Cancelled) => Outcome {
            exit_code: 130,
            ..Outcome::default()
        },
        Err(error) => Outcome::failure(format!("Export failed: {error}\n")),
    }
}

fn stream_request(matches: &ArgMatches) -> Result<StreamRequest, String> {
    let output_format = if matches.get_flag("json") {
        "json".to_owned()
    } else {
        let value = value(matches, "output-format");
        if value.is_empty() {
            "mcap0".to_owned()
        } else {
            value
        }
    };
    let request = StreamRequest {
        recording_id: value(matches, "recording-id"),
        key: value(matches, "key"),
        import_id: value(matches, "import-id"),
        project_id: value(matches, "project-id"),
        device_id: value(matches, "device-id"),
        device_name: value(matches, "device-name"),
        start: parse_export_timestamp(&value(matches, "start"), "start")?,
        end: parse_export_timestamp(&value(matches, "end"), "end")?,
        output_format,
        compression_format: matches
            .value_source("compression")
            .is_some()
            .then(|| value(matches, "compression")),
        include_attachments: matches.get_flag("include-attachments"),
        replay_policy: value(matches, "replay-policy"),
        replay_lookback_seconds: parse_replay_lookback(matches)?,
        topics: value(matches, "topics")
            .split(',')
            .filter(|topic| !topic.is_empty())
            .map(str::to_owned)
            .collect(),
        session_id: value(matches, "session-id"),
        session_key: value(matches, "session-key"),
    };
    request.validate()?;
    Ok(request)
}

#[derive(Clone, Copy, Debug, Default)]
struct ExportInfo {
    max_time: u64,
    message_count: u64,
}

struct PartialExport {
    path: PathBuf,
    info: ExportInfo,
}

/// Download into a private sibling directory, repair each interrupted response,
/// and replace the requested destination only after the complete result exists.
async fn resumable_export(
    runtime: &Runtime,
    mut request: StreamRequest,
    destination: &Path,
    cancellation: &tokio_util::sync::CancellationToken,
) -> Result<(), api::ApiError> {
    let staging = create_export_staging(destination).map_err(api::ApiError::Write)?;
    let result =
        resumable_export_inner(runtime, &mut request, destination, &staging, cancellation).await;
    let _ = fs::remove_dir_all(&staging);
    result
}

async fn resumable_export_inner(
    runtime: &Runtime,
    request: &mut StreamRequest,
    destination: &Path,
    staging: &Path,
    cancellation: &tokio_util::sync::CancellationToken,
) -> Result<(), api::ApiError> {
    let mut partials = Vec::new();
    let mut empty_downloads = 0_u8;
    let mut repeated_starts = 0_u8;
    loop {
        let path = staging.join(format!("export-{}", partials.len()));
        let mut output = tokio::fs::File::create(&path)
            .await
            .map_err(api::ApiError::Write)?;
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
            }
            output.flush().await.map_err(api::ApiError::Write)
        }
        .await;
        drop(output);
        // A transport error after receiving bytes is a recoverable truncated
        // download. Local write failures and cancellation must preserve the
        // destination rather than being mistaken for a partial response.
        match download {
            Ok(()) => {}
            Err(api::ApiError::Transport(_)) if bytes > 0 => {}
            Err(error) => return Err(error),
        }
        let (complete, info) = reindex_partial(&path, &request.output_format).map_err(|error| {
            api::ApiError::Conversion(format!("failed to reindex partial export: {error}"))
        })?;
        partials.push(PartialExport { path, info });
        if complete {
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
    let merged = staging.join("complete");
    if partials.len() == 1 {
        fs::rename(&partials[0].path, &merged).map_err(api::ApiError::Write)?;
    } else {
        merge_partials(&partials, &merged, &request.output_format).map_err(|error| {
            api::ApiError::Conversion(format!("failed to merge partial exports: {error}"))
        })?;
    }
    fs::rename(&merged, destination).map_err(api::ApiError::Write)
}

fn create_export_staging(destination: &Path) -> std::io::Result<PathBuf> {
    let parent = destination
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    for attempt in 0..100_u32 {
        let path = parent.join(format!(".foxglove-export-{}-{attempt}", std::process::id()));
        match fs::create_dir(&path) {
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

fn reindex_partial(path: &Path, format: &str) -> Result<(bool, ExportInfo), FormatError> {
    match format {
        "mcap0" => reindex_mcap(path),
        "bag1" => reindex_bag(path),
        other => Err(FormatError::Invalid(format!(
            "unrecognized export format: {other}"
        ))),
    }
}

fn reindex_mcap(path: &Path) -> Result<(bool, ExportInfo), FormatError> {
    let mut input = File::open(path)?;
    let mut info = McapInfoSink::default();
    match crate::format::read_mcap(&mut input, &mut info) {
        Ok(()) => return Ok((true, info.info)),
        Err(FormatError::TruncatedMcap) => {}
        Err(error) => return Err(error),
    }
    let recovered = path.with_extension("reindexed");
    input = File::open(path)?;
    let output = File::create(&recovered)?;
    let mut sink = McapFileSink {
        writer: McapWriter::new(output)?,
        info: ExportInfo::default(),
    };
    let complete = crate::format::read_mcap_recover(&mut input, &mut sink)?;
    sink.writer.finish()?;
    fs::rename(recovered, path)?;
    Ok((complete, sink.info))
}

#[derive(Default)]
struct McapInfoSink {
    info: ExportInfo,
}

impl RecordSink for McapInfoSink {
    fn schema(&mut self, _: Schema) -> Result<(), FormatError> {
        Ok(())
    }
    fn channel(&mut self, _: Channel) -> Result<(), FormatError> {
        Ok(())
    }
    fn message(&mut self, message: Message) -> Result<(), FormatError> {
        self.info.message_count += 1;
        self.info.max_time = self.info.max_time.max(message.log_time);
        Ok(())
    }
    fn metadata(&mut self, _: String, _: BTreeMap<String, String>) -> Result<(), FormatError> {
        Ok(())
    }
    fn attachment(&mut self, _: Attachment) -> Result<(), FormatError> {
        Ok(())
    }
}

fn reindex_bag(path: &Path) -> Result<(bool, ExportInfo), FormatError> {
    let recovered = path.with_extension("reindexed");
    let mut input = File::open(path)?;
    let output = File::create(&recovered)?;
    let mut sink = BagFileSink {
        writer: RosbagWriter::new(output)?,
        info: ExportInfo::default(),
        connections: HashSet::new(),
    };
    let _ = crate::format::read_rosbag_recover(&mut input, &mut sink)?;
    sink.writer.finish()?;
    fs::rename(recovered, path)?;
    Ok((false, sink.info))
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
        self.info.message_count += 1;
        self.info.max_time = self.info.max_time.max(message.log_time);
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
        self.info.message_count += 1;
        self.info.max_time = self.info.max_time.max(message.time);
        self.writer.message(&message)
    }
}

fn merge_partials(
    partials: &[PartialExport],
    output: &Path,
    format: &str,
) -> Result<(), FormatError> {
    match format {
        "mcap0" => merge_mcap_partials(partials, output),
        "bag1" => merge_bag_partials(partials, output),
        other => Err(FormatError::Invalid(format!(
            "unrecognized export format: {other}"
        ))),
    }
}

fn scan_through(index: usize, partials: &[PartialExport]) -> u64 {
    if index + 1 == partials.len() {
        partials[index].info.max_time
    } else {
        partials[index].info.max_time.saturating_sub(1)
    }
}

fn merge_mcap_partials(partials: &[PartialExport], output: &Path) -> Result<(), FormatError> {
    let file = File::create(output)?;
    let mut sink = McapMergeSink {
        writer: McapWriter::new(file)?,
        schema_offset: 0,
        channel_offset: 0,
        max_schema: 0,
        max_channel: 0,
        scan_through: 0,
    };
    for (index, partial) in partials.iter().enumerate() {
        if partial.info.message_count == 0 {
            continue;
        }
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
        channel.schema_id = channel
            .schema_id
            .checked_add(self.schema_offset)
            .ok_or_else(|| FormatError::Invalid("too many MCAP schemas while merging".into()))?;
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
        self.writer.metadata(name, metadata)
    }
    fn attachment(&mut self, attachment: Attachment) -> Result<(), FormatError> {
        self.writer.attachment(&attachment)
    }
}

fn merge_bag_partials(partials: &[PartialExport], output: &Path) -> Result<(), FormatError> {
    let file = File::create(output)?;
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

fn parse_replay_lookback(matches: &ArgMatches) -> Result<f64, String> {
    let raw = value(matches, "replay-lookback-seconds");
    if raw.is_empty() {
        return Ok(0.0);
    }
    raw.parse()
        .map_err(|error| format!("invalid value for --replay-lookback-seconds: {error}"))
}

fn parse_export_timestamp(raw: &str, label: &str) -> Result<Option<OffsetDateTime>, String> {
    if raw.is_empty() {
        return Ok(None);
    }
    let parsed = OffsetDateTime::parse(raw, &Rfc3339)
        .or_else(|_| {
            let format = time::format_description::parse("[year]-[month]-[day]")
                .map_err(|error| error.to_string())?;
            let date = Date::parse(raw, &format).map_err(|error| error.to_string())?;
            Ok(PrimitiveDateTime::new(date, Time::MIDNIGHT).assume_offset(UtcOffset::UTC))
        })
        .map_err(|error: String| format!("failed to parse {label} time: {error}"))?;
    parsed
        .replace_nanosecond(0)
        .map(Some)
        .map_err(|error| format!("failed to parse {label} time: {error}"))
}

async fn render_mcap_json_stream(
    stream: &mut api::ResponseStream,
    stdout: &mut dyn Write,
) -> Result<(), api::ApiError> {
    // The duplex buffer bounds memory use and provides backpressure: the
    // network reader only advances as the MCAP decoder consumes records.
    let (mut reader, mut writer) = tokio::io::duplex(64 * 1024);
    let mut sink = JsonSink::new(stdout);
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

pub(crate) fn list_imports(runtime: &Runtime, matches: &ArgMatches, format: Format) -> Outcome {
    let mut query = query();
    let start = match parse_timestamp(&value(matches, "start"), "start") {
        Ok(value) => value,
        Err(error) => return Outcome::failure(format!("{error}\n")),
    };
    let end = match parse_timestamp(&value(matches, "end"), "end") {
        Ok(value) => value,
        Err(error) => return Outcome::failure(format!("{error}\n")),
    };
    let data_start = match parse_timestamp(&value(matches, "data-start"), "data start") {
        Ok(value) => value,
        Err(error) => return Outcome::failure(format!("{error}\n")),
    };
    let data_end = match parse_timestamp(&value(matches, "data-end"), "data end") {
        Ok(value) => value,
        Err(error) => return Outcome::failure(format!("{error}\n")),
    };
    add_str(&mut query, "dataEnd", &data_end);
    add_str(&mut query, "dataStart", &data_start);
    add_str(&mut query, "deviceId", &value(matches, "device-id"));
    add(
        &mut query,
        "includeDeleted",
        "true",
        matches.get_flag("include-deleted"),
    );
    add_str(&mut query, "end", &end);
    add_str(&mut query, "start", &start);
    sort_query(&mut query);
    finish_list(
        runtime,
        format,
        "Failed to list imports",
        move |client| async move {
            client
                .get::<_, Vec<Import>>("/v1/data/imports", &query)
                .await
        },
    )
}

pub(crate) fn list_coverage(runtime: &Runtime, matches: &ArgMatches, format: Format) -> Outcome {
    let project_id = value(matches, "project-id").or_project(&runtime.project_id);
    if let Some(error) = session_key_error(matches, &project_id) {
        return Outcome::failure(error);
    }
    let start = match parse_timestamp(&value(matches, "start"), "start") {
        Ok(value) => value,
        Err(error) => return Outcome::failure(format!("{error}\n")),
    };
    let end = match parse_timestamp(&value(matches, "end"), "end") {
        Ok(value) => value,
        Err(error) => return Outcome::failure(format!("{error}\n")),
    };
    let mut query = query();
    add_str(&mut query, "device.id", &value(matches, "device-id"));
    add_str(&mut query, "device.name", &value(matches, "device-name"));
    add_str(&mut query, "end", &end);
    add(
        &mut query,
        "includeEdgeRecordings",
        "true",
        matches.get_flag("include-edge-recordings"),
    );
    add_str(&mut query, "projectId", &project_id);
    add_str(&mut query, "recordingId", &value(matches, "recording-id"));
    add_str(&mut query, "sessionId", &value(matches, "session-id"));
    add_str(&mut query, "sessionKey", &value(matches, "session-key"));
    add_str(&mut query, "start", &start);
    let tolerance = match parse_i64(matches, "tolerance", 0) {
        Ok(value) => value,
        Err(error) => return Outcome::failure(format!("{error}\n")),
    };
    add(&mut query, "tolerance", tolerance, tolerance != 0);
    sort_query(&mut query);
    finish_list(
        runtime,
        format,
        "Failed to list coverage",
        move |client| async move {
            client
                .get::<_, Vec<Coverage>>("/v1/data/coverage", &query)
                .await
        },
    )
}

#[derive(Deserialize)]
struct ImportFromEdgeResponse {
    #[allow(dead_code)]
    id: String,
}

#[derive(Serialize)]
struct EmptyRequest {}

pub(crate) fn import_from_edge(runtime: &Runtime, matches: &ArgMatches) -> Outcome {
    let id = value(matches, "edge-recording-id");
    match crate::read_helpers::block_on(runtime.client.post::<_, ImportFromEdgeResponse>(
        &format!("/v1/recordings/{id}/import"),
        &EmptyRequest {},
    )) {
        Ok(_) => Outcome::default(),
        Err(error) => Outcome::failure(format!("Failed to import edge recording: {error}\n")),
    }
}

fn validate_import(path: &Path) -> Result<(), String> {
    let mut file = File::open(path).map_err(|error| error.to_string())?;
    crate::format::validate_import(&mut file).map_err(|error| error.to_string())
}

pub(crate) fn import_file(runtime: &Runtime, matches: &ArgMatches) -> Outcome {
    let filename = crate::read_helpers::positional(matches, 0);
    let path = Path::new(&filename);
    let project_id = value(matches, "project-id").or_project(&runtime.project_id);
    if let Some(error) = session_key_error(matches, &project_id) {
        return Outcome::failure(error);
    }
    if let Err(error) = validate_import(path) {
        return Outcome::failure(format!("Failed to import {filename}: {error}\n"));
    }
    let upload_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_owned();
    let request = UploadRequest {
        filename: upload_name,
        project_id,
        key: value(matches, "key"),
        device_id: value(matches, "device-id"),
        device_name: value(matches, "device-name"),
        session_id: value(matches, "session-id"),
        session_key: value(matches, "session-key"),
    };
    let result = crate::read_helpers::block_on(async {
        let file = tokio::fs::File::open(path)
            .await
            .map_err(|error| format!("failed to open input file: {error}"))?;
        let cancellation = crate::api::ctrl_c_cancellation_token();
        runtime
            .client
            .upload_with_cancellation(file, &request, &cancellation)
            .await
            .map_err(|error| error.to_string())
    });
    match result {
        Ok(()) => Outcome::default(),
        Err(error) if error == "operation cancelled" => Outcome {
            exit_code: 130,
            ..Outcome::default()
        },
        Err(error) => Outcome::failure(format!("Failed to import {filename}: {error}\n")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    use crate::format::McapWriter;

    #[test]
    fn renders_ros1_mcap_as_go_compatible_ndjson() {
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
        let value = parse_export_timestamp("2024-03-01T01:02:03.123456789Z", "start")
            .expect("timestamp")
            .expect("value");
        assert_eq!(value.nanosecond(), 0);
    }
}
