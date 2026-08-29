//! Production help catalog, intentionally independent from compatibility fixtures.
//!
//! Raw-string delimiters are mechanically generated with extra hashes so
//! arbitrary captured help text remains safe to embed.
#![allow(clippy::needless_raw_string_hashes, clippy::too_many_lines)]

pub struct HelpPage {
    pub stdout: &'static str,
    pub stderr: &'static str,
}

pub fn page(id: &str) -> Option<HelpPage> {
    match id {
        "attachments" => Some(HelpPage {
            stdout: r####"Query and modify data attachments

Usage:
  foxglove attachments [command]

Available Commands:
  download    Download an MCAP attachment by ID
  list        List MCAP attachments

Flags:
  -h, --help   help for attachments

Global Flags:
      --client-id string   foxglove client ID (default "d51173be08ed4cf7a734aed9ac30afd0")
      --config string      config file (default is $HOME/.foxglove.yaml)
      --debug              enable debug logging

Use "foxglove attachments [command] --help" for more information about a command.
"####,
            stderr: r####""####,
        }),
        "attachments-download" => Some(HelpPage {
            stdout: r####"Download an MCAP attachment by ID

Usage:
  foxglove attachments download [flags]

Flags:
  -h, --help   help for download

Global Flags:
      --client-id string   foxglove client ID (default "d51173be08ed4cf7a734aed9ac30afd0")
      --config string      config file (default is $HOME/.foxglove.yaml)
      --debug              enable debug logging
"####,
            stderr: r####""####,
        }),
        "attachments-list" => Some(HelpPage {
            stdout: r####"List MCAP attachments

Usage:
  foxglove attachments list [flags]

Flags:
      --format string         render output in specified format (table, json, csv)
  -h, --help                  help for list
      --import-id string      Import ID
      --json                  alias for --format json
      --project-id string     Project ID (required when using --session-key)
      --recording-id string   Recording ID
      --session-id string     Session ID
      --session-key string    Session key

Global Flags:
      --client-id string   foxglove client ID (default "d51173be08ed4cf7a734aed9ac30afd0")
      --config string      config file (default is $HOME/.foxglove.yaml)
      --debug              enable debug logging
"####,
            stderr: r####""####,
        }),
        "auth" => Some(HelpPage {
            stdout: r####"Manage authentication

Usage:
  foxglove auth [command]

Available Commands:
  configure-api-key Configure an API key
  info              Display information about the currently authenticated user
  login             Log in to Foxglove Data Platform

Flags:
  -h, --help   help for auth

Global Flags:
      --client-id string   foxglove client ID (default "d51173be08ed4cf7a734aed9ac30afd0")
      --config string      config file (default is $HOME/.foxglove.yaml)
      --debug              enable debug logging

Use "foxglove auth [command] --help" for more information about a command.
"####,
            stderr: r####""####,
        }),
        "auth-configure-api-key" => Some(HelpPage {
            stdout: r####"Configure an API key

Usage:
  foxglove auth configure-api-key [flags]

Flags:
      --api-key string    api key (for non-interactive use)
      --base-url string   API server (default "https://api.foxglove.dev")
  -h, --help              help for configure-api-key

Global Flags:
      --client-id string   foxglove client ID (default "d51173be08ed4cf7a734aed9ac30afd0")
      --config string      config file (default is $HOME/.foxglove.yaml)
      --debug              enable debug logging
"####,
            stderr: r####""####,
        }),
        "auth-info" => Some(HelpPage {
            stdout: r####"Display information about the currently authenticated user

Usage:
  foxglove auth info [flags]

Flags:
  -h, --help   help for info

Global Flags:
      --client-id string   foxglove client ID (default "d51173be08ed4cf7a734aed9ac30afd0")
      --config string      config file (default is $HOME/.foxglove.yaml)
      --debug              enable debug logging
"####,
            stderr: r####""####,
        }),
        "auth-login" => Some(HelpPage {
            stdout: r####"Log in to Foxglove Data Platform

Usage:
  foxglove auth login [flags]

Flags:
      --base-url string   API server (default "https://api.foxglove.dev")
  -h, --help              help for login

Global Flags:
      --client-id string   foxglove client ID (default "d51173be08ed4cf7a734aed9ac30afd0")
      --config string      config file (default is $HOME/.foxglove.yaml)
      --debug              enable debug logging
"####,
            stderr: r####""####,
        }),
        "completion" => Some(HelpPage {
            stdout: r####"Generate the autocompletion script for foxglove for the specified shell.
See each sub-command's help for details on how to use the generated script.

Usage:
  foxglove completion [command]

Available Commands:
  bash        Generate the autocompletion script for bash
  fish        Generate the autocompletion script for fish
  powershell  Generate the autocompletion script for powershell
  zsh         Generate the autocompletion script for zsh

Flags:
  -h, --help   help for completion

Global Flags:
      --client-id string   foxglove client ID (default "d51173be08ed4cf7a734aed9ac30afd0")
      --config string      config file (default is $HOME/.foxglove.yaml)
      --debug              enable debug logging

Use "foxglove completion [command] --help" for more information about a command.
"####,
            stderr: r####""####,
        }),
        "completion-bash" => Some(HelpPage {
            stdout: r####"Generate the autocompletion script for the bash shell.

This script depends on the 'bash-completion' package.
If it is not installed already, you can install it via your OS's package manager.

To load completions in your current shell session:

	source <(foxglove completion bash)

To load completions for every new session, execute once:

#### Linux:

	foxglove completion bash > /etc/bash_completion.d/foxglove

#### macOS:

	foxglove completion bash > /usr/local/etc/bash_completion.d/foxglove

You will need to start a new shell for this setup to take effect.

Usage:
  foxglove completion bash

Flags:
  -h, --help              help for bash
      --no-descriptions   disable completion descriptions

Global Flags:
      --client-id string   foxglove client ID (default "d51173be08ed4cf7a734aed9ac30afd0")
      --config string      config file (default is $HOME/.foxglove.yaml)
      --debug              enable debug logging
"####,
            stderr: r####""####,
        }),
        "completion-fish" => Some(HelpPage {
            stdout: r####"Generate the autocompletion script for the fish shell.

To load completions in your current shell session:

	foxglove completion fish | source

To load completions for every new session, execute once:

	foxglove completion fish > ~/.config/fish/completions/foxglove.fish

You will need to start a new shell for this setup to take effect.

Usage:
  foxglove completion fish [flags]

Flags:
  -h, --help              help for fish
      --no-descriptions   disable completion descriptions

Global Flags:
      --client-id string   foxglove client ID (default "d51173be08ed4cf7a734aed9ac30afd0")
      --config string      config file (default is $HOME/.foxglove.yaml)
      --debug              enable debug logging
"####,
            stderr: r####""####,
        }),
        "completion-powershell" => Some(HelpPage {
            stdout: r####"Generate the autocompletion script for powershell.

To load completions in your current shell session:

	foxglove completion powershell | Out-String | Invoke-Expression

To load completions for every new session, add the output of the above command
to your powershell profile.

Usage:
  foxglove completion powershell [flags]

Flags:
  -h, --help              help for powershell
      --no-descriptions   disable completion descriptions

Global Flags:
      --client-id string   foxglove client ID (default "d51173be08ed4cf7a734aed9ac30afd0")
      --config string      config file (default is $HOME/.foxglove.yaml)
      --debug              enable debug logging
"####,
            stderr: r####""####,
        }),
        "completion-zsh" => Some(HelpPage {
            stdout: r####"Generate the autocompletion script for the zsh shell.

If shell completion is not already enabled in your environment you will need
to enable it.  You can execute the following once:

	echo "autoload -U compinit; compinit" >> ~/.zshrc

To load completions for every new session, execute once:

#### Linux:

	foxglove completion zsh > "${fpath[1]}/_foxglove"

#### macOS:

	foxglove completion zsh > /usr/local/share/zsh/site-functions/_foxglove

You will need to start a new shell for this setup to take effect.

Usage:
  foxglove completion zsh [flags]

Flags:
  -h, --help              help for zsh
      --no-descriptions   disable completion descriptions

Global Flags:
      --client-id string   foxglove client ID (default "d51173be08ed4cf7a734aed9ac30afd0")
      --config string      config file (default is $HOME/.foxglove.yaml)
      --debug              enable debug logging
"####,
            stderr: r####""####,
        }),
        "config" => Some(HelpPage {
            stdout: r####"Manage CLI configuration values.
Available configuration keys:
  - project-id: Default project ID for commands

Usage:
  foxglove config [command]

Available Commands:
  get         Get a configuration value
  set         Set a configuration value
  unset       Remove a configuration value

Flags:
  -h, --help   help for config

Global Flags:
      --client-id string   foxglove client ID (default "d51173be08ed4cf7a734aed9ac30afd0")
      --config string      config file (default is $HOME/.foxglove.yaml)
      --debug              enable debug logging

Use "foxglove config [command] --help" for more information about a command.
"####,
            stderr: r####""####,
        }),
        "config-get" => Some(HelpPage {
            stdout: r####"Get a configuration value

Usage:
  foxglove config get [KEY] [flags]

Flags:
  -h, --help   help for get

Global Flags:
      --client-id string   foxglove client ID (default "d51173be08ed4cf7a734aed9ac30afd0")
      --config string      config file (default is $HOME/.foxglove.yaml)
      --debug              enable debug logging
"####,
            stderr: r####""####,
        }),
        "config-set" => Some(HelpPage {
            stdout: r####"Set a configuration value

Usage:
  foxglove config set [KEY] [VALUE] [flags]

Flags:
  -h, --help   help for set

Global Flags:
      --client-id string   foxglove client ID (default "d51173be08ed4cf7a734aed9ac30afd0")
      --config string      config file (default is $HOME/.foxglove.yaml)
      --debug              enable debug logging
"####,
            stderr: r####""####,
        }),
        "config-unset" => Some(HelpPage {
            stdout: r####"Remove a configuration value

Usage:
  foxglove config unset [KEY] [flags]

Flags:
  -h, --help   help for unset

Global Flags:
      --client-id string   foxglove client ID (default "d51173be08ed4cf7a734aed9ac30afd0")
      --config string      config file (default is $HOME/.foxglove.yaml)
      --debug              enable debug logging
"####,
            stderr: r####""####,
        }),
        "data" => Some(HelpPage {
            stdout: r####"Data access and management

Usage:
  foxglove data [command]

Available Commands:
  coverage    List coverage ranges
  export      Export a data selection from Foxglove Data Platform
  import      Import a data file to Foxglove Data Platform

Flags:
  -h, --help   help for data

Global Flags:
      --client-id string   foxglove client ID (default "d51173be08ed4cf7a734aed9ac30afd0")
      --config string      config file (default is $HOME/.foxglove.yaml)
      --debug              enable debug logging

Use "foxglove data [command] --help" for more information about a command.
"####,
            stderr: r####""####,
        }),
        "data-coverage" => Some(HelpPage {
            stdout: r####"List coverage ranges

Usage:
  foxglove data coverage [command]

Available Commands:
  list        List coverage ranges

Flags:
  -h, --help   help for coverage

Global Flags:
      --client-id string   foxglove client ID (default "d51173be08ed4cf7a734aed9ac30afd0")
      --config string      config file (default is $HOME/.foxglove.yaml)
      --debug              enable debug logging

Use "foxglove data coverage [command] --help" for more information about a command.
"####,
            stderr: r####""####,
        }),
        "data-coverage-list" => Some(HelpPage {
            stdout: r####"List coverage ranges

Usage:
  foxglove data coverage list [flags]

Flags:
      --device-id string          Device ID
      --device-name string        Device name
      --end string                end of coverage time range (ISO8601)
      --format string             render output in specified format (table, json, csv)
  -h, --help                      help for list
      --include-edge-recordings   Include edge recordings
      --json                      alias for --format json
      --project-id string         Project ID (required when using --session-key)
      --recording-id string       Recording ID
      --session-id string         Session ID
      --session-key string        Session key
      --start string              start of coverage time range (ISO8601)
      --tolerance int             Number of seconds by which ranges must be separated to be considered distinct

Global Flags:
      --client-id string   foxglove client ID (default "d51173be08ed4cf7a734aed9ac30afd0")
      --config string      config file (default is $HOME/.foxglove.yaml)
      --debug              enable debug logging
"####,
            stderr: r####""####,
        }),
        "data-export" => Some(HelpPage {
            stdout: r####"Export a data selection from Foxglove Data Platform by Recording ID, Import ID, Session ID/Key, or Device and time range

Usage:
  foxglove data export [flags]

Flags:
      --compression string              mcap chunk compression format: "" (none), zstd, or lz4 (mcap output only) (default "lz4")
      --device-id string                device ID
      --device-name string              device name
      --end string                      end time (ISO8601 timestamp
  -h, --help                            help for export
      --import-id string                import ID
      --include-attachments             include attachments in streamed data (mcap output only)
      --json                            alias for --output-format json
      --key string                      recording key
  -o, --output-file string              output file
      --output-format string            output format (mcap0, bag1, or json) (default "mcap0")
      --project-id string               Project ID (required when using --session-key)
      --recording-id string             recording ID
      --replay-lookback-seconds float   max seconds to look back before start for lastPerChannel replay policy
      --replay-policy string            replay policy: "" (none) or lastPerChannel
      --session-id string               session ID
      --session-key string              Session key
      --start string                    start time (ISO8601 timestamp)
      --topics string                   comma separated list of topics

Global Flags:
      --client-id string   foxglove client ID (default "d51173be08ed4cf7a734aed9ac30afd0")
      --config string      config file (default is $HOME/.foxglove.yaml)
      --debug              enable debug logging
"####,
            stderr: r####""####,
        }),
        "data-import" => Some(HelpPage {
            stdout: r####"Import a data file to Foxglove Data Platform

Usage:
  foxglove data import [FILE] [flags]

Flags:
      --device-id string           Device ID
      --device-name string         Device name
      --edge-recording-id string   Edge recording ID
  -h, --help                       help for import
      --key string                 Recording key
      --project-id string          Project ID (required when using --session-key)
      --session-id string          Session ID
      --session-key string         Session key

Global Flags:
      --client-id string   foxglove client ID (default "d51173be08ed4cf7a734aed9ac30afd0")
      --config string      config file (default is $HOME/.foxglove.yaml)
      --debug              enable debug logging
"####,
            stderr: r####""####,
        }),
        "data-imports" => Some(HelpPage {
            stdout: r####"Query and modify data imports

Usage:

Flags:
  -h, --help   help for imports

Global Flags:
      --client-id string   foxglove client ID (default "d51173be08ed4cf7a734aed9ac30afd0")
      --config string      config file (default is $HOME/.foxglove.yaml)
      --debug              enable debug logging
"####,
            stderr: r####"Command "imports" is deprecated, use 'recordings list' to list, and 'data import' to import data.
"####,
        }),
        "data-imports-add" => Some(HelpPage {
            stdout: r####"Import a data file to Foxglove Data Platform

Usage:
  foxglove data imports add [FILE] [flags]

Flags:
      --device-id string           Device ID
      --device-name string         Device name
      --edge-recording-id string   Edge recording ID
  -h, --help                       help for add
      --key string                 Recording key
      --project-id string          Project ID (required when using --session-key)
      --session-id string          Session ID
      --session-key string         Session key

Global Flags:
      --client-id string   foxglove client ID (default "d51173be08ed4cf7a734aed9ac30afd0")
      --config string      config file (default is $HOME/.foxglove.yaml)
      --debug              enable debug logging
"####,
            stderr: r####"Command "add" is deprecated, use 'data import' instead.
"####,
        }),
        "data-imports-list" => Some(HelpPage {
            stdout: r####"List imports for a device

Usage:
  foxglove data imports list [flags]

Flags:
      --data-end string     end of data time range (ISO8601)
      --data-start string   start of data time range (ISO8601)
      --device-id string    Device ID
      --end string          end of import time range (ISO8601)
      --format string       render output in specified format (table, json, csv)
  -h, --help                help for list
      --include-deleted     end of data time range
      --json                alias for --format json
      --start string        start of import time range (ISO8601)

Global Flags:
      --client-id string   foxglove client ID (default "d51173be08ed4cf7a734aed9ac30afd0")
      --config string      config file (default is $HOME/.foxglove.yaml)
      --debug              enable debug logging
"####,
            stderr: r####"Command "list" is deprecated, use 'recordings list' instead.
"####,
        }),
        "devices" => Some(HelpPage {
            stdout: r####"List and manage devices

Usage:
  foxglove devices [command]

Available Commands:
  add         Add a device for your organization
  edit        Edit a device
  list        List devices registered to your organization

Flags:
  -h, --help   help for devices

Global Flags:
      --client-id string   foxglove client ID (default "d51173be08ed4cf7a734aed9ac30afd0")
      --config string      config file (default is $HOME/.foxglove.yaml)
      --debug              enable debug logging

Use "foxglove devices [command] --help" for more information about a command.
"####,
            stderr: r####""####,
        }),
        "devices-add" => Some(HelpPage {
            stdout: r####"Add a device for your organization

Usage:
  foxglove devices add [flags]

Flags:
  -h, --help                   help for add
      --name string            name of the device
      --project-id string      Project ID
  -p, --property stringArray   Custom property colon-separated key value pair. Multiple may be specified.
      --serial-number string   Deprecated. Value will be ignored.

Global Flags:
      --client-id string   foxglove client ID (default "d51173be08ed4cf7a734aed9ac30afd0")
      --config string      config file (default is $HOME/.foxglove.yaml)
      --debug              enable debug logging
"####,
            stderr: r####""####,
        }),
        "devices-edit" => Some(HelpPage {
            stdout: r####"Edit a device

Usage:
  foxglove devices edit [flags]

Flags:
  -h, --help                   help for edit
      --name string            New name for the device
      --project-id string      Project ID
  -p, --property stringArray   Custom property colon-separated key value pair. Multiple may be specified.

Global Flags:
      --client-id string   foxglove client ID (default "d51173be08ed4cf7a734aed9ac30afd0")
      --config string      config file (default is $HOME/.foxglove.yaml)
      --debug              enable debug logging
"####,
            stderr: r####""####,
        }),
        "devices-list" => Some(HelpPage {
            stdout: r####"List devices registered to your organization

Usage:
  foxglove devices list [flags]

Flags:
      --format string       render output in specified format (table, json, csv)
  -h, --help                help for list
      --json                alias for --format json
      --project-id string   Project ID

Global Flags:
      --client-id string   foxglove client ID (default "d51173be08ed4cf7a734aed9ac30afd0")
      --config string      config file (default is $HOME/.foxglove.yaml)
      --debug              enable debug logging
"####,
            stderr: r####""####,
        }),
        "event-types" => Some(HelpPage {
            stdout: r####"List event types

Usage:
  foxglove event-types [command]

Available Commands:
  list        List event types

Flags:
  -h, --help   help for event-types

Global Flags:
      --client-id string   foxglove client ID (default "d51173be08ed4cf7a734aed9ac30afd0")
      --config string      config file (default is $HOME/.foxglove.yaml)
      --debug              enable debug logging

Use "foxglove event-types [command] --help" for more information about a command.
"####,
            stderr: r####""####,
        }),
        "event-types-list" => Some(HelpPage {
            stdout: r####"List event types

Usage:
  foxglove event-types list [flags]

Flags:
      --format string   render output in specified format (table, json, csv)
  -h, --help            help for list
      --json            alias for --format json

Global Flags:
      --client-id string   foxglove client ID (default "d51173be08ed4cf7a734aed9ac30afd0")
      --config string      config file (default is $HOME/.foxglove.yaml)
      --debug              enable debug logging
"####,
            stderr: r####""####,
        }),
        "events" => Some(HelpPage {
            stdout: r####"List and manage events

Usage:
  foxglove events [command]

Available Commands:
  add         Add an event
  list        List events

Flags:
  -h, --help   help for events

Global Flags:
      --client-id string   foxglove client ID (default "d51173be08ed4cf7a734aed9ac30afd0")
      --config string      config file (default is $HOME/.foxglove.yaml)
      --debug              enable debug logging

Use "foxglove events [command] --help" for more information about a command.
"####,
            stderr: r####""####,
        }),
        "events-add" => Some(HelpPage {
            stdout: r####"Add an event

Usage:
  foxglove events add [flags]

Flags:
      --device-id string       Device ID
      --end string             End of event (inclusive), RFC 3339 date-time format
      --event-type-id string   Event type ID to associate with this event (e.g. evtt_123)
  -h, --help                   help for add
  -m, --metadata stringArray   Metadata colon-separated key value pair. Multiple may be specified.
      --start string           Start of event, RFC 3339 date-time format

Global Flags:
      --client-id string   foxglove client ID (default "d51173be08ed4cf7a734aed9ac30afd0")
      --config string      config file (default is $HOME/.foxglove.yaml)
      --debug              enable debug logging
"####,
            stderr: r####""####,
        }),
        "events-list" => Some(HelpPage {
            stdout: r####"List events

Usage:
  foxglove events list [flags]

Flags:
      --device-id string          Device ID
      --device-name string        Device name
      --end string                Exclude events after this time, RFC 3339 or ISO 8601 format
      --event-type-id string      Filter by event type ID (e.g. evtt_123)
      --format string             render output in specified format (table, json, csv)
  -h, --help                      help for list
      --json                      alias for --format json
      --limit int                 limit (default 100)
      --offset int                offset
      --query string              Filter by properties or metadata, e.g. "$key:$value". See API docs for query syntax.
      --query-field stringArray   Fields to query by ("metadata" or "properties"). Multiple may be specified. Defaults to "metadata".
      --sort-by string            name of sort column
      --sort-order string         sort order (default "asc")
      --start string              Exclude events before this time, RFC 3339 or ISO 8601 format

Global Flags:
      --client-id string   foxglove client ID (default "d51173be08ed4cf7a734aed9ac30afd0")
      --config string      config file (default is $HOME/.foxglove.yaml)
      --debug              enable debug logging
"####,
            stderr: r####""####,
        }),
        "extensions" => Some(HelpPage {
            stdout: r####"List and publish Studio extensions

Usage:
  foxglove extensions [command]

Available Commands:
  list        List Studio extensions created for your organization
  publish     Publish a Studio extension (.foxe) to your organization
  unpublish   Delete and unpublish a Studio extension from your organization

Flags:
  -h, --help   help for extensions

Global Flags:
      --client-id string   foxglove client ID (default "d51173be08ed4cf7a734aed9ac30afd0")
      --config string      config file (default is $HOME/.foxglove.yaml)
      --debug              enable debug logging

Use "foxglove extensions [command] --help" for more information about a command.
"####,
            stderr: r####""####,
        }),
        "extensions-list" => Some(HelpPage {
            stdout: r####"List Studio extensions created for your organization

Usage:
  foxglove extensions list [flags]

Flags:
      --format string   render output in specified format (table, json, csv)
  -h, --help            help for list
      --json            alias for --format json

Global Flags:
      --client-id string   foxglove client ID (default "d51173be08ed4cf7a734aed9ac30afd0")
      --config string      config file (default is $HOME/.foxglove.yaml)
      --debug              enable debug logging
"####,
            stderr: r####""####,
        }),
        "extensions-publish" => Some(HelpPage {
            stdout: r####"Publish a Studio extension (.foxe) to your organization

Usage:
  foxglove extensions publish [FILE] [flags]

Flags:
  -h, --help   help for publish

Global Flags:
      --client-id string   foxglove client ID (default "d51173be08ed4cf7a734aed9ac30afd0")
      --config string      config file (default is $HOME/.foxglove.yaml)
      --debug              enable debug logging
"####,
            stderr: r####""####,
        }),
        "extensions-unpublish" => Some(HelpPage {
            stdout: r####"Delete and unpublish a Studio extension from your organization

Usage:
  foxglove extensions unpublish [ID] [flags]

Flags:
  -h, --help   help for unpublish

Global Flags:
      --client-id string   foxglove client ID (default "d51173be08ed4cf7a734aed9ac30afd0")
      --config string      config file (default is $HOME/.foxglove.yaml)
      --debug              enable debug logging
"####,
            stderr: r####""####,
        }),
        "help" => Some(HelpPage {
            stdout: r####"Help provides help for any command in the application.
Simply type foxglove help [path to command] for full details.

Usage:
  foxglove help [command] [flags]

Flags:
  -h, --help   help for help

Global Flags:
      --client-id string   foxglove client ID (default "d51173be08ed4cf7a734aed9ac30afd0")
      --config string      config file (default is $HOME/.foxglove.yaml)
      --debug              enable debug logging
"####,
            stderr: r####""####,
        }),
        "pending-imports" => Some(HelpPage {
            stdout: r####"List pending imports

Usage:
  foxglove pending-imports [command]

Available Commands:
  list        List the pending and errored import jobs for uploaded recordings

Flags:
  -h, --help   help for pending-imports

Global Flags:
      --client-id string   foxglove client ID (default "d51173be08ed4cf7a734aed9ac30afd0")
      --config string      config file (default is $HOME/.foxglove.yaml)
      --debug              enable debug logging

Use "foxglove pending-imports [command] --help" for more information about a command.
"####,
            stderr: r####""####,
        }),
        "pending-imports-list" => Some(HelpPage {
            stdout: r####"List the pending and errored import jobs for uploaded recordings

Usage:
  foxglove pending-imports list [flags]

Flags:
      --device-id string       Device ID
      --device-name string     Device name
      --error string           Filter based on error messages
      --filename string        Filename
      --format string          render output in specified format (table, json, csv)
  -h, --help                   help for list
      --json                   alias for --format json
      --key string             Key
      --project-id string      Project ID (required when using --session-key)
      --request-id string      Request ID
      --session-id string      Session ID
      --session-key string     Session key
      --show-completed         Show completed requests
      --show-quarantined       Show quarantined requests
      --site-id string         Site ID
      --updated-since string   Filter pending imports updated since this time (ISO8601)
      --without-project        Filter to pending imports without a project

Global Flags:
      --client-id string   foxglove client ID (default "d51173be08ed4cf7a734aed9ac30afd0")
      --config string      config file (default is $HOME/.foxglove.yaml)
      --debug              enable debug logging
"####,
            stderr: r####""####,
        }),
        "projects" => Some(HelpPage {
            stdout: r####"List and manage projects

Usage:
  foxglove projects [command]

Available Commands:
  list        List projects

Flags:
  -h, --help   help for projects

Global Flags:
      --client-id string   foxglove client ID (default "d51173be08ed4cf7a734aed9ac30afd0")
      --config string      config file (default is $HOME/.foxglove.yaml)
      --debug              enable debug logging

Use "foxglove projects [command] --help" for more information about a command.
"####,
            stderr: r####""####,
        }),
        "projects-list" => Some(HelpPage {
            stdout: r####"List projects

Usage:
  foxglove projects list [flags]

Flags:
      --format string   render output in specified format (table, json, csv)
  -h, --help            help for list
      --json            alias for --format json

Global Flags:
      --client-id string   foxglove client ID (default "d51173be08ed4cf7a734aed9ac30afd0")
      --config string      config file (default is $HOME/.foxglove.yaml)
      --debug              enable debug logging
"####,
            stderr: r####""####,
        }),
        "recordings" => Some(HelpPage {
            stdout: r####"Query recordings

Usage:
  foxglove recordings [command]

Available Commands:
  delete      Delete a recording from your organization
  list        List recordings

Flags:
  -h, --help   help for recordings

Global Flags:
      --client-id string   foxglove client ID (default "d51173be08ed4cf7a734aed9ac30afd0")
      --config string      config file (default is $HOME/.foxglove.yaml)
      --debug              enable debug logging

Use "foxglove recordings [command] --help" for more information about a command.
"####,
            stderr: r####""####,
        }),
        "recordings-delete" => Some(HelpPage {
            stdout: r####"Delete a recording from your organization

Usage:
  foxglove recordings delete [ID] [flags]

Flags:
  -h, --help   help for delete

Global Flags:
      --client-id string   foxglove client ID (default "d51173be08ed4cf7a734aed9ac30afd0")
      --config string      config file (default is $HOME/.foxglove.yaml)
      --debug              enable debug logging
"####,
            stderr: r####""####,
        }),
        "recordings-list" => Some(HelpPage {
            stdout: r####"List recordings

Usage:
  foxglove recordings list [flags]

Flags:
      --device-id string       Device ID
      --device-name string     Device name
      --edge-site-id string    Edge site ID
      --end string             End of data range (ISO8601 format)
      --format string          render output in specified format (table, json, csv)
  -h, --help                   help for list
      --import-status string   Import status
      --json                   alias for --format json
      --limit int              Max number of recordings to return (default 2000)
      --offset int             Number of recordings to skip
      --path string            Recording file path
      --project-id string      Project ID (required when using --session-key)
      --session-id string      Session ID
      --session-key string     Session key
      --site-id string         Primary site ID
      --sort-by string         Sort recordings by a field
      --sort-order string      Sort order: 'asc' 'desc'
      --start string           Start of data range (ISO8601 format)

Global Flags:
      --client-id string   foxglove client ID (default "d51173be08ed4cf7a734aed9ac30afd0")
      --config string      config file (default is $HOME/.foxglove.yaml)
      --debug              enable debug logging
"####,
            stderr: r####""####,
        }),
        "root" => Some(HelpPage {
            stdout: r####"Command line client for the Foxglove data platform

Usage:
  foxglove [command]

Available Commands:
  attachments     Query and modify data attachments
  auth            Manage authentication
  completion      Generate the autocompletion script for the specified shell
  config          Manage CLI configuration
  data            Data access and management
  devices         List and manage devices
  event-types     List event types
  events          List and manage events
  extensions      List and publish Studio extensions
  help            Help about any command
  pending-imports List pending imports
  projects        List and manage projects
  recordings      Query recordings
  sessions        List and manage sessions
  topics          List topics
  version         print Foxglove CLI version

Flags:
      --client-id string   foxglove client ID (default "d51173be08ed4cf7a734aed9ac30afd0")
      --config string      config file (default is $HOME/.foxglove.yaml)
      --debug              enable debug logging
  -h, --help               help for foxglove

Use "foxglove [command] --help" for more information about a command.
"####,
            stderr: r####""####,
        }),
        "sessions" => Some(HelpPage {
            stdout: r####"List and manage sessions

Usage:
  foxglove sessions [command]

Available Commands:
  add         Create a session
  delete      Delete a session
  get         Get a session by ID or key
  list        List sessions in your organization
  recordings  List, add, or remove recordings in a session

Flags:
  -h, --help   help for sessions

Global Flags:
      --client-id string   foxglove client ID (default "d51173be08ed4cf7a734aed9ac30afd0")
      --config string      config file (default is $HOME/.foxglove.yaml)
      --debug              enable debug logging

Use "foxglove sessions [command] --help" for more information about a command.
"####,
            stderr: r####""####,
        }),
        "sessions-add" => Some(HelpPage {
            stdout: r####"Create a session

Usage:
  foxglove sessions add [flags]

Flags:
      --device-id string    Device ID (required)
  -h, --help                help for add
      --name string         Name of the session
      --project-id string   Project ID

Global Flags:
      --client-id string   foxglove client ID (default "d51173be08ed4cf7a734aed9ac30afd0")
      --config string      config file (default is $HOME/.foxglove.yaml)
      --debug              enable debug logging
"####,
            stderr: r####""####,
        }),
        "sessions-delete" => Some(HelpPage {
            stdout: r####"Delete a session

Usage:
  foxglove sessions delete [session-id-or-key] [flags]

Flags:
  -h, --help                help for delete
      --project-id string   Project ID (required when session-id-or-key is a session key)

Global Flags:
      --client-id string   foxglove client ID (default "d51173be08ed4cf7a734aed9ac30afd0")
      --config string      config file (default is $HOME/.foxglove.yaml)
      --debug              enable debug logging
"####,
            stderr: r####""####,
        }),
        "sessions-get" => Some(HelpPage {
            stdout: r####"Get a session by ID or key

Usage:
  foxglove sessions get [session-id-or-key] [flags]

Flags:
  -h, --help                help for get
      --project-id string   Project ID (required when session-id-or-key is a session key)

Global Flags:
      --client-id string   foxglove client ID (default "d51173be08ed4cf7a734aed9ac30afd0")
      --config string      config file (default is $HOME/.foxglove.yaml)
      --debug              enable debug logging
"####,
            stderr: r####""####,
        }),
        "sessions-list" => Some(HelpPage {
            stdout: r####"List sessions in your organization

Usage:
  foxglove sessions list [flags]

Flags:
      --device-id string     Filter by device ID
      --device-name string   Filter by device name
      --format string        render output in specified format (table, json, csv)
  -h, --help                 help for list
      --json                 alias for --format json
      --project-id string    Project ID (optional filter)

Global Flags:
      --client-id string   foxglove client ID (default "d51173be08ed4cf7a734aed9ac30afd0")
      --config string      config file (default is $HOME/.foxglove.yaml)
      --debug              enable debug logging
"####,
            stderr: r####""####,
        }),
        "sessions-recordings" => Some(HelpPage {
            stdout: r####"List, add, or remove recordings in a session

Usage:
  foxglove sessions recordings [command]

Available Commands:
  add         Assign a recording to a session
  list        List recording IDs in a session
  remove      Remove a recording from a session

Flags:
  -h, --help   help for recordings

Global Flags:
      --client-id string   foxglove client ID (default "d51173be08ed4cf7a734aed9ac30afd0")
      --config string      config file (default is $HOME/.foxglove.yaml)
      --debug              enable debug logging

Use "foxglove sessions recordings [command] --help" for more information about a command.
"####,
            stderr: r####""####,
        }),
        "sessions-recordings-add" => Some(HelpPage {
            stdout: r####"Assign a recording to a session

Usage:
  foxglove sessions recordings add [session-id-or-key] [recording-id] [flags]

Flags:
  -h, --help                help for add
      --project-id string   Project ID (required when session-id-or-key is a session key)

Global Flags:
      --client-id string   foxglove client ID (default "d51173be08ed4cf7a734aed9ac30afd0")
      --config string      config file (default is $HOME/.foxglove.yaml)
      --debug              enable debug logging
"####,
            stderr: r####""####,
        }),
        "sessions-recordings-list" => Some(HelpPage {
            stdout: r####"List recording IDs in a session

Usage:
  foxglove sessions recordings list [session-id-or-key] [flags]

Flags:
  -h, --help                help for list
      --project-id string   Project ID (required when session-id-or-key is a session key)

Global Flags:
      --client-id string   foxglove client ID (default "d51173be08ed4cf7a734aed9ac30afd0")
      --config string      config file (default is $HOME/.foxglove.yaml)
      --debug              enable debug logging
"####,
            stderr: r####""####,
        }),
        "sessions-recordings-remove" => Some(HelpPage {
            stdout: r####"Remove a recording from a session

Usage:
  foxglove sessions recordings remove [session-id-or-key] [recording-id] [flags]

Flags:
  -h, --help                help for remove
      --project-id string   Project ID (required when session-id-or-key is a session key)

Global Flags:
      --client-id string   foxglove client ID (default "d51173be08ed4cf7a734aed9ac30afd0")
      --config string      config file (default is $HOME/.foxglove.yaml)
      --debug              enable debug logging
"####,
            stderr: r####""####,
        }),
        "topics" => Some(HelpPage {
            stdout: r####"List topics

Usage:
  foxglove topics [command]

Available Commands:
  list        List topics

Flags:
  -h, --help   help for topics

Global Flags:
      --client-id string   foxglove client ID (default "d51173be08ed4cf7a734aed9ac30afd0")
      --config string      config file (default is $HOME/.foxglove.yaml)
      --debug              enable debug logging

Use "foxglove topics [command] --help" for more information about a command.
"####,
            stderr: r####""####,
        }),
        "topics-list" => Some(HelpPage {
            stdout: r####"List topics.

One of --device-id, --device-name, --recording-id, --recording-key, --session-id, or --session-key is required.

Usage:
  foxglove topics list [flags]

Flags:
      --device-id string       Device ID
      --device-name string     Device name
      --end string             End of topic time range (ISO8601)
      --format string          render output in specified format (table, json, csv)
  -h, --help                   help for list
      --include-schemas        Include full topic schemas
      --json                   alias for --format json
      --limit int              Maximum number of topics to return
      --offset int             Number of topics to skip
      --project-id string      Project ID (required when using --session-key)
      --recording-id string    Recording ID
      --recording-key string   Recording key
      --session-id string      Session ID
      --session-key string     Session key
      --sort-by string         Sort by topic or version
      --sort-order string      Sort order (asc or desc)
      --start string           Start of topic time range (ISO8601)

Global Flags:
      --client-id string   foxglove client ID (default "d51173be08ed4cf7a734aed9ac30afd0")
      --config string      config file (default is $HOME/.foxglove.yaml)
      --debug              enable debug logging
"####,
            stderr: r####""####,
        }),
        "version" => Some(HelpPage {
            stdout: r####"print Foxglove CLI version

Usage:
  foxglove version [flags]

Flags:
  -h, --help   help for version

Global Flags:
      --client-id string   foxglove client ID (default "d51173be08ed4cf7a734aed9ac30afd0")
      --config string      config file (default is $HOME/.foxglove.yaml)
      --debug              enable debug logging
"####,
            stderr: r####""####,
        }),
        _ => None,
    }
}
