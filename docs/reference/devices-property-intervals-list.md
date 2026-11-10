<!-- Generated from rust/src/cli.rs by `make docs`. Do not edit directly. -->

# `foxglove devices property-intervals list`

[Command reference](README.md) · [Parent command](devices-property-intervals.md)

```text
List the values a device's custom properties held and when

Usage: foxglove devices property-intervals list [OPTIONS] <DEVICE>

Arguments:
  <DEVICE>
          Device name or ID

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
          Include intervals active before this time (ISO 8601)

      --debug[=<DEBUG>]
          Print debug diagnostics, including HTTP requests, to stderr

          [default: false]

      --limit <LIMIT>
          Maximum number of items to return (1-2000, default: 50)

      --offset <OFFSET>
          Number of items to skip before returning the results

      --project-id <PROJECT_ID>
          Project ID (defaults to FOXGLOVE_DEFAULT_PROJECT_ID, then saved default_project_id; --project-id= bypasses defaults)

      --query <QUERY>
          Filter by property key and value, for example 'location:sf model:*'

      --sort-by <SORT_BY>
          Field to sort intervals by: id, key, start, or end

      --sort-order <SORT_ORDER>
          Sort order for the --sort-by field: asc or desc

      --start <START>
          Include intervals active on or after this time (ISO 8601)

  -h, --help
          Print help
```
