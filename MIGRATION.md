# Foxglove CLI Rust migration

The compatibility baseline is the released Go CLI at tag **v1.0.33**. The Go
implementation remains runnable as the oracle until the Rust release candidate
passes every gate. Unlisted behavior changes are not permitted.

## Phase status

| Phase | Deliverable | Status | Gate |
| --- | --- | --- | --- |
| 0 | Oracle, tracker, command surface, wire fixtures, approved deltas | Complete | `make compat` and existing Go tests pass from a clean checkout |
| 1 | Rust project and CLI/config/output core | Complete | Offline CLI contract passes |
| 2 | Async API client, errors, streaming, cancellation | Complete | Wire contract and cancellation tests pass |
| 3 | Read-only commands | Complete | Read command parity passes |
| 4 | Auth, configuration, mutations, uploads/downloads | Complete | Mutation and transfer parity passes |
| 5 | MCAP, ROS 1 bag, ROS 1 message, protobuf foundation | Complete | Format conformance passes |
| 6 | Direct and JSON export | Complete | Direct byte and JSON parity passes |
| 7 | Resumable export, reindex, merge | Not started | Resilient export conformance passes |
| 8 | Six-platform prerelease and cutover | Not started | Release candidate is approved |

## Compatibility rules

- Match command hierarchy, flags, arguments, defaults, validation, help,
  stdout, stderr, exit status, config, wire requests, and file behavior.
- Rewritten MCAP and ROS bag files are compared semantically rather than byte
  for byte. Completion scripts may differ textually but must behave the same.
- Normalize only nondeterministic fixture ports, temporary paths, progress
  timing, and runtime stack traces.
- Golden updates require review and an explanation in this file.
- Every new direct runtime crate requires a dependency-ledger entry before it
  lands.

## Command and coverage matrix

`compat/goldens/v1.0.33/command_surface.json` captures help for every command
below. `wire_contract.json` captures representative authenticated request and
response behavior; existing focused Go tests remain part of the baseline.

| Area | Commands | Phase | Phase 0 contract |
| --- | --- | --- | --- |
| Root | root, help, version, completion bash/fish/powershell/zsh | 1 | Help/version goldens |
| Config/auth | config get/set/unset, auth configure-api-key/info/login | 1, 4 | Help, config mutation, auth-info wire; login unit tests |
| Devices/projects | devices list/add/edit, projects list | 3, 4 | Help plus GET/POST/PATCH wire fixtures |
| Recordings/imports | recordings list/delete, data imports list/add, data import | 3, 4 | Help, list/delete/edge-import wire; upload unit tests |
| Coverage/topics | data coverage list, topics list | 3 | Help plus query wire fixtures |
| Attachments | attachments list/download | 3, 4 | Help, success/error wire, approved deltas |
| Events/types | events list/add, event-types list | 3, 4 | Help plus GET/POST wire fixtures |
| Sessions | sessions list/get/add/delete, recordings list/add/remove | 3, 4 | Help plus GET/POST/PATCH/DELETE wire fixtures |
| Extensions | extensions list/publish/unpublish | 3, 4 | Help, list/delete wire; publish unit tests |
| Pending imports | pending-imports list | 3 | Help plus query wire fixture |
| Export | data export | 6, 7 | Help, initial-failure delta, existing format tests |

## Approved Rust behavior changes

The machine-readable expected results live in `compat/approved_deltas.json`.

| ID | Approved behavior |
| --- | --- |
| `attachments-list-error` | Name attachments rather than imports in the failure prefix. |
| `attachment-download-404` | Reject non-2xx before writing stdout and use shared API error mapping. |
| `devices-empty-csv` | Emit a header-only CSV and exit successfully. This applies to every list response type. |
| `device-completion-empty-filter` | Request `/v1/devices` without an empty query string when dynamic completion has no project filter. |
| `export-initial-download-error` | Fail immediately, keep stdout clean, remove partials, preserve the destination. |
| `http-response-lifecycle` | Consume or drop every HTTP response on all branches. |
| `transfer-cancellation` | Cancel active work, clean staging files, preserve destinations, and exit 130 on Ctrl-C. |
| `clap-parser-diagnostics` | Preserve Clap's native diagnostics and usage text for parser-level failures (for example unknown flags or missing flag values), rather than reproducing Cobra's wording. Command-specific validation errors remain compatibility-tested. |

