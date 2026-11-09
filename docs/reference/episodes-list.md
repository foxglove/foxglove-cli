<!-- Generated from rust/src/cli.rs by `make docs`. Do not edit directly. -->

# `foxglove episodes list`

[Command reference](README.md) · [Parent command](episodes.md)

```text
List episodes

Usage: foxglove episodes list [OPTIONS]

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

      --exact-recording-ids <EXACT_RECORDING_IDS>
          Filter to episodes whose recordings are exactly these display IDs; comma-separated (maximum 50)

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

      --project-id <PROJECT_ID>
          Project ID (defaults to FOXGLOVE_DEFAULT_PROJECT_ID, then saved default_project_id; --project-id= bypasses defaults)

      --recording-id <RECORDING_ID>
          Filter to episodes containing this recording display ID

      --sort-by <SORT_BY>
          Field to sort episodes by: createdAt, startTime, or endTime (default: createdAt)

      --sort-order <SORT_ORDER>
          Sort order for the --sort-by field: asc or desc

      --start <START>
          Start of a time range the episode's window must overlap (ISO 8601); give with --end

      --include-properties[=<INCLUDE_PROPERTIES>]
          Include episode custom properties (requires API access to episode properties)

          [default: false]

      --properties-query <PROPERTIES_QUERY>
          Filter by episode custom properties, for example 'weather:rain note:"night run"' (requires API access to episode properties)

  -h, --help
          Print help
```
