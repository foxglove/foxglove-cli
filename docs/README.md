# Foxglove CLI documentation

- [Installation and getting started](../README.md)
- [Command reference](reference/README.md): generated from the CLI's command definitions
- [Migrating from v1 to v2](v2-migration.md)

## Updating the command reference

Run these commands from the repository root:

```sh
make docs        # Regenerate docs/reference/
make docs-check  # Check that every generated page is current
```

Edit command descriptions and argument help in `rust/src/cli.rs`, then run
`make docs` and commit the generated pages alongside the code change. The
generator uses Clap's long help, including inherited global options, and adds
links between commands. It runs without credentials or network requests.

The entire `docs/reference/` directory is generated; regeneration removes Markdown
pages for commands that no longer exist. Keep handwritten guides outside it.
CI checks for missing, changed, and obsolete reference pages.

These docs describe the current checkout. Use `foxglove <command> --help` for the
options supported by an installed release.
