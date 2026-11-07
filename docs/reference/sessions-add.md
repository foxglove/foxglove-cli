<!-- Generated from rust/src/cli.rs by `make docs`. Do not edit directly. -->

# `foxglove sessions add`

[Command reference](README.md) · [Parent command](sessions.md)

```text
Create a session

Usage: foxglove sessions add [OPTIONS]

Options:
      --client-id <CLIENT_ID>
          Foxglove client ID

      --device-id <DEVICE_ID>
          Device ID (required)

      --config <CONFIG>
          Config file

      --key <KEY>
          Session key, unique within the project (cannot start or end with whitespace)

      --debug[=<DEBUG>]
          Print debug diagnostics, including HTTP requests, to stderr

          [default: false]

      --project-id <PROJECT_ID>
          Project ID (defaults to FOXGLOVE_DEFAULT_PROJECT_ID, then saved default_project_id; --project-id= bypasses defaults)

  -h, --help
          Print help
```
