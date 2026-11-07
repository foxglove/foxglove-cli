<!-- Generated from rust/src/cli.rs by `make docs`. Do not edit directly. -->

# `foxglove datasets versions`

[Command reference](README.md) · [Parent command](datasets.md)

```text
List, compare, and restore dataset versions

Usage: foxglove datasets versions [OPTIONS] <COMMAND>

Commands:
  compare  List the episodes added and removed between two versions
  get      Get a dataset version
  list     List a dataset's versions
  restore  Restore a committed version's episodes as pending changes
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

- [`foxglove datasets versions compare`](datasets-versions-compare.md)
- [`foxglove datasets versions get`](datasets-versions-get.md)
- [`foxglove datasets versions list`](datasets-versions-list.md)
- [`foxglove datasets versions restore`](datasets-versions-restore.md)
