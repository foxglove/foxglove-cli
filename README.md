# Foxglove CLI

Interact with Foxglove, directly from the command line.

## Installation

Download the binary for your OS and architecture from the
[latest release](https://github.com/foxglove/foxglove-cli/releases/latest).
See the [installation guide](docs/installation.md) for download commands,
platform setup, and building from source.

## Getting started

Create a [Foxglove account](https://app.foxglove.dev/signup), then log in and
upload a recording:

```sh
foxglove auth login
foxglove devices add --name RobotA
foxglove upload recording.mcap --device-name RobotA
```

## Documentation

- [Usage guide](docs/usage.md): authentication, pagination, uploads, exports, datasets, and more
- [Command reference](docs/reference/README.md): every command and option, generated from the code
- [Migrating from v1 to v2](docs/v2-migration.md)
- [Release notes](https://github.com/foxglove/foxglove-cli/releases)

See the [documentation index](docs/README.md) for the full guide and instructions
for updating the reference.

## Development

See the [development guide](rust/README.md) for building, testing, and
publishing releases.

## Stay in touch

Join our [Discord community](https://foxglove.dev/chat) to ask questions, share
feedback, and stay up to date on what our team is working on.
