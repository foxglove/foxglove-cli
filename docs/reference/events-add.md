<!-- Generated from rust/src/cli.rs by `make docs`. Do not edit directly. -->

# `foxglove events add`

[Command reference](README.md) · [Parent command](events.md)

```text
Add an event

Usage: foxglove events add [OPTIONS] --device-id <DEVICE_ID> --end <END> --start <START>

Options:
      --client-id <CLIENT_ID>
          Foxglove client ID

      --project-id <PROJECT_ID>
          Project ID (defaults to FOXGLOVE_DEFAULT_PROJECT_ID, then saved default_project_id; --project-id= bypasses defaults)

      --config <CONFIG>
          Config file

      --device-id <DEVICE_ID>
          Device ID (required)

      --debug[=<DEBUG>]
          Print debug diagnostics, including HTTP requests, to stderr

          [default: false]

      --end <END>
          End of event (inclusive), RFC 3339 (required)

      --event-type-id <EVENT_TYPE_ID>
          Associated event type ID

  -m, --metadata <METADATA>
          Metadata colon-separated key/value pair

      --start <START>
          Start of event, RFC 3339 (required)

  -h, --help
          Print help
```
