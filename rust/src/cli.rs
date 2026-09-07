//! Structured command parsing and offline command execution.

use std::ffi::OsString;
use std::io::{BufRead, Write};
use std::path::PathBuf;

use clap::builder::Resettable;
use clap::error::ErrorKind;
use clap::{
    Arg, ArgAction, ArgMatches, Command, FromArgMatches, Parser, Subcommand, ValueEnum, ValueHint,
};
use clap_complete::{generate, Shell};
use serde_yaml_ng::Value;

use crate::config::Config;
use crate::{
    attachments, auth, data, devices, event_types, events, extensions, pending_imports, projects,
    read_helpers, recordings, sessions, topics,
};

const ROOT_COMMAND: &str = "foxglove";

/// The command hierarchy used for type-directed dispatch. Option and argument
/// metadata lives in the Clap builder below so help and completions share the
/// same source as parsing.
#[derive(Debug, Parser)]
#[command(name = ROOT_COMMAND)]
struct Cli {
    #[command(subcommand)]
    command: Option<CliCommand>,
}

#[derive(Debug, Subcommand)]
enum CliCommand {
    #[command(subcommand)]
    Attachments(AttachmentsCommand),
    #[command(subcommand)]
    Auth(AuthCommand),
    #[command(subcommand)]
    Completion(CompletionCommand),
    #[command(subcommand)]
    Config(ConfigCommand),
    #[command(subcommand)]
    Data(DataCommand),
    #[command(subcommand)]
    Devices(DevicesCommand),
    #[command(name = "event-types")]
    #[command(subcommand)]
    EventTypes(EventTypesCommand),
    #[command(subcommand)]
    Events(EventsCommand),
    #[command(subcommand)]
    Extensions(ExtensionsCommand),
    #[command(name = "pending-imports")]
    #[command(subcommand)]
    PendingImports(PendingImportsCommand),
    #[command(subcommand)]
    Projects(ProjectsCommand),
    #[command(subcommand)]
    Recordings(RecordingsCommand),
    #[command(subcommand)]
    Sessions(SessionsCommand),
    #[command(subcommand)]
    Topics(TopicsCommand),
    Version,
}

macro_rules! command_group {
    ($name:ident { $($variant:ident),+ $(,)? }) => {
        #[derive(Debug, Subcommand)]
        enum $name { $($variant),+ }
    };
}

command_group!(AttachmentsCommand { Download, List });
command_group!(AuthCommand {
    ConfigureApiKey,
    Info,
    Login
});
command_group!(CompletionCommand {
    Bash,
    Fish,
    Powershell,
    Zsh
});
#[derive(Debug, Subcommand)]
enum ConfigCommand {
    Get,
    Set,
    Unset,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum ConfigKey {
    ProjectId,
}
#[derive(Debug, Subcommand)]
enum DataCommand {
    #[command(subcommand)]
    Coverage(CoverageCommand),
    Export,
    Import,
}
command_group!(CoverageCommand { List });
command_group!(DevicesCommand { Add, Edit, List });
command_group!(EventTypesCommand { List });
command_group!(EventsCommand { Add, List });
command_group!(ExtensionsCommand {
    List,
    Publish,
    Unpublish
});
command_group!(PendingImportsCommand { List });
command_group!(ProjectsCommand { List });
command_group!(RecordingsCommand { Delete, List });
#[derive(Debug, Subcommand)]
enum SessionsCommand {
    Add,
    Delete,
    Get,
    List,
    #[command(subcommand)]
    Recordings(SessionRecordingsCommand),
}
command_group!(SessionRecordingsCommand { Add, List, Remove });
command_group!(TopicsCommand { List });

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
    let matches = match command().try_get_matches_from(argv) {
        Ok(matches) => matches,
        Err(error) if error.kind() == ErrorKind::DisplayHelp => {
            return Outcome::success(error.to_string())
        }
        Err(error) => return Outcome::failure(error.to_string()),
    };
    let cli = match Cli::from_arg_matches(&matches) {
        Ok(cli) => cli,
        Err(error) => return Outcome::failure(error.to_string()),
    };
    let leaf = deepest_matches(&matches);
    dispatch(
        cli.command,
        read_helpers::config_path(leaf),
        leaf,
        stdin,
        prompt_writer,
    )
    .await
}

