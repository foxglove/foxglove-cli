<!-- Generated from rust/src/cli.rs by `make docs`. Do not edit directly. -->

# `foxglove recordings delete`

[Command reference](README.md) · [Parent command](recordings.md)

```text
Delete a recording and its data. For recordings imported from an Edge Site, only the imported data is removed: the edge copy and session membership remain. Use `recordings transfer RECORDING_ID` to restore its data.

Usage: foxglove recordings delete [OPTIONS] <RECORDING_ID>

Arguments:
  <RECORDING_ID>


Options:
      --client-id <CLIENT_ID>
          Foxglove client ID

      --config <CONFIG>
          Config file

      --debug[=<DEBUG>]
          Print debug diagnostics, including HTTP requests, to stderr

          [default: false]

  -h, --help
          Print help (see a summary with '-h')
```
