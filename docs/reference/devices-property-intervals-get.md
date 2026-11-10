<!-- Generated from rust/src/cli.rs by `make docs`. Do not edit directly. -->

# `foxglove devices property-intervals get`

[Command reference](README.md) · [Parent command](devices-property-intervals.md)

```text
Get one property time interval

Usage: foxglove devices property-intervals get [OPTIONS] <DEVICE> <INTERVAL_ID>

Arguments:
  <DEVICE>
          Device name or ID

  <INTERVAL_ID>
          Property time interval ID

Options:
      --client-id <CLIENT_ID>
          Foxglove client ID

      --format <FORMAT>
          Render output in table, JSON, or CSV format

          [default: table]
          [possible values: table, json, csv]

      --config <CONFIG>
          Config file

      --project-id <PROJECT_ID>
          Project ID (defaults to FOXGLOVE_DEFAULT_PROJECT_ID, then saved default_project_id; --project-id= bypasses defaults)

      --debug[=<DEBUG>]
          Print debug diagnostics, including HTTP requests, to stderr

          [default: false]

  -h, --help
          Print help
```
