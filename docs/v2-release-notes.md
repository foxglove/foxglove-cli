# Foxglove CLI v2.0.0

This release moves the CLI from Go to Rust. These notes cover changes since
v1.0.33, including changes first delivered in v1.0.34 release candidates.
See the [migration guide](https://github.com/foxglove/foxglove-cli/blob/main/docs/v2-migration.md)
before upgrading scripts.

- Replace `data import`, `data export`, and `data coverage list` with `upload`,
  `export`, and `coverage list`. Old paths are removed without aliases.
- Use `recordings transfer ID` to request an Edge-to-Primary transfer without a
  dummy local file. The command reports actual status and does not wait.
- Remove long-deprecated `data imports` commands, `--json`, and the ignored
  `devices add --serial-number` flag. Import listing requires migration of filters
  and output, not just a command rename.
- Preserve fractional timestamp precision and consistently apply configured
  project defaults, including export and attachment listing. Explicit
  `--project-id=` bypasses defaults.
- Add dataset and episode listings, including dataset episode membership, and
  improve human-readable tables to fit terminal width.
- Generate help and shell completion from the Rust command tree. API-backed
  device completion is no longer provided.
- Correct literal API identifier encoding, signed ROS 1 JSON values, event query
  field encoding, empty CSV headers, attachment HTTP errors, and initial export
  failures. HTTP 401 responses consistently provide sign-in guidance.
- Honor JSON export output files, improve indexed bag/MCAP recovery (including
  schemaless channels), and release HTTP responses reliably.
- Preserve existing destination files on failed/cancelled transfers, clean up
  partial files, protect new credential/staging files, and use atomic replacement
  on Windows as well as Unix. Cancellation retains exit code 130.
- Report malformed/unreadable configuration while preserving credentials,
  configuration location, unknown keys, and environment precedence.

Native artifacts continue to cover Linux, macOS, and Windows on amd64 and arm64.
