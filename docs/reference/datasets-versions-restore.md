<!-- Generated from rust/src/cli.rs by `make docs`. Do not edit directly. -->

# `foxglove datasets versions restore`

[Command reference](README.md) · [Parent command](datasets-versions.md)

```text
Restore a committed version's episodes as pending changes

Usage: foxglove datasets versions restore [OPTIONS] <DATASET_ID> <VERSION>

Arguments:
  <DATASET_ID>


  <VERSION>


Options:
      --client-id <CLIENT_ID>
          Foxglove client ID

      --force[=<FORCE>]
          Discard the dataset's pending changes before restoring

          [default: false]

      --config <CONFIG>
          Config file

      --debug[=<DEBUG>]
          Print debug diagnostics, including HTTP requests, to stderr

          [default: false]

  -h, --help
          Print help
```
