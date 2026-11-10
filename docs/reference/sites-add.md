<!-- Generated from rust/src/cli.rs by `make docs`. Do not edit directly. -->

# `foxglove sites add`

[Command reference](README.md) · [Parent command](sites.md)

```text
Create a site

Usage: foxglove sites add [OPTIONS] --name <NAME> --type <TYPE>

Options:
      --client-id <CLIENT_ID>
          Foxglove client ID

      --name <NAME>
          Name of the site

      --config <CONFIG>
          Config file

      --type <TYPE>
          Type of site to create

          [possible values: self-hosted, edge]

      --debug[=<DEBUG>]
          Print debug diagnostics, including HTTP requests, to stderr

          [default: false]

      --retain-recordings-seconds <SECONDS>
          How long an Edge Site keeps the recordings it creates, in seconds; 0 keeps them indefinitely

  -h, --help
          Print help
```
