<!-- Generated from rust/src/cli.rs by `make docs`. Do not edit directly. -->

# `foxglove org-members list`

[Command reference](README.md) · [Parent command](org-members.md)

```text
List the members of your organization. Requires an API key with the orgMembers.list capability that is not limited to specific projects.

Usage: foxglove org-members list [OPTIONS]

Options:
      --client-id <CLIENT_ID>
          Foxglove client ID

      --format <FORMAT>
          Render output in table, JSON, or CSV format

          [default: table]
          [possible values: table, json, csv]

      --config <CONFIG>
          Config file

      --limit <LIMIT>
          Maximum number of items to return (1-2000, default: 50)

      --debug[=<DEBUG>]
          Print debug diagnostics, including HTTP requests, to stderr

          [default: false]

      --offset <OFFSET>
          Number of items to skip before returning the results

      --project-id <PROJECT_ID>
          Filter by project ID; the default project does not apply

      --search <SEARCH>
          Filter by name or email

      --role <ROLE>
          Filter by role: admin or user

      --status <STATUS>
          Filter by status: active, invited, pending-approval, or denied-approval

      --sort-by <SORT_BY>
          Field to sort members by: displayName, email, role, lastSeenAt, createdAt, status, or projectCount

      --sort-order <SORT_ORDER>
          Sort order for the --sort-by field: asc or desc

  -h, --help
          Print help (see a summary with '-h')
```