fn deepest_matches(mut matches: &ArgMatches) -> &ArgMatches {
    while let Some((_, child)) = matches.subcommand() {
        matches = child;
    }
    matches
}

async fn dispatch(
    command: Option<CliCommand>,
    config_path: Option<&std::path::Path>,
    leaf: &ArgMatches,
    stdin: &mut dyn BufRead,
    writer: &mut dyn Write,
) -> Outcome {
    let Some(command) = command else {
        return root_help_outcome();
    };
    match command {
        CliCommand::Version => Outcome::success(format!("{}\n", read_helpers::version())),
        CliCommand::Config(ConfigCommand::Get) => run_config_get(leaf, config_path),
        CliCommand::Config(ConfigCommand::Set) => run_config_set(leaf, config_path),
        CliCommand::Config(ConfigCommand::Unset) => run_config_unset(leaf, config_path),
        CliCommand::Auth(AuthCommand::ConfigureApiKey) => configure_api_key(leaf, stdin, writer),
        CliCommand::Auth(AuthCommand::Login) => auth::login(leaf, writer).await,
        CliCommand::Completion(shell) => completion_script(
            match shell {
                CompletionCommand::Bash => "bash",
                CompletionCommand::Fish => "fish",
                CompletionCommand::Powershell => "powershell",
                CompletionCommand::Zsh => "zsh",
            },
            leaf.get_flag("no-descriptions"),
        ),
        other => dispatch_api_command(other, leaf, writer).await,
    }
}

async fn dispatch_api_command(
    command: CliCommand,
    matches: &ArgMatches,
    writer: &mut dyn Write,
) -> Outcome {
    let runtime = match read_helpers::runtime(matches) {
        Ok(runtime) => runtime,
        Err(error) => return Outcome::failure(error),
    };
    let format = match read_helpers::resolve_format(matches) {
        Ok(format) => format,
        Err(error) => return Outcome::failure(error),
    };
    match command {
        CliCommand::Attachments(AttachmentsCommand::Download) => {
            attachments::download_attachment(&runtime, matches, writer).await
        }
        CliCommand::Attachments(AttachmentsCommand::List) => {
            attachments::list_attachments(&runtime, matches, format).await
        }
        CliCommand::Auth(AuthCommand::Info) => auth::info(&runtime).await,
        CliCommand::Data(DataCommand::Coverage(CoverageCommand::List)) => {
            data::list_coverage(&runtime, matches, format).await
        }
        CliCommand::Data(DataCommand::Export) => {
            let diagnostic = matches
                .get_flag("debug")
                .then(|| data::export_debug_request(matches))
                .flatten();
            if let Some(diagnostic) = diagnostic {
                let _ = std::io::stderr().write_all(diagnostic.as_bytes());
            }
            data::export_data(&runtime, matches, writer).await
        }
        CliCommand::Data(DataCommand::Import)
            if !read_helpers::value(matches, "edge-recording-id").is_empty() =>
        {
            data::import_from_edge(&runtime, matches).await
        }
        CliCommand::Data(DataCommand::Import) => data::import_file(&runtime, matches).await,
        CliCommand::Devices(DevicesCommand::Add) => devices::add_device(&runtime, matches).await,
        CliCommand::Devices(DevicesCommand::Edit) => devices::edit_device(&runtime, matches).await,
        CliCommand::Devices(DevicesCommand::List) => {
            devices::list_devices(&runtime, matches, format).await
        }
        CliCommand::EventTypes(EventTypesCommand::List) => {
            event_types::list_event_types(&runtime, format).await
        }
        CliCommand::Events(EventsCommand::Add) => events::add_event(&runtime, matches).await,
        CliCommand::Events(EventsCommand::List) => {
            events::list_events(&runtime, matches, format).await
        }
        CliCommand::Extensions(ExtensionsCommand::List) => {
            extensions::list_extensions(&runtime, format).await
        }
        CliCommand::Extensions(ExtensionsCommand::Publish) => {
            extensions::publish_extension(&runtime, matches).await
        }
        CliCommand::Extensions(ExtensionsCommand::Unpublish) => {
            extensions::unpublish_extension(&runtime, matches).await
        }
        CliCommand::PendingImports(PendingImportsCommand::List) => {
            pending_imports::list_pending_imports(&runtime, matches, format).await
        }
        CliCommand::Projects(ProjectsCommand::List) => {
            projects::list_projects(&runtime, format).await
        }
        CliCommand::Recordings(RecordingsCommand::Delete) => {
            recordings::delete_recording(&runtime, matches).await
        }
        CliCommand::Recordings(RecordingsCommand::List) => {
            recordings::list_recordings(&runtime, matches, format).await
        }
        CliCommand::Sessions(SessionsCommand::Add) => {
            sessions::add_session(&runtime, matches).await
        }
        CliCommand::Sessions(SessionsCommand::Delete) => {
            sessions::delete_session(&runtime, matches).await
        }
        CliCommand::Sessions(SessionsCommand::Get) => {
            sessions::get_session(&runtime, matches).await
        }
        CliCommand::Sessions(SessionsCommand::List) => {
            sessions::list_sessions(&runtime, matches, format).await
        }
        CliCommand::Sessions(SessionsCommand::Recordings(SessionRecordingsCommand::Add)) => {
            sessions::patch_session_recordings(&runtime, matches, true).await
        }
        CliCommand::Sessions(SessionsCommand::Recordings(SessionRecordingsCommand::List)) => {
            sessions::list_session_recordings(&runtime, matches).await
        }
        CliCommand::Sessions(SessionsCommand::Recordings(SessionRecordingsCommand::Remove)) => {
            sessions::patch_session_recordings(&runtime, matches, false).await
        }
        CliCommand::Topics(TopicsCommand::List) => {
            topics::list_topics(&runtime, matches, format).await
        }
        _ => Outcome::failure("This command is not implemented in the Rust migration yet\n"),
    }
}

