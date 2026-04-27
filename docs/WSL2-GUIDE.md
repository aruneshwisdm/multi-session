# JC on WSL2 — Getting Started Guide

This guide walks you through setting up and using **jc**, a multi-session Claude Code orchestrator, on Windows via WSL2. It assumes you have basic familiarity with the terminal but are new to WSL2 and this tool.

## What is jc?

jc lets you run multiple Claude Code sessions side-by-side in a single window. Instead of juggling terminal tabs, you get:

- Multiple Claude sessions per project, switchable with Ctrl+1..9
- A built-in TODO editor that tracks which session owns which task
- Git diff review without leaving the app
- A problem cycling system that surfaces errors, permission prompts, and idle sessions
- File and snippet pickers for quick navigation

## 1. WSL2 Setup

### Install WSL2

Open **PowerShell as Administrator** on Windows and run:

```powershell
wsl --install -d Ubuntu
```

Restart your machine when prompted. After reboot, Ubuntu will finish installing and ask you to create a username and password.

### Verify WSLg (GUI support)

WSL2 on Windows 11 includes WSLg, which lets Linux GUI apps render on your Windows desktop. Verify it works:

```bash
echo $DISPLAY
# Should print something like :0 or :0.0
```

If `$DISPLAY` is empty, make sure your Windows 11 is updated to at least build 22000. WSLg ships with the Windows Subsystem for Linux update — run `wsl --update` from PowerShell to get the latest.

### Install system dependencies

```bash
sudo apt update && sudo apt upgrade -y

sudo apt install -y \
  build-essential \
  cmake \
  libssl-dev \
  pkg-config \
  libfontconfig1-dev \
  libxkbcommon-dev \
  libwayland-dev \
  wayland-protocols \
  git \
  curl
```

### Install the Rust toolchain

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Accept the defaults (option 1). Then reload your shell:

```bash
source ~/.cargo/env
```

Verify:

```bash
rustc --version
# Should print rustc 1.XX.X or later
```

### Install Claude CLI

