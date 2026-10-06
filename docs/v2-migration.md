# Migrating to Foxglove CLI v2

v2 is a Rust rewrite that removes deprecated commands and flags. Your saved
login, API key, and `~/.foxgloverc` keep working. Scripts and CI jobs usually
need the changes below.

## Environment variables

v2 reads only these:

| v1 | v2 |
| --- | --- |
| `BEARER_TOKEN` | `FOXGLOVE_BEARER_TOKEN` |
| `BASE_URL` | `FOXGLOVE_BASE_URL` |
| `DEFAULT_PROJECT_ID` | `FOXGLOVE_DEFAULT_PROJECT_ID` (old name deprecated but still read) |
| `AUTH_TYPE` | Remove |

`BEARER_TOKEN`, `BASE_URL`, and `DEFAULT_PROJECT_ID` print a warning. The base URL must be `https`, or `http` for
loopback. If `HOME` (or `USERPROFILE`) is unset, pass `--config PATH`.

## Commands and flags

Old paths and flags are removed, not aliased.

| v1 | v2 |
| --- | --- |
| `data import FILE` | `upload FILE` |
| `data export …` | `export …` |
| `data coverage list …` | `coverage list …` |
| `data import FILE --edge-recording-id ID` | `recordings transfer ID` |
| `data imports add FILE` | `upload FILE` |
| `data imports list` | `recordings list` (see below) |
| `--json` | `--format json`, or `export --output-format json` |
| `export --output-format mcap0` | `export --output-format mcap` (still the default) |
| `--import-id IMPORT_ID` on `export` and `attachments list` | `--recording-id RECORDING_ID` |
| `devices add --serial-number` | Remove; v1 ignored it |
| `pending-imports list --error` | Filter the `error` field of `--format json` |

Import IDs are not recording IDs; find the recording ID with `recordings list`.

`recordings list` differs from `data imports list`: use `--start`/`--end` in
place of `--data-start`/`--data-end`. The old `--start`/`--end` (import time)
and `--include-deleted` have no equivalent.

`recordings transfer` returns without waiting. Follow progress with
`recordings list --import-status`.

## List output

List commands with `--limit` now return 50 results by default; pass `--limit`
(up to 2000) and page with `--offset`, or `--cursor` for datasets and episodes.
v1 returned all results for most commands. `projects list`, `extensions list`,
and `event-types list` still return everything.

JSON list output is wrapped in an object, so read `.data`:

```sh
foxglove devices list --format json | jq '.data[]'   # v1: jq '.[]'
```

## Project defaults

`export`, `attachments list`, `events list`, and `events add` ignored the
default project in v1 and now use it. To search all projects, pass
`--project-id=`.

Precedence is `--project-id`, then `FOXGLOVE_DEFAULT_PROJECT_ID`, then
`DEFAULT_PROJECT_ID`, then the saved `config set project-id` value. `config set project-id` rejects empty
values; use `config unset project-id`.

## Other changes

- `export --start` and `--end` must be given together.
- `export --output-format json --output-file` writes to the file; v1 wrote to
  stdout.
- A malformed config file is an error; v1 ignored it.
- `events list` returns newest first, `--sort-order` requires `--sort-by`, and
  `events add` requires `--device-id`, `--start`, and `--end`.
- `upload --session-key` no longer requires a project and cannot be combined
  with `--session-id`.
- Timestamps keep fractional seconds, so `00:00:00.500Z` no longer rounds down
  to `00:00:00Z`.
- Deleting a missing resource exits non-zero; v1 exited 0.
- `recordings list` drops `messageCount` and adds `sessionId`.
- `--config` is honored; v1 always used `~/.foxgloverc`.
- `attachments download` refuses to write to a terminal; use `--output-file`.
- Table and message text changed. Parse `--format json` or `csv`. Exit codes
  are unchanged.
- Regenerate shell completion with `foxglove completion SHELL`. Device names no
  longer complete.
