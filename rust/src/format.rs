#![allow(
    clippy::assigning_clones,
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::missing_errors_doc,
    clippy::option_option,
    clippy::too_many_lines,
    clippy::type_complexity
)]
//! Streaming file-format primitives shared by export phases.
//!
//! This module deliberately has no CLI entrypoint.  It validates import files today and
//! exposes the decoded record model that direct export and recovery will use later.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt;
use std::io::{self, Cursor, Read, Seek, SeekFrom, Write};

use base64::Engine;
use lz4_flex::frame::FrameDecoder;
use mcap::records::{self, Record};
use mcap::sans_io::linear_reader::{LinearReadEvent, LinearReader, LinearReaderOptions};
use prost_reflect::{DescriptorPool, DynamicMessage};
use serde_json::Value;
use tokio::io::AsyncRead;

pub const MCAP_MAGIC: &[u8] = mcap::MAGIC;
pub const ROSBAG_MAGIC: &[u8] = b"#ROSBAG V2.0\n";
const MAX_RECORD_LEN: usize = 64 * 1024 * 1024;
// Keep both the chunk payload and its per-connection index in bounded memory. A
// single record can exceed this target (ROS bags cannot split a record), but
// ordinary exports never accumulate an unbounded number of records in a chunk.
const ROSBAG_CHUNK_TARGET_SIZE: usize = 768 * 1024;

