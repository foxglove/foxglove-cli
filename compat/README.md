# Foxglove CLI compatibility contract

This directory contains the language-neutral compatibility contract for the
Rust CLI. The v1.0.33 fixtures are captured by the black-box Go
harness in `foxglove/compat`; the Rust compatibility tests read the same
files.

Keep the v1.0.33 Go source unchanged: the harness checks it against the release
tag. Preserve command arguments, defaults, validation, stdout/stderr, exit
status, configuration, HTTP requests, and file behavior except for the approved
differences. Rewritten MCAP and ROS bag files are compared semantically.

The committed goldens are reviewed inputs, not disposable snapshots. To
refresh them intentionally:

```sh
cd foxglove
UPDATE_COMPAT_GOLDENS=1 go test ./compat -count=1
git diff -- ../compat/goldens
```

Do not accept a changed golden merely because the generator produced it.
Document intentional behavior changes and their rationale in
[`approved_deltas.json`](approved_deltas.json), the source of truth for accepted
differences from the Go baseline. Deprecated commands omitted from the Rust
release CLI are listed there and excluded from the Rust command-surface check.

The Rust command tree is maintained directly with normal Clap definitions in
`rust/src/cli.rs`. Help is rendered by Clap from that tree, and completion
scripts are generated from the same tree by `clap_complete`. The Go help and
completion fixtures remain a historical behavioral baseline; their exact text
and script bytes are not Rust release requirements.

All harness runs use an isolated home directory and a deterministic local HTTP
server. Dynamic paths, fixture ports, and the known Go empty-CSV stack trace
are normalized before comparison.

Additional release-candidate regressions live in
`foxglove/compat/release_candidate_test.go`. They compare real-world optional
response fields, event query parameters, and nested ROS schemas against the Go
oracle without regenerating historical goldens. They also test custom TLS trust
using a temporary CA file and Unix Ctrl-C during incomplete HTTP response bodies.
These tests use isolated configuration and do not change the OS trust store.
