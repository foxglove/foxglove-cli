<!-- Generated from rust/src/cli.rs by `make docs`. Do not edit directly. -->

# `foxglove datasets versions clone`

[Command reference](README.md) · [Parent command](datasets-versions.md)

```text
Create a new dataset in the same project with the episodes and episode versions of a committed version. The new dataset is independent of the original. It has a single committed version, or with --include-history, copies of every version up to and including the cloned one.

Usage: foxglove datasets versions clone [OPTIONS] --name <NAME> <DATASET_ID> <VERSION>

Arguments:
  <DATASET_ID>


  <VERSION>


Options:
      --client-id <CLIENT_ID>
          Foxglove client ID

      --description <DESCRIPTION>
          Optional display description of the new dataset

      --config <CONFIG>
          Config file

      --include-history[=<INCLUDE_HISTORY>]
          Copy the versions up to and including the cloned version

          [default: false]

      --debug[=<DEBUG>]
          Print debug diagnostics, including HTTP requests, to stderr

          [default: false]

      --name <NAME>
          Display name of the new dataset, unique within the project

  -h, --help
          Print help (see a summary with '-h')
```
