<!-- Generated from rust/src/cli.rs by `make docs`. Do not edit directly. -->

# `foxglove episodes edit-bulk`

[Command reference](README.md) · [Parent command](episodes.md)

```text
Apply episode edits atomically from a JSON file

Usage: foxglove episodes edit-bulk [OPTIONS] <FILE>

Arguments:
  <FILE>
          JSON file with an episodes array of edits (1-2000 unique IDs)

Options:
      --client-id <CLIENT_ID>
          Foxglove client ID

      --format <FORMAT>
          Render output in table, JSON, or CSV format

          [default: table]
          [possible values: table, json, csv]

      --config <CONFIG>
          Config file

      --debug[=<DEBUG>]
          Print debug diagnostics, including HTTP requests, to stderr

          [default: false]

  -h, --help
          Print help
```
