<!-- Generated from rust/src/cli.rs by `make docs`. Do not edit directly. -->

# `foxglove recordings`

[Command reference](README.md) · [Parent command](README.md)

```text
Query recordings

Usage: foxglove recordings [OPTIONS] <COMMAND>

Commands:
  delete    Delete a recording from your organization
  list      List recordings
  transfer  Request transfer of a recording from its Edge Site to its configured Primary Site
  help      Print this message or the help of the given subcommand(s)

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

- [`foxglove recordings delete`](recordings-delete.md)
- [`foxglove recordings list`](recordings-list.md)
- [`foxglove recordings transfer`](recordings-transfer.md)
