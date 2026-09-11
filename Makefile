.PHONY: lint test compat go-test rust-fmt rust-lint rust-test rust-test-ignored rust-doc rust-audit rust-build build install release-package

RUST_MANIFEST := rust/Cargo.toml

lint:
	$(MAKE) -C foxglove lint
	$(MAKE) rust-lint

rust-fmt:
	cargo fmt --manifest-path $(RUST_MANIFEST) --check

rust-lint: rust-fmt
	cargo clippy --manifest-path $(RUST_MANIFEST) --locked --all-targets --all-features -- -D warnings

rust-test:
	cargo test --manifest-path $(RUST_MANIFEST) --locked --all-features

# Loopback HTTP contract tests are deliberately ignored by default so they can
# run in restricted developer environments. CI and release candidates run them.
rust-test-ignored:
	cargo test --manifest-path $(RUST_MANIFEST) --locked --all-features -- --include-ignored

rust-doc:
	RUSTDOCFLAGS="-D warnings" cargo doc --manifest-path $(RUST_MANIFEST) --locked --all-features --no-deps

# Install the pinned tool with `cargo install cargo-audit --locked --version 0.22.2`.
# RUSTSEC-2024-0436 is the reviewed unmaintained `paste` build dependency
# documented in RELEASE.md; known vulnerabilities still fail this command.
rust-audit:
	cargo audit --file rust/Cargo.lock --ignore RUSTSEC-2024-0436

go-test:
	cd foxglove && go test ./... -count=1

test:
	$(MAKE) rust-test
	$(MAKE) go-test

compat:
	$(MAKE) -C foxglove compat

build:
	$(MAKE) rust-build

rust-build:
	cargo build --manifest-path $(RUST_MANIFEST) --locked --release

# Package a native Rust release artifact. PLATFORM is one of linux, macos, or
# windows; ARCH is amd64 or arm64. Cross-compilation is intentionally not
# hidden here: the release workflow builds each artifact on its native runner.
release-package:
	@test -n "$(PLATFORM)" && test -n "$(ARCH)" || (echo "Usage: make release-package PLATFORM=<linux|macos|windows> ARCH=<amd64|arm64>" >&2; exit 2)
	./scripts/package-rust-release.sh "$(PLATFORM)" "$(ARCH)"

install:
	cargo build --manifest-path $(RUST_MANIFEST) --locked --release --bin foxglove-rust
	@set -eu; \
	install_root="$${CARGO_INSTALL_ROOT:-$${CARGO_HOME:-$${HOME}/.cargo}}"; \
	case "$$(uname -s)" in MINGW*|MSYS*|CYGWIN*) suffix=.exe ;; *) suffix= ;; esac; \
	mkdir -p "$$install_root/bin"; \
	cp "rust/target/release/foxglove-rust$$suffix" "$$install_root/bin/foxglove$$suffix"
