<!-- Generated from rust/src/cli.rs by `make docs`. Do not edit directly. -->

# `foxglove custom-properties edit`

[Command reference](README.md) · [Parent command](custom-properties.md)

```text
Edit a custom property's label or enum values

Usage: foxglove custom-properties edit [OPTIONS] <CUSTOM_PROPERTY_ID>

Arguments:
  <CUSTOM_PROPERTY_ID>


Options:
      --client-id <CLIENT_ID>
          Foxglove client ID

      --enum-value <ENUM_VALUE>
          Complete list of allowed enum values, in display order; repeat for each value. It must include every existing value, so it can only reorder or add values

      --config <CONFIG>
          Config file

      --label <LABEL>
          New display label

      --debug[=<DEBUG>]
          Print debug diagnostics, including HTTP requests, to stderr

          [default: false]

  -h, --help
          Print help
```
