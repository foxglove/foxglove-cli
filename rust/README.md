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
