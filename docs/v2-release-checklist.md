# v2 release checklist

These are release gates, not claims that publication or cross-platform validation
has already happened.

- [ ] Merge all pre-v2 changes, then review the migration guide and release notes
  against final help and every `### Changelog` entry since v1.0.33.
- [ ] Run Rust tests (including ignored HTTP tests), lint, docs, audit, and native
  build/package checks in CI.
- [ ] Exercise migration examples, removed-path and removed-flag rejection,
  existing credentials, `FOXGLOVE_` environment variables and legacy-name
  warnings, project overrides, list paging and the JSON envelope, fractional
  boundaries, and edge transfer status.
- [ ] Verify six native artifacts, checksums, embedded versions, and Linux
  distribution smoke tests. The release tag/`FOXGLOVE_VERSION` is authoritative;
  Cargo's package version supplies the untagged build fallback.
- [ ] Update public [CLI/data upload examples](https://docs.foxglove.dev/docs/data/importing-data),
  [exports](https://docs.foxglove.dev/docs/data/exporting-data),
  [recordings](https://docs.foxglove.dev/docs/data/recordings), and
  [edge workflows](https://docs.foxglove.dev/docs/data/edge-sites/manage-data).
  Replace `data` paths, explain transfer initiation/status, and ensure recording
  deletion is documented as available. Publish the docs updates when CLI v2 is
  available.
- [ ] Publish and test a new `v2.0.0-rc.N` with the final interface. Leave existing
  `v1.0.34-rc.*` tags intact. Review curated notes for the full v1.0.33→v2 change;
  do not use only commits since the preceding RC.
- [ ] After RC acceptance, set Cargo package/lock versions to `2.0.0`, verify the
  final notes, and publish `v2.0.0` through the existing release workflow.

Additive conveniences (output aliases, structured output extensions, and
`--wait`) are deferred.
