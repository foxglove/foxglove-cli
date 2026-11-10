<!-- Generated from rust/src/cli.rs by `make docs`. Do not edit directly. -->

# `foxglove import-errors list`

[Command reference](README.md) · [Parent command](import-errors.md)

```text
List uploaded recordings whose import failed during processing

Usage: foxglove import-errors list [OPTIONS]

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

      --device-name <DEVICE_NAME>
          Device name

      --filename-matches <FILENAME_MATCHES>
          Pattern to match against the filename

      --error <ERROR>
          Only import errors whose error message matches this text

      --project-id <PROJECT_ID>
          Project ID (defaults to FOXGLOVE_DEFAULT_PROJECT_ID, then saved default_project_id; --project-id= bypasses defaults)

      --site-backfill-job-id <SITE_BACKFILL_JOB_ID>
          Only import errors from imports enqueued by this site backfill job

      --sort-by <SORT_BY>
          Field to sort import errors by: createdAt, deviceName, error, requestId, or updatedAt

      --sort-order <SORT_ORDER>
          Sort order for the --sort-by field: asc or desc (default: asc)

  -h, --help
          Print help
```
