<!-- Generated from rust/src/cli.rs by `make docs`. Do not edit directly. -->

# `foxglove datasets commit`

[Command reference](README.md) · [Parent command](datasets.md)

```text
Commit a dataset's pending changes as a new version. The summary reports added, removed, and changed episodes. A changed episode is present in the new dataset version and the previous committed dataset version, but uses a different episode version.

Usage: foxglove datasets commit [OPTIONS] <DATASET_ID>

Arguments:
  <DATASET_ID>


Options:
      --client-id <CLIENT_ID>
          Foxglove client ID

      --description <DESCRIPTION>
          Description of the new version (default: the draft's description); pass an empty value to clear it

      --config <CONFIG>
          Config file

      --debug[=<DEBUG>]
          Print debug diagnostics, including HTTP requests, to stderr

          [default: false]

  -h, --help
          Print help (see a summary with '-h')
```
