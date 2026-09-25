//! Structured command parsing and offline command execution.

use std::ffi::OsString;
use std::io::{BufRead, Write};
use std::path::PathBuf;

use clap::builder::Resettable;
use clap::error::ErrorKind;
use clap::{Args, Command, CommandFactory, Parser, Subcommand, ValueEnum, ValueHint};
use clap_complete::{generate, Shell};
use serde_yaml_ng::Value;

use crate::config::Config;
use crate::output::Format;
use crate::{
    attachments, auth, data, datasets, devices, episodes, event_types, events, extensions,
    pending_imports, projects, recordings, runtime, sessions, topics,
};

const ROOT_COMMAND: &str = "foxglove";

/// Match Go's strconv.ParseBool, as used by pflag. Explicit values require `=`
/// so a bare boolean flag never consumes the next positional argument.
fn parse_bool(value: &str) -> Result<bool, String> {
    match value {
        "1" | "t" | "T" | "TRUE" | "true" | "True" => Ok(true),
        "0" | "f" | "F" | "FALSE" | "false" | "False" => Ok(false),
        _ => Err(format!("invalid boolean value: {value:?}")),
    }
}

/// The complete command hierarchy. Parsing, help, dispatch metadata, and shell
/// completions are all generated from these types.
#[derive(Debug, Parser)]
#[command(
    name = ROOT_COMMAND,
    about = "Command line client for the Foxglove data platform",
    disable_version_flag = true,
    subcommand_precedence_over_arg = true,
    args_override_self = true
)]
struct Cli {
    #[arg(
        long,
        global = true,
        help = "Foxglove client ID",
        allow_hyphen_values = true
    )]
    client_id: Option<String>,
    #[arg(long, global = true, help = "Config file", value_hint = ValueHint::FilePath)]
    config: Option<PathBuf>,
    #[arg(
        long, global = true, help = "Enable debug logging",
        action = clap::ArgAction::Set,
        num_args = 0..=1,
        require_equals = true,
        default_missing_value = "true",
        default_value = "false",
        value_parser = parse_bool
    )]
    debug: bool,
    #[command(subcommand)]
    command: Option<CliCommand>,
}

#[derive(Debug, Subcommand)]
enum CliCommand {
    #[command(about = "Query and modify data attachments", subcommand)]
    Attachments(AttachmentsCommand),
    #[command(about = "Manage authentication", subcommand)]
    Auth(AuthCommand),
    #[command(about = "Generate a shell completion script", subcommand)]
    Completion(CompletionCommand),
    #[command(about = "Manage CLI configuration values", subcommand)]
    Config(ConfigCommand),
    #[command(about = "Inspect data coverage", subcommand)]
    Coverage(CoverageCommand),
    #[command(about = "Export data by recording, import, session, or device and time range")]
    Export(DataExportArgs),
    #[command(about = "Upload a local data file to Foxglove")]
    Upload(DataImportArgs),
    #[command(about = "List datasets and their episodes", subcommand)]
    Datasets(DatasetsCommand),
    #[command(about = "List and manage devices", subcommand)]
    Devices(DevicesCommand),
    #[command(about = "List episodes", subcommand)]
    Episodes(EpisodesCommand),
    #[command(name = "event-types", about = "List event types", subcommand)]
    EventTypes(EventTypesCommand),
    #[command(about = "List and manage events", subcommand)]
    Events(EventsCommand),
    #[command(about = "List and publish Studio extensions", subcommand)]
    Extensions(ExtensionsCommand),
    #[command(name = "pending-imports", about = "List pending imports", subcommand)]
    PendingImports(PendingImportsCommand),
    #[command(about = "List and manage projects", subcommand)]
    Projects(ProjectsCommand),
    #[command(about = "Query recordings", subcommand)]
    Recordings(RecordingsCommand),
    #[command(about = "List and manage sessions", subcommand)]
    Sessions(SessionsCommand),
    #[command(about = "List topics", subcommand)]
    Topics(TopicsCommand),
    #[command(about = "Print Foxglove CLI version")]
    Version,
}

#[derive(Debug, Subcommand)]
enum AttachmentsCommand {
    #[command(about = "Download an MCAP attachment by ID")]
    Download(AttachmentDownloadArgs),
    #[command(about = "List MCAP attachments")]
    List(AttachmentListArgs),
}

#[derive(Debug, Args)]
pub(crate) struct AttachmentDownloadArgs {
    #[arg(value_name = "ATTACHMENT_ID")]
    pub(crate) attachment_id: String,
}

#[derive(Debug, Args)]
pub(crate) struct AttachmentListArgs {
    #[command(flatten)]
    format: FormatArgs,
    #[arg(long, help = "Import ID", allow_hyphen_values = true)]
    pub(crate) import_id: Option<String>,
    #[arg(long, help = "Project ID", allow_hyphen_values = true)]
    pub(crate) project_id: Option<String>,
    #[arg(long, help = "Recording ID", allow_hyphen_values = true)]
    pub(crate) recording_id: Option<String>,
    #[arg(long, help = "Session ID", allow_hyphen_values = true)]
    pub(crate) session_id: Option<String>,
    #[arg(long, help = "Session key", allow_hyphen_values = true)]
    pub(crate) session_key: Option<String>,
}

#[derive(Debug, Subcommand)]
enum AuthCommand {
    #[command(about = "Configure an API key")]
    ConfigureApiKey(ConfigureApiKeyArgs),
    #[command(about = "Display information about the currently authenticated user")]
    Info,
    #[command(about = "Log in to Foxglove Data Platform")]
    Login(LoginArgs),
}

#[derive(Debug, Args)]
struct ConfigureApiKeyArgs {
    #[arg(
        long,
        help = "API key for non-interactive use",
        allow_hyphen_values = true
    )]
    api_key: Option<String>,
    #[arg(
        long,
        help = "API server (default: https://api.foxglove.dev)",
        allow_hyphen_values = true
    )]
    base_url: Option<String>,
}

#[derive(Debug, Args)]
pub(crate) struct LoginArgs {
    #[arg(
        long,
        help = "API server (default: https://api.foxglove.dev)",
        allow_hyphen_values = true
    )]
    pub(crate) base_url: Option<String>,
}

#[derive(Debug, Subcommand)]
enum CompletionCommand {
    #[command(about = "Generate completions for Bash")]
    Bash(CompletionArgs),
    #[command(about = "Generate completions for Fish")]
    Fish(CompletionArgs),
    #[command(about = "Generate completions for PowerShell")]
    Powershell(CompletionArgs),
    #[command(about = "Generate completions for Zsh")]
    Zsh(CompletionArgs),
}

