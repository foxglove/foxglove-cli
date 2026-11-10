<!-- Generated from rust/src/cli.rs by `make docs`. Do not edit directly. -->

# `foxglove automations delete`

[Command reference](README.md) · [Parent command](automations.md)

```text
Delete an automation and its trigger. Its runs are no longer listed. Runs in progress finish, and runs that have not started are rejected.

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
