<!-- Generated from rust/src/cli.rs by `make docs`. Do not edit directly. -->

# `foxglove devices edit`

[Command reference](README.md) · [Parent command](devices.md)

```text
Edit a device

Usage: foxglove devices edit [OPTIONS] <DEVICE_ID>

Arguments:
  <DEVICE_ID>


Options:
      --client-id <CLIENT_ID>
          Foxglove client ID

      --name <NAME>
          Name of the device

      --config <CONFIG>
          Config file

      --project-id <PROJECT_ID>
          Project ID (defaults to FOXGLOVE_DEFAULT_PROJECT_ID, then saved default_project_id; --project-id= bypasses defaults)

      --debug[=<DEBUG>]
          Print debug diagnostics, including HTTP requests, to stderr

          [default: false]

  -p, --property <PROPERTY>
          Custom property colon-separated key/value pair; repeat a multi-enum key to set its full list of values

  -h, --help
          Print help
```
