# JC Port Analysis: macOS to Windows + WSL2

**Date:** 2026-03-30
**Source:** https://github.com/jeapostrophe/jc
**Goal:** Evaluate feasibility of building/porting JC (Claude Code orchestrator) to Windows + WSL2

---

## 1. What is JC?

JC is a Rust + GPUI application that manages multiple Claude Code sessions across projects from a single window. It embeds terminal views, provides diff/code viewers, syntax highlighting, and desktop notifications.

**Codebase:** ~8,600 lines of Rust across 3 crates:

| Crate | Lines | Purpose | Portable? |
|-------|-------|---------|-----------|
| jc-core | 2,274 | Config, data models, problem tracking | Yes (100%) |
| jc-terminal | 1,412 | Embedded terminal emulator | Mostly (rendering tied to GPUI) |
| jc-app | 4,931 | GUI views, notifications, IPC | Partially (3 macOS subsystems) |

**Explicit design constraint:** DESIGN.md states *"macOS only. No cross-platform concerns."*

---

## 2. macOS-Specific Blockers

### Blocker 1: GPUI (GPU-Accelerated UI Framework)

GPUI is Zed's UI framework. It uses Metal for rendering and Cocoa for windowing on macOS.

**Current cross-platform status (2026):**
- **Linux:** Stable since mid-2024 using Vulkan-based Blade renderer
- **Windows:** Closed alpha, targeting 1.0 release
- **WSL2:** Not tested/supported by Zed team

**Verdict:** GPUI's Linux support exists but Windows is immature. Not viable for a Windows+WSL2 target today.

### Blocker 2: Objective-C Bindings (Notifications)

`notify.rs` (271 lines) uses:
- `objc2-user-notifications` (UNUserNotificationCenter)
- `objc2-app-kit` (NSApplication, dock bounce)
- `objc2-foundation` (NSString, NSArray)
- `block2` (Objective-C blocks)

### Blocker 3: Unix-Only IPC

`ipc.rs` (170 lines) uses `std::os::unix::net::{UnixListener, UnixStream}`.

### Blocker 4: Unix Signal Handling

`main.rs` uses `libc` for SIGINT/SIGTERM handlers.

---

## 3. Windows/Linux Equivalents

### 3.1 GUI Framework Replacements for GPUI

| Framework | Crate | Rendering | Windows | Linux/WSL2 | GPU | Terminal Embed | Paradigm | Maturity |
|-----------|-------|-----------|---------|------------|-----|----------------|----------|----------|
| **egui** | `egui` | wgpu (Vulkan/DX12/GL) | Yes | Yes | Yes | `egui_term` | Immediate mode | Stable |
| **iced** | `iced` | wgpu (Vulkan/DX12) | Yes | Yes | Yes | `iced_term` | Elm (reactive) | Stable (0.14) |
| **floem** | `floem` | wgpu + tiny-skia | Yes | Yes | Yes | Not yet | Fine-grained reactive | Pre-1.0 |
| **slint** | `slint` | Skia/femtovg/SW | Yes | Yes | Yes | Custom needed | Declarative | Stable (1.4) |
| **dioxus** | `dioxus` | wgpu (experimental) | Yes | Yes | Yes | Planned | React-like | Evolving |
| **tauri** | `tauri` | WebView2 | Yes | Yes | No | xterm.js | Web-based | Stable (v2) |

**Recommendation ranking:**

1. **iced** - Best match for GPUI's reactive paradigm. Elm architecture is closest to GPUI's entity model. `iced_term` exists for terminal embedding. Used by COSMIC desktop.

2. **egui** - Most flexible. `egui_term` provides terminal embedding via alacritty_terminal. Largest ecosystem. Immediate mode gives fine control.

3. **floem** - Closest API paradigm to GPUI (fine-grained reactivity, signals-based). From Lapce editor team. Risk: pre-1.0, no terminal widget yet.

4. **tauri** - If web UI is acceptable. Proven on WSL2. Uses xterm.js for terminal. Smallest binaries.

### 3.2 Notification Replacements

