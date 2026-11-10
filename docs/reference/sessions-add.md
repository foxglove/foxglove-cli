<!-- Generated from rust/src/cli.rs by `make docs`. Do not edit directly. -->

# `foxglove sessions add`

[Command reference](README.md) · [Parent command](sessions.md)

```text
Create a session

Usage: foxglove sessions add [OPTIONS] <--device-id <DEVICE_ID>|--recording-id <RECORDING_ID>>

Options:
      --client-id <CLIENT_ID>
          Foxglove client ID

      --device-id <DEVICE_ID>
          Device ID (required unless --recording-id is given)

      --config <CONFIG>
          Config file

      --recording-id <RECORDING_ID>
          Recording to add to the session; repeat for more. All must belong to one device, which is used when --device-id is omitted

      --debug[=<DEBUG>]
          Print debug diagnostics, including HTTP requests, to stderr

          [default: false]

      --key <KEY>
          Session key, unique within the project (cannot start or end with whitespace)

      --project-id <PROJECT_ID>
          Project ID (defaults to FOXGLOVE_DEFAULT_PROJECT_ID, then saved default_project_id; --project-id= bypasses defaults)

  -p, --property <PROPERTY>
          Custom property colon-separated key/value pair; repeat a multi-enum key to set its full list of values

  -h, --help
          Print help
```
