# Migrating to Foxglove CLI v2

Foxglove CLI v2 adds support for Datasets and improves download reliability,
error handling and standardizes commands. We've also ported the CLI to Rust
and restructured our code to make future changes simpler.

## What's better in v2

- Resume exports more reliably. Failed downloads keep existing files, and
  completed downloads replace files atomically on all platforms.
- Diagnose failures with clearer authentication, permission, and connection
  errors, upload request IDs, and HTTP request logs from `--debug`. HTTP
  timeouts and retries for transient GET failures improve request reliability.
- Manage datasets and episodes, compare and restore dataset versions, and
  download a committed dataset version as MCAP files with `datasets download`.
- Edit sessions with `sessions edit` and assign keys with `sessions add --key`.
  Update device properties without renaming the device, including
  multiline-string and multi-enum properties.

See [GitHub Releases](https://github.com/foxglove/foxglove-cli/releases) for the
changelog. The sections below explain the changes needed when upgrading from v1.

## Environment variables

Update your environment variables:

| v1 | v2 |
| --- | --- |
| `BEARER_TOKEN` | `FOXGLOVE_BEARER_TOKEN` |
| `BASE_URL` | `FOXGLOVE_BASE_URL` |
| `DEFAULT_PROJECT_ID` | `FOXGLOVE_DEFAULT_PROJECT_ID` |
| `AUTH_TYPE` | Remove |

## Commands and flags

| v1 | v2 |
| --- | --- |
| `data import FILE` | `upload FILE` |
| `data export …` | `export …` |
| `data coverage list …` | `coverage list …` |
| `data import FILE --edge-recording-id ID` | `recordings transfer ID` |
| `data imports add FILE` | `upload FILE` |
| `data imports list` | `recordings list` (see below) |
| `--json` | `--format json` |
| `export --output-format mcap0` | `export --output-format mcap` (still the default) |
| `--import-id IMPORT_ID` on `export` and `attachments list` | `--recording-id RECORDING_ID` |
| `pending-imports list --error` | Filter the `error` field of `--format json` |

Imports have long been deprecated in favor of Recordings. If you still have
workflows using Import IDs you'll need to switch over to Recording IDs.

`recordings list` differs from `data imports list`: use `--start`/`--end` in
place of `--data-start`/`--data-end`. The old `--start`/`--end` (import time)
and `--include-deleted` have no equivalent.

`recordings transfer` returns without waiting. Follow progress with
`recordings list --import-status`.

## List output

List commands with `--limit` return 50 results by default. Pass `--limit`
to change this.

JSON list output is now wrapped in an object to provide room for metadata.
Read data via `.data`:

```sh
foxglove devices list --format json | jq '.data[]'   # v1: jq '.[]'
```

## Project defaults

When relevant a project ID is selected in order of `--project-id`,
`FOXGLOVE_DEFAULT_PROJECT_ID`, then the saved `config set project-id` value.

## Other changes

- `export --start` and `--end` must be given together.
- `export --output-format json --output-file` writes to the file; v1 wrote to
  stdout.
- `events list` returns newest first, `--sort-order` requires `--sort-by`, and
  `events add` requires `--device-id`, `--start`, and `--end`.
- `upload --session-key` no longer requires a project and cannot be combined
  with `--session-id`.
- Timestamps preserve precise boundaries by keeping fractional seconds, so
  `00:00:00.500Z` no longer rounds down to `00:00:00Z`.
- Blank values for ID, key, name, and other filter flags are now treated as errors.
- Deleting a missing resource exits non-zero; v1 exited 0.
- `recordings list` drops `messageCount` and adds `sessionId`.
- `--config` is honored; v1 always used `~/.foxgloverc`.
- `attachments download` refuses to write to a terminal; use `--output-file`.
- Various table and message outputs have changed. For programmatic parsing,
  use `--format json` or `--format csv`.
- Regenerate shell completion with `foxglove completion SHELL`.
