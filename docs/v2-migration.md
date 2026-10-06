# Migrating to Foxglove CLI v2

v2 replaces the Go CLI with a Rust rewrite and removes deprecated commands and
flags. Your saved login, API key, and config file (`~/.foxgloverc`) keep working;
you do not need to sign in again.

Most interactive use carries over unchanged. Scripts and CI jobs usually need
updates. Work through the checklist, then read the sections for the commands
you use.

## Upgrade checklist

1. Rename environment variables to their `FOXGLOVE_` names
   ([Environment variables](#environment-variables)).
2. Replace `data …` commands and removed flags
   ([Commands](#commands) and [Removed flags](#removed-flags)).
3. Add `--limit` or pagination to list commands, and read JSON list output from
   `.data` ([List output](#list-output)).
4. If you have a default project, check `export`, `attachments list`, and
   `events` calls ([Project defaults](#project-defaults)).
5. Check `export` and `events` arguments ([`export`](#export) and
   [`events`](#events)).
6. Check scripts that rely on timestamp truncation, table output, `--config`,
   or deletes succeeding for missing resources ([Other behavior
   changes](#other-behavior-changes)).
7. Regenerate shell completion ([Shell completion](#shell-completion)).

## Environment variables

v1 read every config key from an environment variable of the same name. v2
reads only these:

| v1 | v2 |
| --- | --- |
| `BEARER_TOKEN` | `FOXGLOVE_BEARER_TOKEN` |
| `BASE_URL` | `FOXGLOVE_BASE_URL` |
| `DEFAULT_PROJECT_ID` | `FOXGLOVE_DEFAULT_PROJECT_ID` |
| `AUTH_TYPE` | Remove it; the auth type comes from the config file |

`BEARER_TOKEN` and `BASE_URL` are ignored and print a warning if set.
`DEFAULT_PROJECT_ID` still works but prints a deprecation warning, and
`FOXGLOVE_DEFAULT_PROJECT_ID` wins when both are set. Empty values count as
unset.

The base URL must use `https`, or `http` for `localhost` or another loopback
address. v2 needs `HOME` (or `USERPROFILE` on Windows) to find the config file;
if neither is set, pass `--config PATH`.

For example, a CI job that used:

```sh
export BEARER_TOKEN=fox_sk_...
export DEFAULT_PROJECT_ID=prj_...
```

should now use:

```sh
export FOXGLOVE_BEARER_TOKEN=fox_sk_...
export FOXGLOVE_DEFAULT_PROJECT_ID=prj_...
```

## Commands

All commands start with `foxglove`. Old paths are removed, not aliased, and
fail with an unrecognized-command error.

| v1 | v2 |
| --- | --- |
| `data import FILE` | `upload FILE` |
| `data export …` | `export …` |
| `data coverage list …` | `coverage list …` |
| `data import IGNORED_FILE --edge-recording-id ID` | `recordings transfer ID` |
| `data imports add FILE` (deprecated) | `upload FILE` |
| `data imports list` (deprecated) | `recordings list` (see [below](#replacing-data-imports-list)) |

`upload`, `export`, and `coverage list` keep the flags of the v1 commands they
replace, except where noted in [Removed flags](#removed-flags) and
[`export`](#export). Other command names are unchanged.

```sh
# v1
foxglove data import robot.mcap --device-id dev_123
foxglove data export --recording-id rec_123 --output-file out.mcap
# v2
foxglove upload robot.mcap --device-id dev_123
foxglove export --recording-id rec_123 --output-file out.mcap
```

`upload` now prints the upload request ID to stderr when it finishes; pass it
to `pending-imports list --request-id` to follow the import. `--session-key` no
longer requires a project, and combining `--session-id` with `--session-key` is
now an error.

### Transferring edge recordings

`recordings transfer ID` asks the recording's Edge Site to send it to its
configured Primary Site. It needs no local file. It reports the recording ID
and its `importStatus` on stderr and returns without waiting: a queued request
is not a finished transfer, and `complete` means the data is already
available. A missing or unavailable recording is an error. Use
`recordings list --import-status` or `recordings list --format json` to follow
progress.

`recordings delete ID` deletes a recording and its data. For an imported edge
recording, only the imported data is removed; the Edge Site copy and its
session membership remain, and you can transfer the recording again.

### Replacing `data imports list`

`recordings list` lists recordings, not imports, so its filters and output
differ:

| `data imports list` | `recordings list` |
| --- | --- |
| `--data-start`, `--data-end` | `--start`, `--end` |
| `--start`, `--end` (import time) | No equivalent; do not copy these unchanged |
| `--include-deleted` | No equivalent |
| Import ID and import fields | Recording ID and recording fields |

Import IDs and recording IDs are different values. For upload status, use
`pending-imports list`.

## Removed flags

| v1 | v2 |
| --- | --- |
| `--json` on list and get commands | `--format json` |
| `--json` on `data export` | `export --output-format json` |
| `devices add --serial-number VALUE` | Remove it; v1 ignored it |
| `data export --import-id IMPORT_ID` | `export --recording-id RECORDING_ID` |
| `attachments list --import-id IMPORT_ID` | `attachments list --recording-id RECORDING_ID` |
| `pending-imports list --error …` | Remove it and filter on the `error` field of `--format json` output |

Import IDs are not recording IDs; find the recording ID with `recordings list`.
Passing a removed flag fails with an unexpected-argument error.

## List output

v1 returned everything the API sent in one response for most list commands,
and defaulted to 2000 results for `recordings list` and 100 for `events list`.
v2 list commands that accept `--limit` return one page of 50 results by
default; pass `--limit` (1–2000) to change it. When more results may exist, a
hint on stderr says how to fetch the next page. `projects list`,
`extensions list`, and `event-types list` have no `--limit` and still return
all results.

`--format json` prints an object instead of a bare array, and is no longer
indented:

```json
{"data": [{"id": "dev_...", "name": "Robot A"}]}
```

Update JSON consumers to read `.data`, for example `jq '.data[]'` instead of
`jq '.[]'`. `datasets`, `episodes`, and `datasets versions compare` also return
`nextCursor`; pass it to `--cursor` for the next page. The other commands that
accept `--limit` page with `--offset`.

To fetch everything, loop until a page has fewer items than `--limit`:

```sh
offset=0
while :; do
  page=$(foxglove recordings list --format json --limit 2000 --offset "$offset")
  echo "$page" | jq -c '.data[]'
  [ "$(echo "$page" | jq '.data | length')" -lt 2000 ] && break
  offset=$((offset + 2000))
done
```

CSV output is not wrapped; it is still one header row plus one row per item.

## Project defaults

Commands that accept `--project-id` choose a project in this order:

1. `--project-id ID`.
2. `FOXGLOVE_DEFAULT_PROJECT_ID`, then the deprecated `DEFAULT_PROJECT_ID`.
3. `default_project_id` in the config file, set with
   `foxglove config set project-id ID`.
4. Otherwise, no project.

`--project-id=` (or `--project-id ""`) skips steps 2 and 3. Running without a
project does not mean "only resources without a project", and does not bypass
permissions.

In v1, `data export`, `attachments list`, `events list`, and `events add`
ignored the default project. In v2 they use it. If you have a default project
and relied on those commands searching every project, add `--project-id=`:

```sh
foxglove events list --project-id=
```

`pending-imports list --without-project` lists imports without a project and
ignores defaults; it cannot be combined with a non-empty `--project-id` or
`--session-key`.

Defaults never move an existing resource to a different project, and the CLI
does not retry in another project when a lookup fails. `--session-key` needs a
project on `export`, `recordings list`, `coverage list`, and `topics list`;
`upload` accepts it without one.

`config set project-id` trims whitespace and rejects an empty value. Use
`config unset project-id` to clear it.

## `export`

- `--output-format mcap0` is now `--output-format mcap`, which is still the
  default. `bag1` and `json` are unchanged.
- `--start` and `--end` must be given together.
- `--import-id` is removed; use `--recording-id` with the recording's ID from
  `recordings list`, not the import ID.
- With `--output-format json`, `--output-file` now writes to the file; v1
  wrote JSON to stdout. JSON exports are still newline-delimited.

## `events`

- `events list` with no sort flags returns newest events first.
- `--sort-order` now takes effect, and requires `--sort-by`.
- `events add` requires `--device-id`, `--start`, and `--end`.
- Both commands use the default project; see
  [Project defaults](#project-defaults).

## Other behavior changes

- **Fractional timestamps.** `--start` and `--end` keep fractional seconds (up to
  nanoseconds) instead of truncating to the second. `2026-01-01T00:00:00.500Z`
  now starts at half a second past midnight, not at midnight, so results near
  a boundary can change. Accepted formats, offsets, and the UTC default are
  unchanged.
- **Deletes of missing resources fail.** v1 exited 0; v2 exits non-zero.
  Ignore the error if your script expects the resource may already be gone.
- **`recordings list` fields.** The always-zero Message Count column and
  `messageCount` JSON field are removed. A Session ID column and `sessionId`
  field are added.
- **`--config` is honored.** v1 ignored `--config` and always used
  `~/.foxgloverc`. v2 reads and writes the file you pass, so remove the flag if
  it points somewhere other than the file you actually use.
- **`attachments download`** refuses to write to an interactive terminal; pass
  `--output-file` or redirect stdout.
- **Malformed config.** A config file that cannot be read or parsed is now an
  error instead of being ignored.
- **Text output.** Help text, tables, error messages, and progress output are
  worded and laid out differently. Parse `--format json` or `--format csv`, not
  tables. Apart from the list envelope and the `recordings list` fields above,
  JSON items keep v1's field names and null and default values. Exit codes are
  unchanged, including 130 for cancellation. JSON whitespace, YAML key order,
  and query-parameter order are not stable.

## Shell completion

Regenerate your completion script:

```sh
foxglove completion bash > /etc/bash_completion.d/foxglove
foxglove completion zsh > "${fpath[1]}/_foxglove"
foxglove completion fish > ~/.config/fish/completions/foxglove.fish
foxglove completion powershell >> $PROFILE
```

Commands, flags, and file paths complete as before. Device names fetched from
the API no longer complete.

See the [v2 release notes](v2-release-notes.md) for everything else that
changed.
