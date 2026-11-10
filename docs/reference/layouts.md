<!-- Generated from rust/src/cli.rs by `make docs`. Do not edit directly. -->

# `foxglove layouts`

[Command reference](README.md) · [Parent command](README.md)

```text
List and manage layouts

Usage: foxglove layouts [OPTIONS] <COMMAND>

Commands:
  add      Create a layout from a JSON file of layout data
  delete   Delete a layout; restore brings it back
  edit     Update a layout's name, folder, permission, or data
  get      Get a layout
  list     List layouts shared with your organization, plus your personal layouts when signed in with `foxglove auth login`
  restore  Restore a deleted layout
  help     Print this message or the help of the given subcommand(s)

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

- [`foxglove layouts add`](layouts-add.md)
- [`foxglove layouts delete`](layouts-delete.md)
- [`foxglove layouts edit`](layouts-edit.md)
- [`foxglove layouts get`](layouts-get.md)
- [`foxglove layouts list`](layouts-list.md)
- [`foxglove layouts restore`](layouts-restore.md)
