# JC Port Execution Plan: Windows + WSL2

**Date:** 2026-03-30
**Source:** https://github.com/jeapostrophe/jc
**Companion:** [jc-port-analysis.md](./jc-port-analysis.md)

---

## Overview

Port the JC Claude Code orchestrator (~8,600 lines Rust) from macOS-only (GPUI + ObjC) to a cross-platform app targeting Windows + WSL2. The port preserves jc-core unchanged, rewrites the GUI layer with iced, and replaces 3 macOS-specific subsystems.

**Total estimated effort:** 3-4 weeks
**Chosen GUI framework:** iced (Elm-reactive, closest to GPUI paradigm, `iced_term` for terminals)

---

## Phase 0: Environment Setup (Day 1)

### 0.1 Prerequisites
- [ ] Install Rust toolchain (rustup) on WSL2
- [ ] Install Windows build tools: `Visual Studio Build Tools 2022` (for native Windows target)
- [ ] Install WSLg prerequisites (verify GPU acceleration: `glxinfo | grep renderer`)
- [ ] Install system deps for git2/libgit2: `sudo apt install cmake libssl-dev pkg-config`
- [ ] Install system deps for alacritty_terminal: `sudo apt install libfontconfig1-dev`

### 0.2 Repository Setup
- [ ] Fork `jeapostrophe/jc` to personal GitHub
- [ ] Clone fork into WSL2 ext4 filesystem (NOT Windows mount — I/O performance critical)
  ```bash
  cd ~/projects
  git clone git@github.com:YOUR_USER/jc.git
  cd jc
  git checkout -b port/windows-wsl2
  ```
- [ ] Verify original structure compiles on macOS if available (baseline)

### 0.3 Validation Gate
- [ ] `cargo check -p jc-core` succeeds on WSL2 (should work as-is)
- [ ] Document any unexpected failures

---

## Phase 1: Extract & Validate Portable Core (Days 2-3)

### 1.1 Validate jc-core Compiles Unchanged
**Files:** All 10 files in `jc-core/src/` (2,274 lines)
**Expected result:** Zero changes needed

```bash
cargo build -p jc-core
cargo test -p jc-core
```

- [ ] All tests pass (todo.rs has extensive tests, snippets.rs has tests, status_script.rs has tests)
- [ ] Verify `dirs` crate resolves `~/.config/jc/` correctly on Linux
- [ ] Verify `tiny_http` hook server starts on Linux
- [ ] Verify `config.rs` path functions work (`config_path()`, `state_path()`, `theme_path()`)

### 1.2 Create Platform Abstraction Crate
**New crate:** `jc-platform` — thin abstraction layer for platform-specific services

```bash
cargo init jc-platform --lib
```

Add to workspace `Cargo.toml`:
```toml
members = ["jc-core", "jc-terminal", "jc-app", "jc-platform"]
```

**File: `jc-platform/src/lib.rs`**
```rust
pub mod notifications;
pub mod ipc;
pub mod signals;
```

### 1.3 Define Notification Trait
**Replaces:** `jc-app/src/notify.rs` (215 lines, 100% macOS ObjC)

**File: `jc-platform/src/notifications.rs`**

Define trait:
```rust
pub trait Notifier: Send + Sync {
    fn init() -> anyhow::Result<Self> where Self: Sized;
    fn notify(&self, title: &str, message: &str, critical: bool, session_id: Option<&str>) -> anyhow::Result<()>;
    fn action_receiver(&self) -> Option<flume::Receiver<String>>;
}
```

Platform implementations:
- [ ] `notifications/linux.rs` — `notify-rust` with D-Bus backend
- [ ] `notifications/windows.rs` — `winrt-notification` or `notify-rust` Windows backend
- [ ] `notifications/macos.rs` — preserve original ObjC code (for future macOS support)

Conditional compilation in `Cargo.toml`:
```toml
[target.'cfg(target_os = "linux")'.dependencies]
notify-rust = "4"

[target.'cfg(target_os = "windows")'.dependencies]
notify-rust = "4"
winrt-notification = "0.5"
```

### 1.4 Define IPC Abstraction
**Replaces:** `jc-app/src/ipc.rs` (145 lines, Unix sockets)

**File: `jc-platform/src/ipc.rs`**