#[derive(Debug, Args)]
struct CompletionArgs {
    #[arg(
        long, help = "Disable completion descriptions",
        action = clap::ArgAction::Set,
        num_args = 0..=1,
        require_equals = true,
        default_missing_value = "true",
        default_value = "false",
        value_parser = parse_bool
    )]
    no_descriptions: bool,
}

#[derive(Debug, Subcommand)]
enum ConfigCommand {
    #[command(about = "Get a configuration value")]
    Get(ConfigKeyArgs),
    #[command(about = "Set a configuration value")]
    Set(ConfigSetArgs),
    #[command(about = "Remove a configuration value")]
    Unset(ConfigKeyArgs),
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum ConfigKey {
    ProjectId,
}

#[derive(Debug, Args)]
struct ConfigKeyArgs {
    #[arg(value_name = "KEY")]
    key: ConfigKey,
}

#[derive(Debug, Args)]
struct ConfigSetArgs {
    #[arg(value_name = "KEY")]
    key: ConfigKey,
    #[arg(value_name = "VALUE")]
    value: String,
}

#[derive(Debug, Subcommand)]
enum CoverageCommand {
    #[command(about = "List coverage ranges")]
    List(CoverageListArgs),
}

#[derive(Debug, Args)]
pub(crate) struct CoverageListArgs {
    #[command(flatten)]
    format: FormatArgs,
    #[arg(long, help = "Device ID", allow_hyphen_values = true)]
    pub(crate) device_id: Option<String>,
    #[arg(long, help = "Device name", allow_hyphen_values = true)]
    pub(crate) device_name: Option<String>,
    #[arg(
        long,
        help = "End of coverage time range (ISO 8601)",
        allow_hyphen_values = true
    )]
    pub(crate) end: Option<String>,
    #[arg(
        long, help = "Include edge recordings",
        action = clap::ArgAction::Set,
        num_args = 0..=1,
        require_equals = true,
        default_missing_value = "true",
        default_value = "false",
        value_parser = parse_bool
    )]
    pub(crate) include_edge_recordings: bool,
    #[arg(long, help = "Project ID", allow_hyphen_values = true)]
    pub(crate) project_id: Option<String>,
    #[arg(long, help = "Recording ID", allow_hyphen_values = true)]
    pub(crate) recording_id: Option<String>,
    #[arg(long, help = "Session ID", allow_hyphen_values = true)]
    pub(crate) session_id: Option<String>,
    #[arg(long, help = "Session key", allow_hyphen_values = true)]
    pub(crate) session_key: Option<String>,
    #[arg(
        long,
        help = "Start of coverage time range (ISO 8601)",
        allow_hyphen_values = true
    )]
    pub(crate) start: Option<String>,
    #[arg(
        long,
        help = "Coverage separation tolerance in seconds",
        allow_hyphen_values = true
    )]
    pub(crate) tolerance: Option<i64>,
}

#[derive(Debug, Args)]
pub(crate) struct DataExportArgs {
    #[arg(
        long,
        help = "MCAP chunk compression: empty, zstd, or lz4 (default: lz4)",
        allow_hyphen_values = true
    )]
    pub(crate) compression: Option<String>,
    #[arg(long, help = "Device ID", allow_hyphen_values = true)]
    pub(crate) device_id: Option<String>,
    #[arg(long, help = "Device name", allow_hyphen_values = true)]
    pub(crate) device_name: Option<String>,
    #[arg(long, help = "End time (ISO 8601)", allow_hyphen_values = true)]
    pub(crate) end: Option<String>,
    #[arg(long, help = "Import ID", allow_hyphen_values = true)]
    pub(crate) import_id: Option<String>,
    #[arg(
        long, help = "Include MCAP attachments",
        action = clap::ArgAction::Set,
        num_args = 0..=1,
        require_equals = true,
        default_missing_value = "true",
        default_value = "false",
        value_parser = parse_bool
    )]
    pub(crate) include_attachments: bool,
    #[arg(long, help = "Recording key", allow_hyphen_values = true)]
    pub(crate) key: Option<String>,
    #[arg(long, short = 'o', help = "Output file", value_hint = ValueHint::FilePath, allow_hyphen_values = true)]
    pub(crate) output_file: Option<String>,
    #[arg(
        long,
        help = "Output format: mcap0, bag1, or json (default: mcap0)",
        allow_hyphen_values = true
    )]
    pub(crate) output_format: Option<String>,
    #[arg(long, help = "Project ID", allow_hyphen_values = true)]
    pub(crate) project_id: Option<String>,
    #[arg(long, help = "Recording ID", allow_hyphen_values = true)]
    pub(crate) recording_id: Option<String>,
    #[arg(
        long,
        help = "Maximum replay lookback in seconds",
        allow_hyphen_values = true
    )]
    pub(crate) replay_lookback_seconds: Option<f64>,
    #[arg(long, help = "Replay policy", allow_hyphen_values = true)]
    pub(crate) replay_policy: Option<String>,
    #[arg(long, help = "Session ID", allow_hyphen_values = true)]
    pub(crate) session_id: Option<String>,
    #[arg(long, help = "Session key", allow_hyphen_values = true)]
    pub(crate) session_key: Option<String>,
    #[arg(long, help = "Start time (ISO 8601)", allow_hyphen_values = true)]
    pub(crate) start: Option<String>,
    #[arg(long, help = "Comma-separated topic list", allow_hyphen_values = true)]
    pub(crate) topics: Option<String>,
}

#[derive(Debug, Args)]
pub(crate) struct DataImportArgs {
    #[arg(long, help = "Device ID", allow_hyphen_values = true)]
    pub(crate) device_id: Option<String>,
    #[arg(long, help = "Device name", allow_hyphen_values = true)]
    pub(crate) device_name: Option<String>,
    #[arg(long, help = "Recording key", allow_hyphen_values = true)]
    pub(crate) key: Option<String>,
    #[arg(long, help = "Project ID", allow_hyphen_values = true)]
    pub(crate) project_id: Option<String>,
    #[arg(long, help = "Session ID", allow_hyphen_values = true)]
    pub(crate) session_id: Option<String>,
    #[arg(long, help = "Session key", allow_hyphen_values = true)]
    pub(crate) session_key: Option<String>,
    #[arg(value_name = "FILE", value_hint = ValueHint::FilePath)]
    pub(crate) file: String,
}

#[derive(Debug, Subcommand)]
enum DatasetsCommand {
    #[command(about = "List the episodes in a dataset", subcommand)]
    Episodes(DatasetEpisodesCommand),
    #[command(about = "List datasets")]
    List(DatasetListArgs),
}

#[derive(Debug, Subcommand)]
enum DatasetEpisodesCommand {
    #[command(about = "List the episodes in a dataset")]
    List(DatasetEpisodeListArgs),
}

