<!-- Generated from rust/src/cli.rs by `make docs`. Do not edit directly. -->

# `foxglove datasets versions get`

[Command reference](README.md) · [Parent command](datasets-versions.md)

```text
Get a dataset version. The Changed column counts episodes present in this dataset version and the previous dataset version that use a different episode version. For a draft, the comparison is against the newest committed dataset version.

Usage: foxglove datasets versions get [OPTIONS] <DATASET_ID> <VERSION>

Arguments:
  <DATASET_ID>


  <VERSION>


Options:
      --client-id <CLIENT_ID>
          Foxglove client ID

      --format <FORMAT>
          Render output in table, JSON, or CSV format

          [default: table]
          [possible values: table, json, csv]

      --config <CONFIG>
          Config file

      --debug[=<DEBUG>]
          Print debug diagnostics, including HTTP requests, to stderr

          [default: false]

  -h, --help
          Print help (see a summary with '-h')
```
