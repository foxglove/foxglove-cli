<!-- Generated from rust/src/cli.rs by `make docs`. Do not edit directly. -->

# `foxglove coverage list`

[Command reference](README.md) · [Parent command](coverage.md)

```text
List coverage ranges

Usage: foxglove coverage list [OPTIONS]

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
          End of coverage time range (ISO 8601)

      --include-edge-recordings[=<INCLUDE_EDGE_RECORDINGS>]
          Include edge recordings

          [default: false]

      --project-id <PROJECT_ID>
          Project ID (defaults to FOXGLOVE_DEFAULT_PROJECT_ID, then saved default_project_id; --project-id= bypasses defaults)

      --recording-id <RECORDING_ID>
          Recording ID

      --session-id <SESSION_ID>
          Session ID

      --session-key <SESSION_KEY>
          Session key

      --start <START>
          Start of coverage time range (ISO 8601)

      --tolerance <TOLERANCE>
          Coverage separation tolerance in seconds

  -h, --help
          Print help
```
