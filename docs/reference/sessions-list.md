<!-- Generated from rust/src/cli.rs by `make docs`. Do not edit directly. -->

# `foxglove sessions list`

[Command reference](README.md) · [Parent command](sessions.md)

```text
List sessions in your organization

Usage: foxglove sessions list [OPTIONS]

Options:
      --client-id <CLIENT_ID>
          Foxglove client ID

      --format <FORMAT>
          Render output in table, JSON, or CSV format

          [default: table]
          [possible values: table, json, csv]

      --config <CONFIG>
          Config file

      --limit <LIMIT>
          Maximum number of items to return (1-2000, default: 50)

      --debug[=<DEBUG>]
          Print debug diagnostics, including HTTP requests, to stderr

          [default: false]

      --offset <OFFSET>
          Number of items to skip before returning the results

      --device-id <DEVICE_ID>
          Filter by device ID

      --device-name <DEVICE_NAME>
          Filter by device name

      --project-id <PROJECT_ID>
          Project ID (defaults to FOXGLOVE_DEFAULT_PROJECT_ID, then saved default_project_id; --project-id= bypasses defaults)

  -h, --help
          Print help
```
