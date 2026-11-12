<!-- Generated from rust/src/cli.rs by `make docs`. Do not edit directly. -->

# `foxglove automations edit`

[Command reference](README.md) · [Parent command](automations.md)

```text
Edit an automation's name, saved instructions, or model family. Runs already created keep the instructions they started with.

Usage: foxglove automations edit [OPTIONS] <AUTOMATION_ID>

Arguments:
  <AUTOMATION_ID>


Options:
      --action-prompt <ACTION_PROMPT>
          New instructions the agent follows on every run

      --client-id <CLIENT_ID>
          Foxglove client ID

      --clear-model-selection
          Clear the saved model family, so runs use the organization's default model

      --config <CONFIG>
          Config file

      --debug[=<DEBUG>]
          Print debug diagnostics, including HTTP requests, to stderr

          [default: false]

      --model-selection <MODEL_SELECTION>
          New model family for the automation's runs: auto (the recommended model), sonnet, or opus (the current version of that family)

      --name <NAME>
          New display name (at most 100 characters)

  -h, --help
          Print help (see a summary with '-h')
```