#[derive(Debug, Args)]
pub(crate) struct DatasetListArgs {
    #[command(flatten)]
    format: FormatArgs,
    #[arg(
        long,
        help = "Maximum number of items to return (0-2000, default: 2000)",
        allow_hyphen_values = true
    )]
    pub(crate) limit: Option<i64>,
    #[arg(
        long,
        help = "Number of items to skip before returning the results",
        allow_hyphen_values = true
    )]
    pub(crate) offset: Option<i64>,
    #[arg(long, help = "Filter datasets by project", allow_hyphen_values = true)]
    pub(crate) project_id: Option<String>,
    #[arg(
        long,
        help = "Field to sort datasets by: name, createdAt, or updatedAt (default: createdAt)",
        allow_hyphen_values = true
    )]
    pub(crate) sort_by: Option<String>,
    #[arg(
        long,
        help = "Sort order for the --sort-by field: asc or desc",
        allow_hyphen_values = true
    )]
    pub(crate) sort_order: Option<String>,
}

#[derive(Debug, Args)]
pub(crate) struct DatasetEpisodeListArgs {
    #[command(flatten)]
    format: FormatArgs,
    #[arg(value_name = "DATASET_ID")]
    pub(crate) dataset_id: String,
    #[arg(
        long,
        help = "End of a time range the episode's window must overlap (ISO 8601); give with --start",
        allow_hyphen_values = true
    )]
    pub(crate) end: Option<String>,
    #[arg(
        long, help = "Include the member recordings of each episode",
        action = clap::ArgAction::Set,
        num_args = 0..=1,
        require_equals = true,
        default_missing_value = "true",
        default_value = "false",
        value_parser = parse_bool
    )]
    pub(crate) include_recordings: bool,
    #[arg(
        long,
        help = "Maximum number of items to return (0-2000, default: 2000)",
        allow_hyphen_values = true
    )]
    pub(crate) limit: Option<i64>,
    #[arg(
        long,
        help = "Number of items to skip before returning the results",
        allow_hyphen_values = true
    )]
    pub(crate) offset: Option<i64>,
    #[arg(
        long,
        help = "Filter to episodes containing this recording display ID",
        allow_hyphen_values = true
    )]
    pub(crate) recording_id: Option<String>,
    #[arg(
        long,
        help = "Field to sort episodes by: addedAt (when the episode joined the dataset), createdAt, startTime, or endTime (default: addedAt)",
        allow_hyphen_values = true
    )]
    pub(crate) sort_by: Option<String>,
    #[arg(
        long,
        help = "Sort order for the --sort-by field: asc or desc",
        allow_hyphen_values = true
    )]
    pub(crate) sort_order: Option<String>,
    #[arg(
        long,
        help = "Start of a time range the episode's window must overlap (ISO 8601); give with --end",
        allow_hyphen_values = true
    )]
    pub(crate) start: Option<String>,
}

#[derive(Debug, Subcommand)]
enum DevicesCommand {
    #[command(about = "Add a device for your organization")]
    Add(DeviceWriteArgs),
    #[command(about = "Edit a device")]
    Edit(DeviceEditArgs),
    #[command(about = "List devices registered to your organization")]
    List(DeviceListArgs),
}

#[derive(Debug, Args)]
pub(crate) struct DeviceWriteArgs {
    #[arg(long, help = "Name of the device", allow_hyphen_values = true)]
    pub(crate) name: Option<String>,
    #[arg(long, help = "Project ID", allow_hyphen_values = true)]
    pub(crate) project_id: Option<String>,
    #[arg(long, short = 'p', help = "Custom property colon-separated key/value pair", allow_hyphen_values = true, action = clap::ArgAction::Append)]
    pub(crate) property: Vec<String>,
}

#[derive(Debug, Args)]
pub(crate) struct DeviceEditArgs {
    #[command(flatten)]
    pub(crate) update: DeviceWriteArgs,
    #[arg(value_name = "DEVICE_ID")]
    pub(crate) id: String,
}

#[derive(Debug, Args)]
pub(crate) struct DeviceListArgs {
    #[command(flatten)]
    format: FormatArgs,
    #[arg(long, help = "Project ID", allow_hyphen_values = true)]
    pub(crate) project_id: Option<String>,
}

#[derive(Debug, Subcommand)]
enum EpisodesCommand {
    #[command(about = "List episodes")]
    List(EpisodeListArgs),
}

#[derive(Debug, Args)]
pub(crate) struct EpisodeListArgs {
    #[command(flatten)]
    format: FormatArgs,
    #[arg(
        long,
        help = "End of a time range the episode's window must overlap (ISO 8601); give with --start",
        allow_hyphen_values = true
    )]
    pub(crate) end: Option<String>,
    #[arg(
        long, help = "Include the member recordings of each episode",
        action = clap::ArgAction::Set,
        num_args = 0..=1,
        require_equals = true,
        default_missing_value = "true",
        default_value = "false",
        value_parser = parse_bool
    )]
    pub(crate) include_recordings: bool,
    #[arg(
        long,
        help = "Maximum number of items to return (0-2000, default: 2000)",
        allow_hyphen_values = true
    )]
    pub(crate) limit: Option<i64>,
    #[arg(
        long,
        help = "Number of items to skip before returning the results",
        allow_hyphen_values = true
    )]
    pub(crate) offset: Option<i64>,
    #[arg(long, help = "Filter episodes by project", allow_hyphen_values = true)]
    pub(crate) project_id: Option<String>,
    #[arg(
        long,
        help = "Filter to episodes containing this recording display ID",
        allow_hyphen_values = true
    )]
    pub(crate) recording_id: Option<String>,
    #[arg(
        long,
        help = "Field to sort episodes by: createdAt, startTime, or endTime (default: createdAt)",
        allow_hyphen_values = true
    )]
    pub(crate) sort_by: Option<String>,
    #[arg(
        long,
        help = "Sort order for the --sort-by field: asc or desc",
        allow_hyphen_values = true
    )]
    pub(crate) sort_order: Option<String>,
    #[arg(
        long,
        help = "Start of a time range the episode's window must overlap (ISO 8601); give with --end",
        allow_hyphen_values = true
    )]
    pub(crate) start: Option<String>,
}

#[derive(Debug, Subcommand)]
enum EventTypesCommand {
    #[command(about = "List event types")]
    List(FormatArgs),
}

#[derive(Debug, Subcommand)]
enum EventsCommand {
    #[command(about = "Add an event")]
    Add(EventAddArgs),
    #[command(about = "List events")]
    List(EventListArgs),
}

