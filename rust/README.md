# Rust CLI development

Run these commands from the **repository root**:

```sh
make rust-lint rust-test-ignored rust-doc
cargo install cargo-audit --locked --version 0.22.2
make rust-audit
make lint
make test
make build
./rust/target/release/foxglove-rust --help
```

Run `make docs` after changing command definitions to regenerate the
[command reference](../docs/reference/README.md). `make docs-check` checks for
stale generated pages without modifying them.

## Publishing a release

1. Open [GitHub Releases](https://github.com/foxglove/foxglove-cli/releases) and click
   **Draft a new release**. Under **Choose a tag**, enter the new version
   (for example, `v2.0.0`) and select **Create new tag** with **Target: main**.
   Tags must use `vMAJOR.MINOR.PATCH`, optionally followed by a prerelease suffix
   such as `v2.0.0-rc.1`. CI rejects tags without the `v` prefix or three version numbers.
2. Enter a title and click **Generate release notes**.
3. Click **Publish release** to start the CI workflow. Saving a draft or pushing a tag alone does not build a release.
4. Check the release's CI workflow in [Actions](https://github.com/foxglove/foxglove-cli/actions).
   Once it passes, the release will have all six binaries and `CHECKSUMS.txt`.
