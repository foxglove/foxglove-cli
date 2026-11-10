<!-- Generated from rust/src/cli.rs by `make docs`. Do not edit directly. -->

# `foxglove sites delete`

[Command reference](README.md) · [Parent command](sites.md)

```text
Delete a site. Deletion is permanent: recordings stored at the site are no longer available through Foxglove. Shut down the site's deployment before deleting it. Files in the site's own storage are not removed.

Usage: foxglove sites delete [OPTIONS] <SITE_ID>

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
