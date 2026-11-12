<!-- Generated from rust/src/cli.rs by `make docs`. Do not edit directly. -->

# `foxglove projects members add`

[Command reference](README.md) · [Parent command](projects-members.md)

```text
Give organization members access to a project. Members with the user role need this access to use an invite-only project. Organization admins can access every project, and members can access a project that is not invite-only without being added.

Every ID must belong to a member of the organization; if one is unknown, no members are added. Members who already have access are left unchanged. Requires an organization admin, or an API key with the projectMembers.update capability for the project.

Usage: foxglove projects members add [OPTIONS] <PROJECT_ID> <MEMBER_ID>...

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
