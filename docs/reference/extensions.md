<!-- Generated from rust/src/cli.rs by `make docs`. Do not edit directly. -->

# `foxglove extensions`

[Command reference](README.md) · [Parent command](README.md)

```text
List and publish Studio extensions

Usage: foxglove extensions [OPTIONS] <COMMAND>

Commands:
  list       List Studio extensions created for your organization
  publish    Publish a Studio extension (.foxe) to your organization
  unpublish  Delete and unpublish a Studio extension from your organization
  help       Print this message or the help of the given subcommand(s)

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

- [`foxglove extensions list`](extensions-list.md)
- [`foxglove extensions publish`](extensions-publish.md)
- [`foxglove extensions unpublish`](extensions-unpublish.md)
