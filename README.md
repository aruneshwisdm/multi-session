# jc — Run Multiple Claude Code Sessions in One Window

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-2024_edition-orange.svg)](https://www.rust-lang.org/)
[![Platform](https://img.shields.io/badge/Platform-Linux_%7C_WSL2-green.svg)](#prerequisites)

**jc** is a multi-session orchestrator for [Claude Code](https://claude.ai). Run parallel Claude sessions across projects, track their status, and switch between them instantly — all from a single GUI window.

Cross-platform port of [jeapostrophe/jc](https://github.com/jeapostrophe/jc) (macOS), rebuilt for **Linux and Windows (WSL2)** using the [iced](https://iced.rs) GUI framework.

## Why jc?

If you use Claude Code heavily, you've probably hit this:

- You ask Claude to implement something, and while it's working, you have nothing to do
- You open another terminal, start a second Claude session, but lose track of the first one
- A session asks for permission and you don't notice for 10 minutes

jc solves this with a purpose-built orchestrator:

```
+------------------+------------------+------------------+
|  Claude Session  |   TODO Editor    |   Code Viewer    |
|                  |                  |                  |
|  > implementing  |  ## Auth feature |  fn validate()   |
|    auth flow...  |  - [x] model     |    ...           |
|                  |  - [ ] endpoint  |                  |
+------------------+------------------+------------------+
  [Session 1*] [Session 2] [Session 3]    my-project
       ^busy        ^idle       ^needs permission
```

## Features

- **Multi-session** — run 1-9 Claude Code sessions in parallel, switch with Ctrl+1..9
- **Smart problem cycling** — Ctrl+; walks you through what needs attention (errors > permission prompts > unreviewed diffs > idle sessions)
- **Hook integration** — knows when Claude is busy, idle, needs permission, or has failed
- **Multi-pane layout** — 1/2/3 panes showing any mix of: terminal, code viewer, TODO editor, git diff
- **Built-in editors** — edit code and TODO.md with syntax highlighting, right inside jc
- **File/command picker** — fuzzy-search files (Ctrl+P), commands (Ctrl+Shift+P), snippets (Ctrl+K)
- **Git diff review** — see what Claude changed, track reviewed files
- **IPC singleton** — running `jc .` from another terminal focuses the existing window

## Quick Start

### 1. Install prerequisites

**Linux / WSL2:**

```bash
# System dependencies
sudo apt update && sudo apt install -y \
  build-essential cmake libssl-dev pkg-config \
  libfontconfig1-dev libxkbcommon-dev libwayland-dev wayland-protocols

# Rust toolchain (if you don't have it)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
```

**Claude CLI** must be in your PATH — install from [claude.ai/download](https://claude.ai/download).

### 2. Install jc

```bash
git clone https://github.com/aruneshwisdm/multi-session.git
cd multi-session
./install.sh
```

This builds a release binary and installs it to `~/.local/bin/jc`. Make sure `~/.local/bin` is in your PATH:

```bash
export PATH="$HOME/.local/bin:$PATH"  # add to ~/.bashrc to persist
```

<details>
<summary>Manual build (if install.sh doesn't suit your setup)</summary>

```bash
cargo build --release -p jc-app
cp target/release/jc-app ~/.local/bin/jc
```

On WSL2 building from a Windows mount (`/mnt/c/...`), set the target dir to avoid cross-device link errors:

```bash
CARGO_TARGET_DIR=/tmp/jc-build cargo build --release -p jc-app
```

</details>

### 3. Run

```bash
# Open your project
jc .

# Or specify a path
jc ~/projects/my-app

# Clean up stale hooks from all projects
jc clean-hooks
```

## Keybindings

All shortcuts use **Ctrl** as the modifier.

| Key | Action |
|-----|--------|
| **Sessions** | |
| Ctrl+T | New Claude session |
| Ctrl+W | Close active session |
| Ctrl+1..9 | Switch to session 1-9 |
| **Navigation** | |
| Ctrl+; | Next problem (cycles through issues by priority) |
| Ctrl+/ | Move focus between panes |
| Ctrl+J | Reset to 3-pane layout |
| Ctrl+D | Show git diff |
| **Pickers** | |
| Ctrl+P | File picker |
| Ctrl+Shift+P | Command palette |
| Ctrl+K | Snippet picker |
| Ctrl+O | Project picker |
| Ctrl+L | Go to line |
| **Editing** | |
| Ctrl+S | Save file |
| Ctrl+Shift+C | Copy from terminal |
| Ctrl+Shift+V | Paste into terminal |
| **Other** | |
| Ctrl+? | Show keybinding help |
| Ctrl+Q | Quit |

## Architecture

```
jc-core/       Config, hooks, TODO parsing, problems (shared with upstream)
jc-platform/   Cross-platform abstractions: notifications, IPC, signals
jc-terminal/   Terminal emulation: alacritty_terminal + portable-pty
jc-app/        GUI application built with iced (Elm architecture)
```

Built with:

| Component | Crate |
|-----------|-------|
| GUI framework | [iced](https://iced.rs) 0.14 (wgpu renderer) |
| Terminal emulation | [alacritty_terminal](https://crates.io/crates/alacritty_terminal) 0.25 |
| PTY | [portable-pty](https://crates.io/crates/portable-pty) 0.9 |
| Syntax highlighting | [tree-sitter](https://tree-sitter.github.io/) (Rust, JS, TS, Python, Go, Markdown) |
| Git operations | [git2](https://crates.io/crates/git2) (libgit2) |

## Platform Notes

### WSL2 (Windows)

- **Build on the Linux filesystem** (`~/projects/`), not Windows mounts (`/mnt/c/`). The ext4 filesystem is 5-10x faster for builds.
- WSLg provides GUI support automatically on Windows 11. Verify: `echo $DISPLAY` should print `:0`.
- If D-Bus isn't running for notifications: `systemctl --user start dbus` (notifications are optional).

### Linux

Works natively on any Linux desktop with Wayland or X11.

## Documentation

- **[WSL2 Getting Started Guide](docs/WSL2-GUIDE.md)** — detailed setup, workflow, and troubleshooting for new users
- **[How It Works](HOW-IT-WORKS.md)** — plain-English guide explaining jc's internals

## Contributing

```bash
# Run tests
cargo test --workspace

# Build in debug mode
cargo build -p jc-app

# Run from source
cargo run -p jc-app -- .
```

## Credits

- Original [jc](https://github.com/jeapostrophe/jc) by Jay McCarthy (macOS/GPUI)
- Windows/WSL2 port using [iced](https://iced.rs) framework

## License

[MIT](LICENSE)
