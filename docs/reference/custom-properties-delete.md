<!-- Generated from rust/src/cli.rs by `make docs`. Do not edit directly. -->

# `foxglove custom-properties delete`

[Command reference](README.md) · [Parent command](custom-properties.md)

```text
Delete a custom property. Deleting a device custom property removes all of its values. An event custom property can only be deleted when no event has a value for it.

Usage: foxglove custom-properties delete [OPTIONS] <CUSTOM_PROPERTY_ID>

Arguments:
  <CUSTOM_PROPERTY_ID>


Options:
      --client-id <CLIENT_ID>
          Foxglove client ID

      --config <CONFIG>
          Config file

      --debug[=<DEBUG>]
          Print debug diagnostics, including HTTP requests, to stderr

          [default: false]

  -h, --help
          Print help (see a summary with '-h')
```