#[derive(Debug, Args)]
pub(crate) struct EventAddArgs {
    #[arg(long, help = "Device ID", allow_hyphen_values = true)]
    pub(crate) device_id: Option<String>,
    #[arg(
        long,
        help = "End of event (inclusive), RFC 3339",
        allow_hyphen_values = true
    )]
    pub(crate) end: Option<String>,
    #[arg(long, help = "Associated event type ID", allow_hyphen_values = true)]
    pub(crate) event_type_id: Option<String>,
    #[arg(long, short = 'm', help = "Metadata colon-separated key/value pair", allow_hyphen_values = true, action = clap::ArgAction::Append)]
    pub(crate) metadata: Vec<String>,
    #[arg(long, help = "Start of event, RFC 3339", allow_hyphen_values = true)]
    pub(crate) start: Option<String>,
}

#[derive(Debug, Args)]
pub(crate) struct EventListArgs {
    #[command(flatten)]
    format: FormatArgs,
    #[arg(long, help = "Device ID", allow_hyphen_values = true)]
    pub(crate) device_id: Option<String>,
    #[arg(long, help = "Device name", allow_hyphen_values = true)]
    pub(crate) device_name: Option<String>,
    #[arg(
        long,
        help = "Exclude events after this time",
        allow_hyphen_values = true
    )]
    pub(crate) end: Option<String>,
    #[arg(long, help = "Event type ID", allow_hyphen_values = true)]
    pub(crate) event_type_id: Option<String>,
    #[arg(long, help = "Result limit (default: 100)", allow_hyphen_values = true)]
    pub(crate) limit: Option<i64>,
    #[arg(long, help = "Result offset", allow_hyphen_values = true)]
    pub(crate) offset: Option<i64>,
    #[arg(long, help = "Property or metadata query", allow_hyphen_values = true)]
    pub(crate) query: Option<String>,
    #[arg(long, help = "Fields to query by", allow_hyphen_values = true, action = clap::ArgAction::Append)]
    pub(crate) query_field: Vec<String>,
    #[arg(long, help = "Sort column", allow_hyphen_values = true)]
    pub(crate) sort_by: Option<String>,
    #[arg(long, help = "Sort order (default: asc)", allow_hyphen_values = true)]
    pub(crate) sort_order: Option<String>,
    #[arg(
        long,
        help = "Exclude events before this time",
        allow_hyphen_values = true
    )]
    pub(crate) start: Option<String>,
}

#[derive(Debug, Subcommand)]
enum ExtensionsCommand {
    #[command(about = "List Studio extensions created for your organization")]
    List(FormatArgs),
    #[command(about = "Publish a Studio extension (.foxe) to your organization")]
    Publish(FileArgs),
    #[command(about = "Delete and unpublish a Studio extension from your organization")]
    Unpublish(ExtensionIdArgs),
}

#[derive(Debug, Args)]
pub(crate) struct FileArgs {
    #[arg(value_name = "FILE", value_hint = ValueHint::FilePath)]
    pub(crate) file: String,
}

#[derive(Debug, Args)]
pub(crate) struct ExtensionIdArgs {
    #[arg(value_name = "EXTENSION_ID")]
    pub(crate) extension_id: String,
}

#[derive(Debug, Subcommand)]
enum PendingImportsCommand {
    #[command(about = "List pending and errored import jobs for uploaded recordings")]
    List(PendingImportListArgs),
}

#[derive(Debug, Args)]
pub(crate) struct PendingImportListArgs {
    #[command(flatten)]
    format: FormatArgs,
    #[arg(long, help = "Device ID", allow_hyphen_values = true)]
    pub(crate) device_id: Option<String>,
    #[arg(long, help = "Device name", allow_hyphen_values = true)]
    pub(crate) device_name: Option<String>,
    #[arg(long, help = "Filter by error message", allow_hyphen_values = true)]
    pub(crate) error: Option<String>,
    #[arg(long, help = "Filename", allow_hyphen_values = true)]
    pub(crate) filename: Option<String>,
    #[arg(long, help = "Key", allow_hyphen_values = true)]
    pub(crate) key: Option<String>,
    #[arg(long, help = "Project ID", allow_hyphen_values = true)]
    pub(crate) project_id: Option<String>,
    #[arg(long, help = "Request ID", allow_hyphen_values = true)]
    pub(crate) request_id: Option<String>,
    #[arg(long, help = "Session ID", allow_hyphen_values = true)]
    pub(crate) session_id: Option<String>,
    #[arg(long, help = "Session key", allow_hyphen_values = true)]
    pub(crate) session_key: Option<String>,
    #[arg(
        long, help = "Show completed requests",
        action = clap::ArgAction::Set,
        num_args = 0..=1,
        require_equals = true,
        default_missing_value = "true",
        default_value = "false",
        value_parser = parse_bool
    )]
    pub(crate) show_completed: bool,
    #[arg(
        long, help = "Show quarantined requests",
        action = clap::ArgAction::Set,
        num_args = 0..=1,
        require_equals = true,
        default_missing_value = "true",
        default_value = "false",
        value_parser = parse_bool
    )]
    pub(crate) show_quarantined: bool,
    #[arg(long, help = "Site ID", allow_hyphen_values = true)]
    pub(crate) site_id: Option<String>,
    #[arg(
        long,
        help = "Only imports updated since this time",
        allow_hyphen_values = true
    )]
    pub(crate) updated_since: Option<String>,
    #[arg(
        long, help = "Only imports without a project",
        action = clap::ArgAction::Set,
        num_args = 0..=1,
        require_equals = true,
        default_missing_value = "true",
        default_value = "false",
        value_parser = parse_bool
    )]
    pub(crate) without_project: bool,
}

#[derive(Debug, Subcommand)]
enum ProjectsCommand {
    #[command(about = "List projects")]
    List(FormatArgs),
}

#[derive(Debug, Subcommand)]
enum RecordingsCommand {
    #[command(
        about = "Request transfer of a recording from its Edge Site to its configured Primary Site"
    )]
    Transfer(RecordingTransferArgs),
    #[command(
        about = "Delete a recording from your organization",
        long_about = "Delete a recording and its data. For recordings imported from an Edge Site, only the imported data is removed: the edge copy and session membership remain, and the recording can be transferred again."
    )]
    Delete(RecordingDeleteArgs),
    #[command(about = "List recordings")]
    List(Box<RecordingListArgs>),
}

#[derive(Debug, Args)]
pub(crate) struct RecordingTransferArgs {
    #[arg(value_name = "RECORDING_ID")]
    pub(crate) id: String,
}

#[derive(Debug, Args)]
pub(crate) struct RecordingDeleteArgs {
    #[arg(value_name = "RECORDING_ID")]
    pub(crate) id: String,
}

