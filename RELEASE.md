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

Each matrix job runs the Rust suite natively, performs a locked release build,
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
