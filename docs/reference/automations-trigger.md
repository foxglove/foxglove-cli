<!-- Generated from rust/src/cli.rs by `make docs`. Do not edit directly. -->

# `foxglove automations trigger`

[Command reference](README.md) · [Parent command](automations.md)

```text
Get, set, or delete the trigger that starts an automation

Usage: foxglove automations trigger [OPTIONS] <COMMAND>

Commands:
  delete  Delete an automation's trigger, so it runs only on demand
  get     Get an automation's trigger
  set     Replace an automation's trigger
  help    Print this message or the help of the given subcommand(s)

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

- [`foxglove automations trigger delete`](automations-trigger-delete.md)
- [`foxglove automations trigger get`](automations-trigger-get.md)
- [`foxglove automations trigger set`](automations-trigger-set.md)