#[derive(Debug, Args)]
pub(crate) struct RecordingListArgs {
    #[command(flatten)]
    format: FormatArgs,
    #[arg(long, help = "Device ID", allow_hyphen_values = true)]
    pub(crate) device_id: Option<String>,
    #[arg(long, help = "Device name", allow_hyphen_values = true)]
    pub(crate) device_name: Option<String>,
    #[arg(long, help = "Edge site ID", allow_hyphen_values = true)]
    pub(crate) edge_site_id: Option<String>,
    #[arg(
        long,
        help = "End of data range (ISO 8601)",
        allow_hyphen_values = true
    )]
    pub(crate) end: Option<String>,
    #[arg(long, help = "Import status", allow_hyphen_values = true)]
    pub(crate) import_status: Option<String>,
    #[arg(
        long,
        help = "Maximum result count (default: 2000)",
        allow_hyphen_values = true
    )]
    pub(crate) limit: Option<i64>,
    #[arg(
        long,
        help = "Number of recordings to skip",
        allow_hyphen_values = true
    )]
    pub(crate) offset: Option<i64>,
    #[arg(long, help = "Recording file path", allow_hyphen_values = true)]
    pub(crate) path: Option<String>,
    #[arg(long, help = "Project ID", allow_hyphen_values = true)]
    pub(crate) project_id: Option<String>,
    #[arg(long, help = "Session ID", allow_hyphen_values = true)]
    pub(crate) session_id: Option<String>,
    #[arg(long, help = "Session key", allow_hyphen_values = true)]
    pub(crate) session_key: Option<String>,
    #[arg(long, help = "Primary site ID", allow_hyphen_values = true)]
    pub(crate) site_id: Option<String>,
    #[arg(long, help = "Sort field", allow_hyphen_values = true)]
    pub(crate) sort_by: Option<String>,
    #[arg(long, help = "Sort order: asc or desc", allow_hyphen_values = true)]
    pub(crate) sort_order: Option<String>,
    #[arg(
        long,
        help = "Start of data range (ISO 8601)",
        allow_hyphen_values = true
    )]
    pub(crate) start: Option<String>,
}

#[derive(Debug, Subcommand)]
enum SessionsCommand {
    #[command(about = "Create a session")]
    Add(SessionAddArgs),
    #[command(about = "Delete a session")]
    Delete(SessionLookupArgs),
    #[command(about = "Get a session by ID or key")]
    Get(SessionLookupArgs),
    #[command(about = "List sessions in your organization")]
    List(SessionListArgs),
    #[command(about = "List, add, or remove recordings in a session", subcommand)]
    Recordings(SessionRecordingsCommand),
}

#[derive(Debug, Args)]
pub(crate) struct SessionAddArgs {
    #[arg(long, help = "Device ID (required)", allow_hyphen_values = true)]
    pub(crate) device_id: Option<String>,
    #[arg(long, help = "Session name", allow_hyphen_values = true)]
    pub(crate) name: Option<String>,
    #[arg(long, help = "Project ID", allow_hyphen_values = true)]
    pub(crate) project_id: Option<String>,
}

#[derive(Debug, Args)]
pub(crate) struct SessionLookupArgs {
    #[arg(long, help = "Project ID", allow_hyphen_values = true)]
    pub(crate) project_id: Option<String>,
    #[arg(value_name = "SESSION_ID_OR_KEY")]
    pub(crate) session: String,
}

#[derive(Debug, Args)]
pub(crate) struct SessionListArgs {
    #[command(flatten)]
    format: FormatArgs,
    #[arg(long, help = "Filter by device ID", allow_hyphen_values = true)]
    pub(crate) device_id: Option<String>,
    #[arg(long, help = "Filter by device name", allow_hyphen_values = true)]
    pub(crate) device_name: Option<String>,
    #[arg(long, help = "Project ID", allow_hyphen_values = true)]
    pub(crate) project_id: Option<String>,
}

#[derive(Debug, Subcommand)]
enum SessionRecordingsCommand {
    #[command(about = "Assign a recording to a session")]
    Add(SessionRecordingMutationArgs),
    #[command(about = "List recording IDs in a session")]
    List(SessionLookupArgs),
    #[command(about = "Remove a recording from a session")]
    Remove(SessionRecordingMutationArgs),
}

#[derive(Debug, Args)]
pub(crate) struct SessionRecordingMutationArgs {
    #[arg(long, help = "Project ID", allow_hyphen_values = true)]
    pub(crate) project_id: Option<String>,
    #[arg(value_name = "SESSION")]
    pub(crate) session: String,
    #[arg(value_name = "RECORDING")]
    pub(crate) recording: String,
}

#[derive(Debug, Subcommand)]
enum TopicsCommand {
    #[command(about = "List topics")]
    List(TopicListArgs),
}

#[derive(Debug, Args)]
pub(crate) struct TopicListArgs {
    #[command(flatten)]
    format: FormatArgs,
    #[arg(long, help = "Device ID", allow_hyphen_values = true)]
    pub(crate) device_id: Option<String>,
    #[arg(long, help = "Device name", allow_hyphen_values = true)]
    pub(crate) device_name: Option<String>,
    #[arg(
        long,
        help = "End of topic time range (ISO 8601)",
        allow_hyphen_values = true
    )]
    pub(crate) end: Option<String>,
    #[arg(
        long, help = "Include full topic schemas",
        action = clap::ArgAction::Set,
        num_args = 0..=1,
        require_equals = true,
        default_missing_value = "true",
        default_value = "false",
        value_parser = parse_bool
    )]
    pub(crate) include_schemas: bool,
    #[arg(long, help = "Maximum number of topics", allow_hyphen_values = true)]
    pub(crate) limit: Option<i64>,
    #[arg(long, help = "Number of topics to skip", allow_hyphen_values = true)]
    pub(crate) offset: Option<i64>,
    #[arg(long, help = "Project ID", allow_hyphen_values = true)]
    pub(crate) project_id: Option<String>,
    #[arg(long, help = "Recording ID", allow_hyphen_values = true)]
    pub(crate) recording_id: Option<String>,
    #[arg(long, help = "Recording key", allow_hyphen_values = true)]
    pub(crate) recording_key: Option<String>,
    #[arg(long, help = "Session ID", allow_hyphen_values = true)]
    pub(crate) session_id: Option<String>,
    #[arg(long, help = "Session key", allow_hyphen_values = true)]
    pub(crate) session_key: Option<String>,
    #[arg(long, help = "Sort by topic or version", allow_hyphen_values = true)]
    pub(crate) sort_by: Option<String>,
    #[arg(long, help = "Sort order: asc or desc", allow_hyphen_values = true)]
    pub(crate) sort_order: Option<String>,
    #[arg(
        long,
        help = "Start of topic time range (ISO 8601)",
        allow_hyphen_values = true
    )]
    pub(crate) start: Option<String>,
}

