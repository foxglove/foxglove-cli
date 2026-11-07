<!-- Generated from rust/src/cli.rs by `make docs`. Do not edit directly. -->

# `foxglove datasets add`

[Command reference](README.md) · [Parent command](datasets.md)

```text
Create a dataset

Usage: foxglove datasets add [OPTIONS] --name <NAME>

Options:
      --client-id <CLIENT_ID>
          Foxglove client ID

      --description <DESCRIPTION>
          Optional display description

      --config <CONFIG>
          Config file

      --episode-id <EPISODE_ID>
          Episode to add to the dataset; repeat to add more

      --debug[=<DEBUG>]
          Print debug diagnostics, including HTTP requests, to stderr

          [default: false]

      --name <NAME>
          Display name, unique within the project

      --project-id <PROJECT_ID>
          Project ID (required; defaults to FOXGLOVE_DEFAULT_PROJECT_ID, then saved default_project_id; cannot be empty)

  -h, --help
          Print help
```
