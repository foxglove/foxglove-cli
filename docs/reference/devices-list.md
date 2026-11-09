<!-- Generated from rust/src/cli.rs by `make docs`. Do not edit directly. -->

# `foxglove devices list`

[Command reference](README.md) · [Parent command](devices.md)

```text
List devices registered to your organization

Usage: foxglove devices list [OPTIONS]

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

      --debug[=<DEBUG>]
          Print debug diagnostics, including HTTP requests, to stderr

          [default: false]

      --offset <OFFSET>
          Number of items to skip before returning the results

      --project-id <PROJECT_ID>
          Project ID (defaults to FOXGLOVE_DEFAULT_PROJECT_ID, then saved default_project_id; --project-id= bypasses defaults)

      --query <QUERY>
          Filter by device custom properties, for example 'properties.location:sf properties.model:*'

      --sort-by <SORT_BY>
          Field to sort devices by: id, name, or a custom property key prefixed with properties.

      --sort-order <SORT_ORDER>
          Sort order for the --sort-by field: asc or desc

  -h, --help
          Print help
```