#[derive(Debug, Args)]
struct FormatArgs {
    #[arg(
        long,
        help = "Render output in table, JSON, or CSV format",
        value_enum,
        default_value = "table"
    )]
    format: Format,
}

/// Captured process output and status for one invocation.
#[derive(Debug, Default, Eq, PartialEq)]
pub struct Outcome {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub exit_code: u8,
}

impl Outcome {
    pub(crate) fn success(stdout: impl Into<Vec<u8>>) -> Self {
        Self {
            stdout: stdout.into(),
            ..Self::default()
        }
    }

    pub(crate) fn failure(stderr: impl Into<Vec<u8>>) -> Self {
        Self {
            stderr: stderr.into(),
            exit_code: 1,
            ..Self::default()
        }
    }
}

pub async fn run_async(
    cli_args: &[OsString],
    stdin: &mut dyn BufRead,
    prompt_writer: &mut dyn std::io::Write,
) -> Outcome {
    let mut argv = Vec::with_capacity(cli_args.len() + 1);
    argv.push(OsString::from(ROOT_COMMAND));
    // Clap's generated `help` subcommand accepts a target path, but treats a
    // following `--help` as a target name. Convert `<path> help --help` to the
    // equivalent `<path> --help`, preserving global flags in either position.
    for (index, argument) in cli_args.iter().enumerate() {
        if argument == "help" && cli_args.get(index + 1).is_some_and(|next| next == "--help") {
            continue;
        }
        argv.push(argument.clone());
    }
    let cli = match Cli::try_parse_from(argv) {
        Ok(cli) => cli,
        Err(error) if error.kind() == ErrorKind::DisplayHelp => {
            return Outcome::success(error.to_string())
        }
        Err(error) => return Outcome::failure(error.to_string()),
    };
    dispatch(cli, stdin, prompt_writer).await
}

async fn dispatch(cli: Cli, stdin: &mut dyn BufRead, writer: &mut dyn Write) -> Outcome {
    let Cli {
        client_id,
        config,
        debug,
        command,
    } = cli;
    let Some(command) = command else {
        return root_help_outcome();
    };
    match command {
        CliCommand::Version => Outcome::success(format!("{}\n", runtime::version())),
        CliCommand::Config(ConfigCommand::Get(args)) => run_config_get(args.key, config.as_deref()),
        CliCommand::Config(ConfigCommand::Set(args)) => run_config_set(&args, config.as_deref()),
        CliCommand::Config(ConfigCommand::Unset(args)) => {
            run_config_unset(args.key, config.as_deref())
        }
        CliCommand::Auth(AuthCommand::ConfigureApiKey(args)) => {
            configure_api_key(&args, config.as_deref(), stdin, writer)
        }
        CliCommand::Auth(AuthCommand::Login(args)) => {
            auth::login(&args, config.as_deref(), client_id.as_deref(), writer).await
        }
        CliCommand::Completion(shell) => {
            let (shell, no_descriptions) = match shell {
                CompletionCommand::Bash(args) => ("bash", args.no_descriptions),
                CompletionCommand::Fish(args) => ("fish", args.no_descriptions),
                CompletionCommand::Powershell(args) => ("powershell", args.no_descriptions),
                CompletionCommand::Zsh(args) => ("zsh", args.no_descriptions),
            };
            completion_script(shell, no_descriptions)
        }
        other => {
            dispatch_api_command(
                other,
                config.as_deref(),
                client_id.as_deref(),
                debug,
                writer,
            )
            .await
        }
    }
}

async fn dispatch_api_command(
    command: CliCommand,
    config_path: Option<&std::path::Path>,
    client_id: Option<&str>,
    debug: bool,
    writer: &mut dyn Write,
) -> Outcome {
    let runtime = match runtime::load(config_path, client_id) {
        Ok(runtime) => runtime,
        Err(error) => return Outcome::failure(error),
    };
    match command {
        CliCommand::Attachments(AttachmentsCommand::Download(args)) => {
            attachments::download_attachment(&runtime, &args, writer).await
        }
        CliCommand::Attachments(AttachmentsCommand::List(args)) => {
            let format = args.format.format;
            attachments::list_attachments(&runtime, &args, format).await
        }
        CliCommand::Auth(AuthCommand::Info) => auth::info(&runtime).await,
        CliCommand::Coverage(CoverageCommand::List(args)) => {
            let format = args.format.format;
            data::list_coverage(&runtime, &args, format).await
        }
        CliCommand::Export(args) => {
            let diagnostic = debug.then(|| data::export_debug_request(&args)).flatten();
            if let Some(diagnostic) = diagnostic {
                let _ = std::io::stderr().write_all(diagnostic.as_bytes());
            }
            data::export_data(&runtime, &args, writer).await
        }
        CliCommand::Upload(args) => data::import_file(&runtime, &args).await,
        CliCommand::Recordings(RecordingsCommand::Transfer(args)) => {
            recordings::transfer_recording(&runtime, &args).await
        }
        CliCommand::Datasets(DatasetsCommand::Episodes(DatasetEpisodesCommand::List(args))) => {
            let format = args.format.format;
            datasets::list_dataset_episodes(&runtime, &args, format).await
        }
        CliCommand::Datasets(DatasetsCommand::List(args)) => {
            let format = args.format.format;
            datasets::list_datasets(&runtime, &args, format).await
        }
        CliCommand::Devices(DevicesCommand::Add(args)) => {
            devices::add_device(&runtime, &args).await
        }
        CliCommand::Devices(DevicesCommand::Edit(args)) => {
            devices::edit_device(&runtime, &args).await
        }
        CliCommand::Devices(DevicesCommand::List(args)) => {
            let format = args.format.format;
            devices::list_devices(&runtime, &args, format).await
        }
        CliCommand::Episodes(EpisodesCommand::List(args)) => {
            let format = args.format.format;
            episodes::list_episodes(&runtime, &args, format).await
        }
        CliCommand::EventTypes(EventTypesCommand::List(args)) => {
            event_types::list_event_types(&runtime, args.format).await
        }
        CliCommand::Events(EventsCommand::Add(args)) => events::add_event(&runtime, &args).await,
        CliCommand::Events(EventsCommand::List(args)) => {
            let format = args.format.format;
            events::list_events(&runtime, &args, format).await
        }
        CliCommand::Extensions(ExtensionsCommand::List(args)) => {
            extensions::list_extensions(&runtime, args.format).await
        }
        CliCommand::Extensions(ExtensionsCommand::Publish(args)) => {
            extensions::publish_extension(&runtime, &args).await
        }
        CliCommand::Extensions(ExtensionsCommand::Unpublish(args)) => {
            extensions::unpublish_extension(&runtime, &args).await
        }
        CliCommand::PendingImports(PendingImportsCommand::List(args)) => {
            let format = args.format.format;
            pending_imports::list_pending_imports(&runtime, &args, format).await
        }
        CliCommand::Projects(ProjectsCommand::List(args)) => {
            projects::list_projects(&runtime, args.format).await
        }
        CliCommand::Recordings(RecordingsCommand::Delete(args)) => {
            recordings::delete_recording(&runtime, &args).await
        }
        CliCommand::Recordings(RecordingsCommand::List(args)) => {
            let format = args.format.format;
            recordings::list_recordings(&runtime, &args, format).await
        }
        CliCommand::Sessions(command) => dispatch_session_command(&runtime, command).await,
        CliCommand::Topics(TopicsCommand::List(args)) => {
            let format = args.format.format;
            topics::list_topics(&runtime, &args, format).await
        }
        CliCommand::Auth(AuthCommand::ConfigureApiKey(_) | AuthCommand::Login(_))
        | CliCommand::Completion(_)
        | CliCommand::Config(_)
        | CliCommand::Version => unreachable!("handled before API dispatch"),
    }
}

