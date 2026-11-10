<!-- Generated from rust/src/cli.rs by `make docs`. Do not edit directly. -->

# `foxglove events add`

[Command reference](README.md) · [Parent command](events.md)

```text
Add an event

Usage: foxglove events add [OPTIONS] --end <END> --start <START> <--device-id <DEVICE_ID>|--device-name <DEVICE_NAME>>

Options:
      --client-id <CLIENT_ID>
          Foxglove client ID

      --project-id <PROJECT_ID>
          Project ID (defaults to FOXGLOVE_DEFAULT_PROJECT_ID, then saved default_project_id; --project-id= bypasses defaults)

      --config <CONFIG>
          Config file

      --device-id <DEVICE_ID>
          Device ID (required unless --device-name is given)

      --debug[=<DEBUG>]
          Print debug diagnostics, including HTTP requests, to stderr

          [default: false]

      --device-name <DEVICE_NAME>
          Device name, looked up in the project (required unless --device-id is given)

      --end <END>
          End of event (inclusive), RFC 3339 (required)

      --event-type-id <EVENT_TYPE_ID>
          Associated event type ID

  -m, --metadata <METADATA>
          Metadata colon-separated key/value pair

  -p, --property <PROPERTY>
          Custom property colon-separated key/value pair; repeat a multi-enum key to set its full list of values

      --start <START>
          Start of event, RFC 3339 (required)

  -h, --help
          Print help
```
