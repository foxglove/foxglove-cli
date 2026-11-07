<!-- Generated from rust/src/cli.rs by `make docs`. Do not edit directly. -->

# `foxglove datasets versions list`

[Command reference](README.md) · [Parent command](datasets-versions.md)

```text
List a dataset's versions

Usage: foxglove datasets versions list [OPTIONS] <DATASET_ID>

Arguments:
  <DATASET_ID>


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

      --cursor <CURSOR>
          Cursor from a previous page

      --debug[=<DEBUG>]
          Print debug diagnostics, including HTTP requests, to stderr

          [default: false]

      --sort-order <SORT_ORDER>
          Sort order by version number: asc or desc (default: desc)

  -h, --help
          Print help
```
