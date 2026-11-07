<!-- Generated from rust/src/cli.rs by `make docs`. Do not edit directly. -->

# `foxglove sessions`

[Command reference](README.md) · [Parent command](README.md)

```text
List and manage sessions

Usage: foxglove sessions [OPTIONS] <COMMAND>

Commands:
  add         Create a session
  delete      Delete a session
  edit        Change or remove a session key
  get         Get a session by ID or key
  list        List sessions in your organization
  recordings  List, add, or remove recordings in a session
  help        Print this message or the help of the given subcommand(s)

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

- [`foxglove sessions add`](sessions-add.md)
- [`foxglove sessions delete`](sessions-delete.md)
- [`foxglove sessions edit`](sessions-edit.md)
- [`foxglove sessions get`](sessions-get.md)
- [`foxglove sessions list`](sessions-list.md)
- [`foxglove sessions recordings`](sessions-recordings.md)