Define trait:
```rust
pub trait IpcTransport: Send {
    fn try_send_to_running(path: &Path) -> anyhow::Result<bool> where Self: Sized;
    fn cleanup() -> anyhow::Result<()> where Self: Sized;
    fn start_server() -> anyhow::Result<(Self, flume::Receiver<PathBuf>)> where Self: Sized;
}
```

Platform implementations:
- [ ] `ipc/unix.rs` — keep `UnixListener`/`UnixStream` at `~/.config/jc/jc.sock`
- [ ] `ipc/windows.rs` — Named Pipe `\\.\pipe\jc-orchestrator` or TCP `127.0.0.1:PORT`

### 1.5 Define Signal Handling
**Replaces:** `main.rs` lines using `libc::signal` (25 lines)

**File: `jc-platform/src/signals.rs`**
- [ ] Use `ctrlc` crate for cross-platform Ctrl+C / SIGINT / SIGTERM handling
- [ ] ~20 lines, wraps `ctrlc::set_handler`

### 1.6 Phase 1 Validation Gate
- [ ] `cargo build -p jc-core` — passes
- [ ] `cargo build -p jc-platform` — passes on WSL2
- [ ] `cargo test -p jc-core` — all tests pass
- [ ] Notification trait sends a test notification on WSL2 (D-Bus)
- [ ] IPC server starts and accepts connections on WSL2

---

## Phase 2: Port Terminal Crate (Days 4-7)

### 2.1 Analyze GPUI Usage in jc-terminal
**Files and their GPUI dependency:**

| File | Lines | GPUI Usage | Portable? |
|------|-------|-----------|-----------|
| `pty.rs` | 80 | None | Yes (100%) |
| `terminal.rs` | 75 | None | Yes (100%) |
| `colors.rs` | 100 | `gpui::Hsla`, `gpui::rgba` | Replace color types |
| `input.rs` | 80 | `gpui::Keystroke` | Replace keystroke types |
| `render.rs` | 220 | Heavy GPUI rendering | Full rewrite |
| `view.rs` | 540 | Full GPUI widget | Full rewrite |

**Portable as-is:** `pty.rs` (80 lines) + `terminal.rs` (75 lines) = 155 lines
**Needs adaptation:** `colors.rs` (100) + `input.rs` (80) = 180 lines
**Full rewrite:** `render.rs` (220) + `view.rs` (540) = 760 lines

### 2.2 Keep Portable Files Unchanged
- [ ] `pty.rs` — `PtyHandle` with `portable-pty` — works on Windows/Linux as-is
- [ ] `terminal.rs` — `TerminalState` wrapping `alacritty_terminal::term::Term` — cross-platform

### 2.3 Abstract Color System
**File:** `colors.rs` (100 lines)
**Current:** Uses `gpui::Hsla` color type and `gpui::rgba()` macro
**Action:** Replace with framework-agnostic color representation

- [ ] Define `pub struct Color { r: f32, g: f32, b: f32, a: f32 }` in jc-platform or use `iced::Color`
- [ ] Replace `gpui::Hsla` → `iced::Color` (or intermediary)
- [ ] Replace `gpui::rgba()` → `Color::from_rgba8()` or hex parser
- [ ] `Palette::resolve()` return type changes from `Hsla` → `Color`
- [ ] `Palette::for_appearance()` — logic unchanged, only type signatures

### 2.4 Abstract Input Handling
**File:** `input.rs` (80 lines)
**Current:** `pub fn keystroke_to_bytes(keystroke: &gpui::Keystroke, mode: TermMode) -> Option<Vec<u8>>`
**Action:** Replace GPUI keystroke type with iced keyboard events

- [ ] Replace `gpui::Keystroke` → `iced::keyboard::Key` + modifiers
- [ ] Map iced key events to terminal byte sequences (same logic, different input type)
- [ ] Handle: arrows, function keys, home/end/pgup/pgdn, ctrl+key combos, alt+key

### 2.5 Rewrite Terminal Rendering for iced
**File:** `render.rs` (220 lines) — full rewrite
**Current:** 3-pass GPUI rendering (backgrounds → selection → text → cursor)
**Target:** iced `Canvas` widget or `iced_term` integration

**Two approaches:**