## Preserved Go issues awaiting a decision

| ID | Existing behavior | Disposition |
| --- | --- | --- |
| `GO-001` | `--config` is parsed after configuration is loaded and therefore does not select the file used for that invocation. | Preserve |
| `GO-002` | Help says `.foxglove.yaml`; the actual default is `~/.foxgloverc`. | Preserve |
| `GO-003` | Required positional arguments are missing from `devices edit` and `attachments download` usage text. | Preserve |
| `GO-004` | The `data export --end` help text is missing a closing parenthesis. | Preserve |

New issues must include a reproduction, impact, proposed behavior, and explicit
approval status before any Rust divergence is implemented.

## Runtime dependency ledger

Phase 1 adds the following locked direct runtime dependencies. The release
binary remains an implementation-only compatibility binary; the Go release
path is unchanged.

| Crate | Exact version and enabled features | Reason | Alternative considered |
| --- | --- | --- | --- |
| `clap` | `4.6.4`; `default-features = false`; `error-context`, `help`, `std`, `suggestions`, `usage` | Structured command tree and compatible validation | Continue the in-tree argument matcher; rejected because it no longer safely covered global flags, aliases, and completion. |
| `serde` | `1.0.228`; `default-features = false`; `derive`, `std` | YAML and JSON data model plus typed API payloads | Handwritten typed/config conversion; rejected for correctness and maintenance risk. |
| `serde_json` | `1.0.151`; `default-features = false`; `std` | Stable JSON rendering | Existing narrow in-tree JSON reader; retained nowhere in production. |
| `serde_yaml_ng` | `0.10.0` | Preserve and rewrite YAML configuration, including unknown values | In-tree YAML writer; rejected because it could not safely retain arbitrary nested YAML. |

`cargo tree` on 2026-08-29 resolves these transitive runtime crates:
`anstyle 1.0.14`, `clap_builder 4.6.2`, `clap_lex 1.1.0`, `equivalent 1.0.2`,
`hashbrown 0.17.1`, `indexmap 2.14.1`, `itoa 1.0.18`, `memchr 2.8.3`,
`ryu 1.0.23`, `serde_core 1.0.228`, `strsim 0.11.1`,
`unsafe-libyaml 0.2.11`, and `zmij 1.0.23`.

License review from each resolved crate manifest found only permissive terms:
MIT (`serde_yaml_ng`, `strsim`, `unsafe-libyaml`, `zmij`); MIT or Apache-2.0
(`anstyle`, `clap`, `clap_builder`, `clap_lex`, `hashbrown`, `itoa`, `serde`,
`serde_core`, `serde_json`); Apache-2.0 or MIT (`equivalent`, `indexmap`);
Unlicense or MIT (`memchr`); and Apache-2.0 or BSL-1.0 (`ryu`).

`cargo-audit 0.22.2` scanned `rust/Cargo.lock` against 1,226 RustSec
advisories on 2026-08-29 and exited successfully with no reported
vulnerabilities. The optimized macOS arm64 `rust/foxglove-rust` artifact is
1,366,672 bytes. It has no previous Rust artifact for a like-for-like
comparison, so the Phase 1 release-size delta is +1,366,672 bytes relative to
no Rust compatibility binary.