#[derive(Debug)]
pub enum Error {
    InvalidMagic,
    TruncatedMcap,
    UnsupportedCompression(String),
    Invalid(String),
    Io(io::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidMagic => f.write_str("magic bytes do not match bag or mcap format"),
            Self::TruncatedMcap => f.write_str("truncated mcap file"),
            Self::UnsupportedCompression(compression) => {
                write!(f, "unsupported rosbag compression: {compression}")
            }
            Self::Invalid(message) => f.write_str(message),
            Self::Io(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for Error {}
impl From<io::Error> for Error {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Schema {
    pub id: u16,
    pub name: String,
    pub encoding: String,
    pub data: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Channel {
    pub id: u16,
    pub schema_id: u16,
    pub topic: String,
    pub message_encoding: String,
    pub metadata: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Message {
    pub channel_id: u16,
    pub sequence: u32,
    pub log_time: u64,
    pub publish_time: u64,
    pub data: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Attachment {
    pub log_time: u64,
    pub create_time: u64,
    pub name: String,
    pub media_type: String,
    pub data: Vec<u8>,
}

/// A streaming sink for the MCAP records Phase 6 and 7 need to preserve.
pub trait RecordSink {
    fn schema(&mut self, schema: Schema) -> Result<(), Error>;
    fn channel(&mut self, channel: Channel) -> Result<(), Error>;
    fn message(&mut self, message: Message) -> Result<(), Error>;
    fn metadata(&mut self, name: String, metadata: BTreeMap<String, String>) -> Result<(), Error>;
    fn attachment(&mut self, attachment: Attachment) -> Result<(), Error>;
}

/// A streaming sink for the connection and message records in a ROS 1 bag.
///
/// Unlike [`RecordSink`], this deliberately omits index records: callers use
/// it to rebuild a canonical indexed bag from both complete and interrupted
/// downloads.
pub trait RosbagSink {
    fn connection(&mut self, connection: RosbagConnection) -> Result<(), Error>;
    fn message(&mut self, message: RosbagMessage) -> Result<(), Error>;
}

pub fn validate_import<R: Read + Seek>(reader: &mut R) -> Result<(), Error> {
    let mut magic = [0_u8; 8];
    reader
        .read_exact(&mut magic)
        .map_err(|error| Error::Invalid(format!("failed to read magic bytes: {error}")))?;
    reader.seek(SeekFrom::Start(0))?;
    if magic == MCAP_MAGIC {
        return validate_mcap(reader);
    }
    let mut bag_magic = [0_u8; ROSBAG_MAGIC.len()];
    reader
        .read_exact(&mut bag_magic)
        .map_err(|error| Error::Invalid(format!("failed to read magic bytes: {error}")))?;
    reader.seek(SeekFrom::Start(0))?;
    if bag_magic == ROSBAG_MAGIC {
        return validate_rosbag(reader);
    }
    Err(Error::InvalidMagic)
}

/// Validate and emit an MCAP stream without buffering the entire input.
pub fn read_mcap<R: Read, S: RecordSink>(reader: &mut R, sink: &mut S) -> Result<(), Error> {
    read_mcap_inner(reader, sink, false).map(|_| ())
}

/// Emit every complete MCAP record before a truncated tail. The return value
/// is `true` if the input had a valid closing magic and `false` if recovery
/// stopped at a malformed or incomplete tail.
pub fn read_mcap_recover<R: Read, S: RecordSink>(
    reader: &mut R,
    sink: &mut S,
) -> Result<bool, Error> {
    read_mcap_inner(reader, sink, true)
}

fn read_mcap_inner<R: Read, S: RecordSink>(
    reader: &mut R,
    sink: &mut S,
    recover: bool,
) -> Result<bool, Error> {
    let options = LinearReaderOptions::default()
        .with_check_finishes_after_end_magic(true)
        .with_validate_chunk_crcs(true)
        .with_validate_data_section_crc(true)
        .with_record_length_limit(MAX_RECORD_LEN);
    let mut linear = LinearReader::new_with_options(options);
    let mut in_data_section = true;
    loop {
        let Some(event) = linear.next_event() else {
            return Ok(true);
        };
        let event = match event.map_err(map_mcap_error) {
            Ok(event) => event,
            Err(Error::TruncatedMcap) if recover => return Ok(false),
            Err(error) => return Err(error),
        };
        match event {
            LinearReadEvent::ReadRequest(length) => {
                let read = match reader.read(linear.insert(length)) {
                    Ok(read) => read,
                    Err(error) if recover && error.kind() == io::ErrorKind::UnexpectedEof => {
                        return Ok(false)
                    }
                    Err(error) => return Err(Error::Io(error)),
                };
                linear.notify_read(read);
            }
            LinearReadEvent::Record { opcode, data } => {
                let record = match mcap::parse_record(opcode, data).map_err(map_mcap_error) {
                    Ok(record) => record,
                    Err(Error::TruncatedMcap) if recover => return Ok(false),
                    Err(error) => return Err(error),
                };
                if matches!(record, Record::DataEnd(_)) {
                    in_data_section = false;
                } else if in_data_section {
                    emit_mcap_record(record, sink)?;
                }
            }
        }
    }
}

/// Asynchronously validate and emit an MCAP stream without retaining its
/// contents in memory.
///
/// This is the counterpart to [`read_mcap`] for network-backed readers. The
/// parser still processes records as soon as enough bytes arrive, while the
/// async reader supplies additional input only when requested.
pub async fn read_mcap_async<R: AsyncRead + Unpin, S: RecordSink>(
    reader: &mut R,
    sink: &mut S,
) -> Result<(), Error> {
    let options = LinearReaderOptions::default()
        .with_check_finishes_after_end_magic(true)
        .with_validate_chunk_crcs(true)
        .with_validate_data_section_crc(true)
        .with_record_length_limit(MAX_RECORD_LEN);
    let mut linear = LinearReader::new_with_options(options);
    let mut in_data_section = true;
    loop {
        let Some(event) = linear.next_event() else {
            return Ok(());
        };
        match event.map_err(map_mcap_error)? {
            LinearReadEvent::ReadRequest(length) => {
                let read = tokio::io::AsyncReadExt::read(reader, linear.insert(length))
                    .await
                    .map_err(Error::Io)?;
                linear.notify_read(read);
            }
            LinearReadEvent::Record { opcode, data } => {
                let record = mcap::parse_record(opcode, data).map_err(map_mcap_error)?;
                if matches!(record, Record::DataEnd(_)) {
                    in_data_section = false;
                } else if in_data_section {
                    emit_mcap_record(record, sink)?;
                }
            }
        }
    }
}

pub fn validate_mcap<R: Read>(reader: &mut R) -> Result<(), Error> {
    struct Validator;
    impl RecordSink for Validator {
        fn schema(&mut self, _: Schema) -> Result<(), Error> {
            Ok(())
        }
        fn channel(&mut self, _: Channel) -> Result<(), Error> {
            Ok(())
        }
        fn message(&mut self, _: Message) -> Result<(), Error> {
            Ok(())
        }
        fn metadata(&mut self, _: String, _: BTreeMap<String, String>) -> Result<(), Error> {
            Ok(())
        }
        fn attachment(&mut self, _: Attachment) -> Result<(), Error> {
            Ok(())
        }
    }
    read_mcap(reader, &mut Validator)
}

/// MCAP writer that preserves externally supplied schema and channel IDs.
/// It is intentionally format-only; command-level file staging belongs to Phase 7.
pub struct McapWriter<W: Write + Seek> {
    inner: mcap::Writer<W>,
}

impl<W: Write + Seek> McapWriter<W> {
    pub fn new(writer: W) -> Result<Self, Error> {
        mcap::Writer::new(writer)
            .map(|inner| Self { inner })
            .map_err(map_mcap_error)
    }

    pub fn schema(&mut self, schema: &Schema) -> Result<(), Error> {
        self.inner
            .add_schema_with_id(schema.id, &schema.name, &schema.encoding, &schema.data)
            .map(|_| ())
            .map_err(map_mcap_error)
    }

    pub fn channel(&mut self, channel: &Channel) -> Result<(), Error> {
        self.inner
            .add_channel_with_id(
                channel.id,
                channel.schema_id,
                &channel.topic,
                &channel.message_encoding,
                &channel.metadata,
            )
            .map(|_| ())
            .map_err(map_mcap_error)
    }

    pub fn message(&mut self, message: &Message) -> Result<(), Error> {
        self.inner
            .write_to_known_channel(
                &records::MessageHeader {
                    channel_id: message.channel_id,
                    sequence: message.sequence,
                    log_time: message.log_time,
                    publish_time: message.publish_time,
                },
                &message.data,
            )
            .map_err(map_mcap_error)
    }

    pub fn metadata(
        &mut self,
        name: String,
        metadata: BTreeMap<String, String>,
    ) -> Result<(), Error> {
        self.inner
            .write_metadata(&records::Metadata { name, metadata })
            .map_err(map_mcap_error)
    }

    pub fn attachment(&mut self, attachment: &Attachment) -> Result<(), Error> {
        self.inner
            .attach(&mcap::Attachment {
                log_time: attachment.log_time,
                create_time: attachment.create_time,
                name: attachment.name.clone(),
                media_type: attachment.media_type.clone(),
                data: std::borrow::Cow::Borrowed(&attachment.data),
            })
            .map_err(map_mcap_error)
    }

    pub fn finish(&mut self) -> Result<(), Error> {
        self.inner.finish().map(|_| ()).map_err(map_mcap_error)
    }

    pub fn finish_into(mut self) -> Result<W, Error> {
        self.finish()?;
        Ok(self.inner.into_inner())
    }
}

fn emit_mcap_record(record: Record<'_>, sink: &mut impl RecordSink) -> Result<(), Error> {
    match record {
        Record::Schema { header, data } => sink.schema(Schema {
            id: header.id,
            name: header.name,
            encoding: header.encoding,
            data: data.into_owned(),
        }),
        Record::Channel(channel) => sink.channel(Channel {
            id: channel.id,
            schema_id: channel.schema_id,
            topic: channel.topic,
            message_encoding: channel.message_encoding,
            metadata: channel.metadata,
        }),
        Record::Message { header, data } => sink.message(Message {
            channel_id: header.channel_id,
            sequence: header.sequence,
            log_time: header.log_time,
            publish_time: header.publish_time,
            data: data.into_owned(),
        }),
        Record::Metadata(metadata) => sink.metadata(metadata.name, metadata.metadata),
        Record::Attachment { header, data, .. } => sink.attachment(Attachment {
            log_time: header.log_time,
            create_time: header.create_time,
            name: header.name,
            media_type: header.media_type,
            data: data.into_owned(),
        }),
        _ => Ok(()),
    }
}

fn map_mcap_error(error: mcap::McapError) -> Error {
    match error {
        mcap::McapError::BadMagic => Error::InvalidMagic,
        mcap::McapError::UnexpectedEof
        | mcap::McapError::BadFooter
        | mcap::McapError::Parse(_)
        | mcap::McapError::RecordTooLarge { .. } => Error::TruncatedMcap,
        other => Error::Invalid(other.to_string()),
    }
}

/// A dynamic protobuf decoder, cached by the MCAP schema ID.
#[derive(Default)]
pub struct ProtobufDecoder {
    descriptors: HashMap<u16, prost_reflect::MessageDescriptor>,
}

impl ProtobufDecoder {
    pub fn decode_json(&mut self, schema: &Schema, data: &[u8]) -> Result<Value, Error> {
        let descriptor = if let Some(descriptor) = self.descriptors.get(&schema.id) {
            descriptor.clone()
        } else {
            let pool = DescriptorPool::decode(schema.data.as_slice()).map_err(|error| {
                Error::Invalid(format!("failed to build file descriptor set: {error}"))
            })?;
            let descriptor = pool.get_message_by_name(&schema.name).ok_or_else(|| {
                Error::Invalid(format!("failed to find descriptor: {}", schema.name))
            })?;
            self.descriptors.insert(schema.id, descriptor.clone());
            descriptor
        };
        let message = DynamicMessage::decode(descriptor, data)
            .map_err(|error| Error::Invalid(format!("failed to parse message: {error}")))?;
        serde_json::to_value(message)
            .map_err(|error| Error::Invalid(format!("failed to marshal message: {error}")))
    }
}

/// A compiled ROS 1 message definition. `transcode_json` preserves ROS's binary
/// field order and renders values according to their declared ROS types.
#[derive(Clone, Debug)]
pub struct Ros1Decoder {
    root: RosType,
    definitions: HashMap<String, Vec<RosField>>,
    package: String,
}

#[derive(Clone, Debug)]
struct RosField {
    name: String,
    ty: RosType,
}
#[derive(Clone, Debug)]
struct RosType {
    name: String,
    array: Option<Option<usize>>,
}

impl Ros1Decoder {
    pub fn new(parent_package: &str, definition: &[u8]) -> Result<Self, Error> {
        let text = std::str::from_utf8(definition)
            .map_err(|_| Error::Invalid("ros1 schema is not UTF-8".into()))?;
        let mut definitions = HashMap::new();
        let mut name = "__root__".to_owned();
        let mut lines = Vec::new();
        let mut flush = |name: &str, lines: &mut Vec<String>| -> Result<(), Error> {
            let mut fields = Vec::new();
            for line in lines.drain(..) {
                let line = line.split('#').next().unwrap_or_default().trim();
                if line.is_empty() || line.contains('=') {
                    continue;
                }
                let mut parts = line.split_whitespace();
                let ty = parts
                    .next()
                    .ok_or_else(|| Error::Invalid("invalid ros1 field".into()))?;
                let field = parts
                    .next()
                    .ok_or_else(|| Error::Invalid("invalid ros1 field".into()))?;
                if parts.next().is_some() {
                    return Err(Error::Invalid("invalid ros1 field".into()));
                }
                fields.push(RosField {
                    name: field.to_owned(),
                    ty: parse_ros_type(ty)?,
                });
            }
            definitions.insert(name.to_owned(), fields);
            Ok(())
        };
        for raw in text.lines() {
            if let Some(next) = raw.trim().strip_prefix("MSG: ") {
                flush(&name, &mut lines)?;
                name = next.trim().to_owned();
            } else if !raw.trim().starts_with("===") {
                lines.push(raw.to_owned());
            }
        }
        flush(&name, &mut lines)?;
        Ok(Self {
            root: RosType {
                name: "__root__".into(),
                array: None,
            },
            definitions,
            package: parent_package.to_owned(),
        })
    }

    pub fn transcode_json(&self, data: &[u8]) -> Result<Vec<u8>, Error> {
        let mut input = Cursor::new(data);
        let mut output = Vec::new();
        self.write_json_type(&self.root, &mut input, &mut output)?;
        if input.position() != data.len() as u64 {
            return Err(Error::Invalid("ros1 message has trailing bytes".into()));
        }
        Ok(output)
    }

    fn write_json_type(
        &self,
        ty: &RosType,
        input: &mut Cursor<&[u8]>,
        output: &mut Vec<u8>,
    ) -> Result<(), Error> {
        if let Some(array) = ty.array {
            let count = match array {
                Some(count) => count,
                None => read_u32(input)? as usize,
            };
            if count > MAX_RECORD_LEN {
                return Err(Error::Invalid("ros1 array exceeds configured limit".into()));
            }
            if matches!(ty.name.as_str(), "uint8" | "byte" | "char") {
                let mut bytes = vec![0; count];
                input.read_exact(&mut bytes)?;
                return serde_json::to_writer(output, &base64(&bytes))
                    .map_err(|error| Error::Invalid(error.to_string()));
            }
            output.push(b'[');
            let item = RosType {
                name: ty.name.clone(),
                array: None,
            };
            for index in 0..count {
                if index > 0 {
                    output.push(b',');
                }
                self.write_json_type(&item, input, output)?;
            }
            output.push(b']');
            return Ok(());
        }
        let mut scalar = |size: usize| -> Result<Vec<u8>, Error> {
            let mut bytes = vec![0; size];
            input.read_exact(&mut bytes)?;
            Ok(bytes)
        };
        match ty.name.as_str() {
            "bool" => match scalar(1)?[0] {
                0 => output.extend_from_slice(b"false"),
                1 => output.extend_from_slice(b"true"),
                _ => return Err(Error::Invalid("invalid ros1 bool".into())),
            },
            "int8" => output.extend_from_slice(
                i8::from_le_bytes(scalar(1)?.try_into().expect("size"))
                    .to_string()
                    .as_bytes(),
            ),
            "uint8" | "byte" | "char" => {
                output.extend_from_slice(scalar(1)?[0].to_string().as_bytes());
            }
            "int16" => output.extend_from_slice(
                i16::from_le_bytes(scalar(2)?.try_into().expect("size"))
                    .to_string()
                    .as_bytes(),
            ),
            "uint16" => output.extend_from_slice(
                u16::from_le_bytes(scalar(2)?.try_into().expect("size"))
                    .to_string()
                    .as_bytes(),
            ),
            "int32" => output.extend_from_slice(
                i32::from_le_bytes(scalar(4)?.try_into().expect("size"))
                    .to_string()
                    .as_bytes(),
            ),
            "uint32" => output.extend_from_slice(
                u32::from_le_bytes(scalar(4)?.try_into().expect("size"))
                    .to_string()
                    .as_bytes(),
            ),
            "int64" => output.extend_from_slice(
                i64::from_le_bytes(scalar(8)?.try_into().expect("size"))
                    .to_string()
                    .as_bytes(),
            ),
            "uint64" => serde_json::to_writer(
                output,
                &u64::from_le_bytes(scalar(8)?.try_into().expect("size")).to_string(),
            )
            .map_err(|error| Error::Invalid(error.to_string()))?,
            "float32" => write_json_float(
                f64::from(f32::from_le_bytes(scalar(4)?.try_into().expect("size"))),
                output,
            )?,
            "float64" => write_json_float(
                f64::from_le_bytes(scalar(8)?.try_into().expect("size")),
                output,
            )?,
            "string" => {
                let length = read_u32(input)? as usize;
                if length > MAX_RECORD_LEN {
                    return Err(Error::Invalid(
                        "ros1 string exceeds configured limit".into(),
                    ));
                }
                let mut bytes = vec![0; length];
                input.read_exact(&mut bytes)?;
                serde_json::to_writer(output, &String::from_utf8_lossy(&bytes))
                    .map_err(|error| Error::Invalid(error.to_string()))?;
            }
            "time" => {
                let secs = read_u32(input)?;
                let nanos = read_u32(input)?;
                output.extend_from_slice(format!("{secs}.{nanos:09}").as_bytes());
            }
            "duration" => {
                let secs = i32::from_le_bytes(scalar(4)?.try_into().expect("size"));
                let nanos = i32::from_le_bytes(scalar(4)?.try_into().expect("size"));
                let total_nanos = i64::from(secs) * 1_000_000_000 + i64::from(nanos);
                let sign = if total_nanos < 0 { "-" } else { "" };
                let magnitude = total_nanos.unsigned_abs();
                output.extend_from_slice(
                    format!(
                        "{sign}{}.{:09}",
                        magnitude / 1_000_000_000,
                        magnitude % 1_000_000_000
                    )
                    .as_bytes(),
                );
            }
            name => {
                let full_name = if name.contains('/') {
                    name.to_owned()
                } else {
                    format!("{}/{}", self.package, name)
                };
                let fields = self
                    .definitions
                    .get(name)
                    .or_else(|| self.definitions.get(&full_name))
                    .or_else(|| {
                        self.definitions.iter().find_map(|(candidate, fields)| {
                            candidate
                                .rsplit_once('/')
                                .is_some_and(|(_, short)| short == name)
                                .then_some(fields)
                        })
                    })
                    .ok_or_else(|| Error::Invalid(format!("unknown ros1 message type: {name}")))?;
                output.push(b'{');
                for (index, field) in fields.iter().enumerate() {
                    if index > 0 {
                        output.push(b',');
                    }
                    serde_json::to_writer(&mut *output, &field.name)
                        .map_err(|error| Error::Invalid(error.to_string()))?;
                    output.push(b':');
                    self.write_json_type(&field.ty, input, output)?;
                }
                output.push(b'}');
            }
        }
        Ok(())
    }
}

/// ROS 1 message decoders, compiled once per MCAP schema ID.
///
/// MCAP permits multiple schemas with the same name but different definitions,
/// so caching by schema name would be incorrect. The schema ID is stable within
/// an MCAP stream and is the key used by channels and messages.
#[derive(Default)]
pub struct Ros1DecoderCache {
    decoders: HashMap<u16, Ros1Decoder>,
}

impl Ros1DecoderCache {
    /// Decode one ROS 1 payload to JSON.
    ///
    /// # Errors
    ///
    /// Returns an error when the schema is not a ROS 1 message definition or
    /// the payload cannot be decoded according to that definition.
    pub fn decode_json(&mut self, schema: &Schema, data: &[u8]) -> Result<Vec<u8>, Error> {
        if schema.encoding != "ros1msg" {
            return Err(Error::Invalid(format!(
                "expected ros1msg schema, found {}",
                schema.encoding
            )));
        }
        let decoder = match self.decoders.entry(schema.id) {
            std::collections::hash_map::Entry::Occupied(entry) => entry.into_mut(),
            std::collections::hash_map::Entry::Vacant(entry) => {
                let package = schema
                    .name
                    .split_once('/')
                    .map_or(schema.name.as_str(), |(package, _)| package);
                entry.insert(Ros1Decoder::new(package, &schema.data)?)
            }
        };
        decoder.transcode_json(data)
    }
}

fn parse_ros_type(source: &str) -> Result<RosType, Error> {
    if let Some(start) = source.find('[') {
        let end = source
            .strip_suffix(']')
            .ok_or_else(|| Error::Invalid("invalid ros1 array type".into()))?;
        let width = &end[start + 1..];
        return Ok(RosType {
            name: source[..start].to_owned(),
            array: Some(if width.is_empty() {
                None
            } else {
                Some(
                    width
                        .parse()
                        .map_err(|_| Error::Invalid("invalid ros1 fixed array size".into()))?,
                )
            }),
        });
    }
    Ok(RosType {
        name: source.to_owned(),
        array: None,
    })
}
fn write_json_float(value: f64, output: &mut Vec<u8>) -> Result<(), Error> {
    if let Some(value) = json_float_sentinel(value) {
        return serde_json::to_writer(output, value)
            .map_err(|error| Error::Invalid(error.to_string()));
    }
    output.extend_from_slice(value.to_string().as_bytes());
    Ok(())
}

fn json_float_sentinel(value: f64) -> Option<&'static str> {
    if value.is_nan() {
        Some("NaN")
    } else if value == f64::INFINITY {
        Some("Infinity")
    } else if value == f64::NEG_INFINITY {
        Some("-Infinity")
    } else {
        None
    }
}
fn base64(data: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(data)
}

/// ROS 1 bag connection metadata.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RosbagConnection {
    pub id: u32,
    pub topic: String,
    pub type_name: String,
    pub md5sum: String,
    pub message_definition: Vec<u8>,
    pub caller_id: Option<String>,
    pub latching: Option<bool>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RosbagMessage {
    pub connection_id: u32,
    pub time: u64,
    pub data: Vec<u8>,
}

/// Writes a seekable, indexed ROS bag v2.0 using bounded-size uncompressed chunks.
pub struct RosbagWriter<W: Write + Seek> {
    output: W,
    chunk: Vec<u8>,
    connections: Vec<RosbagConnection>,
    chunk_connection_counts: BTreeMap<u32, u32>,
    chunk_indexes: BTreeMap<u32, Vec<(u64, u32)>>,
    chunk_start_time: Option<u64>,
    chunk_end_time: Option<u64>,
    chunks: Vec<RosbagChunkInfo>,
}

#[derive(Debug)]
struct RosbagChunkInfo {
    position: u64,
    start_time: u64,
    end_time: u64,
    counts: BTreeMap<u32, u32>,
}

impl<W: Write + Seek> RosbagWriter<W> {
    pub fn new(mut output: W) -> Result<Self, Error> {
        output.write_all(ROSBAG_MAGIC)?;
        write_bag_header(&mut output, 0, 0, 0)?;
        Ok(Self {
            output,
            chunk: Vec::new(),
            connections: Vec::new(),
            chunk_connection_counts: BTreeMap::new(),
            chunk_indexes: BTreeMap::new(),
            chunk_start_time: None,
            chunk_end_time: None,
            chunks: Vec::new(),
        })
    }

    pub fn connection(&mut self, connection: RosbagConnection) -> Result<(), Error> {
        if self
            .connections
            .iter()
            .any(|existing| existing.id == connection.id)
        {
            return Err(Error::Invalid(format!(
                "duplicate rosbag connection: {}",
                connection.id
            )));
        }
        // Linear readers and recovery need the connection before the messages
        // it describes inside the chunk. `finish` writes the required second
        // copy in the post-chunk index section.
        self.flush_before_record(bag_record_len(
            &bag_header(&[
                ("op", vec![7]),
                ("conn", connection.id.to_le_bytes().to_vec()),
                ("topic", connection.topic.as_bytes().to_vec()),
            ]),
            connection_record_data_len(&connection),
        )?)?;
        write_connection_record(&mut self.chunk, &connection)?;
        self.connections.push(connection);
        Ok(())
    }

    pub fn message(&mut self, message: &RosbagMessage) -> Result<(), Error> {
        if !self
            .connections
            .iter()
            .any(|connection| connection.id == message.connection_id)
        {
            return Err(Error::Invalid(format!(
                "unknown rosbag connection: {}",
                message.connection_id
            )));
        }
        let header = bag_header(&[
            ("op", vec![2]),
            ("conn", message.connection_id.to_le_bytes().to_vec()),
            ("time", ros_time_bytes(message.time)?.to_vec()),
        ]);
        self.flush_before_record(bag_record_len(&header, message.data.len())?)?;
        let offset = u32::try_from(self.chunk.len())
            .map_err(|_| Error::Invalid("rosbag chunk exceeds u32 length".into()))?;
        write_bag_record(&mut self.chunk, &header, &message.data)?;
        *self
            .chunk_connection_counts
            .entry(message.connection_id)
            .or_default() += 1;
        self.chunk_indexes
            .entry(message.connection_id)
            .or_default()
            .push((message.time, offset));
        self.chunk_start_time = Some(
            self.chunk_start_time
                .map_or(message.time, |time| time.min(message.time)),
        );
        self.chunk_end_time = Some(
            self.chunk_end_time
                .map_or(message.time, |time| time.max(message.time)),
        );
        Ok(())
    }

    pub fn finish(&mut self) -> Result<(), Error> {
        self.flush_chunk()?;
        let index_pos = self.output.stream_position()?;
        for connection in &self.connections {
            write_connection_record(&mut self.output, connection)?;
        }
        for chunk in &self.chunks {
            write_chunk_info(
                &mut self.output,
                chunk.position,
                chunk.start_time,
                chunk.end_time,
                &chunk.counts,
            )?;
        }
        self.output
            .seek(SeekFrom::Start(u64::try_from(ROSBAG_MAGIC.len()).map_err(
                |_| Error::Invalid("rosbag magic length exceeds u64".into()),
            )?))?;
        write_bag_header(
            &mut self.output,
            index_pos,
            u32::try_from(self.connections.len())
                .map_err(|_| Error::Invalid("too many rosbag connections".into()))?,
            u32::try_from(self.chunks.len())
                .map_err(|_| Error::Invalid("too many rosbag chunks".into()))?,
        )?;
        Ok(())
    }

    pub fn finish_into(mut self) -> Result<W, Error> {
        self.finish()?;
        Ok(self.output)
    }

    fn flush_before_record(&mut self, record_len: usize) -> Result<(), Error> {
        if !self.chunk.is_empty()
            && self
                .chunk
                .len()
                .checked_add(record_len)
                .is_none_or(|size| size > ROSBAG_CHUNK_TARGET_SIZE)
        {
            self.flush_chunk()?;
        }
        Ok(())
    }

    fn flush_chunk(&mut self) -> Result<(), Error> {
        if self.chunk.is_empty() {
            return Ok(());
        }
        let chunk_pos = self.output.stream_position()?;
        let chunk = std::mem::take(&mut self.chunk);
        let chunk_size = u32::try_from(chunk.len())
            .map_err(|_| Error::Invalid("rosbag chunk exceeds u32 length".into()))?;
        let header = bag_header(&[
            ("op", vec![5]),
            ("compression", b"none".to_vec()),
            ("size", chunk_size.to_le_bytes().to_vec()),
        ]);
        write_bag_record(&mut self.output, &header, &chunk)?;
        for (connection_id, entries) in &self.chunk_indexes {
            write_index_data(&mut self.output, *connection_id, entries)?;
        }
        self.chunks.push(RosbagChunkInfo {
            position: chunk_pos,
            start_time: self.chunk_start_time.unwrap_or(0),
            end_time: self.chunk_end_time.unwrap_or(0),
            counts: std::mem::take(&mut self.chunk_connection_counts),
        });
        self.chunk_indexes.clear();
        self.chunk_start_time = None;
        self.chunk_end_time = None;
        Ok(())
    }
}

fn bag_record_len(header: &[u8], data_len: usize) -> Result<usize, Error> {
    header
        .len()
        .checked_add(data_len)
        .and_then(|length| length.checked_add(8))
        .ok_or_else(|| Error::Invalid("rosbag record length overflows usize".into()))
}

fn connection_record_data_len(connection: &RosbagConnection) -> usize {
    let mut length = bag_field_len("topic", connection.topic.len())
        + bag_field_len("type", connection.type_name.len())
        + bag_field_len("md5sum", connection.md5sum.len())
        + bag_field_len("message_definition", connection.message_definition.len());
    if let Some(caller_id) = &connection.caller_id {
        length += bag_field_len("callerid", caller_id.len());
    }
    if connection.latching.is_some() {
        length += bag_field_len("latching", 1);
    }
    length
}

const fn bag_field_len(key: &str, value_len: usize) -> usize {
    // Field framing is: u32(field length), key, '=', value.
    4 + key.len() + 1 + value_len
}

fn bag_header(fields: &[(&str, Vec<u8>)]) -> Vec<u8> {
    let mut header = Vec::new();
    for (key, value) in fields {
        let mut field = Vec::with_capacity(key.len() + 1 + value.len());
        field.extend_from_slice(key.as_bytes());
        field.push(b'=');
        field.extend_from_slice(value);
        header.extend_from_slice(
            &u32::try_from(field.len())
                .expect("bag header field length fits u32")
                .to_le_bytes(),
        );
        header.extend_from_slice(&field);
    }
    header
}
fn write_bag_record(writer: &mut impl Write, header: &[u8], data: &[u8]) -> Result<(), Error> {
    writer.write_all(
        &u32::try_from(header.len())
            .map_err(|_| Error::Invalid("rosbag header exceeds u32 length".into()))?
            .to_le_bytes(),
    )?;
    writer.write_all(header)?;
    writer.write_all(
        &u32::try_from(data.len())
            .map_err(|_| Error::Invalid("rosbag data exceeds u32 length".into()))?
            .to_le_bytes(),
    )?;
    writer.write_all(data)?;
    Ok(())
}
fn write_bag_header(
    writer: &mut impl Write,
    index_pos: u64,
    connection_count: u32,
    chunk_count: u32,
) -> Result<(), Error> {
    let header = bag_header(&[
        ("op", vec![3]),
        ("index_pos", index_pos.to_le_bytes().to_vec()),
        ("conn_count", connection_count.to_le_bytes().to_vec()),
        ("chunk_count", chunk_count.to_le_bytes().to_vec()),
    ]);
    let padding = 4096usize
        .checked_sub(header.len())
        .ok_or_else(|| Error::Invalid("rosbag header exceeds reserved size".into()))?;
    write_bag_record(writer, &header, &vec![b' '; padding])
}
fn write_connection_record(
    writer: &mut impl Write,
    connection: &RosbagConnection,
) -> Result<(), Error> {
    let header = bag_header(&[
        ("op", vec![7]),
        ("conn", connection.id.to_le_bytes().to_vec()),
        ("topic", connection.topic.as_bytes().to_vec()),
    ]);
    let mut fields = vec![
        ("topic", connection.topic.as_bytes().to_vec()),
        ("type", connection.type_name.as_bytes().to_vec()),
        ("md5sum", connection.md5sum.as_bytes().to_vec()),
        ("message_definition", connection.message_definition.clone()),
    ];
    if let Some(caller_id) = &connection.caller_id {
        fields.push(("callerid", caller_id.as_bytes().to_vec()));
    }
    if let Some(latching) = connection.latching {
        fields.push(("latching", if latching { b"1" } else { b"0" }.to_vec()));
    }
    write_bag_record(writer, &header, &bag_header(&fields))
}
fn write_chunk_info(
    writer: &mut impl Write,
    chunk_pos: u64,
    start_time: u64,
    end_time: u64,
    counts: &BTreeMap<u32, u32>,
) -> Result<(), Error> {
    let header = bag_header(&[
        ("op", vec![6]),
        ("ver", 1_u32.to_le_bytes().to_vec()),
        ("chunk_pos", chunk_pos.to_le_bytes().to_vec()),
        ("start_time", ros_time_bytes(start_time)?.to_vec()),
        ("end_time", ros_time_bytes(end_time)?.to_vec()),
        (
            "count",
            u32::try_from(counts.len())
                .map_err(|_| Error::Invalid("too many rosbag connection counts".into()))?
                .to_le_bytes()
                .to_vec(),
        ),
    ]);
    let mut data = Vec::with_capacity(counts.len() * 8);
    for (connection, count) in counts {
        data.extend_from_slice(&connection.to_le_bytes());
        data.extend_from_slice(&count.to_le_bytes());
    }
    write_bag_record(writer, &header, &data)
}

fn write_index_data(
    writer: &mut impl Write,
    connection_id: u32,
    entries: &[(u64, u32)],
) -> Result<(), Error> {
    let header = bag_header(&[
        ("op", vec![4]),
        ("ver", 1_u32.to_le_bytes().to_vec()),
        ("conn", connection_id.to_le_bytes().to_vec()),
        (
            "count",
            u32::try_from(entries.len())
                .map_err(|_| Error::Invalid("too many rosbag index entries".into()))?
                .to_le_bytes()
                .to_vec(),
        ),
    ]);
    let mut data = Vec::with_capacity(entries.len() * 12);
    for (time, offset) in entries {
        data.extend_from_slice(&ros_time_bytes(*time)?);
        data.extend_from_slice(&offset.to_le_bytes());
    }
    write_bag_record(writer, &header, &data)
}

fn ros_time_bytes(time: u64) -> Result<[u8; 8], Error> {
    let seconds = u32::try_from(time / 1_000_000_000)
        .map_err(|_| Error::Invalid("rosbag timestamp exceeds u32 seconds".into()))?;
    let nanoseconds = u32::try_from(time % 1_000_000_000).expect("nanoseconds remainder fits u32");
    let mut output = [0; 8];
    output[..4].copy_from_slice(&seconds.to_le_bytes());
    output[4..].copy_from_slice(&nanoseconds.to_le_bytes());
    Ok(output)
}

// ROS bag v2.0 uses the same length-prefixed field framing both at top level and inside chunks.
fn validate_rosbag<R: Read>(reader: &mut R) -> Result<(), Error> {
    let mut magic = [0_u8; ROSBAG_MAGIC.len()];
    reader.read_exact(&mut magic)?;
    if magic != ROSBAG_MAGIC {
        return Err(Error::InvalidMagic);
    }
    validate_bag_records(reader)
}

/// Emit all complete connection and message records from a ROS bag. An indexed
/// bag is complete only after the counts declared in its bag header are matched
/// by complete chunks, post-chunk connections, and chunk-info records. This
/// prevents a record-boundary truncation from being mistaken for a finished
/// indexed download.
pub fn read_rosbag_recover<R: Read, S: RosbagSink>(
    reader: &mut R,
    sink: &mut S,
) -> Result<bool, Error> {
    let mut magic = [0_u8; ROSBAG_MAGIC.len()];
    reader.read_exact(&mut magic)?;
    if magic != ROSBAG_MAGIC {
        return Err(Error::InvalidMagic);
    }
    let Some((header, _)) = read_bag_record(reader)? else {
        return Ok(false);
    };
    if header.get("op").and_then(|value| value.first()) != Some(&0x03) {
        return Err(Error::Invalid("rosbag stream is missing bag header".into()));
    }
    let indexed = header_u64(&header, "index_pos")? != 0;
    let declared_connections = header_u32(&header, "conn_count")?;
    let declared_chunks = header_u32(&header, "chunk_count")?;
    read_rosbag_records_recover(
        reader,
        sink,
        RosbagRecoveryState::new(indexed, declared_connections, declared_chunks),
    )
}

#[derive(Debug)]
struct RosbagRecoveryState {
    indexed: bool,
    declared_connections: u32,
    declared_chunks: u32,
    chunks: u32,
    post_chunk_connections: u32,
    chunk_infos: u32,
    pending_indexes: BTreeSet<u32>,
    in_post_chunk_index: bool,
}

impl RosbagRecoveryState {
    const fn new(indexed: bool, declared_connections: u32, declared_chunks: u32) -> Self {
        Self {
            indexed,
            declared_connections,
            declared_chunks,
            chunks: 0,
            post_chunk_connections: 0,
            chunk_infos: 0,
            pending_indexes: BTreeSet::new(),
            in_post_chunk_index: false,
        }
    }

    fn complete(&self) -> bool {
        self.indexed
            && self.chunks == self.declared_chunks
            && self.post_chunk_connections == self.declared_connections
            && self.chunk_infos == self.declared_chunks
            && self.pending_indexes.is_empty()
    }

    fn before_non_index_record(&mut self) -> bool {
        self.pending_indexes.is_empty()
    }
}

fn read_rosbag_records_recover<R: Read, S: RosbagSink>(
    reader: &mut R,
    sink: &mut S,
    mut state: RosbagRecoveryState,
) -> Result<bool, Error> {
    loop {
        let (header, data) = match read_bag_record(reader) {
            Ok(Some(record)) => record,
            Ok(None) => return Ok(state.complete()),
            Err(Error::Io(error)) if error.kind() == io::ErrorKind::UnexpectedEof => {
                return Ok(false)
            }
            Err(error) => return Err(error),
        };
        match header.get("op").and_then(|value| value.first()).copied() {
            Some(0x05) => {
                if !state.before_non_index_record() || state.in_post_chunk_index {
                    return Ok(false);
                }
                let compression = header
                    .get("compression")
                    .and_then(|value| std::str::from_utf8(value).ok())
                    .unwrap_or("none");
                let size = header_u32(&header, "size")? as usize;
                if size > MAX_RECORD_LEN {
                    return Err(Error::Invalid(
                        "rosbag chunk exceeds configured limit".into(),
                    ));
                }
                let mut indexed_connections = BTreeSet::new();
                let complete = match compression {
                    "none" => read_rosbag_chunk_recover(
                        &mut Cursor::new(data),
                        sink,
                        &mut indexed_connections,
                    )?,
                    "lz4" => {
                        let mut decompressed = Vec::with_capacity(size);
                        let limit = u64::try_from(size + 1).expect("configured size fits u64");
                        match FrameDecoder::new(Cursor::new(data))
                            .take(limit)
                            .read_to_end(&mut decompressed)
                        {
                            Ok(_) if decompressed.len() == size => read_rosbag_chunk_recover(
                                &mut Cursor::new(decompressed),
                                sink,
                                &mut indexed_connections,
                            )?,
                            Ok(_) | Err(_) => false,
                        }
                    }
                    other => return Err(Error::UnsupportedCompression(other.to_owned())),
                };
                if !complete {
                    return Ok(false);
                }
                state.chunks = state.chunks.saturating_add(1);
                if state.indexed {
                    state.pending_indexes = indexed_connections;
                }
            }
            Some(0x04) => {
                if !state.indexed || state.in_post_chunk_index {
                    return Ok(false);
                }
                let connection_id = header_u32(&header, "conn")?;
                if !state.pending_indexes.remove(&connection_id) {
                    return Ok(false);
                }
            }
            Some(0x07) => {
                if !state.before_non_index_record() {
                    return Ok(false);
                }
                state.in_post_chunk_index = true;
                state.post_chunk_connections = state.post_chunk_connections.saturating_add(1);
                sink.connection(parse_rosbag_connection(&header, &data)?)?;
            }
            Some(0x06) => {
                if !state.before_non_index_record() {
                    return Ok(false);
                }
                state.in_post_chunk_index = true;
                state.chunk_infos = state.chunk_infos.saturating_add(1);
            }
            Some(0x02) => sink.message(parse_rosbag_message(&header, data)?)?,
            Some(_) => {}
            None => return Err(Error::Invalid("rosbag record is missing op field".into())),
        }
    }
}

fn read_rosbag_chunk_recover<R: Read, S: RosbagSink>(
    reader: &mut R,
    sink: &mut S,
    indexed_connections: &mut BTreeSet<u32>,
) -> Result<bool, Error> {
    loop {
        let (header, data) = match read_bag_record(reader) {
            Ok(Some(record)) => record,
            Ok(None) => return Ok(true),
            Err(Error::Io(error)) if error.kind() == io::ErrorKind::UnexpectedEof => {
                return Ok(false)
            }
            Err(error) => return Err(error),
        };
        match header.get("op").and_then(|value| value.first()).copied() {
            Some(0x07) => sink.connection(parse_rosbag_connection(&header, &data)?)?,
            Some(0x02) => {
                let message = parse_rosbag_message(&header, data)?;
                indexed_connections.insert(message.connection_id);
                sink.message(message)?;
            }
            Some(_) => {}
            None => return Err(Error::Invalid("rosbag record is missing op field".into())),
        }
    }
}

fn parse_rosbag_connection(
    header: &BTreeMap<String, Vec<u8>>,
    data: &[u8],
) -> Result<RosbagConnection, Error> {
    let metadata = parse_bag_fields(data)?;
    Ok(RosbagConnection {
        id: header_u32(header, "conn")?,
        topic: header_string(header, "topic")?,
        type_name: field_string(&metadata, "type")?,
        md5sum: field_string(&metadata, "md5sum")?,
        message_definition: metadata
            .get("message_definition")
            .cloned()
            .unwrap_or_default(),
        caller_id: metadata
            .get("callerid")
            .map(|value| std::str::from_utf8(value).map(str::to_owned))
            .transpose()
            .map_err(|_| Error::Invalid("invalid rosbag callerid".into()))?,
        latching: metadata.get("latching").map(|value| value == b"1"),
    })
}

fn parse_rosbag_message(
    header: &BTreeMap<String, Vec<u8>>,
    data: Vec<u8>,
) -> Result<RosbagMessage, Error> {
    let time = header
        .get("time")
        .ok_or_else(|| Error::Invalid("rosbag record is missing time field".into()))?;
    if time.len() != 8 {
        return Err(Error::Invalid("rosbag time field has invalid size".into()));
    }
    let seconds = u32::from_le_bytes(time[..4].try_into().expect("checked length"));
    let nanoseconds = u32::from_le_bytes(time[4..].try_into().expect("checked length"));
    if nanoseconds >= 1_000_000_000 {
        return Err(Error::Invalid(
            "rosbag time nanoseconds are out of range".into(),
        ));
    }
    Ok(RosbagMessage {
        connection_id: header_u32(header, "conn")?,
        time: u64::from(seconds) * 1_000_000_000 + u64::from(nanoseconds),
        data,
    })
}

fn parse_bag_fields(data: &[u8]) -> Result<BTreeMap<String, Vec<u8>>, Error> {
    let mut cursor = Cursor::new(data);
    let mut fields = BTreeMap::new();
    while (cursor.position() as usize) < data.len() {
        let len = checked_len(read_u32(&mut cursor)?)?;
        let mut field = vec![0; len];
        cursor.read_exact(&mut field)?;
        let Some(separator) = field.iter().position(|byte| *byte == b'=') else {
            return Err(Error::Invalid("invalid rosbag header field".into()));
        };
        let key = std::str::from_utf8(&field[..separator])
            .map_err(|_| Error::Invalid("invalid rosbag header key".into()))?
            .to_owned();
        if fields
            .insert(key, field[separator + 1..].to_vec())
            .is_some()
        {
            return Err(Error::Invalid("duplicate rosbag header field".into()));
        }
    }
    Ok(fields)
}

fn header_string(header: &BTreeMap<String, Vec<u8>>, name: &str) -> Result<String, Error> {
    field_string(header, name)
}

fn field_string(fields: &BTreeMap<String, Vec<u8>>, name: &str) -> Result<String, Error> {
    let value = fields
        .get(name)
        .ok_or_else(|| Error::Invalid(format!("rosbag record is missing {name} field")))?;
    String::from_utf8(value.clone())
        .map_err(|_| Error::Invalid(format!("invalid rosbag {name} field")))
}

fn validate_bag_records<R: Read>(reader: &mut R) -> Result<(), Error> {
    loop {
        let Some((header, data)) = read_bag_record(reader)? else {
            return Ok(());
        };
        match header.get("op").and_then(|value| value.first()).copied() {
            Some(0x05) => {
                // Chunk
                let compression = header
                    .get("compression")
                    .and_then(|value| std::str::from_utf8(value).ok())
                    .unwrap_or("none");
                let size = header_u32(&header, "size")? as usize;
                if size > MAX_RECORD_LEN {
                    return Err(Error::Invalid(
                        "rosbag chunk exceeds configured limit".into(),
                    ));
                }
                match compression {
                    "none" => validate_bag_records(&mut Cursor::new(data))?,
                    "lz4" => {
                        let mut decompressed = Vec::with_capacity(size);
                        let limit = u64::try_from(size + 1).expect("configured size fits u64");
                        FrameDecoder::new(Cursor::new(data))
                            .take(limit)
                            .read_to_end(&mut decompressed)
                            .map_err(|error| {
                                Error::Invalid(format!(
                                    "failed to decompress lz4 rosbag chunk: {error}"
                                ))
                            })?;
                        if decompressed.len() != size {
                            return Err(Error::Invalid(
                                "lz4 rosbag chunk has unexpected decompressed size".into(),
                            ));
                        }
                        validate_bag_records(&mut Cursor::new(decompressed))?;
                    }
                    other => return Err(Error::UnsupportedCompression(other.to_owned())),
                }
            }
            Some(_) => {}
            None => return Err(Error::Invalid("rosbag record is missing op field".into())),
        }
    }
}

fn read_bag_record<R: Read>(
    reader: &mut R,
) -> Result<Option<(BTreeMap<String, Vec<u8>>, Vec<u8>)>, Error> {
    let Some(header_len) = read_u32_optional(reader)? else {
        return Ok(None);
    };
    let header_len = checked_len(header_len)?;
    let mut header_bytes = vec![0; header_len];
    reader.read_exact(&mut header_bytes)?;
    let data_len = checked_len(read_u32(reader)?)?;
    let mut data = vec![0; data_len];
    reader.read_exact(&mut data)?;
    let mut header = BTreeMap::new();
    let mut cursor = Cursor::new(header_bytes);
    while (cursor.position() as usize) < cursor.get_ref().len() {
        let len = checked_len(read_u32(&mut cursor)?)?;
        let mut field = vec![0; len];
        cursor.read_exact(&mut field)?;
        let Some(separator) = field.iter().position(|byte| *byte == b'=') else {
            return Err(Error::Invalid("invalid rosbag header field".into()));
        };
        let (key, value) = (&field[..separator], &field[separator + 1..]);
        let key = std::str::from_utf8(key)
            .map_err(|_| Error::Invalid("invalid rosbag header key".into()))?
            .to_owned();
        if header.insert(key, value.to_vec()).is_some() {
            return Err(Error::Invalid("duplicate rosbag header field".into()));
        }
    }
    Ok(Some((header, data)))
}

fn header_u32(header: &BTreeMap<String, Vec<u8>>, name: &str) -> Result<u32, Error> {
    let value = header
        .get(name)
        .ok_or_else(|| Error::Invalid(format!("rosbag record is missing {name} field")))?;
    if value.len() != 4 {
        return Err(Error::Invalid(format!(
            "rosbag {name} field has invalid size"
        )));
    }
    Ok(u32::from_le_bytes(
        value.clone().try_into().expect("checked length"),
    ))
}
fn header_u64(header: &BTreeMap<String, Vec<u8>>, name: &str) -> Result<u64, Error> {
    let value = header
        .get(name)
        .ok_or_else(|| Error::Invalid(format!("rosbag record is missing {name} field")))?;
    if value.len() != 8 {
        return Err(Error::Invalid(format!(
            "rosbag {name} field has invalid size"
        )));
    }
    Ok(u64::from_le_bytes(
        value.clone().try_into().expect("checked length"),
    ))
}
fn checked_len(length: u32) -> Result<usize, Error> {
    let length = length as usize;
    if length > MAX_RECORD_LEN {
        Err(Error::Invalid(
            "rosbag record exceeds configured limit".into(),
        ))
    } else {
        Ok(length)
    }
}
fn read_u32<R: Read>(reader: &mut R) -> Result<u32, Error> {
    let mut bytes = [0; 4];
    reader.read_exact(&mut bytes)?;
    Ok(u32::from_le_bytes(bytes))
}
fn read_u32_optional<R: Read>(reader: &mut R) -> Result<Option<u32>, Error> {
    let mut bytes = [0; 4];
    match reader.read(&mut bytes)? {
        0 => Ok(None),
        4 => Ok(Some(u32::from_le_bytes(bytes))),
        read => {
            reader.read_exact(&mut bytes[read..])?;
            Ok(Some(u32::from_le_bytes(bytes)))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[derive(Default)]
    struct CollectSink {
        schemas: Vec<Schema>,
        channels: Vec<Channel>,
        messages: Vec<Message>,
        metadata: Vec<(String, BTreeMap<String, String>)>,
        attachments: Vec<Attachment>,
    }

    impl RecordSink for CollectSink {
        fn schema(&mut self, schema: Schema) -> Result<(), Error> {
            self.schemas.push(schema);
            Ok(())
        }

        fn channel(&mut self, channel: Channel) -> Result<(), Error> {
            self.channels.push(channel);
            Ok(())
        }

        fn message(&mut self, message: Message) -> Result<(), Error> {
            self.messages.push(message);
            Ok(())
        }

        fn metadata(
            &mut self,
            name: String,
            metadata: BTreeMap<String, String>,
        ) -> Result<(), Error> {
            self.metadata.push((name, metadata));
            Ok(())
        }

        fn attachment(&mut self, attachment: Attachment) -> Result<(), Error> {
            self.attachments.push(attachment);
            Ok(())
        }
    }

    #[derive(Default)]
    struct CollectBagSink {
        connections: Vec<RosbagConnection>,
        messages: Vec<RosbagMessage>,
    }

    impl RosbagSink for CollectBagSink {
        fn connection(&mut self, connection: RosbagConnection) -> Result<(), Error> {
            self.connections.push(connection);
            Ok(())
        }

        fn message(&mut self, message: RosbagMessage) -> Result<(), Error> {
            self.messages.push(message);
            Ok(())
        }
    }

    #[test]
    fn rejects_truncated_mcap() {
        let mut bytes = Cursor::new([MCAP_MAGIC, b"not complete"].concat());
        let outcome = validate_import(&mut bytes);
        assert!(matches!(outcome, Err(Error::TruncatedMcap)), "{outcome:?}");
    }

    #[test]
    fn recovers_complete_records_before_a_truncated_mcap_tail() {
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
                sequence: 0,
                log_time: 10,
                publish_time: 10,
                data: vec![1],
            })
            .expect("message");
        let mut bytes = writer.finish_into().expect("finish").into_inner();
        bytes.truncate(bytes.len() - 4);
        let mut sink = CollectSink::default();
        assert!(!read_mcap_recover(&mut Cursor::new(bytes), &mut sink).expect("recover"));
        assert_eq!(sink.messages.len(), 1);
    }

    #[test]
    fn recovers_rosbag_messages_with_canonical_timestamps() {
        let mut writer = RosbagWriter::new(Cursor::new(Vec::new())).expect("writer");
        writer
            .connection(RosbagConnection {
                id: 7,
                topic: "/example".into(),
                type_name: "example/Message".into(),
                md5sum: "abc".into(),
                message_definition: b"uint8 value\n".to_vec(),
                caller_id: None,
                latching: None,
            })
            .expect("connection");
        writer
            .message(&RosbagMessage {
                connection_id: 7,
                time: 1_700_000_000_123_456_789,
                data: vec![3],
            })
            .expect("message");
        let bytes = writer.finish_into().expect("finish").into_inner();
        let mut sink = CollectBagSink::default();
        assert!(read_rosbag_recover(&mut Cursor::new(bytes), &mut sink).expect("recover"));
        assert_eq!(sink.connections.len(), 2, "chunk and index connections");
        assert_eq!(sink.messages[0].time, 1_700_000_000_123_456_789);
    }

    #[test]
    fn reports_truncated_indexed_rosbag_as_incomplete() {
        let mut writer = RosbagWriter::new(Cursor::new(Vec::new())).expect("writer");
        writer
            .connection(RosbagConnection {
                id: 1,
                topic: "/example".into(),
                type_name: "example/Message".into(),
                md5sum: "abc".into(),
                message_definition: vec![],
                caller_id: None,
                latching: None,
            })
            .expect("connection");
        writer
            .message(&RosbagMessage {
                connection_id: 1,
                time: 1,
                data: vec![1],
            })
            .expect("message");
        let mut bytes = writer.finish_into().expect("finish").into_inner();
        bytes.pop();
        let mut sink = CollectBagSink::default();
        assert!(!read_rosbag_recover(&mut Cursor::new(bytes), &mut sink).expect("recover"));
    }

    #[test]
    fn treats_unindexed_rosbags_as_recoverable_but_incomplete() {
        let mut writer = RosbagWriter::new(Cursor::new(Vec::new())).expect("writer");
        writer.connection(rosbag_connection()).expect("connection");
        writer
            .message(&RosbagMessage {
                connection_id: 1,
                time: 1,
                data: vec![1],
            })
            .expect("message");
        let bytes = writer.finish_into().expect("finish").into_inner();
        let mut cursor = Cursor::new(bytes);
        cursor
            .seek(SeekFrom::Start(ROSBAG_MAGIC.len() as u64))
            .expect("seek to header");
        write_bag_header(&mut cursor, 0, 1, 1).expect("rewrite unindexed header");
        let mut sink = CollectBagSink::default();
        assert!(
            !read_rosbag_recover(&mut Cursor::new(cursor.into_inner()), &mut sink)
                .expect("recover")
        );
        assert_eq!(sink.messages.len(), 1);
    }

    fn rosbag_connection() -> RosbagConnection {
        RosbagConnection {
            id: 1,
            topic: "/example".into(),
            type_name: "example/Message".into(),
            md5sum: "abc".into(),
            message_definition: vec![],
            caller_id: None,
            latching: None,
        }
    }

    fn rosbag_record_boundaries(bytes: &[u8]) -> Vec<(u8, usize)> {
        let mut cursor = Cursor::new(bytes);
        cursor
            .seek(SeekFrom::Start(ROSBAG_MAGIC.len() as u64))
            .expect("seek past magic");
        let _ = read_bag_record(&mut cursor).expect("bag header");
        let mut records = Vec::new();
        while (cursor.position() as usize) < bytes.len() {
            let (header, _) = read_bag_record(&mut cursor)
                .expect("record")
                .expect("record before EOF");
            records.push((
                *header
                    .get("op")
                    .and_then(|op| op.first())
                    .expect("operation"),
                cursor.position() as usize,
            ));
        }
        records
    }

    #[test]
    fn writes_large_rosbags_as_multiple_indexed_chunks() {
        let mut writer = RosbagWriter::new(Cursor::new(Vec::new())).expect("writer");
        writer.connection(rosbag_connection()).expect("connection");
        for time in 0..3 {
            writer
                .message(&RosbagMessage {
                    connection_id: 1,
                    time,
                    data: vec![u8::try_from(time).expect("small timestamp"); 400 * 1024],
                })
                .expect("message");
        }
        let bytes = writer.finish_into().expect("finish").into_inner();
        let records = rosbag_record_boundaries(&bytes);
        let mut cursor = Cursor::new(&bytes);
        cursor
            .seek(SeekFrom::Start(ROSBAG_MAGIC.len() as u64))
            .expect("seek past magic");
        let (header, _) = read_bag_record(&mut cursor)
            .expect("bag header")
            .expect("bag header before EOF");
        assert_eq!(header_u32(&header, "chunk_count").expect("chunk count"), 3);
        assert_eq!(
            header_u32(&header, "conn_count").expect("connection count"),
            1
        );
        assert_eq!(
            records.iter().filter(|(op, _)| *op == 0x05).count(),
            3,
            "each 400 KiB message should flush a bounded chunk"
        );
        assert_eq!(records.iter().filter(|(op, _)| *op == 0x04).count(), 3);
        assert_eq!(records.iter().filter(|(op, _)| *op == 0x06).count(), 3);

        let mut sink = CollectBagSink::default();
        assert!(read_rosbag_recover(&mut Cursor::new(bytes), &mut sink).expect("recover"));
        assert_eq!(sink.messages.len(), 3);
    }

    #[test]
    fn detects_exact_boundary_rosbag_truncation_before_indexes_and_chunk_infos() {
        let mut writer = RosbagWriter::new(Cursor::new(Vec::new())).expect("writer");
        writer.connection(rosbag_connection()).expect("connection");
        for time in 0..3 {
            writer
                .message(&RosbagMessage {
                    connection_id: 1,
                    time,
                    data: vec![0; 400 * 1024],
                })
                .expect("message");
        }
        let bytes = writer.finish_into().expect("finish").into_inner();
        let records = rosbag_record_boundaries(&bytes);

        let first_chunk_end = records
            .iter()
            .find_map(|(op, end)| (*op == 0x05).then_some(*end))
            .expect("chunk record");
        let mut sink = CollectBagSink::default();
        assert!(
            !read_rosbag_recover(&mut Cursor::new(&bytes[..first_chunk_end]), &mut sink)
                .expect("recover")
        );
        assert_eq!(sink.messages.len(), 1);

        let first_chunk_info_end = records
            .iter()
            .find_map(|(op, end)| (*op == 0x06).then_some(*end))
            .expect("chunk info record");
        let mut sink = CollectBagSink::default();
        assert!(
            !read_rosbag_recover(&mut Cursor::new(&bytes[..first_chunk_info_end]), &mut sink)
                .expect("recover")
        );
        assert_eq!(sink.messages.len(), 3);
    }

    #[test]
    fn rejects_unknown_bag_compression() {
        let mut bytes = ROSBAG_MAGIC.to_vec();
        let header = [
            field(b"op", &[5]),
            field(b"compression", b"bz2"),
            field(b"size", &0_u32.to_le_bytes()),
        ]
        .concat();
        bytes.extend_from_slice(&(header.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&header);
        bytes.extend_from_slice(&0_u32.to_le_bytes());
        assert!(
            matches!(validate_import(&mut Cursor::new(bytes)), Err(Error::UnsupportedCompression(value)) if value == "bz2")
        );
    }

    #[test]
    fn rejects_partial_rosbag_record_length() {
        let mut bytes = ROSBAG_MAGIC.to_vec();
        bytes.extend_from_slice(&[1, 0]);
        assert!(validate_import(&mut Cursor::new(bytes)).is_err());
    }

    #[test]
    fn transcodes_nested_ros1_messages_and_byte_arrays() {
        let decoder = Ros1Decoder::new("example", b"uint32 count\nuint8[] payload\nChild child\n================================================================================\nMSG: example/Child\ntime stamp\nstring label\n").expect("valid schema");
        let mut bytes = 2_u32.to_le_bytes().to_vec();
        bytes.extend_from_slice(&3_u32.to_le_bytes());
        bytes.extend_from_slice(&[1, 2, 3]);
        bytes.extend_from_slice(&5_u32.to_le_bytes());
        bytes.extend_from_slice(&7_u32.to_le_bytes());
        bytes.extend_from_slice(&2_u32.to_le_bytes());
        bytes.extend_from_slice(b"ok");
        assert_eq!(
            decoder.transcode_json(&bytes).expect("valid payload"),
            br#"{"count":2,"payload":"AQID","child":{"stamp":5.000000007,"label":"ok"}}"#
        );
    }

    #[test]
    fn decodes_signed_ros1_integers() {
        let decoder =
            Ros1Decoder::new("example", b"int8 a\nint16 b\nint32 c\n").expect("valid schema");
        assert_eq!(
            decoder
                .transcode_json(&[0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff])
                .expect("valid payload"),
            br#"{"a":-1,"b":-1,"c":-1}"#
        );
    }

    #[test]
    fn decodes_signed_ros1_duration() {
        let decoder = Ros1Decoder::new("example", b"duration value\n").expect("valid schema");
        let mut bytes = (-1_i32).to_le_bytes().to_vec();
        bytes.extend_from_slice(&500_000_000_i32.to_le_bytes());
        assert_eq!(
            decoder.transcode_json(&bytes).expect("valid payload"),
            br#"{"value":-0.500000000}"#
        );
    }

    #[test]
    fn writes_mcap_records_and_attachments() {
        let output = Cursor::new(Vec::new());
        let mut writer = McapWriter::new(output).expect("writer");
        writer
            .schema(&Schema {
                id: 1,
                name: "example/Msg".into(),
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
                sequence: 0,
                log_time: 1,
                publish_time: 1,
                data: vec![7],
            })
            .expect("message");
        writer
            .metadata(
                "capture".into(),
                BTreeMap::from([("source".into(), "conformance".into())]),
            )
            .expect("metadata");
        writer
            .attachment(&Attachment {
                log_time: 1,
                create_time: 1,
                name: "note.txt".into(),
                media_type: "text/plain".into(),
                data: b"note".to_vec(),
            })
            .expect("attachment");
        let bytes = writer.finish_into().expect("finish").into_inner();
        validate_import(&mut Cursor::new(&bytes)).expect("valid mcap");

        let mut actual = CollectSink::default();
        read_mcap(&mut Cursor::new(bytes), &mut actual).expect("read mcap");
        assert!(actual.schemas.iter().any(|schema| schema.id == 1));
        assert!(actual.channels.iter().any(|channel| channel.id == 1));
        assert_eq!(actual.messages.len(), 1);
        assert_eq!(actual.messages[0].data, vec![7]);
        assert_eq!(actual.metadata.len(), 1);
        assert_eq!(actual.metadata[0].0, "capture");
        assert_eq!(actual.attachments.len(), 1);
        assert_eq!(actual.attachments[0].name, "note.txt");
    }

    #[test]
    fn ros1_decoder_cache_is_keyed_by_schema_id() {
        let mut cache = Ros1DecoderCache::default();
        let number = Schema {
            id: 1,
            name: "example/Message".into(),
            encoding: "ros1msg".into(),
            data: b"uint8 value\n".to_vec(),
        };
        let text = Schema {
            id: 2,
            name: "example/Message".into(),
            encoding: "ros1msg".into(),
            data: b"string value\n".to_vec(),
        };
        assert_eq!(cache.decode_json(&number, &[7]).unwrap(), br#"{"value":7}"#);
        assert_eq!(
            cache.decode_json(&text, &[2, 0, 0, 0, b'o', b'k']).unwrap(),
            br#"{"value":"ok"}"#
        );
        assert_eq!(cache.decode_json(&number, &[9]).unwrap(), br#"{"value":9}"#);
    }

    #[test]
    fn validates_committed_mcap_and_rosbag_assets() {
        for path in [
            "../foxglove/testdata/gps.mcap",
            "../foxglove/testdata/gps.bag",
        ] {
            let mut file = std::fs::File::open(path).expect("committed fixture");
            validate_import(&mut file).unwrap_or_else(|error| panic!("{path}: {error}"));
        }
    }

    #[test]
    fn writes_a_valid_indexed_rosbag() {
        let output = Cursor::new(Vec::new());
        let mut writer = RosbagWriter::new(output).expect("writer");
        writer
            .connection(RosbagConnection {
                id: 3,
                topic: "/example".into(),
                type_name: "std_msgs/String".into(),
                md5sum: "992ce8a1687cec8c8bd883ec73ca41d1".into(),
                message_definition: b"string data\n".to_vec(),
                caller_id: None,
                latching: None,
            })
            .expect("connection");
        writer
            .message(&RosbagMessage {
                connection_id: 3,
                time: 4_000_000_005,
                data: b"payload".to_vec(),
            })
            .expect("message");
        let bytes = writer.finish_into().expect("finish").into_inner();
        validate_import(&mut Cursor::new(bytes)).expect("valid bag");
    }

    fn field(key: &[u8], value: &[u8]) -> Vec<u8> {
        let mut field = [key, b"=", value].concat();
        let mut result = (field.len() as u32).to_le_bytes().to_vec();
        result.append(&mut field);
        result
    }
}
