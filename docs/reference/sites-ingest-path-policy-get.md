<!-- Generated from rust/src/cli.rs by `make docs`. Do not edit directly. -->

# `foxglove sites ingest-path-policy get`

[Command reference](README.md) · [Parent command](sites-ingest-path-policy.md)

```text
Print a site's latest ingest path policy revision as JSON. A saved policy with an empty rules list ignores every path; a site with no saved policy prints nothing on stdout.

Usage: foxglove sites ingest-path-policy get [OPTIONS] <SITE_ID>

Arguments:
  <SITE_ID>


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
