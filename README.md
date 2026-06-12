# notion-calendar-cli

Agent-oriented Rust CLI for Notion Calendar automation.

`ncal` wraps the Notion Calendar HTTP surface characterized in this repo, with stable command output for humans and scripts. It prefers safe, documented behavior where available and labels the unpublished Notion Calendar endpoints as version-fragile.

[![CI](https://github.com/OlegHQ/notion-calendar-cli/actions/workflows/ci.yml/badge.svg)](https://github.com/OlegHQ/notion-calendar-cli/actions/workflows/ci.yml)
![Version](https://img.shields.io/badge/version-v0.1.0-blue)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
![Built with Rust](https://img.shields.io/badge/built%20with-Rust-orange)

## Install

Prebuilt binaries are published on GitHub Releases for macOS and Linux.

### Homebrew - macOS and Linux

```sh
brew install OlegHQ/tap/ncal-cli
```

Upgrade with:

```sh
brew upgrade ncal-cli
```

### Install Script - macOS and Linux

```sh
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/OlegHQ/notion-calendar-cli/releases/latest/download/ncal-cli-installer.sh | sh
```

### Prebuilt Archives

Download an archive from the [latest release](https://github.com/OlegHQ/notion-calendar-cli/releases/latest), extract it, and put `ncal` on your `PATH`.

Supported targets:

| Platform | Target |
| --- | --- |
| macOS Apple Silicon | `aarch64-apple-darwin` |
| macOS Intel | `x86_64-apple-darwin` |
| Linux x86-64 | `x86_64-unknown-linux-gnu` |
| Linux ARM64 | `aarch64-unknown-linux-gnu` |

Each release includes checksums and a `dist-manifest.json`.

### From Source

```sh
cargo install --path crates/ncal-cli
# or
make install
```

Source builds require a stable Rust toolchain.

## Quick Start

Import the current desktop app session:

```sh
ncal auth from-app
```

Or start the browser-based Notion login flow:

```sh
ncal auth login
```

Check authentication and list calendar data:

```sh
ncal whoami
ncal calendars list
ncal events list
```

Use `--json` for machine-readable output:

```sh
ncal events list --json
```

## Commands

| Command | Description |
| --- | --- |
| `ncal auth login` | OAuth sign-in flow |
| `ncal auth from-app` | Import tokens from the desktop app |
| `ncal auth status` | Show credential source and expiry |
| `ncal auth refresh` | Refresh stored credentials |
| `ncal auth logout` | Remove stored credentials |
| `ncal whoami` | Show current authenticated user |
| `ncal accounts list` | List connected accounts |
| `ncal accounts set-primary <ID>` | Set primary account |
| `ncal accounts remove <ID>` | Disconnect account |
| `ncal calendars list` | List calendars |
| `ncal calendars colors` | Show calendar colors |
| `ncal events list` | List events |
| `ncal events get <ID>` | Show an event |
| `ncal events create` | Create an event |
| `ncal events update <ID>` | Update an event |
| `ncal events delete <ID>` | Cancel or delete an event |
| `ncal holds ...` | Manage scheduling holds |
| `ncal sync` | Fetch incremental changes |
| `ncal contacts` | List contacts |
| `ncal preferences get` | Show user preferences |

## Output Contract

The CLI is designed for both humans and automation:

- Default output is human-readable tables or key-value text.
- `--json` writes compact JSON data to stdout.
- Errors and hints go to stderr.
- JSON errors use `{ "error", "code", "hint" }`.
- Exit codes are stable: `0` success, `2` usage, `3` auth, `4` API/network/I/O.

## Auth and Stability

`ncal auth from-app` reads the Notion Calendar desktop app's Chromium LocalStorage LevelDB from `~/Library/Application Support/Notion Calendar/Local Storage/leveldb/` on macOS. Do not commit tokens, cookies, captures, or personal calendar data.

The backend API base is `https://calendar-api.notion.so`. This is an unpublished Notion Calendar API surface and can change without notice. The research notes and API contract in `research/` are the source of truth for what has been confirmed.

## Development

```sh
make hooks   # install pre-commit/pre-push hooks
make fix     # rustfmt + clippy --fix
make ci      # fmt-check + clippy + tests
make build   # release build
```

The pre-commit and pre-push hooks auto-run `cargo fmt` and `cargo clippy --fix`. If they change files, they stop so you can review and stage the edits before continuing.

## Releasing

Releases are automated with [cargo-dist](https://github.com/axodotdev/cargo-dist). Pushing a `v*` tag builds macOS and Linux binaries, creates GitHub Release assets, publishes checksums and the installer script, and pushes the Homebrew formula to `OlegHQ/homebrew-tap`.

Maintainer commands:

| Goal | Command |
| --- | --- |
| Bump patch + tag + push | `make ship-patch` |
| Bump minor + tag + push | `make ship-minor` |
| Validate release plan | `make dist-plan` |

Homebrew publishing requires a repo secret:

```sh
gh secret set HOMEBREW_TAP_TOKEN
```

The token must have write access to `OlegHQ/homebrew-tap`.

## License

MIT - see [LICENSE](LICENSE).