**Approach A: Use `iced_term` (recommended)**
- [ ] Add `iced_term` dependency — provides pre-built terminal widget
- [ ] `iced_term` already uses `alacritty_terminal` as backend
- [ ] Wire `PtyHandle` from `pty.rs` into `iced_term`'s PTY interface
- [ ] Map jc-core `Palette` colors to `iced_term` theme
- [ ] Estimated: ~100 lines of glue code replacing 220 lines

**Approach B: Custom iced Canvas rendering (fallback)**
- [ ] Implement `iced::widget::canvas::Program` for terminal grid
- [ ] Port 3-pass rendering to iced canvas draw calls
- [ ] Font measurement via iced text shaping
- [ ] Estimated: ~300 lines, more control but more work

### 2.6 Rewrite Terminal View Widget
**File:** `view.rs` (540 lines) — full rewrite
**Current GPUI features used:**
- `Entity<TerminalView>` — GPUI entity model
- `Focusable` trait — focus management
- `Render` trait — frame-by-frame rendering
- `EventEmitter<TerminalViewEvent>` — event bus
- `InteractiveElement` — mouse handlers (click, drag, scroll)
- `KeyDownEvent` — keyboard input
- Clipboard via `cx.read_from_clipboard()`
- Async PTY read loop via `cx.spawn()`

**iced equivalent mapping:**

| GPUI Concept | iced Equivalent |
|-------------|----------------|
| `Entity<T>` | `iced::widget::Component` or top-level `Application` state |
| `Focusable` | `iced::widget::focus` module |
| `Render` | `impl Widget` or `Canvas::draw()` |
| `EventEmitter` | `iced::Command` / message passing |
| `InteractiveElement` mouse | `iced::mouse::Event` in `update()` |
| `KeyDownEvent` | `iced::keyboard::Event` subscription |
| Clipboard | `iced::clipboard` |
| `cx.spawn()` async | `iced::Subscription` for async PTY reads |

- [ ] Create `pub struct TerminalWidget` implementing iced widget traits
- [ ] Port mouse handling: click (position → grid cell), drag (selection), scroll
- [ ] Port keyboard handling: keystroke → bytes → PTY write
- [ ] Port clipboard: paste → bracketed paste, copy selected text
- [ ] Port async PTY reader as `iced::Subscription`
- [ ] Port bell event as iced message
- [ ] Port resize handling: window resize → PTY resize → terminal resize
- [ ] Estimated: ~400-500 lines

### 2.7 Phase 2 Validation Gate
- [ ] `cargo build -p jc-terminal` compiles on WSL2
- [ ] Terminal widget renders text from a shell process
- [ ] Keyboard input works (type commands, see output)
- [ ] Mouse selection works (click-drag to select, copy/paste)
- [ ] Terminal resizing works (PTY + grid recalculate)
- [ ] Bell event fires
- [ ] Scrollback works
- [ ] Test with: `ls --color`, `vim`, `htop` to verify VTE compatibility

---

## Phase 3: Port Application Views (Days 8-18)

This is the largest phase. Each view must be rewritten from GPUI to iced.

### 3.0 Architectural Decisions

**iced Application Model:**
```
JcApp (iced::Application)
├── State
│   ├── projects: Vec<ProjectState>     (from jc-core, reused)
│   ├── active_project: usize
│   ├── pane_layout: PaneLayout
│   ├── modal: Option<ModalKind>
│   ├── config: AppConfig               (from jc-core, reused)
│   └── hook_server: HookServer         (from jc-core, reused)
├── Message enum (all app events)
├── update() → Command                  (event handler)
├── view() → Element                    (UI rendering)
└── subscription() → Subscription       (async: PTY, hooks, IPC, file watcher)
```

**Message enum design:**
```rust
pub enum Message {
    // Terminal
    TerminalOutput(SessionId, Vec<u8>),
    TerminalInput(SessionId, iced::keyboard::Event),
    TerminalBell(SessionId),
    TerminalResize(SessionId, u16, u16),

    // Workspace
    SwitchProject(usize),
    SwitchSession(SessionId),
    NewSession,
    CloseSession(SessionId),
    CyclePane,
    SetPaneLayout(PaneLayout),

    // Hooks
    HookEvent(jc_core::HookEvent),

    // IPC
    IpcProjectOpen(PathBuf),

    // Notifications
    NotificationAction(String),

    // File watcher
    FileChanged(PathBuf),

    // Pickers
    OpenPicker(PickerKind),
    PickerSelect(PickerItem),
    PickerDismiss,

    // Code/Diff
    OpenFile(PathBuf),
    SaveFile,
    DiffReviewed,

    // Signals
    Shutdown,
}
```

