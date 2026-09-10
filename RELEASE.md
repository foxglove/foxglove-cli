# Release candidate checklist

The release assets are native builds of the Rust CLI. The Go CLI is retained
only as the compatibility oracle and must never be substituted for a failed
Rust release build.

## Before tagging

- [ ] Start from a clean, reviewed commit and confirm the Rust version output
  matches the tag to be published.
- [ ] Run `make lint`, `make test`, `make compat`, and `make rust-test-ignored`.
- [ ] Run `cargo build --manifest-path rust/Cargo.toml --locked --release` and
  smoke-test `rust/target/release/foxglove-rust --help`.
- [ ] Review `MIGRATION.md` for any newly approved behavioral deltas and update
  compatibility goldens only through the documented Go-oracle workflow.
- [ ] Review the dependency lockfile and RustSec advisory status; record any
  accepted exception in `MIGRATION.md`.

## Native builds before tagging

Pull requests to main and pushes to main run the same six native test/package
jobs used for tags. Each checkout downloads the LFS format fixtures. Require all
six `Test and package Rust` jobs to pass before tagging; download their workflow
artifacts for platform testing. PR builds do not publish a GitHub release.

## Tag build and publication

Publishing a `v*` tag builds exactly these assets on matching native runners:

| Platform | Asset |
| --- | --- |
| Linux x86_64 | `foxglove-linux-amd64` |
| Linux arm64 | `foxglove-linux-arm64` |
| macOS x86_64 | `foxglove-macos-amd64` |
| macOS arm64 | `foxglove-macos-arm64` |
| Windows x86_64 | `foxglove-windows-amd64.exe` |
| Windows arm64 | `foxglove-windows-arm64.exe` |

Each matrix job runs the Rust suite, including loopback contracts, natively, performs a locked release build,
strips Linux binaries, runs `--help`, and uploads an individual SHA-256 record.
The publish job verifies them and uploads `CHECKSUMS.txt` with the six binaries.
Tags containing a
SemVer prerelease suffix (for example `v1.0.34-rc.1`) are published as GitHub
prereleases and are not marked latest; ordinary version tags are marked latest.

## Candidate verification

- [ ] Confirm all six matrix jobs ran on their named native runner. A missing
  hosted ARM runner is a release blocker, not a reason to cross-compile an
  untested artifact.
- [ ] Download every asset and validate it against `CHECKSUMS.txt` (`shasum -a
  256 -c CHECKSUMS.txt` on macOS or `sha256sum -c CHECKSUMS.txt` on Linux).
- [ ] On each corresponding platform, run `foxglove --help`, `foxglove
  version`, and one authenticated read-only command against a staging account.
- [ ] Verify installation URLs in the README point at the expected binary name
  and that Windows assets retain the `.exe` suffix.
- [ ] Review the commits that will become the generated release notes before
  tagging, then retain the completed workflow URL and checksum file with the
  automatically published release record.

## Staging checks requiring another environment

The local suite uses fixture servers. Before inviting users to test, run these
checks with the packaged binary on each target OS/architecture, including the
oldest OS version you intend to support:

1. Download the matching workflow artifact, validate its `.sha256` file (or
   `CHECKSUMS.txt` after tagging), and run `foxglove --help` and `foxglove version`.
   A loader failure on an older Linux system requires rebuilding against a
   compatible libc baseline and retesting that artifact.
2. Authenticate into a disposable staging configuration:
   `foxglove --config rc-test.yaml auth login --base-url <staging-api-url>`.
   Run `projects list`, `pending-imports list --format json`,
   `recordings list --format json`, and `event-types list --format json` using
   `--config rc-test.yaml`. Include pending imports without a device or error,
   edge recordings not yet imported, and event types without a color.
3. Run `events list --query <known-query> --query-field metadata --query-field
   properties --format json` against staging data where the fields produce
   different matches. Confirm both fields are searched. Export a recording
   with cross-package ROS messages using `data export --recording-id <id>
   --output-format json` and check the nested values.
4. Import a known-good BZ2 bag and a recording containing a record/chunk larger
   than 64 MiB into a disposable staging project. Confirm upload begins without
   a full decode pass and follow `pending-imports list` / `recordings list` to
   verify server-side completion. The CLI's signature check is not a guarantee
   that the server accepts a particular encoding.
5. During a long export to an existing `--output-file`, press Ctrl-C. Expect
   exit 130, unchanged destination contents, and no `.foxglove-export-*`
   directory. Repeat during browser login and check that existing credentials
   remain unchanged. Test actual Ctrl-C delivery in Windows terminals as well;
   the Rust tests exercise cancellation tokens there but the Go subprocess
   signal test is Unix-only.
6. On a machine with your organization's CA already installed, use the CLI
   through its normal TLS-inspecting proxy or private API endpoint. Also test an
   explicit PEM trust bundle via `SSL_CERT_FILE` (and optionally `SSL_CERT_DIR`).
   These environment variables override native roots; include every CA needed
   by API and signed-storage endpoints. Do not disable certificate verification.

Keep these checks in the candidate's release record. Native hosted runners,
real staging imports, actual Windows Ctrl-C delivery, and corporate OS trust
configuration cannot be established by a macOS-only fixture run.
