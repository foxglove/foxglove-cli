<!-- Generated from rust/src/cli.rs by `make docs`. Do not edit directly. -->

# `foxglove layouts add`

[Command reference](README.md) · [Parent command](layouts.md)

```text
Create a layout from a JSON file of layout data

Usage: foxglove layouts add [OPTIONS] --data <FILE> --name <NAME> --permission <PERMISSION>

Options:
      --client-id <CLIENT_ID>
          Foxglove client ID

      --data <FILE>
          JSON file containing the layout data object

      --config <CONFIG>
          Config file

      --folder-name <FOLDER_NAME>
          Folder that organizes the layout in the app; cannot contain a forward slash

      --debug[=<DEBUG>]
          Print debug diagnostics, including HTTP requests, to stderr

          [default: false]

      --id <ID>
          ID for the new layout (at most 36 characters; defaults to a server-generated ID)

      --name <NAME>
          Display name

      --permission <PERMISSION>
          Who can view and edit the layout: ORG_READ, ORG_WRITE, or CREATOR_WRITE (API keys can only use ORG_READ or ORG_WRITE)

      --saved-at <SAVED_AT>
          When the layout was last saved locally (ISO 8601)

  -h, --help
          Print help
```
