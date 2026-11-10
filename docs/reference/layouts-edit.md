<!-- Generated from rust/src/cli.rs by `make docs`. Do not edit directly. -->

# `foxglove layouts edit`

[Command reference](README.md) · [Parent command](layouts.md)

```text
Update a layout's name, folder, permission, or data

Usage: foxglove layouts edit [OPTIONS] <LAYOUT_ID>

Arguments:
  <LAYOUT_ID>


Options:
      --client-id <CLIENT_ID>
          Foxglove client ID

      --data <FILE>
          JSON file containing the new layout data object

      --config <CONFIG>
          Config file

      --folder-name <FOLDER_NAME>
          New folder for the layout; pass an empty value to remove it from its folder

      --debug[=<DEBUG>]
          Print debug diagnostics, including HTTP requests, to stderr

          [default: false]

      --name <NAME>
          New display name

      --permission <PERMISSION>
          Who can view and edit the layout: ORG_READ, ORG_WRITE, or CREATOR_WRITE (API keys can only use ORG_READ or ORG_WRITE)

      --saved-at <SAVED_AT>
          When this change was saved locally (ISO 8601); the API rejects a time older than the layout's last save. Defaults to now when signed in with `foxglove auth login`; omitted for API keys

  -h, --help
          Print help
```
