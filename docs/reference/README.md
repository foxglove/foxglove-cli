<!-- Generated from rust/src/cli.rs by `make docs`. Do not edit directly. -->

# `foxglove`

[Documentation](../README.md) · [Getting started](../../README.md#getting-started)

This reference describes the commands in this checkout. For an installed release, use `foxglove <command> --help` to see its supported options.

```text
Command line client for the Foxglove data platform

Usage: foxglove [OPTIONS] [COMMAND]

Commands:
  attachments      Query and modify data attachments
  auth             Manage authentication
  completion       Generate a shell completion script
  config           Manage CLI configuration values
  coverage         Inspect data coverage
  datasets         List and manage datasets
  devices          List and manage devices
  episodes         List and manage episodes
  event-types      List event types
  events           List and manage events
  export           Export data by recording, session, episode, or device and time range
  extensions       List and publish Studio extensions
  pending-imports  List pending imports
  projects         List and manage projects
  recordings       Query recordings
  sessions         List and manage sessions
  topics           List topics
  upload           Upload a local data file to Foxglove
  version          Print Foxglove CLI version
  help             Print this message or the help of the given subcommand(s)

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

- [`foxglove attachments`](attachments.md)
- [`foxglove auth`](auth.md)
- [`foxglove completion`](completion.md)
- [`foxglove config`](config.md)
- [`foxglove coverage`](coverage.md)
- [`foxglove datasets`](datasets.md)
- [`foxglove devices`](devices.md)
- [`foxglove episodes`](episodes.md)
- [`foxglove event-types`](event-types.md)
- [`foxglove events`](events.md)
- [`foxglove export`](export.md)
- [`foxglove extensions`](extensions.md)
- [`foxglove pending-imports`](pending-imports.md)
- [`foxglove projects`](projects.md)
- [`foxglove recordings`](recordings.md)
- [`foxglove sessions`](sessions.md)
- [`foxglove topics`](topics.md)
- [`foxglove upload`](upload.md)
- [`foxglove version`](version.md)
