<!-- Generated from rust/src/cli.rs by `make docs`. Do not edit directly. -->

# `foxglove upload`

[Command reference](README.md) · [Parent command](README.md)

```text
Upload a local data file to Foxglove. On success, prints the upload request ID to stderr. The file is then imported in the background; see `pending-imports list`.

Usage: foxglove upload [OPTIONS] <FILE>

Arguments:
  <FILE>


Options:
      --client-id <CLIENT_ID>
          Foxglove client ID

      --device-id <DEVICE_ID>
          Device ID

      --config <CONFIG>
          Config file

      --device-name <DEVICE_NAME>
          Device name

      --debug[=<DEBUG>]
          Print debug diagnostics, including HTTP requests, to stderr

          [default: false]

      --key <KEY>
          Recording key

      --project-id <PROJECT_ID>
          Project ID (defaults to FOXGLOVE_DEFAULT_PROJECT_ID, then saved default_project_id; --project-id= bypasses defaults)

      --session-id <SESSION_ID>
          Session ID

      --session-key <SESSION_KEY>
          Session key (cannot be combined with --session-id)

  -h, --help
          Print help (see a summary with '-h')
```
