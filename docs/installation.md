# Installation

[Documentation index](README.md)

Download the latest release for your OS and architecture:

| OS/Arch | Download command |
| --- | --- |
| macOS/arm64 (Apple Silicon) | `curl -fL https://github.com/foxglove/foxglove-cli/releases/latest/download/foxglove-macos-arm64 -o foxglove && chmod +x foxglove` |
| macOS/amd64 (Intel) | `curl -fL https://github.com/foxglove/foxglove-cli/releases/latest/download/foxglove-macos-amd64 -o foxglove && chmod +x foxglove` |
| Linux/amd64 | `curl -fL https://github.com/foxglove/foxglove-cli/releases/latest/download/foxglove-linux-amd64 -o foxglove && chmod +x foxglove` |
| Linux/arm64 | `curl -fL https://github.com/foxglove/foxglove-cli/releases/latest/download/foxglove-linux-arm64 -o foxglove && chmod +x foxglove` |
| Windows/amd64 | `curl.exe -fL https://github.com/foxglove/foxglove-cli/releases/latest/download/foxglove-windows-amd64.exe -o foxglove.exe` |
| Windows/arm64 | `curl.exe -fL https://github.com/foxglove/foxglove-cli/releases/latest/download/foxglove-windows-arm64.exe -o foxglove.exe` |

The download is saved in your current folder. Run `./foxglove --help` to try it,
or install it on macOS/Linux so you can run `foxglove` from any folder:

```sh
sudo mkdir -p /usr/local/bin
sudo install -m 755 ./foxglove /usr/local/bin/foxglove
foxglove --help
foxglove auth login
```

On Windows, run `.\foxglove.exe --help` in PowerShell, or move `foxglove.exe`
to a folder on your `Path` to use `foxglove` from any folder.

To install a specific release, see the [releases page](https://github.com/foxglove/foxglove-cli/releases).

Alternatively, build the Rust CLI from source. Rust 1.98.1 is pinned in
[`rust-toolchain.toml`](../rust-toolchain.toml); [rustup](https://rustup.rs/) installs it automatically
when you run Cargo in this checkout:

    $ git clone git@github.com:foxglove/foxglove-cli.git
    $ cd foxglove-cli
    $ make build
    $ ./rust/target/release/foxglove-rust --help

`make install` copies that source build to `foxglove` in Cargo's bin directory
(`$CARGO_HOME/bin`, or `~/.cargo/bin` by default) on Unix-like systems.
