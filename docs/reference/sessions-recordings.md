<!-- Generated from rust/src/cli.rs by `make docs`. Do not edit directly. -->

# `foxglove sessions recordings`

[Command reference](README.md) · [Parent command](sessions.md)

```text
List, add, or remove recordings in a session

Usage: foxglove sessions recordings [OPTIONS] <COMMAND>

Commands:
  add     Assign a recording to a session
  list    List recording IDs in a session
  remove  Remove a recording from a session
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

- [`foxglove sessions recordings add`](sessions-recordings-add.md)
- [`foxglove sessions recordings list`](sessions-recordings-list.md)
- [`foxglove sessions recordings remove`](sessions-recordings-remove.md)
