# JC Port — Claude Code Multi-Session Orchestrator for Windows + WSL2

## Project Overview

Cross-platform port of [jeapostrophe/jc](https://github.com/jeapostrophe/jc) — a Rust application that manages multiple Claude Code sessions across projects from a single window. The original is macOS-only (GPUI + ObjC). We are porting to Windows + WSL2 using iced as the GUI framework.

See [jc-port-analysis.md](./jc-port-analysis.md) for research and [jc-port-execution-plan.md](./jc-port-execution-plan.md) for the detailed plan.

## Architecture

```
jc-core        (unchanged)    Data models, config, hooks, TODO parser, themes
jc-platform    (new)          Platform abstractions: notifications, IPC, signals
jc-terminal    (partial port) Terminal emulation via alacritty_terminal + portable-pty
jc-app         (rewrite)      GUI shell using iced framework
```

### Crate Dependency Graph

```
jc-app → jc-terminal → jc-core
jc-app → jc-platform
jc-app → jc-core
jc-terminal → jc-core
```

## Tech Stack

| Layer | Crate | Notes |
|-------|-------|-------|
| GUI framework | `iced` 0.14 | Elm-reactive, wgpu renderer (Vulkan/DX12/GL) |
| Terminal widget | `iced_term` | alacritty_terminal backend |
| Terminal emulation | `alacritty_terminal` 0.25 | VTE parser, cross-platform |
| PTY | `portable-pty` 0.9 | Unix PTY / Windows ConPTY |
| Notifications | `notify-rust` 4 | D-Bus on Linux, WinRT on Windows |
| IPC | Unix sockets (Linux) / Named Pipes (Windows) | Via `interprocess` or `#[cfg]` |
| Signals | `ctrlc` 3 | Cross-platform Ctrl+C handler |
| Git | `git2` 0.20 | libgit2 bindings |
| Syntax | `tree-sitter` 0.25 | 6 grammar crates (Rust, JS, TS, Python, Go, Markdown) |
| Diff | `similar` + `diffy` | Pure Rust |
| Clipboard | `arboard` 3 | Win32 / X11 / Wayland |
| File watching | `notify` 7 | inotify / ReadDirectoryChanges |
| Serialization | `serde` + `toml` + `serde_json` | Config and state persistence |
| CLI | `clap` 4 | Argument parsing |

## Build & Run

### Prerequisites (WSL2)

```bash
# Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# System dependencies
sudo apt update
sudo apt install cmake libssl-dev pkg-config libfontconfig1-dev \
  libxkbcommon-dev libwayland-dev wayland-protocols

# Verify WSLg GPU acceleration
glxinfo | grep "OpenGL renderer"
```

### Build

```bash
cargo build -p jc-app
```

### Run

```bash
# Register current directory as a project
cargo run -p jc-app -- .

# Clean hooks from a project
cargo run -p jc-app -- clean-hooks
```

### Test

```bash
# Core tests (pure logic, no GUI)
cargo test -p jc-core

# All tests
cargo test --workspace
```

## Code Conventions

### Rust Style
- Edition 2024
- Use `anyhow::Result` for error handling throughout
- Use `flume` channels for cross-thread communication (not `std::sync::mpsc`)
- Use `parking_lot` mutexes (not `std::sync::Mutex`)
- Config/state stored at `~/.config/jc/` via `dirs` crate

### Platform Abstraction Pattern

All platform-specific code lives in `jc-platform`. Use conditional compilation:

```rust
// In jc-platform/src/notifications.rs
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "macos")]
mod macos;
```

Platform-specific dependencies go in target-gated sections of `Cargo.toml`:

```toml
[target.'cfg(target_os = "linux")'.dependencies]
notify-rust = "4"

[target.'cfg(target_os = "windows")'.dependencies]
winrt-notification = "0.5"
```

Never put `#[cfg]` blocks in `jc-core`. It must remain platform-agnostic.

### iced Application Pattern

The app follows iced's Elm architecture:

```
State → view() → Element (UI)
     → update(Message) → Command (side effects)
     → subscription() → Subscription (async events)
```

- All state mutations happen in `update()`
- All async work (PTY reads, hook events, IPC, file watching) goes through `Subscription`
- UI is a pure function of state in `view()`
- Use the `Message` enum for all events — never mutate state outside `update()`

### Keybindings

Map macOS `Cmd` to `Ctrl` on Windows/Linux. The canonical mapping is documented in the execution plan (section 3.3.2). Key examples:

| Action | Key |
|--------|-----|
| New session | `Ctrl+T` |
| Close session | `Ctrl+W` |
| File picker | `Ctrl+P` |
| Command palette | `Ctrl+Shift+P` |
| Switch session 1-9 | `Ctrl+1` through `Ctrl+9` |
| Cycle problems | `Ctrl+;` |
| Toggle pane layout | `Ctrl+J` |
| Quit | `Ctrl+Q` |

## Directory Structure

```
multi-session/
├── CLAUDE.md                      # This file
├── jc-port-analysis.md            # Research: platform equivalents, WSL2 capabilities
├── jc-port-execution-plan.md      # Detailed execution plan with file tracker
├── Cargo.toml                     # Workspace root
├── jc-core/                       # Data models, config, hooks (UNCHANGED from upstream)
│   └── src/
│       ├── lib.rs
│       ├── config.rs              # AppConfig, AppState, path helpers
│       ├── hooks.rs               # HookServer (tiny_http + flume)
│       ├── hooks_settings.rs      # .claude/settings.local.json management
│       ├── model.rs               # Project, WindowLayout
│       ├── problem.rs             # 4-layer problem system (L0-L3)
│       ├── snippets.rs            # Snippet document parser
│       ├── status_script.rs       # Status script runner
│       ├── theme.rs               # ThemeConfig (dark/light palettes)
│       └── todo.rs                # TODO.md parser/validator (~1000 lines)
├── jc-platform/                   # NEW: platform abstractions
│   └── src/
│       ├── lib.rs
│       ├── notifications.rs       # Notifier trait + platform impls
│       ├── ipc.rs                 # IPC trait + platform impls
│       └── signals.rs             # Cross-platform signal handling
├── jc-terminal/                   # Terminal emulation (partial rewrite)
│   └── src/
│       ├── lib.rs
│       ├── pty.rs                 # PtyHandle (portable-pty) — UNCHANGED
│       ├── terminal.rs            # TerminalState (alacritty_terminal) — UNCHANGED
│       ├── colors.rs              # Palette — adapt types to iced::Color
│       ├── input.rs               # Keystroke mapping — adapt to iced keyboard
│       ├── render.rs              # Terminal rendering — REWRITE for iced
│       └── view.rs                # Terminal widget — REWRITE for iced
├── jc-app/                        # GUI application (full rewrite)
│   └── src/
│       ├── main.rs                # Entry point, CLI, IPC singleton
│       ├── app.rs                 # iced::Application impl
│       ├── theme.rs               # ThemeConfig → iced::Theme conversion
│       ├── file_watcher.rs        # iced Subscription for notify crate
│       ├── language.rs            # Language enum — UNCHANGED
│       ├── outline.rs             # Tree-sitter outline — UNCHANGED
│       ├── subscriptions.rs       # PTY, hooks, IPC, signals subscriptions
│       └── views/
│           ├── mod.rs             # Shared helpers
│           ├── pane.rs            # PaneGrid layout
│           ├── picker.rs          # Generic picker + 8 delegates
│           ├── session_state.rs   # Session data (port, remove Entity)
│           ├── project_state.rs   # Project data (port, remove Entity)
│           ├── code_view.rs       # Code editor (iced TextEditor)
│           ├── diff_view.rs       # Git diff viewer
│           ├── todo_view.rs       # TODO.md editor
│           ├── comment_panel.rs   # Annotation modal
│           ├── keybinding_help.rs # Help overlay
│           ├── close_confirm.rs   # Quit confirmation
│           └── workspace.rs       # Main orchestrator (state + update + view)
└── data/
    ├── fonts/                     # Lilex font family
    ├── dark_theme.toml
    └── light_theme.toml
```

## Important Constraints

1. **jc-core is sacred.** Do not modify it. It must compile identically to upstream. This gives us 2,274 lines of tested, portable logic for free and makes syncing with upstream trivial.

2. **No GPUI anywhere.** The entire point of this port is removing GPUI. Never add `gpui` as a dependency. Use `iced` for all GUI needs.

3. **No macOS-specific code outside jc-platform.** All `objc2`, `block2`, `AppKit`, and `Foundation` usage is confined to `jc-platform/src/notifications/macos.rs` (future macOS re-support). The rest of the codebase must compile on Linux and Windows.

4. **Clone to ext4, not /mnt/c/.** WSL2 file I/O on Windows mounts is significantly slower. Always work from `~/projects/` or similar ext4 path. The planning docs live on the Windows mount for easy access but the source code should not.

5. **Prefer iced_term over custom terminal rendering.** Only fall back to custom `iced::widget::canvas::Program` if `iced_term` doesn't meet requirements (see Risk Register in execution plan).

6. **Message-driven architecture.** All state changes go through the iced `Message` enum and `update()`. No direct state mutation from event handlers, subscriptions, or view code.

7. **Keep upstream git history.** Fork from `jeapostrophe/jc`, work on a `port/windows-wsl2` branch. This preserves attribution and enables future rebasing onto upstream improvements.

## Current Phase

**Phase 5: Packaging & Distribution** — Completed.

All phases (0-5) are done. The app compiles with 0 errors and 0 warnings. 55 tests pass.

**Remaining work for full feature parity:**
- Terminal canvas rendering (Phase 3b) — PTY infrastructure is in place, rendering is placeholder
- VTE parsing integration — deferred until terminal rendering
- Syntax highlighting in code viewer — tree-sitter grammars are linked but not wired to display
- Full text editing in code viewer and TODO editor (currently read-only display)

### Build Notes

On Windows mount (`/mnt/c/...`), use `CARGO_TARGET_DIR=/tmp/jc-build` to avoid cross-device link errors.

## Upstream Sync Strategy

The upstream `jc` repo is actively developed. To incorporate improvements:

1. `jc-core` changes: cherry-pick or rebase directly (we don't modify this crate)
2. `jc-terminal` logic changes: review and port (pty.rs, terminal.rs are shared; render/view diverged)
3. `jc-app` feature additions: manually port the feature logic into our iced implementation
4. New upstream views/widgets: implement using iced equivalents

Periodically check upstream for:
- New hook event types in `jc-core::hooks`
- New problem types in `jc-core::problem`
- TODO.md format changes in `jc-core::todo`
- New keybindings in `workspace/mod.rs`
