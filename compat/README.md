# Foxglove CLI compatibility contract

This directory contains the language-neutral compatibility contract for the
Go-to-Rust migration. The v1.0.33 fixtures are captured by the black-box Go
harness in `foxglove/compat`; future Rust integration tests will read the same
files.

The committed goldens are reviewed inputs, not disposable snapshots. To
refresh them intentionally:

```sh
cd foxglove
UPDATE_COMPAT_GOLDENS=1 go test ./compat -count=1
git diff -- ../compat/goldens
```

Do not accept a changed golden merely because the generator produced it.
Explain the contract change in `MIGRATION.md`. Rust behavior may differ from
the Go goldens only where `approved_deltas.json` explicitly defines the new
contract.

All harness runs use an isolated home directory and a deterministic local HTTP
server. Dynamic paths, fixture ports, and the known Go empty-CSV stack trace
are normalized before comparison.