async fn dispatch_session_command(runtime: &runtime::Runtime, command: SessionsCommand) -> Outcome {
    match command {
        SessionsCommand::Add(args) => sessions::add_session(runtime, &args).await,
        SessionsCommand::Delete(args) => sessions::delete_session(runtime, &args).await,
        SessionsCommand::Get(args) => sessions::get_session(runtime, &args).await,
        SessionsCommand::List(args) => {
            let format = args.format.format;
            sessions::list_sessions(runtime, &args, format).await
        }
        SessionsCommand::Recordings(SessionRecordingsCommand::Add(args)) => {
            sessions::patch_session_recordings(runtime, &args, true).await
        }
        SessionsCommand::Recordings(SessionRecordingsCommand::List(args)) => {
            sessions::list_session_recordings(runtime, &args).await
        }
        SessionsCommand::Recordings(SessionRecordingsCommand::Remove(args)) => {
            sessions::patch_session_recordings(runtime, &args, false).await
        }
    }
}

fn root_help_outcome() -> Outcome {
    let mut command = Cli::command();
    let mut stdout = Vec::new();
    match command.write_long_help(&mut stdout) {
        Ok(()) => {
            stdout.push(b'\n');
            Outcome::success(stdout)
        }
        Err(error) => Outcome::failure(format!("failed to render help: {error}\n")),
    }
}

fn load_config(path: Option<&std::path::Path>) -> Result<Config, Outcome> {
    Config::load_from_path(path).map_err(Outcome::failure)
}

fn config_key_name(key: ConfigKey) -> &'static str {
    match key {
        ConfigKey::ProjectId => "project-id",
    }
}

fn run_config_get(selected_key: ConfigKey, path: Option<&std::path::Path>) -> Outcome {
    let key = config_key_name(selected_key);
    let config = match load_config(path) {
        Ok(config) => config,
        Err(outcome) => return outcome,
    };
    match config.get_string(config_name(key)) {
        Some(value) => Outcome::success(format!("{value}\n")),
        None => Outcome::failure(format!("No value set for key '{key}'\n")),
    }
}

fn run_config_set(args: &ConfigSetArgs, path: Option<&std::path::Path>) -> Outcome {
    let key = config_key_name(args.key);
    let value = args.value.clone();
    let mut config = match load_config(path) {
        Ok(config) => config,
        Err(outcome) => return outcome,
    };
    config.set(config_name(key), Value::String(value.clone()));
    match config.save() {
        Ok(()) => Outcome {
            stderr: format!("Configuration updated: {key} = {value}\n").into_bytes(),
            ..Outcome::default()
        },
        Err(error) => Outcome::failure(error),
    }
}

fn run_config_unset(selected_key: ConfigKey, path: Option<&std::path::Path>) -> Outcome {
    let key = config_key_name(selected_key);
    let mut config = match load_config(path) {
        Ok(config) => config,
        Err(outcome) => return outcome,
    };
    if !config.remove(config_name(key)) {
        return Outcome::failure(format!("No value set for key '{key}'\n"));
    }
    match config.save() {
        Ok(()) => Outcome {
            stderr: format!("Configuration removed: {key}\n").into_bytes(),
            ..Outcome::default()
        },
        Err(error) => Outcome::failure(error),
    }
}

fn config_name(key: &str) -> &str {
    if key == "project-id" {
        "default_project_id"
    } else {
        key
    }
}

fn configure_api_key(
    args: &ConfigureApiKeyArgs,
    config_path: Option<&std::path::Path>,
    stdin: &mut dyn BufRead,
    prompt_writer: &mut dyn std::io::Write,
) -> Outcome {
    let mut config = match Config::load_from_path(config_path) {
        Ok(config) => config,
        Err(error) => return Outcome::failure(error),
    };
    let token = match args.api_key.clone() {
        Some(token) if !token.is_empty() => token,
        _ => {
            if let Err(error) = writeln!(
                prompt_writer,
                "Enter an API key (will be written to {}):",
                config.path().display()
            )
            .and_then(|()| prompt_writer.flush())
            {
                return Outcome::failure(format!("failed to write prompt: {error}\n"));
            }
            let mut input = String::new();
            let bytes_read = match stdin.read_line(&mut input) {
                Ok(bytes_read) => bytes_read,
                Err(error) => return Outcome::failure(format!("failed to read input: {error}\n")),
            };
            if bytes_read == 0 {
                return Outcome::failure("failed to read input: EOF\n");
            }
            let Some(token) = input.split_whitespace().next() else {
                return Outcome::failure("failed to read input: unexpected newline\n");
            };
            token.to_owned()
        }
    };
    let base_url = args
        .base_url
        .clone()
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| runtime::DEFAULT_BASE_URL.to_owned());
    config.set("auth_type", Value::Number(2.into()));
    config.set("base_url", Value::String(base_url));
    config.set("bearer_token", Value::String(token));
    match config.save() {
        Ok(()) => Outcome::default(),
        Err(error) => Outcome {
            stderr: format!("Configuration failed: {}\n", error.trim_end()).into_bytes(),
            exit_code: 1,
            ..Outcome::default()
        },
    }
}

