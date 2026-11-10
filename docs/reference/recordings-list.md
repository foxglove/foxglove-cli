<!-- Generated from rust/src/cli.rs by `make docs`. Do not edit directly. -->

# `foxglove recordings list`

[Command reference](README.md) · [Parent command](recordings.md)

```text
List recordings

Usage: foxglove recordings list [OPTIONS]

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

      --edge-site-id <EDGE_SITE_ID>
          Edge site ID

      --end <END>
          End of data range (ISO 8601)

      --import-status <IMPORT_STATUS>
          Import status

      --limit <LIMIT>
          Maximum number of items to return (1-2000, default: 50)

      --metadata-query <METADATA_QUERY>
          Only recordings whose metadata matches this query, e.g. key:value

      --offset <OFFSET>
          Number of recordings to skip

      --path <PATH>
          Recording file path

      --project-id <PROJECT_ID>
          Project ID (defaults to FOXGLOVE_DEFAULT_PROJECT_ID, then saved default_project_id; --project-id= bypasses defaults)

      --session-id <SESSION_ID>
          Session ID

      --session-key <SESSION_KEY>
          Session key

      --site-id <SITE_ID>
          Primary site ID

      --sort-by <SORT_BY>
          Sort field

      --sort-order <SORT_ORDER>
          Sort order: asc or desc

      --start <START>
          Start of data range (ISO 8601)

      --topic <TOPIC>
          Only recordings containing this topic

      --without-device[=<WITHOUT_DEVICE>]
          Only recordings without a device

          [default: false]

      --without-session[=<WITHOUT_SESSION>]
          Only recordings without a session

          [default: false]

  -h, --help
          Print help
```
