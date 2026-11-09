<!-- Generated from rust/src/cli.rs by `make docs`. Do not edit directly. -->

# `foxglove datasets episodes`

[Command reference](README.md) · [Parent command](datasets.md)

```text
List, add, or remove the episodes in a dataset

Usage: foxglove datasets episodes [OPTIONS] <COMMAND>

Commands:
  add     Add episodes to a dataset as pending changes
  update  Update episode versions selected in the dataset draft
  list    List the episodes in a dataset
  remove  Remove episodes from a dataset as pending changes
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

- [`foxglove datasets episodes add`](datasets-episodes-add.md)
- [`foxglove datasets episodes update`](datasets-episodes-update.md)
- [`foxglove datasets episodes list`](datasets-episodes-list.md)
- [`foxglove datasets episodes remove`](datasets-episodes-remove.md)
