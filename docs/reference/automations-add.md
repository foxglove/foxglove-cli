<!-- Generated from rust/src/cli.rs by `make docs`. Do not edit directly. -->

# `foxglove automations add`

[Command reference](README.md) · [Parent command](automations.md)

```text
Create an automation

Usage: foxglove automations add [OPTIONS] --action-prompt <ACTION_PROMPT> --name <NAME>

Options:
      --action-prompt <ACTION_PROMPT>
          Instructions the agent follows on every run

      --client-id <CLIENT_ID>
          Foxglove client ID

      --config <CONFIG>
          Config file

      --name <NAME>
          Display name (at most 100 characters)

      --debug[=<DEBUG>]
          Print debug diagnostics, including HTTP requests, to stderr

          [default: false]

      --project-id <PROJECT_ID>
          Project ID (required; defaults to FOXGLOVE_DEFAULT_PROJECT_ID, then saved default_project_id; cannot be empty)

  -h, --help
          Print help
```
