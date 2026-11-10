<!-- Generated from rust/src/cli.rs by `make docs`. Do not edit directly. -->

# `foxglove sites`

[Command reference](README.md) · [Parent command](README.md)

```text
List and manage sites

Usage: foxglove sites [OPTIONS] <COMMAND>

Commands:
  add                 Create a site
  delete              Delete a site
  edit                Edit a site's name, retention period, or URL
  get                 Get a site
  ingest-path-policy  Get or save a site's ingest path policy
  list                List sites
  notify-bucket       Notify a Primary Site that an object in its bucket was created or removed
  help                Print this message or the help of the given subcommand(s)

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

- [`foxglove sites add`](sites-add.md)
- [`foxglove sites delete`](sites-delete.md)
- [`foxglove sites edit`](sites-edit.md)
- [`foxglove sites get`](sites-get.md)
- [`foxglove sites ingest-path-policy`](sites-ingest-path-policy.md)
- [`foxglove sites list`](sites-list.md)
- [`foxglove sites notify-bucket`](sites-notify-bucket.md)