### 3.1 App Shell & Window (app.rs rewrite)
**Original:** `jc-app/src/app.rs` (195 lines)
**Replaces:** GPUI app initialization, theme loading, window creation

- [ ] Implement `iced::Application` for `JcApp`
- [ ] Load `ThemeConfig` from jc-core → convert to iced `Theme`
- [ ] Load fonts (Lilex family from `data/fonts/`)
- [ ] Parse CLI args via `clap` (unchanged)
- [ ] Initialize hook server (jc-core, unchanged)
- [ ] Initialize IPC server (jc-platform)
- [ ] Initialize signal handler (jc-platform)
- [ ] Window title: `"jc — {project_name}"`
- [ ] Window size from `AppState::WindowLayout`
- [ ] Estimated: ~150 lines

### 3.2 Theme System
**Original:** Theme built from `jc-core::ThemeConfig` → `gpui::Theme` + `gpui_component::Theme`
**Target:** `jc-core::ThemeConfig` → `iced::Theme` custom

- [ ] Create `fn theme_from_config(config: &ThemeConfig) -> iced::Theme`
- [ ] Map `PaletteColors` → iced palette (background, text, primary, etc.)
- [ ] Map `EditorColors` → code view styling
- [ ] Map `SyntaxColors` → tree-sitter highlight colors
- [ ] Support dark/light appearance switching
- [ ] Estimated: ~80 lines

### 3.3 Workspace View (workspace/mod.rs rewrite)
**Original:** `workspace/mod.rs` (1,921 lines) — the orchestrator
**This is the most complex component.**

Split into sub-tasks:

#### 3.3.1 State Management (~200 lines)
- [ ] Port `Workspace` struct fields to `JcApp` state
- [ ] Port project/session lifecycle: create, switch, close
- [ ] Port hook event handling: `PromptSubmit`, `Stop`, `StopFailure`, `IdlePrompt`, `PermissionPrompt`
- [ ] Port notification triggers (workspace calls `notify::notify()`)
- [ ] Port clipboard polling (macOS-specific polling → iced clipboard API)

#### 3.3.2 Keybindings (~150 lines)
**Original:** 30+ keybindings registered via `gpui::KeyBinding::new()`

Map to iced keyboard subscriptions:
| GPUI Binding | Key | iced Handler |
|-------------|-----|-------------|
| `cmd-1` through `cmd-9` | `Ctrl+1..9` (on Windows/Linux) | Switch session |
| `cmd-t` | `Ctrl+T` | New session |
| `cmd-w` | `Ctrl+W` | Close session |
| `cmd-;` | `Ctrl+;` | Cycle problems |
| `cmd-shift-;` | `Ctrl+Shift+;` | Cycle problems reverse |
| `cmd-p` | `Ctrl+P` | File picker |
| `cmd-shift-p` | `Ctrl+Shift+P` | Command picker |
| `cmd-k` | `Ctrl+K` | Snippet picker |
| `cmd-o` | `Ctrl+O` | Project picker |
| `cmd-d` | `Ctrl+D` | Git diff |
| `cmd-j` | `Ctrl+J` | Toggle pane layout |
| `cmd-/` | `Ctrl+/` | Move focus between panes |
| `cmd-?` | `Ctrl+?` | Keybinding help |
| `cmd-q` | `Ctrl+Q` | Quit with confirmation |
| `cmd-n` | `Ctrl+N` | Rename session |
| `cmd-l` | `Ctrl+L` | Go to line |
| `cmd-f` | `Ctrl+F` | Find in view |
| `cmd-g` / `cmd-shift-g` | `Ctrl+G` / `Ctrl+Shift+G` | Find next/prev |

- [ ] Create keyboard subscription handler
- [ ] Map Cmd → Ctrl for Windows/Linux
- [ ] Implement all 30+ bindings

