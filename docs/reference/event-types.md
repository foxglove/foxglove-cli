<!-- Generated from rust/src/cli.rs by `make docs`. Do not edit directly. -->

# `foxglove event-types`

[Command reference](README.md) · [Parent command](README.md)

```text
List and manage event types

Usage: foxglove event-types [OPTIONS] <COMMAND>

Commands:
  add     Create an event type
  delete  Delete an event type that has no events
  edit    Edit an event type's name, color, or custom properties
  get     Get an event type
  list    List event types
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

- [`foxglove event-types add`](event-types-add.md)
- [`foxglove event-types delete`](event-types-delete.md)
- [`foxglove event-types edit`](event-types-edit.md)
- [`foxglove event-types get`](event-types-get.md)
- [`foxglove event-types list`](event-types-list.md)