| Phase | Crate/category and exact version/features | Reason | Alternative considered | Status |
| --- | --- | --- | --- | --- |
| 2 | `reqwest 0.12.28`; `default-features = false`; `json`, `rustls-tls`, `stream` | TLS HTTP, JSON API calls, and response streaming | `hyper` directly; rejected because it would duplicate HTTP policy and response/error handling. | Complete |
| 2 | `tokio 1.53.1`; `default-features = false`; `fs`, `io-util`, `macros`, `rt`, `rt-multi-thread`, `signal`, `time` | Async execution, cancellable I/O, portable Ctrl-C handling, and device-code polling | A synchronous client; rejected because it cannot cancel active transfers or poll without blocking a worker. | Complete |
| 2 | `tokio-util 0.7.19`; `default-features = false`; `io`, `rt` | `CancellationToken` and async-reader upload streams | In-tree cancellation primitives; rejected because token propagation and reader adaptation are easy to get subtly wrong. | Complete |
| 2 | `time 0.3.47`; `default-features = false`; `formatting`, `parsing`, `serde` | ISO-8601/RFC3339 request timestamps | String-only timestamps; rejected because it would defer ordering and wire-format validation to each command. | Complete |
| 5 | `mcap 0.25.0`; `default-features = false`; `lz4`, `zstd` | Official Foxglove MCAP reader/writer, including streaming chunk validation | Handwritten MCAP parser/writer; rejected because the official implementation already provides bounded streaming parsing and conformance coverage. | In progress |
| 5 | `prost 0.14.1`; `default-features = false`; `std`; `prost-reflect 0.16.5`; `default-features = false`; `serde` | Dynamic protobuf descriptor/message decoding and canonical JSON conversion | Generated message types; rejected because MCAP schemas are dynamic. | In progress |
| 5 | `lz4_flex 0.14.0`; `default-features = false`; `frame`, `safe-decode`, `safe-encode`, `std` | Pure-Rust ROS 1 bag LZ4-frame chunk compatibility | Native LZ4 bindings; rejected for a portable runtime dependency. | In progress |

CSV, tables, progress display, temporary guards, completion templates, and
Foxglove-specific errors remain in-tree unless implementation evidence shows
that an external crate is safer or materially simpler.

Phase 2 dependency review: `cargo tree` resolves 155 normal crate
dependencies. The new network path brings in the Reqwest/Hyper/Rustls stack,
`futures-util`, and `tokio-util`; all resolved licenses are permissive or
otherwise approved for redistribution (MIT, Apache-2.0, ISC, BSD-3-Clause,
Unicode-3.0, CDLA-Permissive-2.0, and BSL-1.0). `cargo-audit 0.22.2` loaded
1,226 RustSec advisories and reported no vulnerabilities for `rust/Cargo.lock`
on 2026-08-29. The optimized macOS arm64 `rust/foxglove-rust` artifact is
1,418,112 bytes, a +51,440 byte delta from the Phase 1 artifact.

## Phase 0 acceptance checklist

- [x] Baseline tag selected: v1.0.33.
- [x] Command and phase ownership matrix documented.
- [x] Preserved issues and approved changes separated.
- [x] Deterministic subprocess oracle implemented.
- [x] Deterministic local HTTP fixture server implemented.
- [x] Command, completion, offline, and wire goldens generated and reviewed.
- [x] Compatibility target wired into Makefiles.
- [x] Existing Go tests and compatibility suite pass together.

## Phase 1 acceptance checklist

- [x] Standard Cargo project and `rust/foxglove-rust` compatibility binary
  added; the Go binary and release path are unchanged.
- [x] Root hierarchy, help text, aliases, deprecated-command notices, version,
  configuration mutations, and offline validation match the reviewed v1.0.33
  fixtures.
- [x] Rust binary is exercised against every command-surface golden and every
  applicable offline-behavior golden by `foxglove/compat`.
- [x] No HTTP client, async runtime, or network command implementation added.
- [x] `cargo fmt --check`, strict Clippy, `cargo test`, `make compat`, and
  `go test ./... -count=1` pass; `cargo-audit` reports no advisories.

## Phase 2 acceptance checklist

- [x] Async `FoxgloveClient` added with authenticated and unauthenticated
  request paths, typed sign-in/device-code/token payloads, and generic JSON
  GET/POST/PATCH/DELETE helpers.
