<!-- Generated from rust/src/cli.rs by `make docs`. Do not edit directly. -->

# `foxglove export`

[Command reference](README.md) · [Parent command](README.md)

```text
Export data by recording, session, episode, or device and time range

Usage: foxglove export [OPTIONS]

Options:
      --client-id <CLIENT_ID>
          Foxglove client ID

      --compression <COMPRESSION>
          MCAP chunk compression: zstd, lz4, or "" for none (default: lz4)

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
          End time (ISO 8601); give with --start

      --episode-id <EPISODE_ID>
          Episode ID

      --episode-version <EPISODE_VERSION>
          Episode version to export; requires --episode-id (default: latest)

      --include-attachments[=<INCLUDE_ATTACHMENTS>]
          Include MCAP attachments

          [default: false]

      --key <KEY>
          Recording key

  -o, --output-file <OUTPUT_FILE>
          Output file

      --output-format <OUTPUT_FORMAT>
          Output format: mcap, bag1, or json (default: mcap); json supports only ros1msg and protobuf schemas

      --project-id <PROJECT_ID>
          Project ID (defaults to FOXGLOVE_DEFAULT_PROJECT_ID, then saved default_project_id; --project-id= bypasses defaults)

      --recording-id <RECORDING_ID>
          Recording ID

      --replay-lookback-seconds <REPLAY_LOOKBACK_SECONDS>
          Maximum replay lookback in seconds

      --replay-policy <REPLAY_POLICY>
          Replay policy

      --session-id <SESSION_ID>
          Session ID

      --session-key <SESSION_KEY>
          Session key

      --start <START>
          Start time (ISO 8601); give with --end

      --topics <TOPICS>
          Comma-separated topic list

  -h, --help
          Print help
```
