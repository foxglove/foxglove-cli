<!-- Generated from rust/src/cli.rs by `make docs`. Do not edit directly. -->

# `foxglove custom-properties add`

[Command reference](README.md) · [Parent command](custom-properties.md)

```text
Create a custom property

Usage: foxglove custom-properties add [OPTIONS] --key <KEY> --label <LABEL> --resource-type <RESOURCE_TYPE> --value-type <VALUE_TYPE>

Options:
      --client-id <CLIENT_ID>
          Foxglove client ID

      --enum-value <ENUM_VALUE>
          Allowed value for an enum or multi-enum property; repeat to add more, in display order

      --config <CONFIG>
          Config file

      --key <KEY>
          Key used to set and query the property, such as partNumber (letters, digits, - and _; cannot be changed later)

      --debug[=<DEBUG>]
          Print debug diagnostics, including HTTP requests, to stderr

          [default: false]

      --label <LABEL>
          Display label

      --resource-type <RESOURCE_TYPE>
          Resource type the property applies to: device, event, session, or episode

      --value-type <VALUE_TYPE>
          Value type: string, multiline-string, number, boolean, enum, or multi-enum

  -h, --help
          Print help
```
