<!-- Generated from rust/src/cli.rs by `make docs`. Do not edit directly. -->

# `foxglove episodes versions list`

[Command reference](README.md) · [Parent command](episodes-versions.md)

```text
List episode versions, newest first

Usage: foxglove episodes versions list [OPTIONS] <EPISODE_ID>

Arguments:
  <EPISODE_ID>


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

  -h, --help
          Print help
```