fn command() -> Command {
    Command::new(ROOT_COMMAND)
        .bin_name(ROOT_COMMAND)
        .about("Command line client for the Foxglove data platform")
        .disable_version_flag(true)
        .subcommand_precedence_over_arg(true)
        .arg(global_value("client-id", "Foxglove client ID"))
        .arg(
            global_value("config", "Config file")
                .value_parser(clap::value_parser!(PathBuf))
                .value_hint(ValueHint::FilePath),
        )
        .arg(flag("debug", "Enable debug logging").global(true))
        .subcommand(attachments_command())
        .subcommand(auth_command())
        .subcommand(completion_command())
        .subcommand(
            group("config", "Manage CLI configuration values")
                .subcommand(leaf("get", "Get a configuration value").arg(config_key_arg()))
                .subcommand(
                    leaf("set", "Set a configuration value")
                        .arg(config_key_arg())
                        .arg(Arg::new("value").value_name("VALUE").required(true)),
                )
                .subcommand(leaf("unset", "Remove a configuration value").arg(config_key_arg())),
        )
        .subcommand(data_command())
        .subcommand(devices_command())
        .subcommand(
            group("event-types", "List event types")
                .subcommand(with_format(leaf("list", "List event types"))),
        )
        .subcommand(events_command())
        .subcommand(extensions_command())
        .subcommand(pending_imports_command())
        .subcommand(
            group("projects", "List and manage projects")
                .subcommand(with_format(leaf("list", "List projects"))),
        )
        .subcommand(recordings_command())
        .subcommand(sessions_command())
        .subcommand(topics_command())
        .subcommand(leaf("version", "Print Foxglove CLI version"))
}

