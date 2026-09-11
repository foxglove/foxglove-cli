#!/usr/bin/env bash
# Build, strip where supported, and smoke-test one native Rust release asset.
# The GitHub workflow invokes this only on a runner matching PLATFORM/ARCH.
set -euo pipefail

platform=${1:?platform is required}
arch=${2:?architecture is required}
case "$platform/$arch" in
  linux/amd64|linux/arm64|macos/amd64|macos/arm64|windows/amd64|windows/arm64) ;;
  *) echo "unsupported release platform: $platform/$arch" >&2; exit 2 ;;
esac

# Avoid silently producing a host binary under another platform's filename.
host_os=$(uname -s)
case "$platform" in
  linux) [[ "$host_os" == Linux ]] ;;
  macos) [[ "$host_os" == Darwin ]] ;;
  windows) [[ "$host_os" == MINGW* || "$host_os" == MSYS* || "$host_os" == CYGWIN* ]] ;;
esac || { echo "release runner is not native $platform/$arch (host OS: $host_os)" >&2; exit 2; }
# `uname -m` reports the architecture of the current shell process.  Git Bash
# can run x64 under emulation on a native Windows ARM64 runner, so validate the
# Rust host triple instead: it determines the architecture of `cargo build`'s
# default output target.
rust_host=$(rustc -vV | sed -n 's/^host: //p')
case "$arch/$rust_host" in
  amd64/x86_64-*) ;;
  arm64/aarch64-*) ;;
  *) echo "release Rust target does not match $arch (host: ${rust_host:-unknown})" >&2; exit 2 ;;
esac

root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$root"

case "$platform" in
  windows) suffix=.exe ;;
  *) suffix= ;;
esac
artifact="foxglove-${platform}-${arch}${suffix}"
source_binary="rust/target/release/foxglove-rust${suffix}"
release_dir=${RELEASE_DIR:-dist}

cargo build --manifest-path rust/Cargo.toml --locked --release --bin foxglove-rust
test -f "$source_binary"
mkdir -p "$release_dir"
cp "$source_binary" "$release_dir/$artifact"

# Windows PE binaries are left intact; macOS code signing must happen after
# packaging, so do not mutate its binary after this point.
if [[ "$platform" == linux ]] && command -v strip >/dev/null 2>&1; then
  strip --strip-unneeded "$release_dir/$artifact"
fi

"$release_dir/$artifact" --help >/dev/null
if [[ -n "${FOXGLOVE_VERSION:-}" ]]; then
  actual_version=$("$release_dir/$artifact" version)
  if [[ "$actual_version" != "$FOXGLOVE_VERSION" ]]; then
    echo "release version mismatch: expected $FOXGLOVE_VERSION, got $actual_version" >&2
    exit 1
  fi
fi
case "$platform" in
  windows) certutil -hashfile "$release_dir/$artifact" SHA256 | awk -v name="$artifact" 'NR == 2 { print tolower($1) "  " name }' > "$release_dir/$artifact.sha256" ;;
  *) shasum -a 256 "$release_dir/$artifact" | awk -v name="$artifact" '{ print $1 "  " name }' > "$release_dir/$artifact.sha256" ;;
esac

echo "packaged $release_dir/$artifact"
