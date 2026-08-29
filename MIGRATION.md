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
| 3 | Read-only commands | Not started | Read command parity passes |
| 4 | Auth, configuration, mutations, uploads/downloads | Not started | Mutation and transfer parity passes |
| 5 | MCAP, ROS 1 bag, ROS 1 message, protobuf foundation | Not started | Format conformance passes |
| 6 | Direct and JSON export | Not started | Direct byte and JSON parity passes |
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
| 2 | `tokio 1.53.1`; `default-features = false`; `io-util`, `macros`, `rt`, `rt-multi-thread`, `signal` | Async execution, cancellable I/O, and portable Ctrl-C handling | A synchronous client; rejected because it cannot cancel active transfers without blocking a worker. | Complete |
| 2 | `tokio-util 0.7.19`; `default-features = false`; `io`, `rt` | `CancellationToken` and async-reader upload streams | In-tree cancellation primitives; rejected because token propagation and reader adaptation are easy to get subtly wrong. | Complete |
| 2 | `time 0.3.47`; `default-features = false`; `formatting`, `parsing`, `serde` | ISO-8601/RFC3339 request timestamps | String-only timestamps; rejected because it would defer ordering and wire-format validation to each command. | Complete |
| 5 | `mcap` | Official Foxglove MCAP reader/writer | To be evaluated during Phase 5 | Planned |
| 5 | `prost`, `prost-reflect` | Dynamic protobuf-to-JSON conversion | To be evaluated during Phase 5 | Planned |
| 5 | Pure-Rust LZ4 frame crate | ROS 1 bag chunk compatibility | To be evaluated during Phase 5 | Planned |

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
