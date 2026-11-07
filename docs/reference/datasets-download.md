<!-- Generated from rust/src/cli.rs by `make docs`. Do not edit directly. -->

# `foxglove datasets download`

[Command reference](README.md) · [Parent command](datasets.md)

```text
Download a committed dataset version

Usage: foxglove datasets download [OPTIONS] <DATASET_ID>

Arguments:
  <DATASET_ID>


Options:
      --client-id <CLIENT_ID>
          Foxglove client ID

      --include-attachments[=<INCLUDE_ATTACHMENTS>]
          Include MCAP attachments

          [default: true]

      --config <CONFIG>
          Config file

  -o, --output <OUTPUT>
          Output directory (default: the dataset name and version)

      --debug[=<DEBUG>]
          Print debug diagnostics, including HTTP requests, to stderr

          [default: false]

      --strict
          Fail if any episode is skipped or downloaded with missing recordings

      --topics <TOPICS>
          Comma-separated topic list (default: all topics)

      --version <VERSION>
          Committed version to download (default: the newest committed version)

  -h, --help
          Print help
```
