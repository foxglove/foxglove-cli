<!-- Generated from rust/src/cli.rs by `make docs`. Do not edit directly. -->

# `foxglove datasets versions compare`

[Command reference](README.md) · [Parent command](datasets-versions.md)

```text
List the episodes added, removed, or changed between two versions

Usage: foxglove datasets versions compare [OPTIONS] <DATASET_ID> <BASE_VERSION> <TARGET_VERSION>

Arguments:
  <DATASET_ID>


  <BASE_VERSION>
          Version to compare from

  <TARGET_VERSION>
          Version to compare to

Options:
      --client-id <CLIENT_ID>
          Foxglove client ID

      --format <FORMAT>
          Render output in table, JSON, or CSV format

          [default: table]
          [possible values: table, json, csv]

      --config <CONFIG>
          Config file

      --cursor <CURSOR>
          Cursor from a previous compare, to fetch the next page

      --debug[=<DEBUG>]
          Print debug diagnostics, including HTTP requests, to stderr

          [default: false]

      --include-creation-context[=<INCLUDE_CREATION_CONTEXT>]
          Include each episode's creation context: the selection or search that produced it

          [default: false]

      --include-recordings[=<INCLUDE_RECORDINGS>]
          Include the member recordings of each episode

          [default: false]

      --limit <LIMIT>
          Maximum number of changes to return (1-2000, default: 50)

      --include-properties[=<INCLUDE_PROPERTIES>]
          Include episode custom properties (requires API access to episode properties)

          [default: false]

  -h, --help
          Print help
```
