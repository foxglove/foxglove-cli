<!-- Generated from rust/src/cli.rs by `make docs`. Do not edit directly. -->

# `foxglove topics list`

[Command reference](README.md) · [Parent command](topics.md)

```text
List topics

Usage: foxglove topics list [OPTIONS]

Options:
      --client-id <CLIENT_ID>
          Foxglove client ID

      --format <FORMAT>
          Render output in table, JSON, or CSV format

          [default: table]
          [possible values: table, json, csv]

      --config <CONFIG>
          Config file

      --device-id <DEVICE_ID>
          Device ID

      --debug[=<DEBUG>]
          Print debug diagnostics, including HTTP requests, to stderr

          [default: false]

      --device-name <DEVICE_NAME>
          Device name

      --end <END>
          End of topic time range (ISO 8601)

      --episode-id <EPISODE_ID>
          Episode ID

      --episode-version <EPISODE_VERSION>
          Episode version to list topics for; requires --episode-id (default: the newest episode version)

      --include-schemas[=<INCLUDE_SCHEMAS>]
          Include full topic schemas

          [default: false]

      --limit <LIMIT>
          Maximum number of items to return (1-2000, default: 50)

      --offset <OFFSET>
          Number of topics to skip

      --project-id <PROJECT_ID>
          Project ID (defaults to FOXGLOVE_DEFAULT_PROJECT_ID, then saved default_project_id; --project-id= bypasses defaults)

      --recording-id <RECORDING_ID>
          Recording ID

      --recording-key <RECORDING_KEY>
          Recording key

      --session-id <SESSION_ID>
          Session ID

      --session-key <SESSION_KEY>
          Session key

      --sort-by <SORT_BY>
          Sort by topic or version

      --sort-order <SORT_ORDER>
          Sort order: asc or desc

      --start <START>
          Start of topic time range (ISO 8601)

  -h, --help
          Print help
```
