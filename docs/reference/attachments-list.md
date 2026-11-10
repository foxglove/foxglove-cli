<!-- Generated from rust/src/cli.rs by `make docs`. Do not edit directly. -->

# `foxglove attachments list`

[Command reference](README.md) · [Parent command](attachments.md)

```text
List MCAP attachments

Usage: foxglove attachments list [OPTIONS]

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

      --limit <LIMIT>
          Maximum number of items to return (1-2000, default: 50)

      --offset <OFFSET>
          Number of items to skip before returning the results

      --project-id <PROJECT_ID>
          Project ID (defaults to FOXGLOVE_DEFAULT_PROJECT_ID, then saved default_project_id; --project-id= bypasses defaults)

      --recording-id <RECORDING_ID>
          Recording ID

      --site-id <SITE_ID>
          Primary site ID

      --sort-by <SORT_BY>
          Field to sort attachments by: logTime

      --sort-order <SORT_ORDER>
          Sort order for the --sort-by field: asc or desc

  -h, --help
          Print help
```
