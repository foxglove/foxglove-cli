<!-- Generated from rust/src/cli.rs by `make docs`. Do not edit directly. -->

# `foxglove projects members remove`

[Command reference](README.md) · [Parent command](projects-members.md)

```text
Remove organization members' access to a project. Unknown IDs and members without access are left unchanged. Requires an organization admin, or an API key with the projectMembers.update capability for the project.

Usage: foxglove projects members remove [OPTIONS] <PROJECT_ID> <MEMBER_ID>...

Arguments:
  <PROJECT_ID>


  <MEMBER_ID>...
          Organization member IDs (at most 50)

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
