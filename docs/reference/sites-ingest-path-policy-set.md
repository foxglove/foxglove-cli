<!-- Generated from rust/src/cli.rs by `make docs`. Do not edit directly. -->

# `foxglove sites ingest-path-policy set`

[Command reference](README.md) · [Parent command](sites-ingest-path-policy.md)

```text
Save a site's ingest path policy from a JSON file holding schemaVersion and a rules array; the first matching rule wins, and an empty rules list ignores every path. A new revision is recorded only when the policy changes. Only Bring Your Own Storage sites accept a policy.

Usage: foxglove sites ingest-path-policy set [OPTIONS] <SITE_ID> <FILE>

Arguments:
  <SITE_ID>


  <FILE>
          JSON file with the policy's schemaVersion and rules, or the output of `ingest-path-policy get`

Options:
      --client-id <CLIENT_ID>
          Foxglove client ID

      --previous-revision <REVISION>
          Revision the policy was edited from (default: the revision in a saved `get` output); the save fails if another revision has replaced it (0 when the site has no saved policy)

      --config <CONFIG>
          Config file

      --debug[=<DEBUG>]
          Print debug diagnostics, including HTTP requests, to stderr

          [default: false]

  -h, --help
          Print help (see a summary with '-h')
```
