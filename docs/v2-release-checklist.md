# v2 release checklist

These are release gates, not claims that publication or cross-platform validation
has already happened. Merge the command, timestamp, and project-default PRs
before cutting an RC with these notes. Current drafts: [commands #175](https://github.com/foxglove/foxglove-cli/pull/175),
[timestamps #174](https://github.com/foxglove/foxglove-cli/pull/174), and
[project defaults #176](https://github.com/foxglove/foxglove-cli/pull/176).

- [ ] Merge all pre-v2 changes and review the migration guide against final help.
- [ ] Run Rust tests (including ignored HTTP tests), lint, docs, audit, Go
  compatibility, and native build/package checks in CI. Keep the frozen Go
  baseline and historical goldens unchanged.
- [ ] Exercise migration examples, removed-path rejection, existing credentials,
  project overrides, fractional boundaries, and edge transfer status.
- [ ] Verify six native artifacts, checksums, embedded versions, and Linux
  distribution smoke tests. The release tag/`FOXGLOVE_VERSION` is authoritative;
  Cargo's package version supplies the untagged build fallback.
- [ ] Update public [CLI/data upload examples](https://docs.foxglove.dev/docs/data/importing-data),
  [exports](https://docs.foxglove.dev/docs/data/exporting-data),
  [recordings](https://docs.foxglove.dev/docs/data/recordings), and
  [edge workflows](https://docs.foxglove.dev/docs/data/edge-sites/manage-data).
  Replace `data` paths, explain transfer initiation/status, and ensure recording
  deletion is documented as available. A public-docs PR is still needed.
- [ ] Publish and test a new `v2.0.0-rc.N` with the final interface. Leave existing
  `v1.0.34-rc.*` tags intact. Review curated notes for the full v1.0.33→v2 change;
  do not use only commits since the preceding RC.
- [ ] After RC acceptance, set Cargo package/lock versions to `2.0.0`, verify the
  final notes, and publish `v2.0.0` through the existing release workflow.

Additive conveniences (new environment names, output aliases, pagination,
structured output extensions, and `--wait`) and retirement of Go CI are deferred.
