<!-- Generated from rust/src/cli.rs by `make docs`. Do not edit directly. -->

# `foxglove sites edit`

[Command reference](README.md) · [Parent command](sites.md)

```text
Edit a site's name, retention period, or URL

Usage: foxglove sites edit [OPTIONS] <SITE_ID>

Arguments:
  <SITE_ID>


Options:
      --client-id <CLIENT_ID>
          Foxglove client ID

      --name <NAME>
          New name of the site

      --config <CONFIG>
          Config file

      --retain-recordings-seconds <SECONDS>
          How long an Edge Site keeps the recordings it creates, in seconds; 0 keeps them indefinitely

      --debug[=<DEBUG>]
          Print debug diagnostics, including HTTP requests, to stderr

          [default: false]

      --url <URL>
          URL a self-hosted Primary Site exposes for accessing its data

  -h, --help
          Print help
```
