<!-- Generated from rust/src/cli.rs by `make docs`. Do not edit directly. -->

# `foxglove devices`

[Command reference](README.md) · [Parent command](README.md)

```text
List and manage devices

Usage: foxglove devices [OPTIONS] <COMMAND>

Commands:
  add                 Add a device for your organization
  delete              Delete a device
  edit                Edit a device
  get                 Get a device by name or ID
  list                List devices registered to your organization
  property-intervals  List, get, or update the values a device's custom properties held over time
  help                Print this message or the help of the given subcommand(s)

Options:
      --client-id <CLIENT_ID>
          Foxglove client ID

      --config <CONFIG>
          Config file

      --debug[=<DEBUG>]
          Print debug diagnostics, including HTTP requests, to stderr

          [default: false]

  -h, --help
          Print help
```

## Subcommands

- [`foxglove devices add`](devices-add.md)
- [`foxglove devices delete`](devices-delete.md)
- [`foxglove devices edit`](devices-edit.md)
- [`foxglove devices get`](devices-get.md)
- [`foxglove devices list`](devices-list.md)
- [`foxglove devices property-intervals`](devices-property-intervals.md)