| macOS API | Cross-Platform Replacement | Windows | Linux |
|-----------|---------------------------|---------|-------|
| UNUserNotificationCenter | **notify-rust** | winrt-notification backend | D-Bus/libnotify backend |
| Dock icon bounce | Windows taskbar flash | `FlashWindowEx` API | N/A (no dock) |
| NSApplication focus | `SetForegroundWindow` | Win32 API | wmctrl / xdotool |

**Primary crate:** `notify-rust` (5.7M+ downloads, covers all 3 platforms)

```toml
# Cargo.toml
[dependencies]
notify-rust = "4"

# Windows-specific extras
[target.'cfg(target_os = "windows")'.dependencies]
winrt-notification = "0.5"  # Advanced toast features

# Linux-specific extras
[target.'cfg(target_os = "linux")'.dependencies]
# notify-rust uses D-Bus by default, no extra deps
```

**Conditional compilation pattern:**
```rust
#[cfg(target_os = "macos")]
mod notify_macos;
#[cfg(target_os = "windows")]
mod notify_windows;
#[cfg(target_os = "linux")]
mod notify_linux;
```

### 3.3 IPC Replacements

| Unix | Windows Equivalent | Cross-Platform |
|------|--------------------|----------------|
| Unix domain socket (`jc.sock`) | Named Pipes (`\\.\pipe\jc`) | TCP localhost:PORT |

**Recommended approach:** `interprocess` crate or manual `#[cfg]` with:
- Unix: Keep `UnixListener`/`UnixStream`
- Windows: Use `windows::Win32::System::Pipes` or TCP `127.0.0.1`

### 3.4 Signal Handling Replacement

| Unix | Windows Equivalent |
|------|--------------------|
| `libc::SIGINT` / `libc::SIGTERM` | `SetConsoleCtrlHandler` (Ctrl+C, Ctrl+Break) |

**Recommended crate:** `ctrlc` (cross-platform, handles both Unix signals and Windows console events).

---

## 4. Terminal Embedding (Already Portable)

Good news: the terminal stack is already cross-platform.

| Component | Crate | Windows | Linux | Notes |
|-----------|-------|---------|-------|-------|
| Terminal emulation | `alacritty_terminal` v0.25 | Yes | Yes | VTE parser, ConPTY on Windows |
| PTY management | `portable-pty` v0.9 | Yes | Yes | Abstracts Unix PTY / Windows ConPTY |
| Syntax highlighting | `tree-sitter-*` | Yes | Yes | 6 grammars included |
| File watching | `notify` v7 | Yes | Yes | inotify/ReadDirectoryChanges |
| Git operations | `git2` | Yes | Yes | libgit2 bindings |
| Clipboard | `arboard` | Yes | Yes | Win32/X11/Wayland |
| Diff engine | `similar` + `diffy` | Yes | Yes | Pure Rust |

**Terminal embedding widgets exist for:**
- **egui:** `egui_term` - alacritty_terminal backend, immediate mode widget
- **iced:** `iced_term` - alacritty_terminal backend, Elm-style widget

---

## 5. WSL2 GUI Capabilities (2026)

### WSLg Status
- Fully integrated, supports X11 and Wayland apps
- Runs Weston compositor + XWayland inside WSL2 VM
- Apps appear alongside Windows apps (taskbar integration)

### GPU Acceleration
- **OpenGL:** Works via Mesa D3D12 gallium driver (translates to DirectX 12)
- **Vulkan:** Experimental/early stage - not production-ready
- **wgpu apps:** Work via D3D12 backend on Windows, OpenGL fallback in WSL2

### Performance (WSL2 vs Native)
| Workload | WSL2 Performance |
|----------|-----------------|
| CPU-bound | ~95% of native |
| GPU rendering | ~67% of native (33% overhead) |
| File I/O (ext4) | ~90% of native |
| File I/O (Windows mount) | Significantly slower |
| Overall GUI | ~87% of native |

### Known Issues
- Font rendering/DPI scaling problems at high scaling factors
- GPU acceleration can regress with WSL updates
- Vulkan not production-ready
- Some GPU vendors have limited WDDM support

### Architecture Decision: Native Windows vs WSL2 GUI