fn attachments_command() -> Command {
    group("attachments", "Query and modify data attachments")
        .subcommand(
            leaf("download", "Download an MCAP attachment by ID")
                .arg(positional("attachment-id", "ATTACHMENT_ID")),
        )
        .subcommand(
            with_format(leaf("list", "List MCAP attachments"))
                .arg(value("import-id", "Import ID"))
                .arg(value("project-id", "Project ID"))
                .arg(value("recording-id", "Recording ID"))
                .arg(value("session-id", "Session ID"))
                .arg(value("session-key", "Session key")),
        )
}

fn auth_command() -> Command {
    group("auth", "Manage authentication")
        .subcommand(
            leaf("configure-api-key", "Configure an API key")
                .arg(value("api-key", "API key for non-interactive use"))
                .arg(value(
                    "base-url",
                    "API server (default: https://api.foxglove.dev)",
                )),
        )
        .subcommand(leaf(
            "info",
            "Display information about the currently authenticated user",
        ))
        .subcommand(leaf("login", "Log in to Foxglove Data Platform").arg(value(
            "base-url",
            "API server (default: https://api.foxglove.dev)",
        )))
}

fn devices_command() -> Command {
    group("devices", "List and manage devices")
        .subcommand(
            leaf("add", "Add a device for your organization")
                .arg(value("name", "Name of the device"))
                .arg(value("project-id", "Project ID"))
                .arg(repeated_value(
                    "property",
                    Some('p'),
                    "Custom property colon-separated key/value pair",
                )),
        )
        .subcommand(
            leaf("edit", "Edit a device")
                .arg(value("name", "New name for the device"))
                .arg(value("project-id", "Project ID"))
                .arg(repeated_value(
                    "property",
                    Some('p'),
                    "Custom property colon-separated key/value pair",
                ))
                .arg(positional("device-id-arg", "DEVICE_ID")),
        )
        .subcommand(
            with_format(leaf("list", "List devices registered to your organization"))
                .arg(value("project-id", "Project ID")),
        )
}

fn extensions_command() -> Command {
    group("extensions", "List and publish Studio extensions")
        .subcommand(with_format(leaf(
            "list",
            "List Studio extensions created for your organization",
        )))
        .subcommand(
            leaf(
                "publish",
                "Publish a Studio extension (.foxe) to your organization",
            )
            .arg(file_positional("file", "FILE")),
        )
        .subcommand(
            leaf(
                "unpublish",
                "Delete and unpublish a Studio extension from your organization",
            )
            .arg(positional("extension-id", "EXTENSION_ID")),
        )
}

fn data_command() -> Command {
    group("data", "Data access and management")
        .subcommand(
            group("coverage", "List coverage ranges").subcommand(
                with_format(leaf("list", "List coverage ranges"))
                    .arg(value("device-id", "Device ID"))
                    .arg(value("device-name", "Device name"))
                    .arg(value("end", "End of coverage time range (ISO 8601)"))
                    .arg(flag("include-edge-recordings", "Include edge recordings"))
                    .arg(value("project-id", "Project ID"))
                    .arg(value("recording-id", "Recording ID"))
                    .arg(value("session-id", "Session ID"))
                    .arg(value("session-key", "Session key"))
                    .arg(value("start", "Start of coverage time range (ISO 8601)"))
                    .arg(value(
                        "tolerance",
                        "Coverage separation tolerance in seconds",
                    )),
            ),
        )
        .subcommand(
            leaf(
                "export",
                "Export data by recording, import, session, or device and time range",
            )
            .arg(value(
                "compression",
                "MCAP chunk compression: empty, zstd, or lz4 (default: lz4)",
            ))
            .arg(value("device-id", "Device ID"))
            .arg(value("device-name", "Device name"))
            .arg(value("end", "End time (ISO 8601)"))
            .arg(value("import-id", "Import ID"))
            .arg(flag("include-attachments", "Include MCAP attachments"))
            .arg(value("key", "Recording key"))
            .arg(file_value("output-file", Some('o'), "Output file"))
            .arg(value(
                "output-format",
                "Output format: mcap0, bag1, or json (default: mcap0)",
            ))
            .arg(value("project-id", "Project ID"))
            .arg(value("recording-id", "Recording ID"))
            .arg(value(
                "replay-lookback-seconds",
                "Maximum replay lookback in seconds",
            ))
            .arg(value("replay-policy", "Replay policy"))
            .arg(value("session-id", "Session ID"))
            .arg(value("session-key", "Session key"))
            .arg(value("start", "Start time (ISO 8601)"))
            .arg(value("topics", "Comma-separated topic list")),
        )
        .subcommand(import_command())
}

