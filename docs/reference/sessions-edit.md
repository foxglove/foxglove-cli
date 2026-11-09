<!-- Generated from rust/src/cli.rs by `make docs`. Do not edit directly. -->

# `foxglove sessions edit`

[Command reference](README.md) · [Parent command](sessions.md)

```text
Change a session's key or custom properties

Usage: foxglove sessions edit [OPTIONS] <--key <KEY>|--remove-key|--property <PROPERTY>|--remove-property <REMOVE_PROPERTY>> <SESSION_ID_OR_KEY>

Arguments:
  <SESSION_ID_OR_KEY>


Options:
      --client-id <CLIENT_ID>
          Foxglove client ID

      --project-id <PROJECT_ID>
          Project ID (defaults to FOXGLOVE_DEFAULT_PROJECT_ID, then saved default_project_id; --project-id= bypasses defaults)

      --config <CONFIG>
          Config file

      --key <KEY>
          New session key, unique within the project (cannot start or end with whitespace)

      --debug[=<DEBUG>]
          Print debug diagnostics, including HTTP requests, to stderr

          [default: false]

      --remove-key
          Remove the existing session key

  -p, --property <PROPERTY>
          Custom property colon-separated key/value pair; repeat a multi-enum key to set its full list of values

      --remove-property <REMOVE_PROPERTY>
          Remove a property assignment; repeat for more keys

  -h, --help
          Print help
```
