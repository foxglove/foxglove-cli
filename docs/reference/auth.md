<!-- Generated from rust/src/cli.rs by `make docs`. Do not edit directly. -->

# `foxglove auth`

[Command reference](README.md) · [Parent command](README.md)

```text
Manage authentication

Usage: foxglove auth [OPTIONS] <COMMAND>

Commands:
  configure-api-key  Configure an API key
  info               Display information about the currently authenticated user
  login              Log in to Foxglove Data Platform
  help               Print this message or the help of the given subcommand(s)

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

- [`foxglove auth configure-api-key`](auth-configure-api-key.md)
- [`foxglove auth info`](auth-info.md)
- [`foxglove auth login`](auth-login.md)