fn completion_script(shell: &str, no_descriptions: bool) -> Outcome {
    let generator = match shell {
        "bash" => Shell::Bash,
        "fish" => Shell::Fish,
        "powershell" => Shell::PowerShell,
        "zsh" => Shell::Zsh,
        _ => return Outcome::failure(format!("unsupported completion shell: {shell}\n")),
    };
    let mut command = Cli::command();
    if no_descriptions {
        command = without_descriptions(command);
    }
    let mut output = Vec::new();
    generate(generator, &mut command, ROOT_COMMAND, &mut output);
    Outcome::success(output)
}

fn without_descriptions(mut command: Command) -> Command {
    let arguments = command
        .get_arguments()
        .map(|argument| argument.get_id().to_string())
        .collect::<Vec<_>>();
    for argument in arguments {
        command = command.mut_arg(argument, |argument| {
            argument
                .help(Resettable::Reset)
                .long_help(Resettable::Reset)
        });
    }
    let subcommands = command
        .get_subcommands()
        .map(|subcommand| subcommand.get_name().to_owned())
        .collect::<Vec<_>>();
    for subcommand in subcommands {
        command = command.mut_subcommand(subcommand, without_descriptions);
    }
    command
        .about(Resettable::Reset)
        .long_about(Resettable::Reset)
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;
    use std::io::Cursor;

    use clap::{CommandFactory, Parser};

    use crate::output::Format;
    use crate::run;

    fn invoke(args: &[&str]) -> super::Outcome {
        let args = args.iter().map(OsString::from).collect::<Vec<_>>();
        run(&args, &mut Cursor::new(Vec::<u8>::new()))
    }

    #[test]
    fn boolean_flags_accept_go_values_and_preserve_defaults() {
        let cases = [
            (vec![], "debug"),
            (vec!["completion", "bash"], "no-descriptions"),
            (vec!["coverage", "list"], "include-edge-recordings"),
            (vec!["export"], "include-attachments"),
            (vec!["pending-imports", "list"], "show-completed"),
            (vec!["pending-imports", "list"], "show-quarantined"),
            (vec!["pending-imports", "list"], "without-project"),
            (vec!["topics", "list"], "include-schemas"),
        ];
        let values = [
            (None, false),
            (Some(""), true),
            (Some("=1"), true),
            (Some("=t"), true),
            (Some("=T"), true),
            (Some("=TRUE"), true),
            (Some("=true"), true),
            (Some("=True"), true),
            (Some("=0"), false),
            (Some("=f"), false),
            (Some("=F"), false),
            (Some("=FALSE"), false),
            (Some("=false"), false),
            (Some("=False"), false),
        ];
        for (path, flag) in cases {
            let id = flag.replace('-', "_");
            for (suffix, expected) in values {
                let mut argv = vec!["foxglove".to_owned()];
                argv.extend(path.iter().map(|part| (*part).to_owned()));
                if let Some(suffix) = suffix {
                    argv.push(format!("--{flag}{suffix}"));
                }
                let matches = super::Cli::command().try_get_matches_from(&argv).unwrap();
                let mut command = &matches;
                for part in &path {
                    command = command.subcommand_matches(part).unwrap();
                }
                assert_eq!(command.get_one::<bool>(&id), Some(&expected), "{argv:?}");
            }
            for value in ["", "yes", "no", "TrUe", "2"] {
                let mut argv = vec!["foxglove".to_owned()];
                argv.extend(path.iter().map(|part| (*part).to_owned()));
                argv.push(format!("--{flag}={value}"));
                assert!(super::Cli::try_parse_from(&argv).is_err(), "{argv:?}");
            }
        }
    }

    #[test]
    fn bare_boolean_flags_do_not_consume_positionals() {
        let cli = super::Cli::try_parse_from([
            "foxglove",
            "--debug",
            "config",
            "set",
            "project-id",
            "false",
        ])
        .unwrap();
        assert!(cli.debug);
        let Some(super::CliCommand::Config(super::ConfigCommand::Set(args))) = cli.command else {
            panic!("expected config set command");
        };
        assert_eq!(args.value, "false");
        let cli = super::Cli::try_parse_from(["foxglove", "--debug", "--debug=false", "version"])
            .unwrap();
        assert!(!cli.debug);
    }

    #[test]
    fn global_flags_are_accepted_before_and_after_commands() {
        assert_eq!(
            invoke(&["--debug", "version"]).stdout,
            format!("{}\n", crate::runtime::version()).as_bytes()
        );
        assert_eq!(
            invoke(&["version", "--debug"]).stdout,
            format!("{}\n", crate::runtime::version()).as_bytes()
        );
    }

    #[test]
    fn list_format_is_parsed_as_a_typed_value() {
        for (argument, expected) in [(None, Format::Table), (Some("csv"), Format::Csv)] {
            let mut argv = vec!["foxglove", "devices", "list"];
            if let Some(argument) = argument {
                argv.extend(["--format", argument]);
            }
            let cli = super::Cli::try_parse_from(argv).unwrap();
            let Some(super::CliCommand::Devices(super::DevicesCommand::List(list_options))) =
                cli.command
            else {
                panic!("expected devices list command");
            };
            assert_eq!(list_options.format.format, expected);
        }

        let error = super::Cli::try_parse_from(["foxglove", "devices", "list", "--format", "xml"])
            .unwrap_err();
        assert_eq!(error.kind(), clap::error::ErrorKind::InvalidValue);
    }

    #[test]
    fn help_command_routes_to_the_requested_command() {
        let outcome = invoke(&["help", "devices"]);
        assert_eq!(outcome.exit_code, 0);
        assert!(String::from_utf8(outcome.stdout)
            .unwrap()
            .starts_with("List and manage devices\n"));
    }

    #[test]
    fn help_flag_on_generated_help_subcommand_is_normalized() {
        for args in [
            ["help", "--help"].as_slice(),
            ["--debug", "help", "--help"].as_slice(),
            ["devices", "help", "--help"].as_slice(),
        ] {
            let outcome = invoke(args);
            assert_eq!(outcome.exit_code, 0, "{args:?}");
            assert!(!outcome.stdout.is_empty(), "{args:?}");
        }
    }

    #[test]
    fn completion_scripts_are_generated_from_the_command_tree() {
        for shell in ["bash", "fish", "powershell", "zsh"] {
            let outcome = invoke(&["completion", shell]);
            assert_eq!(outcome.exit_code, 0);
            assert!(!outcome.stdout.is_empty());
            assert!(String::from_utf8_lossy(&outcome.stdout).contains("foxglove"));
        }
    }

    #[test]
    fn completion_descriptions_can_be_disabled() {
        let described = invoke(&["completion", "fish"]);
        let plain = invoke(&["completion", "fish", "--no-descriptions"]);
        assert_ne!(described.stdout, plain.stdout);
        assert!(!String::from_utf8_lossy(&plain.stdout)
            .contains("List devices registered to your organization"));
    }
}
