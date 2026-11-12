<!-- Generated from rust/src/cli.rs by `make docs`. Do not edit directly. -->

# `foxglove automations`

[Command reference](README.md) · [Parent command](README.md)

```text
Automations run an AI agent on your data on a schedule, when an event occurs, or on demand. Each automation acts as its owner, with the owner's access to data in the automation's project.

Signed-in members and personal access tokens can create automations. Who can edit, delete, or run an automation depends on its permissions, which `automations get` shows: the owner and organization admins can delete it. Organization API keys can list automations and their runs, and start runs that use only the automation's saved instructions, but can't edit or delete automations.

Automations are in beta.

Usage: foxglove automations [OPTIONS] <COMMAND>

Commands:
  add      Create an automation
  delete   Delete an automation and its trigger
  edit     Edit an automation's name or instructions
  get      Get an automation
  list     List a project's automations, newest first, with their run counts
  runs     Start, list, get, and delete automation runs
  trigger  Get, set, or delete the trigger that starts an automation
  help     Print this message or the help of the given subcommand(s)

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

## Subcommands

- [`foxglove automations add`](automations-add.md)
- [`foxglove automations delete`](automations-delete.md)
- [`foxglove automations edit`](automations-edit.md)
- [`foxglove automations get`](automations-get.md)
- [`foxglove automations list`](automations-list.md)
- [`foxglove automations runs`](automations-runs.md)
- [`foxglove automations trigger`](automations-trigger.md)
