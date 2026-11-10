<!-- Generated from rust/src/cli.rs by `make docs`. Do not edit directly. -->

# `foxglove devices property-intervals update`

[Command reference](README.md) · [Parent command](devices-property-intervals.md)

```text
Record the value a device custom property held over a past time range, or that it had no value.

Existing intervals that overlap the range are split, trimmed, or removed. To change a property's current value, use `devices edit --property`.

Usage: foxglove devices property-intervals update [OPTIONS] --end <END> --key <KEY> --start <START> <DEVICE>

Arguments:
  <DEVICE>
          Device name or ID

Options:
      --client-id <CLIENT_ID>
          Foxglove client ID

      --end <END>
          Exclusive end of the time range (ISO 8601, must be in the past)

      --config <CONFIG>
          Config file

      --key <KEY>
          Device custom property key

      --debug[=<DEBUG>]
          Print debug diagnostics, including HTTP requests, to stderr

          [default: false]

      --project-id <PROJECT_ID>
          Project ID (defaults to FOXGLOVE_DEFAULT_PROJECT_ID, then saved default_project_id; --project-id= bypasses defaults)

      --start <START>
          Inclusive start of the time range (ISO 8601, must be in the past)

      --unset
          Record that the property had no value over the range

      --value <VALUE>
          Value the property held over the range; repeat for each value of a multi-enum property

  -h, --help
          Print help (see a summary with '-h')
```
