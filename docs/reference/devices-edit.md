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

      --remove-property <REMOVE_PROPERTY>
          Remove a property assignment; repeat for more keys

      --enabled[=<ENABLED>]
          Enable or disable the device; a disabled device can't upload recordings

      --remote-access-enabled[=<REMOTE_ACCESS_ENABLED>]
          Allow or block new remote access sessions to the device

      --retain-recordings-seconds <SECONDS>
          How long Foxlet on the device keeps the recordings it creates, in seconds; 0 keeps them indefinitely

      --auto-delete-after-import[=<AUTO_DELETE_AFTER_IMPORT>]
          Have Foxlet on the device delete recordings once they are uploaded and accepted for import

  -h, --help
          Print help
```