#### 3.3.3 Problem Cycling (~285 lines from workspace/problems.rs)
- [ ] Port `ProblemCycleState` — pure logic, minimal GPUI dependency
- [ ] Port 4-layer problem system (L0 cross-session, L1-L3 local)
- [ ] Port layer suppression/acknowledgement logic
- [ ] Wire to `Ctrl+;` / `Ctrl+Shift+;`

#### 3.3.4 Picker System (~310 lines from workspace/pickers.rs + 1,731 lines picker.rs)
**Original:** Generic picker framework with 8 delegate implementations

The picker is the largest single component (1,731 lines). Split work:

- [ ] Port generic `Picker` widget to iced (fuzzy search input + scrollable list)
- [ ] Port `FilePickerDelegate` — file open with fuzzy matching
- [ ] Port `SessionPickerDelegate` — session switching
- [ ] Port `DrillDownPickerDelegate` — hierarchical navigation
- [ ] Port `LineSearchDelegate` — go-to-line / find-in-view
- [ ] Port `ProjectPickerDelegate` — project actions
- [ ] Port `SnippetPickerDelegate` — snippet insertion
- [ ] Port `CommentPanelDelegate` — annotation entry
- [ ] Port `CommandPickerDelegate` — command palette
- [ ] Estimated: ~1,200 lines (simplified from 2,041 total)

### 3.4 Pane Layout (pane.rs + workspace/render.rs)
**Original:** `pane.rs` (80 lines) + `render.rs` (270 lines)

- [ ] Port `PaneLayout` enum: `One`, `Two`, `Three`
- [ ] Implement resizable split panes using `iced::widget::pane_grid::PaneGrid`
  - iced has a built-in `PaneGrid` widget — excellent fit
- [ ] Port pane content types: `ClaudeTerminal`, `GeneralTerminal`, `GitDiff`, `CodeViewer`, `TodoEditor`, `GlobalTodo`
- [ ] Port title bar with problem count indicators
- [ ] Port resize drag handles
- [ ] Estimated: ~200 lines

### 3.5 Code View (code_view.rs rewrite)
**Original:** `code_view.rs` (240 lines)
**Features:** Text editor with syntax highlighting, line numbers, file watching, three-way merge

- [ ] Use `iced::widget::text_editor::TextEditor` as base
- [ ] Port syntax highlighting via tree-sitter (grammars are cross-platform)
- [ ] Port breadcrumb/outline computation (`outline.rs` — 130 lines, no GPUI, reuse as-is)
- [ ] Port file watcher integration (`file_watcher.rs` — 55 lines, replace GPUI async with iced subscription)
- [ ] Port external change detection / three-way merge banner
- [ ] Port dirty state tracking
- [ ] Port line search (`LineSearchable` trait)
- [ ] Estimated: ~200 lines

### 3.6 Diff View (diff_view.rs rewrite)
**Original:** `diff_view.rs` (410 lines)
**Features:** Side-by-side diff display, git log, commit diff, line mapping

- [ ] Port diff generation logic (uses `git2` — cross-platform, reuse as-is)
- [ ] Port unified diff rendering with color-coded additions/deletions
- [ ] Port git log viewer
- [ ] Port commit diff navigation
- [ ] Port `DiffReviewed` event
- [ ] Port `diff_line_to_source_line()` / `source_line_to_diff_line()` mappings
- [ ] Estimated: ~350 lines

### 3.7 TODO View (todo_view.rs rewrite)
**Original:** `todo_view.rs` (335 lines)
**Features:** Wraps CodeView for TODO.md with structured parsing/validation

- [ ] Port TODO.md editor using iced text editor
- [ ] Port structured highlighting (session headings, wait sections, status markers)
- [ ] Port validation display (problems from `jc-core::todo::validate()`)
- [ ] Port session operations: insert heading, toggle disabled, mark expired, send from wait
- [ ] Estimated: ~250 lines

### 3.8 Modal Dialogs
**Original:** 3 modal components

#### 3.8.1 Close Confirmation (close_confirm.rs — 113 lines)
- [ ] Port quit/close confirmation dialog
- [ ] "Save layout?" / "Uninstall hooks?" options
- [ ] Estimated: ~80 lines

