<!-- Generated from rust/src/cli.rs by `make docs`. Do not edit directly. -->

# `foxglove automations edit`

[Command reference](README.md) · [Parent command](automations.md)

```text
Edit an automation's name or saved instructions. Runs already created keep the instructions they started with.

Usage: foxglove automations edit [OPTIONS] <AUTOMATION_ID>

Arguments:
  <AUTOMATION_ID>


Options:
      --action-prompt <ACTION_PROMPT>
          New instructions the agent follows on every run

      --client-id <CLIENT_ID>
          Foxglove client ID

      --config <CONFIG>
          Config file

      --name <NAME>
          New display name (at most 100 characters)

      --debug[=<DEBUG>]
          Print debug diagnostics, including HTTP requests, to stderr

          [default: false]

  -h, --help
          Print help (see a summary with '-h')
```
