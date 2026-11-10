<!-- Generated from rust/src/cli.rs by `make docs`. Do not edit directly. -->

# `foxglove site-notification-tokens`

[Command reference](README.md) · [Parent command](README.md)

```text
List and manage tokens that authenticate a Primary Site's bucket notifications

Usage: foxglove site-notification-tokens [OPTIONS] <COMMAND>

Commands:
  add     Create a bucket notification token for a Primary Site and print it once
  delete  Delete a site notification token
  list    List site notification tokens
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

- [`foxglove site-notification-tokens add`](site-notification-tokens-add.md)
- [`foxglove site-notification-tokens delete`](site-notification-tokens-delete.md)
- [`foxglove site-notification-tokens list`](site-notification-tokens-list.md)
