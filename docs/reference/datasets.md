<!-- Generated from rust/src/cli.rs by `make docs`. Do not edit directly. -->

# `foxglove datasets`

[Command reference](README.md) · [Parent command](README.md)

```text
List and manage datasets

Usage: foxglove datasets [OPTIONS] <COMMAND>

Commands:
  add       Create a dataset
  commit    Commit a dataset's pending changes as a new version
  delete    Delete a dataset
  discard   Discard a dataset's pending changes
  download  Download a committed dataset version
  edit      Edit a dataset's name or description
  episodes  List, add, or remove the episodes in a dataset
  get       Get a dataset
  list      List datasets
  versions  List, compare, edit, restore, and clone dataset versions
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

- [`foxglove datasets add`](datasets-add.md)
- [`foxglove datasets commit`](datasets-commit.md)
- [`foxglove datasets delete`](datasets-delete.md)
- [`foxglove datasets discard`](datasets-discard.md)
- [`foxglove datasets download`](datasets-download.md)
- [`foxglove datasets edit`](datasets-edit.md)
- [`foxglove datasets episodes`](datasets-episodes.md)
- [`foxglove datasets get`](datasets-get.md)
- [`foxglove datasets list`](datasets-list.md)
- [`foxglove datasets versions`](datasets-versions.md)