#### 3.8.2 Keybinding Help (keybinding_help.rs — 171 lines)
- [ ] Port help overlay showing all keybindings
- [ ] Update key labels from Cmd → Ctrl
- [ ] Estimated: ~120 lines

#### 3.8.3 Comment Panel (comment_panel.rs — 102 lines)
- [ ] Port annotation entry modal
- [ ] Estimated: ~70 lines

### 3.9 Subscriptions (Async Event Sources)
**In iced, async work is done via `Subscription`.**

- [ ] **PTY reader subscription** — async read from each terminal's PTY, emit `TerminalOutput` messages
- [ ] **Hook server subscription** — listen on `HookServer::rx` channel, emit `HookEvent` messages
- [ ] **IPC server subscription** — listen for incoming connections, emit `IpcProjectOpen` messages
- [ ] **File watcher subscription** — `notify` crate events → `FileChanged` messages
- [ ] **Clipboard poll subscription** — periodic clipboard check (if needed for snippet detection)
- [ ] **Signal subscription** — `ctrlc` handler → `Shutdown` message

### 3.10 Phase 3 Validation Gate
- [ ] App launches and shows window on WSL2
- [ ] Can register a project directory
- [ ] Terminal view renders shell, accepts input
- [ ] Can create/switch/close sessions
- [ ] Pane layout switches between 1/2/3 panes
- [ ] Code viewer opens files with syntax highlighting
- [ ] Diff viewer shows git changes
- [ ] TODO editor parses and highlights TODO.md
- [ ] All keybindings work
- [ ] Picker opens and navigates
- [ ] Notifications appear on WSL2 (D-Bus)
- [ ] Hook events trigger correctly
- [ ] IPC singleton works (second `jc .` routes to first)

---

## Phase 4: Integration & Polish (Days 19-22)

### 4.1 Cross-Platform Testing Matrix

| Test | WSL2 (Linux) | Native Windows | Native Linux |
|------|-------------|----------------|-------------|
| App launch | | | |
| Terminal rendering | | | |
| Terminal input (arrows, ctrl, alt) | | | |
| PTY spawn (bash/zsh/powershell) | | | |
| Git diff display | | | |
| File watcher | | | |
| Notifications | | | |
| IPC singleton | | | |
| Clipboard copy/paste | | | |
| Font rendering | | | |
| GPU acceleration | | | |
| Resize/DPI scaling | | | |
| Hook server | | | |

### 4.2 WSL2-Specific Fixes
- [ ] Verify D-Bus is available for notifications (fallback: log to stderr)
- [ ] Test font rendering at various DPI scales (100%, 125%, 150%, 200%)
- [ ] Verify WSLg GPU acceleration (wgpu should auto-detect D3D12 → OpenGL)
- [ ] Test with Windows Terminal as host terminal (PATH, env vars)
- [ ] Handle Windows mount paths (`/mnt/c/...`) vs ext4 paths gracefully
- [ ] Verify `claude` CLI is accessible from WSL2 PATH

### 4.3 Windows Native Build (Optional Stretch)
If targeting native Windows (not just WSL2):
- [ ] Cross-compile from WSL2: `cargo build --target x86_64-pc-windows-msvc`
- [ ] Or install Rust on Windows and build natively
- [ ] Test ConPTY terminal spawning (PowerShell, cmd.exe)
- [ ] Test Named Pipe IPC
- [ ] Test Windows toast notifications
- [ ] Handle Windows paths (`C:\Users\...`) in config

