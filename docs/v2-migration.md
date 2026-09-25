# Migrating to Foxglove CLI v2

v2 replaces the Go CLI with Rust and removes deprecated interfaces. Existing
credentials, configuration location, environment variables, and saved defaults
continue to work; no reauthentication or data conversion is required.

## Command replacements

All commands below start with `foxglove`. Old paths are removed, not aliases.

| v1 | v2 |
| --- | --- |
| `data import FILE` | `upload FILE` |
| `data export …` | `export …` |
| `data coverage list …` | `coverage list …` |
| `data import IGNORED_FILE --edge-recording-id ID` | `recordings transfer ID` |
| Deprecated `data imports add FILE` | `upload FILE` |
| Deprecated `data imports list` | `recordings list` (see differences below) |
| Deprecated `--json` on resource commands | `--format json` |
| Deprecated `--json` on export | `--output-format json` |
| Deprecated `devices add --serial-number VALUE` | Remove the ignored flag and its value |

Other command names remain unchanged, including `pending-imports`. Existing
upload, export, and coverage options keep their meanings, except for timestamp
precision and project defaults described below. `--import-id`, `--import-status`,
and API fields such as `importStatus` remain supported where already present.

`recordings transfer ID` requests transfer and processing from an Edge Site to its
configured Primary Site. It needs no local file or destination flag and does not
wait. It reports the returned recording ID and `importStatus` on stderr: an
accepted/queued request is not a completed transfer; `complete` means the data is
already available. Missing or unavailable recordings return an error.

`recordings delete ID` deletes a recording and its data. For an imported edge
recording, only the imported data is removed: the Edge Site copy and session
membership remain. You can transfer the recording again to restore its data.

### Deprecated import listing

`recordings list` is not a drop-in replacement for `data imports list`:

- Old `--start/--end` selected **import time**. Recording `--start/--end` select
  **data time**; do not copy import-time filters unchanged.
- Old `--data-start/--data-end` correspond to recording `--start/--end`.
- Old `--include-deleted` and import-shaped output have no direct replacement.
  Update scripts to consume recording fields; do not treat import and recording
  identifiers as interchangeable.

## Project defaults

Where an endpoint supports project scope, resolution is:

1. Explicit `--project-id ID`.
2. Nonempty `DEFAULT_PROJECT_ID` environment variable.
3. Saved `default_project_id` configuration.
4. No project scope when none is configured.

Explicit `--project-id=` (or `--project-id ""`) bypasses both environment and
configuration defaults. An empty environment value remains unset and allows the
saved default. Unscoped does not mean unassigned-only or bypass permissions.

Export, attachment listing, and event listing now honor defaults consistently.
Event creation also uses the resolved project when looking up its device. If you
previously relied on these operations ignoring your saved project, add
`--project-id=` to preserve unscoped behavior. `pending-imports list --without-project` selects
unassigned records and suppresses defaults; it cannot be combined with a
nonempty explicit project ID or a session key.

Defaults never implicitly move an existing resource between projects. There is
no fallback retry across projects, and `--session-key` requires a resolved project.

## Timestamps and output

Fractional seconds (up to nanoseconds) are preserved in query/export boundaries
instead of truncated. A boundary such as `2026-01-01T00:00:00.500Z` now selects
from that instant, rather than the start of the second. Fractional boundaries
can change results; reversed export boundaries within a second now fail validation.
Accepted shorthand, timezone offsets, and UTC defaults remain unchanged.

Machine compatibility covers command behavior and structured values, not exact
help, tables, diagnostics, progress text, JSON whitespace, YAML key order, or
query-parameter order. Use JSON/CSV for scripts; JSON message exports remain
NDJSON. Existing resource schemas and null/default conventions are retained.
Exit-code conventions remain unchanged, including 130 for cancellation.

Regenerate shell completion with `foxglove completion SHELL` for Bash, Fish,
PowerShell, or Zsh. Static commands, flags, and file completion remain; Go's
API-backed device completion is not included. Malformed/unreadable configuration
now reports an error rather than being silently ignored.

See [v2 release notes](v2-release-notes.md) for retained correctness fixes.
