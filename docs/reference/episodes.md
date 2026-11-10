<!-- Generated from rust/src/cli.rs by `make docs`. Do not edit directly. -->

# `foxglove episodes`

[Command reference](README.md) · [Parent command](README.md)

```text
List and manage episodes

Usage: foxglove episodes [OPTIONS] <COMMAND>

Commands:
  add        Create an episode, or get the existing one with matching window, recordings, and properties
  delete     Delete an episode
  edit       Edit an episode (requires API access to episode properties)
  edit-bulk  Apply episode edits atomically from a JSON file
  get        Get an episode
  list       List episodes
  versions   Inspect episode version history
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

- [`foxglove episodes add`](episodes-add.md)
- [`foxglove episodes delete`](episodes-delete.md)
- [`foxglove episodes edit`](episodes-edit.md)
- [`foxglove episodes edit-bulk`](episodes-edit-bulk.md)
- [`foxglove episodes get`](episodes-get.md)
- [`foxglove episodes list`](episodes-list.md)
- [`foxglove episodes versions`](episodes-versions.md)
