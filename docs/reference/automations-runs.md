<!-- Generated from rust/src/cli.rs by `make docs`. Do not edit directly. -->

# `foxglove automations runs`

[Command reference](README.md) · [Parent command](automations.md)

```text
Start, list, and get automation runs

Usage: foxglove automations runs [OPTIONS] <COMMAND>

Commands:
  add   Start a run of an automation
  get   Get a run, including its prompts and final answer
  list  List a project's automation runs, newest first by default
  help  Print this message or the help of the given subcommand(s)

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

- [`foxglove automations runs add`](automations-runs-add.md)
- [`foxglove automations runs get`](automations-runs-get.md)
- [`foxglove automations runs list`](automations-runs-list.md)
