<!-- Generated from rust/src/cli.rs by `make docs`. Do not edit directly. -->

# `foxglove automations runs list`

[Command reference](README.md) · [Parent command](automations-runs.md)

```text
List a project's automation runs, newest first by default

Usage: foxglove automations runs list [OPTIONS]

Options:
      --client-id <CLIENT_ID>
          Foxglove client ID

      --format <FORMAT>
          Render output in table, JSON, or CSV format

          [default: table]
          [possible values: table, json, csv]

      --automation-id <AUTOMATION_ID>
          Filter to runs of these automations; comma-separated or repeated

      --config <CONFIG>
          Config file

      --created-after <CREATED_AFTER>
          Only include runs created at or after this time (ISO 8601)

      --debug[=<DEBUG>]
          Print debug diagnostics, including HTTP requests, to stderr

          [default: false]

      --created-before <CREATED_BEFORE>
          Only include runs created at or before this time (ISO 8601)

      --execution <EXECUTION>
          Filter to runs with these executions: pending, running, finished, error, or blocked; comma-separated or repeated

      --limit <LIMIT>
          Maximum number of items to return (1-2000, default: 50)

      --offset <OFFSET>
          Number of items to skip before returning the results

      --owner-member-id <OWNER_MEMBER_ID>
          Filter to runs of automations owned by this organization member; use me for your own (not available to organization API keys)

      --project-id <PROJECT_ID>
          Project ID (required; defaults to FOXGLOVE_DEFAULT_PROJECT_ID, then saved default_project_id; cannot be empty)

      --result <RESULT>
          Filter to runs whose agent reported these results: succeeded, needs_review, failed, or no_result; comma-separated or repeated

      --sort-by <SORT_BY>
          Field to sort runs by: createdAt (default: createdAt)

      --sort-order <SORT_ORDER>
          Sort order for the --sort-by field: asc or desc (default: desc)

      --status <STATUS>
          Filter to runs with these statuses: pending, running, succeeded, or failed; comma-separated or repeated

  -h, --help
          Print help
```
