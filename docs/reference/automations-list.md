<!-- Generated from rust/src/cli.rs by `make docs`. Do not edit directly. -->

# `foxglove automations list`

[Command reference](README.md) · [Parent command](automations.md)

```text
List a project's automations, newest first, with their run counts

Usage: foxglove automations list [OPTIONS]

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
          Project ID (required; defaults to FOXGLOVE_DEFAULT_PROJECT_ID, then saved default_project_id; cannot be empty)

  -h, --help
          Print help
```
