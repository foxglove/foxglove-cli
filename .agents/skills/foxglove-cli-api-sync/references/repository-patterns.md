# Repository patterns

These pointers were checked against CLI commit `57f6eae`. Read the current files before copying a pattern; their contracts take precedence over this map.

## Where changes belong

| Concern | Starting point |
| --- | --- |
| Clap command hierarchy, argument structs, dispatch, help, completion | `rust/src/cli.rs` |
| Async HTTP client, typed errors, path encoding, transfers | `rust/src/api.rs` |
| Shared config/client and default project | `rust/src/runtime.rs`, `rust/src/config.rs` |
| List records, time parsing, project fallback, truncation warnings | `rust/src/records.rs` |
| JSON/CSV/table rendering and formats | `rust/src/output.rs`, `rust/src/format.rs` |
| Recent list commands and nested response envelopes | `rust/src/datasets.rs`, `rust/src/episodes.rs` |
| Existing mutations | `rust/src/devices.rs`, `rust/src/events.rs`, `rust/src/sessions.rs` |
| Result/exit handling and entry point | `rust/src/lib.rs`, `rust/src/main.rs` |
| HTTP subprocess tests and isolated local servers | `rust/tests/cli_integration.rs`, `rust/tests/support/mod.rs` |
| Build and checks | `Makefile`, `rust/README.md`, `.github/workflows/ci.yaml` |

## CLI conventions

New behavior needs focused Rust regression coverage. Extend existing tests when they exercise the changed behavior; a separate test is not required for each field. Serde-only response-model additions normally belong in existing model serialization tests, not new HTTP subprocess tests.

Maintain normal Clap definitions in `cli.rs`; help and `clap_complete` derive from that tree. Follow neighboring command verbs (`list`, `add`, etc.), plural resource groups, kebab-case flags, positional IDs, value hints, and the existing explicit-boolean syntax. Wire new commands through dispatch and module declarations as well as parsing. Validate incompatible arguments before sending a request.

Preserve the existing structured `Outcome` flow. Keep machine-readable data on stdout and diagnostics/progress on stderr; retain exit codes, broken-pipe handling, and cancellation behavior. Do not add interactive prompts to otherwise scriptable commands without a specific need and a noninteractive path.

List operations currently use shared default limits and truncation warnings. Match the endpoint's envelope and pagination semantics; do not quietly turn a one-page command into an unbounded fetch. Preserve project fallback and distinguish an omitted flag from an explicit value where meaningful.

## Rust and wire correctness

- Respect `rust-toolchain.toml`, the manifest's MSRV, `unsafe_code = "forbid"`, and Clippy pedantic checks. Keep dependencies pinned with existing feature conventions; add one only with a concrete need. Do not suppress lints broadly or use panics for user/network input.
- Reuse `Runtime.client` and await operations on the existing Tokio runtime. Preserve typed `ApiError` context, HTTP status handling, cancellation, and response cleanup. Do not create a parallel HTTP/config stack or introduce blanket mutation retries.
- Encode dynamic IDs with `encode_path_segment` and preserve existing rejection of invalid path segments. Pass queries separately with Serde wire names; never interpolate raw user IDs or query values into URLs.
- Distinguish omitted, null, empty, false, and zero on requests, especially PATCH-like updates. `Option<T>` alone may not express both omitted and explicit null. On responses, use defaults or `null_to_default` only when the contract and output semantics justify them, not to hide a malformed required field. Handle evolving enums without unnecessarily breaking response decoding.
- Check time helpers before reusing them: preserve the precision required by the endpoint and output format. Do not silently lose precision in a new contract, or alter all existing commands to solve a local requirement.
- Reuse `Record`/renderers for list output; cover new fields in the formats required by the requested behavior and preserve JSON wire names. Update manual headers and fields for CSV/table additions; preserve existing summaries for nested JSON-only metadata. Preserve missing optional expansions versus empty collections where that distinction matters.
- Keep API bearer tokens off presigned storage requests and diagnostics. Reuse transfer staging, cleanup, and cancellation paths for file operations.

## Validation

Read current CI and Make targets; run commands from the repository root. At the checked revision the main checks are:

```sh
make rust-lint
make rust-test-ignored
make rust-doc
make rust-audit
make build
./rust/target/release/foxglove-rust --help
```

`rust-test-ignored` runs the complete Rust suite including deliberately ignored loopback HTTP tests; ordinary `cargo test` omits them. `rust-audit` requires the auditor version pinned in the development instructions; retain only the repository's reviewed advisory exception. If prerequisites, network, loopback permissions, or platform dependencies block a check, report the actual limitation and keep completed checks useful.

Run affected command help and completion checks when the command tree changes. When request or HTTP behavior changes, use the existing local test support to check affected serialization, responses, and output streams. Avoid duplicating unchanged HTTP behavior for a model-only change. Keep contract expectations independent of implementation helpers when sharing them would conceal the same bug in both. Rely on CI for the full native platform matrix; do not claim local testing covers other platforms.
