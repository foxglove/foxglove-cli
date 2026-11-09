<!-- Generated from rust/src/cli.rs by `make docs`. Do not edit directly. -->

# `foxglove devices delete`

[Command reference](README.md) · [Parent command](devices.md)

```text
Delete a device. The API refuses to delete a device that still has associated data; delete that data first.

Usage: foxglove devices delete [OPTIONS] <DEVICE_NAME_OR_ID>

Arguments:
  <DEVICE_NAME_OR_ID>


Options:
      --client-id <CLIENT_ID>
          Foxglove client ID

      --project-id <PROJECT_ID>
          Project ID (defaults to FOXGLOVE_DEFAULT_PROJECT_ID, then saved default_project_id; --project-id= bypasses defaults)

      --config <CONFIG>
          Config file

      --debug[=<DEBUG>]
          Print debug diagnostics, including HTTP requests, to stderr

          [default: false]

  -h, --help
          Print help (see a summary with '-h')
```
