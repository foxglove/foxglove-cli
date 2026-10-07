# v2 release checklist

These are release gates, not claims that publication or cross-platform validation
has already happened.

Release notes live in [GitHub Releases](https://github.com/foxglove/foxglove-cli/releases).
For every release tag, CI builds and checks the artifacts, then creates a draft
release with the six binaries, checksums, and GitHub-generated notes. A maintainer
reviews and publishes the draft in GitHub.

- [ ] Merge all pre-v2 changes, including #212, then review the migration guide
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
- [ ] In a release-version PR, set Cargo package/lock versions for the next
  `v2.0.0-rc.N`, then push that tag with the final interface. Leave existing
  `v1.0.34-rc.*` tags intact. Wait for CI to prepare its draft release.
- [ ] In the GitHub release editor, select `v1.0.33` as the previous tag and
  regenerate the notes so they cover the full v1-to-v2 change. Review the
  generated entries, then add a short summary of user benefits and a link to
  the migration guide at this release's tag. Publish the RC as a prerelease
  and test its downloadable artifacts.
- [ ] After RC acceptance, set Cargo package/lock versions to `2.0.0` in a
  release-version PR, then push `v2.0.0`. Review the draft's artifacts and
  regenerate notes from `v1.0.33` again, rather than from the last RC. Add the
  benefits summary and migration-guide link, then publish the stable release
  and mark it as latest.

For later stable releases, select the previous stable release as the comparison
tag so the notes include changes already shipped in RCs. GitHub generates entries
from merged PR titles, authors, and links; it does not extract `### Changelog`
paragraphs. Write PR titles that describe the change for users and edit generated
entries before publishing when needed.

Additive conveniences (output aliases, structured output extensions, and
`--wait`) are deferred.
