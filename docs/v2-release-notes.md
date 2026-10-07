# Foxglove CLI v2.0.0

This release rewrites the CLI in Rust. These notes cover all changes since
v1.0.33. See the [migration guide](https://github.com/foxglove/foxglove-cli/blob/main/docs/v2-migration.md)
before upgrading scripts.

## Breaking changes

- `data import`, `data export`, and `data coverage list` are now `upload`,
  `export`, and `coverage list`, with no aliases.
- Removed: the `data imports` commands, `--json`, `devices add --serial-number`,
  `pending-imports list --error`, and `--import-id` on `export` and
  `attachments list`.
- Environment variables are now `FOXGLOVE_BEARER_TOKEN`, `FOXGLOVE_BASE_URL`,
  and `FOXGLOVE_DEFAULT_PROJECT_ID`. `BEARER_TOKEN`, `BASE_URL`, and
  `AUTH_TYPE` are ignored; `DEFAULT_PROJECT_ID` is deprecated. The base URL
  must be `https`, or `http` for loopback.
- List commands with `--limit` return 50 results by default.
- `--format json` on list commands prints `{"data": [...]}` instead of a bare
  array.
- `export`, `attachments list`, and `events` now use the default project.
- Timestamps keep fractional seconds instead of truncating to whole seconds.
- `export --output-format mcap0` is now `mcap`, and `--start` and `--end` must
  be given together.
- `events list` returns newest first, `--sort-order` requires `--sort-by`, and
  `events add` requires `--device-id`, `--start`, and `--end`.
- Blank values such as `--recording-id=` or `--key=` are rejected instead of
  silently dropping the filter, and `export --topics ""` is rejected instead of
  exporting all topics.
- Deleting a missing resource fails instead of exiting 0.
- `recordings list` drops message count and adds session ID.
- `--config` is honored; v1 ignored it.
- `upload --session-key` no longer requires a project and cannot be combined
  with `--session-id`.

## New

- `recordings transfer ID` requests an Edge-to-Primary transfer without a local
  file.
- `datasets` and `episodes` commands, including `datasets download` (beta).
- `sessions edit` and `sessions add --key`.
- `recordings list --without-device` and `--without-session`.
- `upload` prints its request ID, and `--debug` logs HTTP requests.
- `event-types list` shows custom properties, and `devices edit -p` works
  without `--name` and supports multiline-string and multi-enum properties.

## Fixes

- HTTP timeouts, retries for 429 and 5xx on GET requests, and `auth login`
  polling that honors the server interval and expiry.
- Clearer errors for 401, 403, 404, and connection failures.
- More reliable `export` resume, JSON output to `--output-file`, and one-pass
  `bag1` exports.
- Failed downloads keep existing files, and files are replaced atomically on
  all platforms.
- `attachments download` reports HTTP errors and refuses to write to a terminal.
- Tables fit the terminal, keep IDs on one line, and escape control characters.
- Malformed config files are reported instead of ignored.
- Encoding fixes for identifiers, signed ROS 1 values, event queries, and empty
  CSV headers.
