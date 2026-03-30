# jc — Claude Code Multi-Session Orchestrator

Cross-platform port of [jeapostrophe/jc](https://github.com/jeapostrophe/jc), a multi-session Claude Code orchestrator built with Rust and [iced](https://iced.rs).

## Features

- **Multi-session management** — run multiple Claude Code sessions in parallel across projects
- **Problem cycling** — 4-layer priority system (L0 cross-session errors → L3 idle sessions) navigated with Ctrl+;
- **Multi-pane layout** — 1/2/3 configurable panes: Claude terminal, general terminal, code viewer, TODO editor, git diff
- **Hook integration** — receives Claude Code hook events (prompt submit, stop, permission) for real-time status
- **File picker** — fuzzy file/session/project/command picker (Ctrl+P)
- **TODO.md editor** — structured TODO tracking with session assignments and validation
- **Git diff viewer** — review changes with per-file tracking
- **IPC singleton** — `jc .` routes to an already-running instance

## Prerequisites

### WSL2 / Linux

```bash
# Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# System dependencies
sudo apt install cmake libssl-dev pkg-config libfontconfig1-dev

# For WSLg GUI support (usually pre-installed)
# Verify: echo $DISPLAY should show :0 or similar
```

### Claude CLI

The `claude` CLI must be in PATH. Install from [claude.ai](https://claude.ai/download).

## Build & Install

```bash
# Quick install to ~/.local/bin/
./install.sh

# Or build manually (use /tmp target dir on Windows mount to avoid cross-device link errors)
CARGO_TARGET_DIR=/tmp/jc-build cargo build --release -p jc-app
```

## Usage

```bash
# Open current directory
jc .

# Open a specific project
jc /path/to/project

# Remove stale hooks from all projects
jc clean-hooks
```

## Keybindings

| Key | Action |
|-----|--------|
| Ctrl+1..9 | Switch to session 1-9 |
| Ctrl+T | New session |
| Ctrl+W | Close session |
| Ctrl+; | Next problem |
| Ctrl+P | File picker |
| Ctrl+Shift+P | Command palette |
| Ctrl+K | Snippet picker |
| Ctrl+O | Project picker |
| Ctrl+D | Git diff |
| Ctrl+J | Toggle pane layout |
| Ctrl+/ | Move focus between panes |
| Ctrl+S | Save file |
| Ctrl+L | Go to line |
| Ctrl+? | Keybinding help |
| Ctrl+Q | Quit |

## Architecture

```
jc-core/       — config, hooks, TODO parsing, problems (unchanged from upstream)
jc-platform/   — cross-platform: notifications (notify-rust), IPC (Unix socket), signals
jc-terminal/   — terminal emulation: PTY (portable-pty), colors, input mapping
jc-app/         — iced GUI application: views, workspace, pickers, keybindings
```

## Platform Notes

### WSL2

- Build on ext4 filesystem (`~/projects/`) not Windows mount (`/mnt/c/`)
- Or set `CARGO_TARGET_DIR=/tmp/jc-build` to avoid cross-device link errors
- WSLg provides the display server; verify with `echo $DISPLAY`
- D-Bus must be running for desktop notifications (`systemctl --user start dbus` if needed)

### Known Limitations

- Terminal canvas rendering is not yet implemented — terminal panes show placeholder text
- VTE parsing is deferred until terminal rendering is added
- Syntax highlighting in code viewer is not yet wired up (tree-sitter grammars are linked)

## License

See upstream [jeapostrophe/jc](https://github.com/jeapostrophe/jc) for license terms.
