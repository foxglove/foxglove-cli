<!-- Generated from rust/src/cli.rs by `make docs`. Do not edit directly. -->

# `foxglove attachments`

[Command reference](README.md) · [Parent command](README.md)

```text
Query and modify data attachments

Usage: foxglove attachments [OPTIONS] <COMMAND>

Commands:
  download  Download an MCAP attachment by ID to stdout
  get       Get an MCAP attachment's details by ID
  list      List MCAP attachments
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

- [`foxglove attachments download`](attachments-download.md)
- [`foxglove attachments get`](attachments-get.md)
- [`foxglove attachments list`](attachments-list.md)