fn import_command() -> Command {
    leaf("import", "Import a data file to Foxglove Data Platform")
        .arg(value("device-id", "Device ID"))
        .arg(value("device-name", "Device name"))
        .arg(value("edge-recording-id", "Edge recording ID"))
        .arg(value("key", "Recording key"))
        .arg(value("project-id", "Project ID"))
        .arg(value("session-id", "Session ID"))
        .arg(value("session-key", "Session key"))
        .arg(file_positional("file", "FILE"))
}

fn events_command() -> Command {
    group("events", "List and manage events")
        .subcommand(
            leaf("add", "Add an event")
                .arg(value("device-id", "Device ID"))
                .arg(value("end", "End of event (inclusive), RFC 3339"))
                .arg(value("event-type-id", "Associated event type ID"))
                .arg(repeated_value(
                    "metadata",
                    Some('m'),
                    "Metadata colon-separated key/value pair",
                ))
                .arg(value("start", "Start of event, RFC 3339")),
        )
        .subcommand(
            with_format(leaf("list", "List events"))
                .arg(value("device-id", "Device ID"))
                .arg(value("device-name", "Device name"))
                .arg(value("end", "Exclude events after this time"))
                .arg(value("event-type-id", "Event type ID"))
                .arg(value("limit", "Result limit (default: 100)"))
                .arg(value("offset", "Result offset"))
                .arg(value("query", "Property or metadata query"))
                .arg(repeated_value("query-field", None, "Fields to query by"))
                .arg(value("sort-by", "Sort column"))
                .arg(value("sort-order", "Sort order (default: asc)"))
                .arg(value("start", "Exclude events before this time")),
        )
}

fn pending_imports_command() -> Command {
    group("pending-imports", "List pending imports").subcommand(
        with_format(leaf(
            "list",
            "List pending and errored import jobs for uploaded recordings",
        ))
        .arg(value("device-id", "Device ID"))
        .arg(value("device-name", "Device name"))
        .arg(value("error", "Filter by error message"))
        .arg(value("filename", "Filename"))
        .arg(value("key", "Key"))
        .arg(value("project-id", "Project ID"))
        .arg(value("request-id", "Request ID"))
        .arg(value("session-id", "Session ID"))
        .arg(value("session-key", "Session key"))
        .arg(flag("show-completed", "Show completed requests"))
        .arg(flag("show-quarantined", "Show quarantined requests"))
        .arg(value("site-id", "Site ID"))
        .arg(value(
            "updated-since",
            "Only imports updated since this time",
        ))
        .arg(flag("without-project", "Only imports without a project")),
    )
}

fn recordings_command() -> Command {
    group("recordings", "Query recordings")
        .subcommand(
            leaf("delete", "Delete a recording from your organization")
                .arg(positional("recording-id-arg", "RECORDING_ID")),
        )
        .subcommand(
            with_format(leaf("list", "List recordings"))
                .arg(value("device-id", "Device ID"))
                .arg(value("device-name", "Device name"))
                .arg(value("edge-site-id", "Edge site ID"))
                .arg(value("end", "End of data range (ISO 8601)"))
                .arg(value("import-status", "Import status"))
                .arg(value("limit", "Maximum result count (default: 2000)"))
                .arg(value("offset", "Number of recordings to skip"))
                .arg(value("path", "Recording file path"))
                .arg(value("project-id", "Project ID"))
                .arg(value("session-id", "Session ID"))
                .arg(value("session-key", "Session key"))
                .arg(value("site-id", "Primary site ID"))
                .arg(value("sort-by", "Sort field"))
                .arg(value("sort-order", "Sort order: asc or desc"))
                .arg(value("start", "Start of data range (ISO 8601)")),
        )
}

