<!-- Generated from rust/src/cli.rs by `make docs`. Do not edit directly. -->

# `foxglove automations delete`

[Command reference](README.md) · [Parent command](automations.md)

```text
Delete an automation, its trigger, and its runs. Each run's chat is deleted with the run, including a run that is still in progress.

Usage: foxglove automations delete [OPTIONS] <AUTOMATION_ID>

Arguments:
  <AUTOMATION_ID>


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
