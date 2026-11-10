<!-- Generated from rust/src/cli.rs by `make docs`. Do not edit directly. -->

# `foxglove layouts list`

[Command reference](README.md) · [Parent command](layouts.md)

```text
List layouts shared with your organization, plus your personal layouts when signed in with `foxglove auth login`

Usage: foxglove layouts list [OPTIONS]

Options:
      --client-id <CLIENT_ID>
          Foxglove client ID

      --format <FORMAT>
          Render output in table, JSON, or CSV format

          [default: table]
          [possible values: table, json, csv]

      --config <CONFIG>
          Config file

      --include-data[=<INCLUDE_DATA>]
          Include each layout's data in JSON output

          [default: false]

      --debug[=<DEBUG>]
          Print debug diagnostics, including HTTP requests, to stderr

          [default: false]

      --updated-since <UPDATED_SINCE>
          Only layouts updated since this time (ISO 8601)

  -h, --help
          Print help
```
