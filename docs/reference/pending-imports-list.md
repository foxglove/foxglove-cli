<!-- Generated from rust/src/cli.rs by `make docs`. Do not edit directly. -->

# `foxglove pending-imports list`

[Command reference](README.md) · [Parent command](pending-imports.md)

```text
List pending and errored import jobs for uploaded recordings

Usage: foxglove pending-imports list [OPTIONS]

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

      --device-id <DEVICE_ID>
          Device ID

      --device-name <DEVICE_NAME>
          Device name

      --filename <FILENAME>
          Exact filename to match

      --key <KEY>
          Key

      --project-id <PROJECT_ID>
          Project ID (defaults to FOXGLOVE_DEFAULT_PROJECT_ID, then saved default_project_id; --project-id= bypasses defaults)

      --request-id <REQUEST_ID>
          Request ID

      --show-completed[=<SHOW_COMPLETED>]
          Show completed requests

          [default: false]

      --show-quarantined[=<SHOW_QUARANTINED>]
          Show quarantined requests

          [default: false]

      --site-id <SITE_ID>
          Site ID

      --site-backfill-job-id <SITE_BACKFILL_JOB_ID>
          Only imports enqueued by this site backfill job

      --sort-by <SORT_BY>
          Field to sort imports by: createdAt, updatedAt, deviceId, deviceName, importId, requestId, or error

      --sort-order <SORT_ORDER>
          Sort order for the --sort-by field: asc or desc (default: asc)

      --updated-since <UPDATED_SINCE>
          Only imports updated since this time

      --without-project[=<WITHOUT_PROJECT>]
          Only imports without a project

          [default: false]

  -h, --help
          Print help
```
