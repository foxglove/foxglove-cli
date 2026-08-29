# Foxglove CLI Rust migration

The compatibility baseline is the released Go CLI at tag **v1.0.33**. The Go
implementation remains runnable as the oracle until the Rust release candidate
passes every gate. Unlisted behavior changes are not permitted.

## Phase status

| Phase | Deliverable | Status | Gate |
| --- | --- | --- | --- |
| 0 | Oracle, tracker, command surface, wire fixtures, approved deltas | Complete | `make compat` and existing Go tests pass from a clean checkout |
| 1 | Rust project and CLI/config/output core | Not started | Offline CLI contract passes |
| 2 | Async API client, errors, streaming, cancellation | Not started | Wire contract and cancellation tests pass |
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

No Rust dependencies have landed yet. Planned candidates are not approved by
this table until their phase PR records exact versions, feature flags,
transitive impact, licenses, advisories, alternatives, and release-size delta.

| Phase | Crate/category | Reason | Status |
| --- | --- | --- | --- |
| 1 | `clap` | Command parsing with customized compatibility output | Planned |
| 1 | `serde`, `serde_json` | Typed API/config data and JSON compatibility | Planned |
| 1 | `serde_yaml_ng` | Correctly read and rewrite existing YAML config, including unknown keys | Planned |
| 2 | `reqwest`, `tokio`, `tokio-util` | TLS HTTP, streaming, portable Ctrl-C, and cancellation tokens | Planned |
| 2 | `time` | ISO-8601/RFC3339 compatibility | Planned |
| 5 | `mcap` | Official Foxglove MCAP reader/writer | Planned |
| 5 | `prost`, `prost-reflect` | Dynamic protobuf-to-JSON conversion | Planned |
| 5 | Pure-Rust LZ4 frame crate | ROS 1 bag chunk compatibility | Planned |

CSV, tables, progress display, temporary guards, completion templates, and
Foxglove-specific errors remain in-tree unless implementation evidence shows
that an external crate is safer or materially simpler.

## Phase 0 acceptance checklist

- [x] Baseline tag selected: v1.0.33.
- [x] Command and phase ownership matrix documented.
- [x] Preserved issues and approved changes separated.
- [x] Deterministic subprocess oracle implemented.
- [x] Deterministic local HTTP fixture server implemented.
- [x] Command, completion, offline, and wire goldens generated and reviewed.
- [x] Compatibility target wired into Makefiles.
- [x] Existing Go tests and compatibility suite pass together.
