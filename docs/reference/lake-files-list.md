<!-- Generated from rust/src/cli.rs by `make docs`. Do not edit directly. -->

# `foxglove lake-files list`

[Command reference](README.md) · [Parent command](lake-files.md)

```text
List the MCAP files that imports wrote to a self-managed Primary Site's lake bucket. Each imported recording produces one file per topic. Limit the query with --device-id, --device-name, --recording-id, or --recording-key. A device query also needs --session-id, --session-key, or --start and --end at most 24 hours apart. Only self-managed sites are supported.

Usage: foxglove lake-files list [OPTIONS] --site-id <SITE_ID> <--device-id <DEVICE_ID>|--device-name <DEVICE_NAME>|--recording-id <RECORDING_ID>|--recording-key <RECORDING_KEY>>

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
          End of an imported recording's time range (ISO 8601); give with --start (required for a device query without a recording or session)

      --recording-id <RECORDING_ID>
          Recording ID

      --recording-key <RECORDING_KEY>
          Recording key

      --session-id <SESSION_ID>
          Only files for recordings in the session with this ID

      --session-key <SESSION_KEY>
          Only files for recordings in the session with this key

      --site-id <SITE_ID>
          Self-managed Primary Site ID (required)

      --start <START>
          Start of an imported recording's time range (ISO 8601); give with --end (required for a device query without a recording or session)

      --topic <TOPIC>
          Only files for this topic

  -h, --help
          Print help (see a summary with '-h')
```
