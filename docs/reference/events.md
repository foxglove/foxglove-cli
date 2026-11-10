<!-- Generated from rust/src/cli.rs by `make docs`. Do not edit directly. -->

# `foxglove events`

[Command reference](README.md) · [Parent command](README.md)

```text
List and manage events

Usage: foxglove events [OPTIONS] <COMMAND>

Commands:
  add     Add an event
  delete  Delete an event
  edit    Edit an event's time range, event type, metadata, or properties
  get     Get an event
  list    List events
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

- [`foxglove events add`](events-add.md)
- [`foxglove events delete`](events-delete.md)
- [`foxglove events edit`](events-edit.md)
- [`foxglove events get`](events-get.md)
- [`foxglove events list`](events-list.md)
