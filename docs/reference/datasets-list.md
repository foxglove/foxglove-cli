<!-- Generated from rust/src/cli.rs by `make docs`. Do not edit directly. -->

# `foxglove datasets list`

[Command reference](README.md) · [Parent command](datasets.md)

```text
List datasets

Usage: foxglove datasets list [OPTIONS]

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

      --name <NAME>
          Filter to datasets whose name contains this value, ignoring case

      --project-id <PROJECT_ID>
          Project ID (defaults to FOXGLOVE_DEFAULT_PROJECT_ID, then saved default_project_id; --project-id= bypasses defaults)

      --sort-by <SORT_BY>
          Field to sort datasets by: name, createdAt, updatedAt, or episodeCount (default: createdAt)

      --sort-order <SORT_ORDER>
          Sort order for the --sort-by field: asc or desc

  -h, --help
          Print help
```
