<!-- Generated from rust/src/cli.rs by `make docs`. Do not edit directly. -->

# `foxglove datasets episodes list`

[Command reference](README.md) · [Parent command](datasets-episodes.md)

```text
List the episodes in a dataset

Usage: foxglove datasets episodes list [OPTIONS] <DATASET_ID>

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

      --end <END>
          End of a time range the episode's window must overlap (ISO 8601); give with --start

      --debug[=<DEBUG>]
          Print debug diagnostics, including HTTP requests, to stderr

          [default: false]

      --has-missing-recordings[=<HAS_MISSING_RECORDINGS>]
          Filter to episodes with, or without, recordings that are no longer available

      --include-creation-context[=<INCLUDE_CREATION_CONTEXT>]
          Include each episode's creation context: the selection or search that produced it

          [default: false]

      --include-recordings[=<INCLUDE_RECORDINGS>]
          Include the member recordings of each episode

          [default: false]

      --limit <LIMIT>
          Maximum number of items to return (1-2000, default: 50)

      --cursor <CURSOR>
          Cursor from a previous page

      --recording-id <RECORDING_ID>
          Filter to episodes containing this recording display ID

      --sort-by <SORT_BY>
          Field to sort episodes by: addedAt (when the episode joined the dataset), createdAt, startTime, or endTime (default: addedAt)

      --sort-order <SORT_ORDER>
          Sort order for the --sort-by field: asc or desc

      --start <START>
          Start of a time range the episode's window must overlap (ISO 8601); give with --end

      --version <VERSION>
          Version number to list, or draft to list the draft with its pending changes (default: the newest committed version, or the draft before the first commit)

      --include-properties[=<INCLUDE_PROPERTIES>]
          Include episode custom properties (requires API access to episode properties)

          [default: false]

      --properties-query <PROPERTIES_QUERY>
          Filter by episode custom properties, for example 'weather:rain note:"night run"' (requires API access to episode properties)

  -h, --help
          Print help
```
