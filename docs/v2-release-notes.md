# Foxglove CLI v2.0.0

This release moves the CLI from Go to Rust. These notes cover changes since
v1.0.33, including changes first delivered in v1.0.34 release candidates.
See the [migration guide](https://github.com/foxglove/foxglove-cli/blob/main/docs/v2-migration.md)
before upgrading scripts.

## Breaking changes

- **`data` commands moved.** `data import`, `data export`, and
  `data coverage list` are now `upload`, `export`, and `coverage list`. The old
  paths are removed, not aliased.
- **Deprecated commands and flags removed.** The `data imports` commands,
  `--json` (use `--format json`, or `--output-format json` on `export`),
  `devices add --serial-number`, `pending-imports list --error`, and
  `--import-id` on `export` and `attachments list` are gone.
- **Environment variables renamed.** Only `FOXGLOVE_BEARER_TOKEN`,
  `FOXGLOVE_BASE_URL`, and `FOXGLOVE_DEFAULT_PROJECT_ID` override the config
  file. `BEARER_TOKEN`, `BASE_URL`, and `AUTH_TYPE` are ignored, with a warning
  for the first two. `DEFAULT_PROJECT_ID` still works but is deprecated. The
  base URL must use `https`, or `http` for a loopback host.
- **List commands return one page.** Most list commands return at most 50
  results unless you pass `--limit` (1–2000), and print a hint on stderr when
  more may exist. v1 returned everything for most commands. Page with
  `--offset`, or `--cursor` for datasets and episodes. `projects list`,
  `extensions list`, and `event-types list` still return all results.
- **JSON list output is wrapped.** `--format json` on list commands prints
  compact `{"data": [...]}` instead of an indented bare array, plus
  `nextCursor` where cursor pagination applies.
- **`--config` is honored.** v1 ignored it and always used `~/.foxgloverc`.
- **Project defaults apply everywhere.** `export`, `attachments list`,
  `events list`, and `events add` now use the default project like other
  commands. Pass `--project-id=` to run without a project.
- **Timestamps keep fractional seconds.** `--start` and `--end` filters and
  export boundaries are no longer truncated to whole seconds, so results near
  a boundary can change.
- **`export` is stricter.** The default format is now spelled `mcap` (`mcap0`
  is rejected), and `--start` and `--end` must be given together.
- **`events` defaults changed.** `events list` returns newest events first,
  `--sort-order` requires `--sort-by`, and `events add` requires `--device-id`,
  `--start`, and `--end`.
- **Deleting a missing resource fails.** Delete commands exit non-zero when the
  resource does not exist; v1 printed a warning and exited 0.
- **`recordings list` fields changed.** The always-zero Message Count column
  and `messageCount` field are gone, and a Session ID column and `sessionId`
  field are added.
- **`upload` session flags.** `--session-key` no longer requires a project, and
  `--session-id` with `--session-key` is now an error.
- **`config set project-id`** trims whitespace and rejects an empty value.
- **Shell completion must be regenerated.** Run `foxglove completion SHELL`.
  Completion of device names from the API is no longer provided.

## New

- `recordings transfer ID` asks an Edge Site to send a recording to its Primary
  Site, without the dummy local file v1 needed. It reports the current status and
  returns without waiting.
- `datasets` and `episodes` commands to list, create, edit, and delete datasets
  and episodes, manage a dataset's episodes, commit or discard pending changes,
  list, compare, and restore versions, and download a committed version as one
  MCAP per episode plus `manifest.json`. Datasets are in beta.
- `sessions edit` changes or removes a session key, and `sessions add --key`
  sets one.
- `upload` prints the upload request ID when it finishes.
- `--debug` logs each HTTP request's method, URL, status, and duration to
  stderr.
- `event-types list` shows each event type's custom properties.
- `devices edit` can change custom properties without `--name`, and `-p`
  accepts multiline-string and multi-enum properties.
- `config` commands say when an environment variable overrides the saved
  project.

## Reliability and output

- Requests time out instead of hanging, GET requests retry 429 and 5xx
  responses, and `auth login` polls at the server's requested interval, retries
  transient errors, and stops when the login request expires.
- Error messages show the API's message for 403 and 404 responses, name the
  server and cause for connection errors, and suggest `foxglove auth login` on
  401. HTML error pages and request URLs are no longer printed.
- `export --output-file` writes JSON to the file instead of stdout, recovers from
  interrupted downloads without duplicating messages, finishes `bag1` exports in
  one pass, and fails without touching the destination when recovery stalls.
- Failed or cancelled downloads leave existing destination files in place and
  remove partial files. Credential and staging files are created with private
  permissions, and files are replaced atomically on Windows as well as Unix.
  Cancellation still exits with code 130.
- `attachments download` reports HTTP errors instead of writing the error body
  as the file, and refuses to write binary data to an interactive terminal.
- Tables align columns, fit the terminal width, keep IDs on one line, and are
  not wrapped when piped. Control and bidirectional-override characters in
  tables and `sessions get` are printed as escapes.
- A malformed or unreadable config file is reported instead of ignored, and
  unknown config keys are preserved when the CLI writes the file.
- Fixes for identifier encoding in URLs, signed ROS 1 values in JSON exports,
  event query fields, empty CSV headers, missing device IDs in `coverage list`,
  and episode recording `location` in JSON output.

Native builds continue to cover Linux, macOS, and Windows on amd64 and arm64.