### 4.4 Performance Optimization
- [ ] Profile terminal rendering FPS (target: 60fps sustained)
- [ ] Profile memory usage with 5+ concurrent terminal sessions
- [ ] Optimize PTY read batching (don't render every byte)
- [ ] Lazy-load syntax highlighting grammars
- [ ] Test with large files (>10K lines) in code viewer

### 4.5 Edge Cases
- [ ] Multiple instances: IPC correctly routes to running instance
- [ ] Graceful shutdown: hooks uninstalled, PTYs closed, state saved
- [ ] Theme hot-reload: detect `theme.toml` changes
- [ ] Config hot-reload: detect `config.toml` changes
- [ ] Handle terminal process exit (show "Process exited" state)
- [ ] Handle missing `claude` CLI (show error, don't crash)

---

## Phase 5: Packaging & Distribution (Days 23-25)

### 5.1 Binary Distribution
- [ ] Build release binary: `cargo build --release -p jc-app`
- [ ] Strip debug symbols: `strip target/release/jc`
- [ ] Test release binary on clean WSL2 installation

### 5.2 Linux Packaging
- [ ] Create `.desktop` file for WSLg app launcher
- [ ] Create shell installer script
- [ ] Document WSL2 prerequisites

### 5.3 Windows Packaging (if native Windows target)
- [ ] Create `.exe` with icon
- [ ] Consider MSIX or portable exe distribution
- [ ] Create Start Menu shortcut

### 5.4 Documentation
- [ ] Update README.md with Windows/WSL2 build instructions
- [ ] Document platform differences (Cmd → Ctrl keybindings)
- [ ] Document WSL2 setup requirements
- [ ] Document known limitations

---

## File-by-File Port Tracker

### jc-core (NO CHANGES)
| File | Lines | Action | Status |
|------|-------|--------|--------|
| lib.rs | 10 | Keep | |
| config.rs | 97 | Keep | |
| hooks.rs | 160 | Keep | |
| hooks_settings.rs | 145 | Keep | |
| model.rs | 28 | Keep | |
| problem.rs | 120 | Keep | |
| snippets.rs | 95 | Keep | |
| status_script.rs | 55 | Keep | |
| theme.rs | 120 | Keep | |
| todo.rs | 1000+ | Keep | |

### jc-platform (NEW)
| File | Lines (est.) | Action | Status |
|------|-------------|--------|--------|
| lib.rs | 5 | Create | |
| notifications.rs | 20 | Create (trait) | |
| notifications/linux.rs | 40 | Create | |
| notifications/windows.rs | 50 | Create | |
| ipc.rs | 20 | Create (trait) | |
| ipc/unix.rs | 80 | Port from ipc.rs | |
| ipc/windows.rs | 80 | Create | |
| signals.rs | 30 | Create | |

### jc-terminal (PARTIAL REWRITE)
| File | Lines | Action | Status |
|------|-------|--------|--------|
| lib.rs | 10 | Update exports | |
| pty.rs | 80 | Keep (100% portable) | |
| terminal.rs | 75 | Keep (100% portable) | |
| colors.rs | 100 | Adapt types (gpui::Hsla → iced::Color) | |
| input.rs | 80 | Adapt types (gpui::Keystroke → iced keyboard) | |
| render.rs | 220 | Rewrite (iced_term or Canvas) | |
| view.rs | 540 | Rewrite (iced widget) | |

### jc-app (FULL GUI REWRITE)
| File | Lines | Action | Status |
|------|-------|--------|--------|
| main.rs | 100 | Port (remove libc signals, use jc-platform) | |
| app.rs | 195 | Rewrite (GPUI → iced::Application) | |
| notify.rs | 215 | Delete (replaced by jc-platform) | |
| ipc.rs | 145 | Delete (replaced by jc-platform) | |
| file_watcher.rs | 55 | Port (GPUI async → iced Subscription) | |
| language.rs | 70 | Keep (no GPUI) | |
| outline.rs | 130 | Keep (no GPUI) | |
| views/mod.rs | 70 | Port (GPUI helpers → iced helpers) | |
| views/pane.rs | 80 | Rewrite (iced PaneGrid) | |
| views/picker.rs | 1731 | Rewrite (iced modal + text input + list) | |
| views/session_state.rs | 110 | Port (remove Entity, keep data) | |
| views/project_state.rs | 190 | Port (remove Entity, keep data) | |
| views/code_view.rs | 240 | Rewrite (iced TextEditor) | |
| views/diff_view.rs | 410 | Rewrite (iced custom widget) | |
| views/todo_view.rs | 335 | Rewrite (iced TextEditor + validation) | |
| views/comment_panel.rs | 102 | Rewrite (iced modal) | |
| views/keybinding_help.rs | 171 | Rewrite (iced overlay) | |
| views/close_confirm.rs | 113 | Rewrite (iced modal) | |
| views/workspace/mod.rs | 1921 | Rewrite (iced Application core) | |
| views/workspace/render.rs | 270 | Rewrite (iced view()) | |
| views/workspace/pickers.rs | 310 | Rewrite (iced picker delegates) | |
| views/workspace/problems.rs | 285 | Port (mostly logic, minimal GPUI) | |

---

## Dependency Changes Summary

### Remove (macOS-only)
```toml
# DELETE from workspace Cargo.toml
gpui = "0.2.2"
gpui-component = { path = "vendor/gpui-component" }
gpui-component-assets = "0.5.1"
block2 = "0.6"
objc2 = "0.6"
objc2-app-kit = "0.3"
objc2-foundation = "0.3"
objc2-user-notifications = "0.3"
```

### Add (cross-platform)
```toml
# ADD to workspace Cargo.toml
iced = { version = "0.14", features = ["wgpu", "tokio", "canvas"] }
iced_term = "0.5"
notify-rust = "4"
ctrlc = "3"
interprocess = "2"
```

### Keep (already cross-platform)
```toml
# UNCHANGED
alacritty_terminal = "0.25"
portable-pty = "0.9"
git2 = { version = "0.20", default-features = false }
tree-sitter = "0.25"
tree-sitter-rust = "0.24"
tree-sitter-md = "0.5"
tree-sitter-python = "0.23"
tree-sitter-go = "0.23"
tree-sitter-javascript = "0.23"
tree-sitter-typescript = "0.23"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"
clap = { version = "4", features = ["derive"] }
anyhow = "1"
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1", features = ["v4", "serde"] }
dirs = "6"
notify = "7"
similar = "2"
diffy = "0.4"
flume = "0.11"
parking_lot = "0.12"
tiny_http = "0.12"
arboard = { version = "3", features = ["wayland-data-control"] }
```

---

## Risk Register

| Risk | Probability | Impact | Mitigation |
|------|------------|--------|------------|
| `iced_term` API doesn't match jc's terminal needs | Medium | High | Fall back to custom iced Canvas terminal renderer |
| iced `PaneGrid` doesn't support jc's exact layout model | Low | Medium | Build custom split layout with `Row`/`Column` |
| iced `TextEditor` lacks features for code view (line numbers, syntax hl) | Medium | High | Use `iced_highlighter` or custom Canvas-based editor |
| WSL2 GPU acceleration fails on user's hardware | Medium | Medium | wgpu has software fallback; document GPU requirements |
| D-Bus notifications not available in WSL2 | Low | Low | Fallback to terminal bell / stderr logging |
| `iced` version churn (0.14 breaking changes) | Medium | Medium | Pin exact version, vendor if needed |
| Terminal performance (60fps with multiple panes) | Low | Medium | Profile early (Phase 2), optimize PTY read batching |
| Font rendering quality in WSL2 | Medium | Low | Document DPI workarounds, test with Lilex font family |

---

## Critical Path

```
Phase 0 (1d) → Phase 1 (2d) → Phase 2 (4d) → Phase 3 (11d) → Phase 4 (4d) → Phase 5 (3d)
                                    ↓
                              Terminal works
                              (earliest demo)
                                                      ↓
                                                Full app works
                                                (feature complete)
```

**Earliest working demo:** End of Phase 2 (Day 7) — terminal renders in iced window
**Feature complete:** End of Phase 3 (Day 18) — all views ported
**Release ready:** End of Phase 5 (Day 25)

---

## Decision Log

| # | Decision | Rationale | Date |
|---|----------|-----------|------|
| 1 | Use iced over egui | Reactive paradigm closer to GPUI; iced_term exists; PaneGrid built-in; COSMIC desktop validates maturity | 2026-03-30 |
| 2 | Create jc-platform crate | Clean separation of platform abstractions; allows future macOS re-integration | 2026-03-30 |
| 3 | Keep jc-core unchanged | 100% portable, 2,274 lines of tested logic we get for free | 2026-03-30 |
| 4 | Map Cmd → Ctrl for keybindings | Standard Windows/Linux convention; keep Cmd on macOS if re-added later | 2026-03-30 |
| 5 | Target WSL2 primary, native Windows secondary | User's environment is WSL2; native Windows adds scope but is stretch goal | 2026-03-30 |
| 6 | Use notify-rust for notifications | 5.7M downloads, covers all 3 platforms, simple API | 2026-03-30 |