- [x] Shared HTTP status mapping returns forbidden/not-found/decoded server
  errors consistently and consumes or drops every response body.
- [x] Stream and attachment downloads validate status before exposing bytes;
  signed upload and download links do not inherit API bearer credentials.
- [x] Cancellation propagates through JSON requests, link resolution,
  streaming reads, destination writes, uploads, and the portable Ctrl-C token
  helper.
- [x] Request serialization, authentication headers, link-following, and
  cancellation behavior are covered by Rust tests; loopback wire tests pass
  when run with local-socket access.
- [x] `cargo fmt --check`, strict Clippy, offline Rust tests, release build,
  and `cargo-audit 0.22.2` pass.

## Phase 3 acceptance checklist

- [x] Read-only list commands implemented for devices, projects, imports,
  coverage, recordings, attachments, sessions, events, event types, pending
  imports, topics, and extensions.
- [x] Session lookup and session-recording listing implemented with the Go
  CLI's human-readable output and not-found/authentication behavior.
- [x] API query names, default filters, ISO-8601 conversion, authentication,
  and user-agent behavior match the v1.0.33 wire contract.
- [x] Table, JSON, and CSV rendering is shared across read commands, including
  header-only CSV for empty responses.
- [x] The Phase 3 Rust fixture contract covers every read command path and the
  approved attachment-error and empty-CSV deltas.
- [x] `cargo fmt --check`, strict Clippy, offline Rust tests, and
  `go test ./compat -count=1` pass.

## Phase 4 acceptance checklist

- [x] Browser device-code login persists a session token and uses the selected
  API base URL; API-key configuration and auth inspection remain compatible.
- [x] Mutation commands cover devices, events, recordings, sessions,
  extensions, edge imports, and normal data imports.
- [x] Uploads, downloads, error handling, and cancellation follow the Go
  client's HTTP and file-safety behavior.
- [x] Focused compatibility fixtures cover mutation, transfer, and login wire
  contracts, including device-code polling and persisted configuration.

## Phase 5 acceptance checklist

- [x] The format module provides bounded MCAP validation/record emission and
  an ID-preserving MCAP writer for the export phases.
- [x] ROS 1 bag validation accepts uncompressed and LZ4-frame chunks and
  rejects unsupported compression without panicking.
- [x] ROS 1 message and dynamic protobuf decoders are cached by MCAP schema ID
  and cover the Phase 5 conformance corpus.
- [x] The corpus compares semantic MCAP/bag content with the Go oracle,
  including metadata, attachments, and malformed-input cases.
- [x] `cargo fmt --check`, strict Clippy, Rust tests, `make compat`, full Go
  tests, release build, and advisory review pass. `cargo audit` on 2026-09-01
  found no vulnerabilities and one explicitly accepted warning for unmaintained
  transitive crate `paste 1.0.15` (RUSTSEC-2024-0436). `paste` is a build-time
  procedural macro pulled in only by `mcap 0.25.0`; no runtime code invokes it
  directly and RustSec provides no fixed version. Reassess this exception when
  upgrading `mcap`.

## Phase 6 acceptance checklist

- [x] `data export` builds the reviewed stream request, including source
  selection, timestamp normalization, explicit compression, topic filtering,
  replay settings, and JSON alias behavior.
- [x] Direct MCAP and ROS 1 bag exports stream signed-link bytes to redirected
  stdout, reject terminal binary output, and propagate Ctrl-C as exit 130.
- [x] JSON exports request MCAP, decode ROS 1 and dynamic protobuf payloads,
  and emit Go-compatible NDJSON with fixed nine-digit decimal timestamps.
- [x] The Go/Rust local-fixture contract compares direct MCAP and bag bytes,
  ROS 1 and protobuf JSON output, signed-link headers, validation, full option
  serialization, and initial stream failures.
- [x] `cargo fmt --check`, strict Clippy, Rust tests, release build,
  `make compat`, and full Go tests pass. Non-JSON `--output-file` recovery,
  reindexing, and merging are explicitly Phase 7 work.
