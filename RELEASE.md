# Releasing the CLI

## Build and publish

Pull requests and pushes to `main` run lint, tests, dependency auditing, and
native packaging for Linux, macOS, and Windows on amd64 and arm64. Require all
six `Test and package Rust` jobs to pass on the reviewed commit before tagging.
Download their workflow artifacts for the checks below; these builds do not
publish releases.

To package locally, run on the matching OS and architecture:

```sh
make release-package PLATFORM=macos ARCH=arm64
```

`scripts/package-rust-release.sh` is shared by this target and CI. It performs
a locked release build, strips Linux binaries, checks `--help`, and writes a
binary and SHA-256 file to `dist/` (override with `RELEASE_DIR`). Set
`FOXGLOVE_VERSION` to embed and verify a specific version.

Publish a `v*` tag on the tested commit to trigger a GitHub release. For an RC,
use a prerelease tag such as `v1.0.34-rc.1`: tags containing `-` are published
as prereleases and do not replace the latest stable release. Review the commits
since the previous tag, which become the generated release notes.

The tag workflow repeats all gates, verifies the embedded version, and publishes
`foxglove-{linux,macos,windows}-{amd64,arm64}` (with `.exe` on Windows), plus
`CHECKSUMS.txt`. No Go binary is published. RC users should download from the
specific release page; the README's `/latest/` URLs continue to serve stable
releases.

## Candidate smoke checks

Record results and the workflow URL in the candidate's release notes. Fixture
tests do not replace testing packaged binaries against staging:

- Verify checksums (`sha256sum -c CHECKSUMS.txt` on Linux or
  `shasum -a 256 -c CHECKSUMS.txt` on macOS, with all assets downloaded). On each
  target OS/architecture, including the oldest supported OS, run `--help`,
  `version`, and an authenticated read. If an older Linux loader rejects the
  binary, rebuild against a compatible libc baseline and retest.
- Log in with an isolated configuration:
  `foxglove --config rc-test.yaml auth login --base-url <staging-api-url>`.
  Use that configuration for subsequent commands. Check JSON list output for
  pending imports without devices/errors, edge recordings not yet imported,
  and event types without colors.
- Run `events list --query <known-query> --query-field metadata --query-field
  properties --format json` against data with matches in each field. Export
  cross-package ROS messages with `data export --recording-id <id>
  --output-format json` and check nested values.
- Import a known-good BZ2 bag and a recording with a record/chunk larger than
  64 MiB into a disposable staging project. Confirm upload and server-side
  completion via `pending-imports list` / `recordings list`.
- Press Ctrl-C during export to an existing `--output-file`: expect exit 130,
  unchanged contents, and no `.foxglove-export-*` directory. Cancel browser
  login and check that existing credentials survive. Include real Windows
  terminals; automated subprocess signal tests cover Unix only.
- Test a private API or TLS-inspecting proxy using an organization's installed
  CA, then an explicit PEM bundle via `SSL_CERT_FILE` / `SSL_CERT_DIR`. These
  overrides replace native roots; include CAs for API and signed-storage hosts.

## Dependency maintenance

Review dependency changes in `rust/Cargo.toml` and `rust/Cargo.lock`, including
licenses, and run `make rust-audit` before release. The one accepted advisory
exception is `RUSTSEC-2024-0436`: unmaintained `paste 1.0.15`, a build-time
procedural macro dependency of `mcap 0.25.0`, with no fixed version at review.
Reassess the exception when upgrading `mcap`; other advisories still fail CI.
Behavior changes belong in the [compatibility contract](compat/README.md).