fn sessions_command() -> Command {
    group("sessions", "List and manage sessions")
        .subcommand(
            leaf("add", "Create a session")
                .arg(value("device-id", "Device ID (required)"))
                .arg(value("name", "Session name"))
                .arg(value("project-id", "Project ID")),
        )
        .subcommand(
            leaf("delete", "Delete a session")
                .arg(value("project-id", "Project ID"))
                .arg(positional("session", "SESSION_ID_OR_KEY")),
        )
        .subcommand(
            leaf("get", "Get a session by ID or key")
                .arg(value("project-id", "Project ID"))
                .arg(positional("session", "SESSION_ID_OR_KEY")),
        )
        .subcommand(
            with_format(leaf("list", "List sessions in your organization"))
                .arg(value("device-id", "Filter by device ID"))
                .arg(value("device-name", "Filter by device name"))
                .arg(value("project-id", "Project ID")),
        )
        .subcommand(
            group("recordings", "List, add, or remove recordings in a session")
                .subcommand(
                    leaf("add", "Assign a recording to a session")
                        .arg(value("project-id", "Project ID"))
                        .arg(positional("session", "SESSION"))
                        .arg(positional("recording", "RECORDING")),
                )
                .subcommand(
                    leaf("list", "List recording IDs in a session")
                        .arg(value("project-id", "Project ID"))
                        .arg(positional("session", "SESSION")),
                )
                .subcommand(
                    leaf("remove", "Remove a recording from a session")
                        .arg(value("project-id", "Project ID"))
                        .arg(positional("session", "SESSION"))
                        .arg(positional("recording", "RECORDING")),
                ),
        )
}

fn topics_command() -> Command {
    group("topics", "List topics").subcommand(
        with_format(leaf("list", "List topics"))
            .arg(value("device-id", "Device ID"))
            .arg(value("device-name", "Device name"))
            .arg(value("end", "End of topic time range (ISO 8601)"))
            .arg(flag("include-schemas", "Include full topic schemas"))
            .arg(value("limit", "Maximum number of topics"))
            .arg(value("offset", "Number of topics to skip"))
            .arg(value("project-id", "Project ID"))
            .arg(value("recording-id", "Recording ID"))
            .arg(value("recording-key", "Recording key"))
            .arg(value("session-id", "Session ID"))
            .arg(value("session-key", "Session key"))
            .arg(value("sort-by", "Sort by topic or version"))
            .arg(value("sort-order", "Sort order: asc or desc"))
            .arg(value("start", "Start of topic time range (ISO 8601)")),
    )
}

fn completion_command() -> Command {
    let mut command = group("completion", "Generate a shell completion script");
    for (shell, description) in [
        ("bash", "Generate completions for Bash"),
        ("fish", "Generate completions for Fish"),
        ("powershell", "Generate completions for PowerShell"),
        ("zsh", "Generate completions for Zsh"),
    ] {
        command = command.subcommand(
            leaf(shell, description)
                .arg(flag("no-descriptions", "Disable completion descriptions")),
        );
    }
    command
}

fn group(name: &'static str, about: &'static str) -> Command {
    Command::new(name)
        .about(about)
        .disable_version_flag(true)
        .subcommand_precedence_over_arg(true)
}

fn leaf(name: &'static str, about: &'static str) -> Command {
    group(name, about)
}

fn value(name: &'static str, help: &'static str) -> Arg {
    Arg::new(name)
        .long(name)
        .help(help)
        .num_args(1)
        .allow_hyphen_values(true)
        .action(ArgAction::Set)
        .overrides_with(name)
}

fn file_value(name: &'static str, short: Option<char>, help: &'static str) -> Arg {
    let argument = value(name, help).value_hint(ValueHint::FilePath);
    short.map_or(argument.clone(), |short| argument.short(short))
}

