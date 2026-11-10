<!-- Generated from rust/src/cli.rs by `make docs`. Do not edit directly. -->

# `foxglove episodes edit`

[Command reference](README.md) · [Parent command](episodes.md)

```text
Edit an episode (requires API access to episode properties)

Usage: foxglove episodes edit [OPTIONS] <EPISODE_ID>

Arguments:
  <EPISODE_ID>


Options:
      --client-id <CLIENT_ID>
          Foxglove client ID

  -p, --property <PROPERTY>
          Custom property colon-separated key/value pair; repeat a multi-enum key to set its full list of values

      --config <CONFIG>
          Config file

      --remove-property <REMOVE_PROPERTY>
          Remove a property assignment; repeat for more keys

      --applies-to-version <APPLIES_TO_VERSION>
          Require this current episode version

      --debug[=<DEBUG>]
          Print debug diagnostics, including HTTP requests, to stderr

          [default: false]

      --start <START>
          New start of the episode window (ISO 8601)

      --end <END>
          New end of the episode window (ISO 8601)

      --add-recording-id <ADD_RECORDING_ID>
          Add a recording; repeat for more IDs

      --remove-recording-id <REMOVE_RECORDING_ID>
          Remove a recording; repeat for more IDs

  -h, --help
          Print help
```
