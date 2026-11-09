<!-- Generated from rust/src/cli.rs by `make docs`. Do not edit directly. -->

# `foxglove devices property-intervals`

[Command reference](README.md) · [Parent command](devices.md)

```text
List, get, or update the values a device's custom properties held over time

Usage: foxglove devices property-intervals [OPTIONS] <COMMAND>

Commands:
  get     Get one property time interval
  list    List the values a device's custom properties held and when
  update  Record the value a custom property held over a past time range
  help    Print this message or the help of the given subcommand(s)

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

- [`foxglove devices property-intervals get`](devices-property-intervals-get.md)
- [`foxglove devices property-intervals list`](devices-property-intervals-list.md)
- [`foxglove devices property-intervals update`](devices-property-intervals-update.md)