fn repeated_value(name: &'static str, short: Option<char>, help: &'static str) -> Arg {
    let argument = Arg::new(name)
        .long(name)
        .help(help)
        .num_args(1)
        .allow_hyphen_values(true)
        .action(ArgAction::Append);
    short.map_or(argument.clone(), |short| argument.short(short))
}

fn flag(name: &'static str, help: &'static str) -> Arg {
    Arg::new(name)
        .long(name)
        .help(help)
        .action(ArgAction::SetTrue)
        .overrides_with(name)
}

fn global_value(name: &'static str, help: &'static str) -> Arg {
    value(name, help).global(true)
}

fn positional(id: &'static str, value_name: &'static str) -> Arg {
    Arg::new(id).value_name(value_name).required(true)
}

fn file_positional(id: &'static str, value_name: &'static str) -> Arg {
    positional(id, value_name).value_hint(ValueHint::FilePath)
}

fn config_key_arg() -> Arg {
    Arg::new("key")
        .value_name("KEY")
        .required(true)
        .value_parser(clap::builder::EnumValueParser::<ConfigKey>::new())
}

fn with_format(command: Command) -> Command {
    command.arg(value(
        "format",
        "Render output in table, JSON, or CSV format",
    ))
}

fn root_help_outcome() -> Outcome {
    let mut command = command();
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

fn selected_config_key(matches: &ArgMatches) -> &'static str {
    config_key_name(
        *matches
            .get_one::<ConfigKey>("key")
            .expect("required by Clap"),
    )
}

fn run_config_get(matches: &ArgMatches, path: Option<&std::path::Path>) -> Outcome {
    let key = selected_config_key(matches);
    let config = match load_config(path) {
        Ok(config) => config,
        Err(outcome) => return outcome,
    };
    match config.get_string(config_name(key)) {
        Some(value) => Outcome::success(format!("{value}\n")),
        None => Outcome::failure(format!("No value set for key '{key}'\n")),
    }
}

fn run_config_set(matches: &ArgMatches, path: Option<&std::path::Path>) -> Outcome {
    let key = selected_config_key(matches);
    let value = matches
        .get_one::<String>("value")
        .expect("required by Clap")
        .clone();
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

fn run_config_unset(matches: &ArgMatches, path: Option<&std::path::Path>) -> Outcome {
    let key = selected_config_key(matches);
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
    matches: &ArgMatches,
    stdin: &mut dyn BufRead,
    prompt_writer: &mut dyn std::io::Write,
) -> Outcome {
    let mut config = match Config::load_from_path(read_helpers::config_path(matches)) {
        Ok(config) => config,
        Err(error) => return Outcome::failure(error),
    };
    let token = match last_value(matches, "api-key") {
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
    let base_url = last_value(matches, "base-url")
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| crate::read_helpers::DEFAULT_BASE_URL.to_owned());
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

fn last_value(matches: &ArgMatches, id: &str) -> Option<String> {
    matches
        .get_many::<String>(id)
        .and_then(|mut values| values.next_back().cloned())
        .or_else(|| matches.get_one::<String>(id).cloned())
}

fn completion_script(shell: &str, no_descriptions: bool) -> Outcome {
    let generator = match shell {
        "bash" => Shell::Bash,
        "fish" => Shell::Fish,
        "powershell" => Shell::PowerShell,
        "zsh" => Shell::Zsh,
        _ => return Outcome::failure(format!("unsupported completion shell: {shell}\n")),
    };
    let mut command = command();
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

    use crate::run;

    fn invoke(args: &[&str]) -> super::Outcome {
        let args = args.iter().map(OsString::from).collect::<Vec<_>>();
        run(&args, &mut Cursor::new(Vec::<u8>::new()))
    }

    #[test]
    fn global_flags_are_accepted_before_and_after_commands() {
        assert_eq!(
            invoke(&["--debug", "version"]).stdout,
            format!("{}\n", crate::read_helpers::version()).as_bytes()
        );
        assert_eq!(
            invoke(&["version", "--debug"]).stdout,
            format!("{}\n", crate::read_helpers::version()).as_bytes()
        );
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