jc launches `claude` in each terminal session. Install the Claude Code CLI from [claude.ai/download](https://claude.ai/download) and make sure it's in your PATH:

```bash
claude --version
# Should print a version number
```

If it's installed on the Windows side, you can access it from WSL2 as `claude.exe`. To use the plain `claude` name, add an alias to your `~/.bashrc`:

```bash
echo 'alias claude=claude.exe' >> ~/.bashrc
source ~/.bashrc
```

## 2. Clone and Build

### Clone to the ext4 filesystem

**This is critical.** Always work from the Linux filesystem, not the Windows mount. Building on `/mnt/c/` is 5-10x slower and causes cross-device link errors.

```bash
mkdir -p ~/projects
cd ~/projects
git clone <repository-url> jc
cd jc
```

### Build

```bash
cargo build --release -p jc-app
```

The first build takes a few minutes — it compiles tree-sitter grammars, alacritty terminal emulation, and the iced GUI framework. Subsequent builds are fast.

### Install (optional)

```bash
# Copy the binary somewhere in your PATH
cp target/release/jc-app ~/.local/bin/jc

# Make sure ~/.local/bin is in PATH
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc
```

Or use the install script:

```bash
./install.sh
```

## 3. Running jc

### Open a project

```bash
# From the project directory
jc .

# Or specify a path
jc ~/projects/my-app
```

jc opens a GUI window with three panes by default:
1. **Claude Terminal** (left) — your active Claude Code session
2. **TODO Editor** (middle) — the project's TODO.md
3. **Global TODO** (right) — your ~/.claude/TODO.md

### What you see on first launch

```
+------------------+------------------+------------------+
|                  |                  |                  |
|  Claude Terminal |   TODO Editor    |   Global TODO    |
|                  |                  |                  |
|  (Session 1)     |   (project       |   (~/.claude/    |
|                  |    TODO.md)      |    TODO.md)      |
|                  |                  |                  |
+------------------+------------------+------------------+
  [Session 1]                              [project-name]
```

The bottom bar shows your sessions (left) and the active project (right).

## 4. Keybindings Reference

All keybindings use **Ctrl** as the modifier key.

### Sessions

| Key | Action |
|-----|--------|
| Ctrl+T | Create a new Claude session |
| Ctrl+W | Close the active session |
| Ctrl+1..9 | Switch to session 1 through 9 |

### Panes and Layout

| Key | Action |
|-----|--------|
| Ctrl+/ | Move focus to the next pane |
| Ctrl+J | Reset to 3-pane layout |
| Ctrl+D | Show git diff in a pane |

### Pickers (fuzzy search overlays)

| Key | Action |
|-----|--------|
| Ctrl+P | File picker — open a file in the code viewer |
| Ctrl+Shift+P | Command palette — run a workspace command |
| Ctrl+K | Snippet picker — insert a saved text snippet into the terminal |
| Ctrl+O | Project picker — switch between registered projects |
| Ctrl+L | Go to line — jump to a specific line in the code viewer |

Inside a picker, use **Up/Down** arrows to navigate and **Enter** to confirm. Press **Escape** to dismiss.

### Editing and Files

| Key | Action |
|-----|--------|
| Ctrl+S | Save the active file |
| Ctrl+; | Jump to the next problem (errors, permission prompts, idle sessions) |
| Ctrl+? | Show/hide the keybinding help overlay |

### Terminal Clipboard

| Key | Action |
|-----|--------|
| Ctrl+Shift+C | Copy selected text from the terminal |
| Ctrl+Shift+V | Paste into the terminal |

Select text in the terminal by clicking and dragging with the mouse. Selected text is automatically copied to the clipboard.

### Quit

| Key | Action |
|-----|--------|
| Ctrl+Q | Quit (prompts to confirm if sessions are running) |

## 5. Daily Workflow

### Starting your day

1. Open your project: `jc ~/projects/my-app`
2. jc resumes your last active session automatically (it reads TODO.md for session info)
3. The Claude terminal is focused — start typing to interact with Claude

### Working with multiple sessions

A common pattern is to have Claude working on one task while you review another:

1. **Session 1**: Claude is implementing a feature (busy indicator shows in the session tab)
2. Press **Ctrl+T** to create Session 2
3. In Session 2, ask Claude to review or work on something else
4. Press **Ctrl+1** / **Ctrl+2** to switch between them
5. The **problem cycling** system (Ctrl+;) alerts you when a session needs attention — permission prompts, failures, or when Claude finishes

### Reviewing changes

1. Press **Ctrl+D** to open the git diff view
2. Review what Claude changed across your sessions
3. Use **Ctrl+P** to open specific files in the code viewer
4. Save edits with **Ctrl+S**

### Problem cycling

The problem system has 4 priority layers:

- **L0**: Cross-session errors (highest priority)
- **L1**: Session-specific issues (permission prompts, failures)
- **L2**: Unreviewed git diffs
- **L3**: Idle sessions waiting for input

Press **Ctrl+;** repeatedly to cycle through problems from highest to lowest priority. jc will switch sessions and panes automatically to show you what needs attention.

### Using snippets

If you have common prompts you send to Claude (e.g., "run the tests", "check for lint errors"):

1. Add them to `~/.claude/snippets.md` as markdown headings with content
2. Press **Ctrl+K** to open the snippet picker
3. Select a snippet — it gets pasted into the active Claude terminal

## 6. Project Configuration

### TODO.md format

jc reads a `TODO.md` file in your project root to track sessions and tasks. The format is:

```markdown
# Project Name

## Session: feature-auth [active]
- [ ] Implement login endpoint
- [ ] Add JWT validation
- [x] Create user model

## Session: bugfix-crash [done]
- [x] Fix null pointer in parser
```

Sessions marked `[active]` are resumed when you open the project. You can edit TODO.md directly in jc's TODO editor pane.

### Hooks

jc installs Claude Code hooks in your project automatically on launch. These hooks let jc know when Claude submits a prompt, finishes, encounters an error, or asks for permission. You'll see this reflected in the session tabs (busy/idle indicators).

To remove hooks from a project:

```bash
jc clean-hooks
```

## 7. Troubleshooting

### "No display" or blank window

WSLg isn't working. Check:

```bash
echo $DISPLAY
# Should print :0

# Test with a simple GUI app
sudo apt install -y x11-apps
xclock
```

If `xclock` doesn't open a window, update WSL: open PowerShell and run `wsl --update`, then restart WSL with `wsl --shutdown` followed by opening your distro again.

### Slow builds on /mnt/c/

If you cloned to `/mnt/c/Users/...`, builds will be painfully slow. Move your source to the Linux filesystem:

```bash
cp -r /mnt/c/Users/YourName/projects/jc ~/projects/jc
cd ~/projects/jc
cargo build --release -p jc-app
```

If you must build from a Windows mount path, set the target directory to avoid cross-device link errors:

```bash
CARGO_TARGET_DIR=/tmp/jc-build cargo build --release -p jc-app
```

### "claude: command not found"

The Claude CLI isn't in your PATH. If it's installed on Windows:

```bash
# Check if Windows version works
claude.exe --version

# Add alias
echo 'alias claude=claude.exe' >> ~/.bashrc
source ~/.bashrc
```

Or install the native Linux version by following the instructions at [claude.ai/download](https://claude.ai/download).

### Notifications not working

Desktop notifications use D-Bus, which may not be running in WSL2:

```bash
# Check D-Bus
systemctl --user status dbus

# Start it if needed
systemctl --user start dbus
```

Notifications are non-essential — jc works fine without them. The problem cycling system (Ctrl+;) is the primary way to track session status.

### Font rendering looks wrong

WSLg can have font rendering issues at high DPI. If text looks blurry or oversized:

1. Right-click your WSL terminal in the Windows taskbar
2. Go to Properties > Compatibility > Change high DPI settings
3. Enable "Override high DPI scaling behavior" and set to "Application"

### Terminal colors look off

jc uses a dark theme by default. If colors look wrong, make sure your WSL2 terminal supports 256 colors:

```bash
echo $TERM
# Should be xterm-256color or similar
```

### Build fails with missing library

If you get link errors about missing libraries:

```bash
sudo apt install -y \
  cmake libssl-dev pkg-config libfontconfig1-dev \
  libxkbcommon-dev libwayland-dev wayland-protocols
```

Then clean and rebuild:

```bash
cargo clean
cargo build --release -p jc-app
```

## 8. Architecture Overview (for the curious)

jc is built in Rust with four crates:

| Crate | Purpose |
|-------|---------|
| `jc-core` | Config, hooks, TODO parser, problem system. Shared with upstream. |
| `jc-platform` | Platform abstractions: notifications, IPC, signals. |
| `jc-terminal` | Terminal emulation via alacritty + portable-pty. Colors, input mapping, grid rendering. |
| `jc-app` | The GUI application. Built with iced (Elm architecture). |

The app follows iced's Elm pattern: all state lives in one struct, all mutations go through an `update()` function triggered by messages, and the UI is a pure function of state rendered by `view()`. Async work (PTY reads, hook events, file watching) feeds back into the app via subscriptions.

Each Claude session runs in its own PTY (pseudo-terminal). When you type in the terminal pane, keystrokes are translated to byte sequences and written to the PTY. The terminal emulator (alacritty_terminal) processes the output and renders a grid of cells with colors, which jc draws using iced's canvas widget.
