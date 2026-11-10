<!-- Generated from rust/src/cli.rs by `make docs`. Do not edit directly. -->

# `foxglove device-tokens add`

[Command reference](README.md) · [Parent command](device-tokens.md)

```text
Create a device token. The token's secret is printed in the output and is not shown again.

Usage: foxglove device-tokens add [OPTIONS]

Options:
      --client-id <CLIENT_ID>
          Foxglove client ID

      --format <FORMAT>
          Render output in table, JSON, or CSV format

          [default: table]
          [possible values: table, json, csv]

      --config <CONFIG>
          Config file

      --device-id <DEVICE_ID>
          Device the token is for; without it, the token can be used to create a device

      --debug[=<DEBUG>]
          Print debug diagnostics, including HTTP requests, to stderr

          [default: false]

      --enabled[=<ENABLED>]
          Create the token enabled or disabled

      --preapproved[=<PREAPPROVED>]
          Whether devices created with the token are preapproved

      --project-id <PROJECT_ID>
          Project ID; with --device-id it must be the device's project and defaults to it, otherwise it defaults to FOXGLOVE_DEFAULT_PROJECT_ID, then saved default_project_id

  -h, --help
          Print help (see a summary with '-h')
```