| Approach | Pros | Cons |
|----------|------|------|
| **WSL2 Linux GUI app** | Single codebase, Linux ecosystem, simpler build | 13-33% perf hit, font issues, GPU limitations |
| **Native Windows app** | Full performance, native integration | Separate build target, more conditional compilation |
| **Hybrid** (recommended) | Best of both worlds | More complex architecture |

**Recommendation:** Build as a native cross-platform app (compiles on both Windows and Linux). Run natively on Windows for best performance, with WSL2 as a secondary target.

---

## 6. Porting Effort Estimate

### Phase 1: Extract Portable Core (1-2 days)
- jc-core compiles as-is (zero changes)
- Create platform abstraction traits for notifications, IPC, signals
- Headless CLI mode for testing

### Phase 2: Platform Abstractions (2-3 days)
- Notifications: `notify-rust` + platform trait (~100 lines)
- IPC: Named pipes on Windows, keep Unix sockets on Linux (~80 lines)
- Signal handling: `ctrlc` crate (~20 lines)

### Phase 3: GUI Framework Migration (2-3 weeks)
This is the bulk of the work.

**Option A: iced (recommended)**
- Rewrite views using iced widgets + `iced_term`
- Port theme system to iced styling
- Reimplement keyboard shortcuts
- ~3,000 lines of view code to rewrite

**Option B: egui**
- More flexible, immediate mode
- `egui_term` for terminal embedding
- `egui_dock` for panel/tab layout
- Faster to prototype, harder to maintain complex state

**Option C: Wait for GPUI Windows support**
- Monitor Zed's Windows port progress
- Lowest effort if GPUI ships Windows support
- Risk: unknown timeline, may never be stable enough

### Phase 4: Testing & Polish (3-5 days)
- WSL2 testing with WSLg
- Native Windows testing
- Font rendering validation
- GPU acceleration verification

### Total Estimate

| Approach | Time | Risk |
|----------|------|------|
| Full port with iced | 3-4 weeks | Medium |
| Full port with egui | 2-3 weeks | Medium |
| Wait for GPUI Windows | Unknown | High |
| Core-only (headless/TUI) | 1 week | Low |

---

## 7. Recommended Stack for Windows + WSL2

```
+------------------+     +-------------------+
|   jc-core        |     |   jc-terminal     |
|   (unchanged)    |     |   (unchanged PTY) |
+--------+---------+     +--------+----------+
         |                        |
+--------+------------------------+----------+
|              jc-app (new GUI layer)        |
|                                            |
|  Framework: iced (or egui)                 |
|  Rendering: wgpu (Vulkan/DX12/OpenGL)     |
|  Terminal:  iced_term (or egui_term)       |
|  Notify:   notify-rust                    |
|  IPC:      Unix sockets / Named Pipes     |
|  Signals:  ctrlc crate                    |
+--------------------------------------------+
```

### Cargo.toml (cross-platform)

```toml
[workspace.dependencies]
# GUI (replaces gpui)
iced = { version = "0.14", features = ["wgpu", "tokio"] }
iced_term = "0.5"

# Notifications (replaces objc2-user-notifications)
notify-rust = "4"

# IPC (replaces std::os::unix::net)
interprocess = "2"

# Signal handling (replaces libc signals)
ctrlc = "3"

# These stay unchanged
alacritty_terminal = "0.25"
portable-pty = "0.9"
git2 = { version = "0.20", default-features = false }
tree-sitter = "0.25"
serde = { version = "1", features = ["derive"] }
# ... rest unchanged
```

---

## 8. Quick-Start: Can We Build It Today?

**As-is from GitHub? No.** It won't compile on Windows or Linux due to GPUI + objc2 dependencies.

**With modifications?** Yes, with the phased approach above. The core logic and terminal emulation are portable today.

**Fastest path to something working:**
1. Clone repo
2. Extract jc-core as standalone
3. Build a minimal TUI (using `ratatui`) or GUI (using `egui`) around it
4. Use `egui_term` or `iced_term` for terminal embedding
5. Replace `notify.rs` with `notify-rust`
6. Replace `ipc.rs` with TCP or named pipes

This gets you a working multi-session Claude Code manager on Windows + WSL2 in approximately 2-3 weeks of focused effort.
