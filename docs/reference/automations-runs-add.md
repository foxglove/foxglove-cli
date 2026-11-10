<!-- Generated from rust/src/cli.rs by `make docs`. Do not edit directly. -->

# `foxglove automations runs add`

[Command reference](README.md) · [Parent command](automations-runs.md)

```text
Start a run of an automation. The run is created pending and executes as the automation's owner, with the owner's data access and AI usage; follow it with `automations runs get`.

Signed-in members and personal access tokens must give --run-prompt. Organization API keys must omit it; their runs use only the automation's saved instructions.

Usage: foxglove automations runs add [OPTIONS] <AUTOMATION_ID>

Arguments:
  <AUTOMATION_ID>


Options:
      --client-id <CLIENT_ID>
          Foxglove client ID

      --run-prompt <RUN_PROMPT>
          Instructions for this run, which may reference recordings or other resources (required for members and personal access tokens; omit with an organization API key)

      --config <CONFIG>
          Config file

      --debug[=<DEBUG>]
          Print debug diagnostics, including HTTP requests, to stderr

          [default: false]

  -h, --help
          Print help (see a summary with '-h')
```
