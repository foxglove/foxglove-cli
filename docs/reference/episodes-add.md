<!-- Generated from rust/src/cli.rs by `make docs`. Do not edit directly. -->

# `foxglove episodes add`

[Command reference](README.md) · [Parent command](episodes.md)

```text
Create an episode, or get the existing one with matching window, recordings, and properties

Usage: foxglove episodes add [OPTIONS] <--event-id <EVENT_ID>|--recording-id <RECORDING_ID>>

Options:
      --client-id <CLIENT_ID>
          Foxglove client ID

      --end <END>
          End of the episode window (ISO 8601); give with --start (default: the end of its recordings)

      --config <CONFIG>
          Config file

      --event-id <EVENT_ID>
          Event whose time range and device's completed recordings form the episode (cannot be combined with --start or --end)

      --debug[=<DEBUG>]
          Print debug diagnostics, including HTTP requests, to stderr

          [default: false]

      --project-id <PROJECT_ID>
          Project ID (required; defaults to FOXGLOVE_DEFAULT_PROJECT_ID, then saved default_project_id; cannot be empty)

      --recording-id <RECORDING_ID>
          Recording in the episode; repeat to add more (required unless --event-id is given)

      --start <START>
          Start of the episode window (ISO 8601); give with --end (default: the start of its recordings)

  -p, --property <PROPERTY>
          Custom property colon-separated key/value pair; repeat a multi-enum key to set its full list of values

  -h, --help
          Print help
```
