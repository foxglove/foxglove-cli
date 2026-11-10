<!-- Generated from rust/src/cli.rs by `make docs`. Do not edit directly. -->

# `foxglove events list`

[Command reference](README.md) · [Parent command](events.md)

```text
List events

Usage: foxglove events list [OPTIONS]

Options:
      --client-id <CLIENT_ID>
          Foxglove client ID

      --format <FORMAT>
          Render output in table, JSON, or CSV format

          [default: table]
          [possible values: table, json, csv]

      --config <CONFIG>
          Config file

      --created-after <CREATED_AFTER>
          Only events created after this time (ISO 8601)

      --debug[=<DEBUG>]
          Print debug diagnostics, including HTTP requests, to stderr

          [default: false]

      --device-id <DEVICE_ID>
          Device ID

      --device-name <DEVICE_NAME>
          Device name

      --end <END>
          Exclude events after this time (ISO 8601)

      --event-id <EVENT_ID>
          Only the event with this ID

      --event-type-id <EVENT_TYPE_ID>
          Event type ID

      --limit <LIMIT>
          Maximum number of items to return (1-2000, default: 50)

      --offset <OFFSET>
          Result offset

      --project-id <PROJECT_ID>
          Project ID (defaults to FOXGLOVE_DEFAULT_PROJECT_ID, then saved default_project_id; --project-id= bypasses defaults)

      --query <QUERY>
          Property or metadata query

      --query-field <QUERY_FIELD>
          Fields to query by

      --sort-by <SORT_BY>
          Sort column

      --sort-order <SORT_ORDER>
          Sort order for the --sort-by field: asc or desc

      --start <START>
          Exclude events before this time (ISO 8601)

      --updated-after <UPDATED_AFTER>
          Only events updated after this time (ISO 8601)

  -h, --help
          Print help
```
