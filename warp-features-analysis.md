# Warp-Inspired Features: Implementation Analysis

Analysis of 8 features inspired by [warpdotdev/warp](https://github.com/warpdotdev/warp), mapped to our codebase's current state, with concrete implementation paths.

---

## 1. Session State Machine (High Value)

### Current State

`session_state.rs` tracks session state via discrete fields:

```rust
pub struct SessionState {
    pub busy: bool,                           // set by hook events
    pub has_ever_been_busy: bool,             // latched on first busy
    pub pending_events: HashSet<PendingEvent>,// ClaudePermission, ClaudeStopFailure, TerminalBell
    pub problems: Vec<SessionProblem>,        // refreshed from pending_events + todo
}
```

There's no single "session activity state" enum. Instead, state is inferred from the combination of `busy`, `has_ever_been_busy`, and `pending_events`. The problem system in `jc-core/problem.rs` then derives priority layers (L0-L3) from these.

### What Warp Does Differently

Warp tracks each Claude Code session with a first-class state enum: `Busy`, `Idle`, `NeedsPermission`, `Error`, `UnreviewedDiffs`. This drives a colored status indicator per tab — one glance tells you what every session needs.

### Implementation Plan

**Add a derived `SessionActivity` enum to `session_state.rs`** (in jc-app, not jc-core):

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionActivity {
    /// Claude is actively working on this session.
    Busy,
    /// Claude finished and is waiting for input.
    Idle,
    /// Claude is blocked on a permission prompt.
    NeedsPermission,
    /// Session hit an API error (StopFailure).
    Error,
    /// Session has unreviewed diffs from agent work.
    HasUnreviewedDiffs,
    /// Fresh session, never started.
    New,
}
```

This enum is **computed, not stored** — it's a projection of existing state:

```rust
impl SessionState {
    pub fn activity(&self) -> SessionActivity {
        if self.pending_events.contains(&PendingEvent::ClaudeStopFailure) {
            SessionActivity::Error
        } else if self.pending_events.contains(&PendingEvent::ClaudePermission) {
            SessionActivity::NeedsPermission
        } else if self.busy {
            SessionActivity::Busy
        } else if !self.has_ever_been_busy {
            SessionActivity::New
        } else {
            // Could check unreviewed diffs here once wired
            SessionActivity::Idle
        }
    }
}
```

**Tab rendering** in the workspace view maps activity to a colored dot:

```rust
fn activity_color(activity: SessionActivity, palette: &Palette) -> iced::Color {
    match activity {
        SessionActivity::Busy => Color::from_rgb(0.2, 0.7, 0.3),      // green
        SessionActivity::Idle => Color::from_rgb(0.5, 0.5, 0.5),      // gray
        SessionActivity::NeedsPermission => Color::from_rgb(1.0, 0.6, 0.0), // orange
        SessionActivity::Error => Color::from_rgb(0.9, 0.2, 0.2),     // red
        SessionActivity::HasUnreviewedDiffs => Color::from_rgb(0.3, 0.5, 0.9), // blue
        SessionActivity::New => Color::from_rgb(0.7, 0.7, 0.7),       // light gray
    }
}
```

### Effort & Risk

- **Effort**: ~2 hours. The enum is purely derived; no state migration needed.
- **Files touched**: `session_state.rs` (add enum + method), workspace `render.rs` (tab bar rendering).
- **Risk**: None. Additive change, no existing behavior modified.
- **Dependencies**: None.

### Connecting `HasUnreviewedDiffs` to diff state

Currently, diff state lives in `ProjectState.unreviewed_files: Vec<PathBuf>`, not per-session. To make `HasUnreviewedDiffs` meaningful, we'd need to track which session generated which diffs (via hook events that correlate session_id to git changes). This is a larger piece of work — see item 4 below. For v1, we can skip this variant and add it when diff attribution is implemented.

---

## 2. Problem-Cycling Priority (Ctrl+;)

### Current State

**Already implemented and mature.** The problem cycling system in `workspace/problems.rs` is 300 lines of working code that:

1. Collects L0 problems cross-session (permission prompts, API errors)
2. Saves a "home" position before jumping to L0 problems in other sessions
3. Returns home when L0 problems are cleared
4. Cycles L1 (bell, unreviewed diffs, script problems) → L2 (unsent WAIT) → L3 (idle session)
5. Suppresses L2 when session is busy or L1 exists
6. Falls back to TODO editor when no problems remain

The priority ordering in `jc-core/problem.rs` already matches Warp's model:

| Our Layer | Rank | Warp Equivalent |
|-----------|------|-----------------|
| L0 | 1-2 | errors, permission prompts |
| L1 | 5, 10 | terminal bell, unreviewed diffs, script problems |
| L2 | 6 | unsent WAIT (ready for new work) |
| L3 | synthetic | idle + has_ever_been_busy |

### What Warp Adds

Warp's implementation is essentially the same priority model. The only concrete difference is their **visual indicator per layer** — showing which layers have problems across all sessions, not just the current one. Our `layer_problem_sessions()` method already computes this data (lines 244-298 of problems.rs) but the render side may not be surfacing it prominently.

### Implementation Plan

**No changes to cycling logic needed.** The existing implementation already matches Warp's priority ordering.

**Possible enhancement**: Surface `layer_problem_sessions()` data in the status bar as colored badges showing cross-session problem counts per layer. This is a render-only change in workspace `render.rs`.

```
[L0: 1] [L1: 3] [L2: 0] [L3: 2]    or    [!1] [?3] [~2]
```

### Effort & Risk

- **Effort**: ~1 hour (status bar badges only).
- **Files touched**: workspace `render.rs`.
- **Risk**: None.
- **Dependencies**: None.

---

## 3. Event Replay on Session Restore

### Current State

When jc crashes or is restarted:
- The `claude` process keeps running in its PTY (orphaned but alive)
- jc spawns a **new** `claude` process via `Terminal::spawn_command()` in `session_state.rs:44`
- The old PTY is lost — no reconnection
- Session state (`busy`, `pending_events`) resets to defaults

The `--resume` flag is already used when a UUID is available (line 83-87 of session_state.rs):

```rust
let command = uuid
    .as_ref()
    .filter(|u| !u.is_empty())
    .map(|u| format!("claude --resume {u}"))
    .unwrap_or_else(|| "claude".to_string());
```

But this starts a **new** claude process that resumes the conversation, not reconnects to the running one.

### What Warp Does

Warp persists enough state to reconnect to still-running Claude Code processes after a restart. They replay lifecycle events so the UI recovers the correct session state (busy/idle/permission).

### Implementation Plan

This is the most complex feature. There are two sub-problems:

#### 3a. Persist session state to disk

Save enough state to reconstruct `SessionState` on restart without re-deriving everything.

**New file**: `~/.config/jc/sessions.json`

```json
{
  "sessions": [
    {
      "project_path": "/home/user/my-project",
      "label": "feature-x",
      "uuid": "abc-123",
      "pty_pid": 12345,
      "busy": true,
      "hook_port": 9123,
      "saved_at": "2026-04-29T10:00:00Z"
    }
  ]
}
```

Write this file on every significant state change (hook event, session create/close). Read it on startup.

#### 3b. Reconnect to orphaned PTY processes

This is the hard part. Options:

**Option A: Don't reconnect to the PTY, just restore state and `--resume`** (recommended)

- On startup, read `sessions.json`
- Check if the old `claude` process (by PID) is still running (`kill(pid, 0)` on Unix)
- If still running: send SIGTERM to the old process, wait briefly, then `claude --resume {uuid}`
- Restore `busy`/`pending_events` from the saved state
- Re-install hooks pointing to the new hook server port

This is what Warp effectively does — they don't literally reattach to the old PTY, they just resume the conversation and replay the state.

**Option B: True PTY reconnection via `tmux` or `abduco`**

- Spawn `claude` inside a `tmux` session
- On restart, `tmux attach` to the existing session
- This gives true output continuity but adds a dependency

**Recommendation**: Option A. It's simpler, works with `claude --resume`, and matches Warp's actual behavior. The user loses scrollback but keeps conversation continuity.

#### 3c. Hook state replay

After reconnecting:
1. Query Claude Code's current state (is it waiting for input? blocked on permission?)
2. There's no direct API for this, but we can infer from the hook server:
   - If we receive a `PermissionPrompt` hook shortly after startup → `NeedsPermission`
   - If no hooks arrive within ~2 seconds → likely `Idle`
   - The persisted `busy` flag provides initial state

### Effort & Risk

- **Effort**: ~8-12 hours total (3a: 2h, 3b-Option-A: 4h, 3c: 2h, testing: 2-4h).
- **Files touched**: New `jc-app/src/session_persistence.rs`, `session_state.rs` (add PID tracking), `main.rs` (startup restore logic), workspace `mod.rs` (save-on-change).
- **Risk**: Medium. PID-based process detection is fragile. Race conditions between old/new hook servers. The old `claude` process may have changed state between crash and restart.
- **Dependencies**: None, but benefits significantly from item 1 (SessionActivity enum gives cleaner restore targets).

### Key Constraint

jc-core is sacred — all persistence logic goes in `jc-app`. The session file format is our own, not upstream's.

---

## 4. Diff Review Workflow

### Current State

**Already partially implemented.** `diff_view.rs` has:

```rust
pub struct FileDiff {
    pub name: String,
    pub content: String,
    pub reviewed: bool,         // <-- field exists
}
```

And tracking methods:
- `reviewed_count()` / `file_count()` — counter display
- `unreviewed_files()` — returns Vec<PathBuf> of unreviewed files
- The header already shows `"Diff [source] (2/5 reviewed)"`

What's **missing**:
1. **UI to toggle `reviewed`** — no `Message::DiffReviewed` handler wired to actually flip the bool
2. **Per-session diff attribution** — diffs are project-level, not linked to which Claude session generated them
3. **Persistence** — review state resets when diff is regenerated (stale → refresh clears all reviews)

The `Message::DiffReviewed` variant exists in the Message enum (line 63) but needs a handler.

### Implementation Plan

#### Phase 1: Wire up the existing reviewed toggle (~1 hour)

In workspace `update()`, handle `Message::DiffReviewed`:

```rust
Message::DiffReviewed => {
    let pi = self.active_project_index;
    if let Some(dv) = self.diff_views.get_mut(pi) {
        if let Some(file) = dv.file_diffs.get_mut(dv.current_file_index) {
            file.reviewed = !file.reviewed;
        }
        // Update unreviewed_files in project state
        self.projects[pi].unreviewed_files = dv.unreviewed_files();
        self.projects[pi].refresh_problems();
    }
}
```

Add a keybinding: `Ctrl+Shift+R` or `Enter` when in diff view toggles reviewed.

Add visual indicator: reviewed files show a checkmark in the file list, unreviewed show a dot.

#### Phase 2: Preserve review state across refreshes (~2 hours)

When `apply_diff_text()` is called with new diff content, carry over `reviewed` status for files whose content hasn't changed:

```rust
pub fn apply_diff_text(&mut self, diff_text: String) -> bool {
    let new_diffs = parse_diff_files(&diff_text);
    // Carry over reviewed status for unchanged files
    for new_file in &mut new_diffs {
        if let Some(old) = self.file_diffs.iter().find(|f| f.name == new_file.name) {
            if old.content == new_file.content {
                new_file.reviewed = old.reviewed;
            }
        }
    }
    let changed = self.file_diffs.len() != new_diffs.len();
    self.file_diffs = new_diffs;
    self.stale = false;
    changed
}
```

#### Phase 3: Per-session diff attribution (future, ~4 hours)

Track which session was active when files were modified. This requires correlating `HookEventKind::Stop` (Claude finished working) with `git diff` changes. Deferred — adds complexity for moderate UX gain.

### Effort & Risk

- **Phase 1 Effort**: ~1 hour.
- **Phase 2 Effort**: ~2 hours.
- **Files touched**: `diff_view.rs` (review preservation), workspace `mod.rs` or the update handler (DiffReviewed message), workspace `render.rs` (checkmark rendering), keybindings.
- **Risk**: Low. Phase 1 is pure wiring. Phase 2 is a straightforward content-comparison merge.
- **Dependencies**: None.

---

## 5. Agent Attribution Controls

### Current State

`AppConfig` in `jc-core/config.rs` is minimal:

```rust
pub struct AppConfig {
    pub editor: String,
    pub window: WindowLayout,
}
```

No attribution settings. When Claude makes commits, the attribution is whatever Claude Code defaults to (typically `Co-Authored-By` trailer).

### What Warp Does

Warp provides team-admin and per-user controls for how AI commits are attributed:
- **Co-author**: Human is author, AI is `Co-Authored-By` (default)
- **Author**: AI is the commit author
- **None**: No AI attribution

### Implementation Plan

**We cannot add fields to `AppConfig` in jc-core** (sacred rule). Instead, add a parallel config in jc-app:

**New struct in jc-app** (e.g., `jc-app/src/app_config.rs`):

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct JcAppConfig {
    pub attribution: AttributionMode,
    pub features: HashMap<String, bool>,  // see item 8
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AttributionMode {
    #[default]
    CoAuthor,
    Author,
    None,
}
```

Stored at `~/.config/jc/jc-app.toml` (separate from upstream's `config.toml`).

**How it takes effect**: When installing hooks via `hooks_settings.rs`, we could inject the attribution mode as an environment variable or configure Claude Code's `--attribution` flag. However, Claude Code doesn't currently expose an attribution flag — this would need to be implemented as a git hook (`prepare-commit-msg`) that adds/removes the `Co-Authored-By` line.

**Alternative (simpler)**: Just add a `prepare-commit-msg` hook that jc installs alongside its other hooks:

```bash
#!/bin/sh
# Managed by jc — attribution mode: co_author
# Adds Co-Authored-By trailer if not present
if ! grep -q "Co-Authored-By:" "$1"; then
  echo "" >> "$1"
  echo "Co-Authored-By: Claude <noreply@anthropic.com>" >> "$1"
fi
```

### Effort & Risk

- **Effort**: ~3-4 hours (config struct, persistence, git hook generation, picker for changing mode).
- **Files touched**: New `jc-app/src/app_config.rs`, `main.rs` (load config), hooks installation code.
- **Risk**: Low-medium. Git hooks are fragile in multi-tool environments. Need to not clobber existing `prepare-commit-msg` hooks.
- **Dependencies**: None.

---

## 6. IPC Singleton with Focus

### Current State

**Already implemented and working.** `jc-platform/src/ipc.rs` provides:

- `SocketServer::bind()` — Unix socket on Linux, TCP with port file on Windows
- `try_send_to_running()` — client tries to connect; if successful, sends `open_project` command
- `main.rs` lines 40-43 implement the singleton pattern:

```rust
if let Some(path) = &project_path {
    if jc_platform::ipc::try_send_to_running(path) {
        return Ok(());  // existing instance handles it
    }
}
```

- Stale socket cleanup on failed connect
- IPC messages flow through `Message::IpcProjectOpen` → workspace adds the project

### What Warp Does

Identical pattern. `warp .` sends to existing instance, which opens/focuses the project.

### Analysis

**No work needed.** Our implementation matches Warp's. The one missing piece is **window focus** — when an IPC message arrives, we should raise the window. iced doesn't have a built-in `window.focus()` command, but on Linux/Wayland this may happen automatically when the UI updates. On X11, we'd need `wmctrl` or similar. This is a minor polish item.

### Effort & Risk

- **Effort**: 0 (already done). ~1 hour if adding window raise.
- **Risk**: None.

---

## 7. Vim Keybinding Mode

### Current State

Keybindings are handled in `workspace/keybindings.rs` (mapping iced keyboard events to Messages). The terminal panes pass keystrokes directly to the PTY via `keystroke_to_bytes()`. There is no modal input system.

The code editor and TODO editor use `iced::widget::text_editor`, which has basic Emacs-like bindings (Ctrl+A/E/K etc.) but no Vim mode.

### What Warp Does

Warp has a dedicated `vim` crate that implements:
- Normal/Insert/Visual/Command modes
- Motion commands (w, b, e, 0, $, gg, G, etc.)
- Operators (d, c, y, p, etc.)
- The full Vim state machine for their input editor (not the terminal — the terminal already has Vim via the shell)

### Implementation Plan

This is a **large scope item** and should be treated as a separate project, not bundled with the other improvements.

#### Where Vim mode applies

NOT in the terminal panes — those already support Vim via the shell. Vim mode applies to:
1. **Code viewer** (`text_editor` widget) — navigating/editing code files
2. **TODO editor** (`text_editor` widget) — editing TODO.md
3. **Picker** — Vim-like navigation (j/k for up/down, / for search)

#### Architecture

```
                        ┌─────────────────┐
  iced keyboard event → │  VimStateMachine │ → VimAction (enum)
                        │  - mode          │
                        │  - pending_op    │
                        │  - count         │
                        └─────────────────┘
                                │
                     ┌──────────┼──────────┐
                     ▼          ▼          ▼
              text_editor   picker     diff_view
              (motions,    (j/k nav)   (navigation)
               editing)
```

#### Phased approach

**Phase 1 — Navigation only (~8 hours)**:
- Normal mode with hjkl, w/b/e, 0/$, gg/G, Ctrl+D/U
- `i`/`a`/`o` to enter insert mode (pass through to text_editor)
- `Esc` to return to normal mode
- Status line showing mode

**Phase 2 — Operators (~12 hours)**:
- `d`/`c`/`y` with motions
- Visual mode (v, V)
- `.` repeat
- Undo/redo mapping

**Phase 3 — Ex commands (~4 hours)**:
- `:w` (save), `:q` (close), `:wq`
- `:/pattern` (search)
- `:line_number` (go to line)

#### Config toggle

Add `vim_mode: bool` to `JcAppConfig` (from item 5). Default false. Toggle via command palette.

### Effort & Risk

- **Total effort**: ~24-30 hours across 3 phases.
- **Files touched**: New `jc-app/src/vim.rs` (state machine), `workspace/keybindings.rs` (mode routing), render (mode indicator).
- **Risk**: Medium. Vim emulation is notoriously hard to get right. Users with Vim muscle memory will notice incorrect behavior. Consider using an existing crate (e.g., `vim-emulation` or porting a subset of `xi-editor`'s Vim mode).
- **Dependencies**: Benefits from item 5 (JcAppConfig for toggle), but can be done independently.

---

## 8. Feature Flag System

### Current State

No feature flags. All functionality is always enabled.

### What Warp Does

200+ feature flags for A/B testing and gradual rollout. Massive infrastructure for their SaaS product.

### Implementation Plan

We don't need A/B testing or remote flags. A simple local config is sufficient.

**Add to `JcAppConfig` (from item 5)**:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct JcAppConfig {
    pub attribution: AttributionMode,
    pub features: HashMap<String, bool>,
    pub vim_mode: bool,
}
```

**Usage pattern**:

```rust
impl JcAppConfig {
    pub fn is_enabled(&self, feature: &str) -> bool {
        self.features.get(feature).copied().unwrap_or(false)
    }
}
```

**Config file** (`~/.config/jc/jc-app.toml`):

```toml
vim_mode = false

[features]
session_restore = true
diff_attribution = false
status_bar_badges = true
```

**Gate experimental features**:

```rust
if self.app_config.is_enabled("session_restore") {
    self.restore_sessions();
}
```

### Effort & Risk

- **Effort**: ~1 hour (just a HashMap + accessor, bundled with item 5's config struct).
- **Files touched**: `jc-app/src/app_config.rs` (if created for item 5), call sites for gated features.
- **Risk**: None. It's a HashMap lookup.
- **Dependencies**: Best implemented alongside item 5 (shared config struct).

---

## Implementation Priority & Dependency Graph

```
                    ┌──────────────────────┐
                    │  5. AppConfig +       │
                    │  8. Feature Flags     │─────────────────┐
                    │  (~2h combined)       │                 │
                    └──────────┬───────────┘                 │
                               │                             │
              ┌────────────────┼────────────────┐            │
              ▼                ▼                ▼            ▼
   ┌──────────────┐  ┌─────────────────┐ ┌──────────┐ ┌──────────┐
   │ 1. Session   │  │ 4. Diff Review  │ │ 7. Vim   │ │ 3. Event │
   │ State Machine│  │ Phase 1+2       │ │ Mode     │ │ Replay   │
   │ (~2h)        │  │ (~3h)           │ │ (~24h)   │ │ (~10h)   │
   └──────────────┘  └─────────────────┘ └──────────┘ └──────────┘
         │                    │
         ▼                    ▼
   ┌──────────────┐  ┌─────────────────┐
   │ 2. Status    │  │ 4. Phase 3      │
   │ Bar Badges   │  │ Diff Attribution│
   │ (~1h)        │  │ (~4h, future)   │
   └──────────────┘  └─────────────────┘
```

### Recommended Order

| Priority | Item | Effort | Value | Notes |
|----------|------|--------|-------|-------|
| 1 | 5+8: JcAppConfig + Feature Flags | ~2h | Foundation | Unblocks 7 and 3 toggles |
| 2 | 1: Session State Machine | ~2h | High | Immediate UX improvement |
| 3 | 4 (Phase 1): Wire DiffReviewed | ~1h | High | Existing code, just needs wiring |
| 4 | 2: Status Bar Badges | ~1h | Medium | Quick win from existing data |
| 5 | 4 (Phase 2): Review Persistence | ~2h | Medium | Better UX across refreshes |
| 6 | 3: Event Replay | ~10h | High | Complex but high value |
| 7 | 7: Vim Mode | ~24h | Medium | Large scope, separate project |
| — | 6: IPC Singleton | 0h | — | Already done |

### Total Estimated Effort

- **Quick wins (items 1-5, excluding 3 and 7)**: ~8 hours
- **Medium project (item 3)**: ~10 hours
- **Large project (item 7)**: ~24 hours
- **Grand total**: ~42 hours

Items 1-5 can be shipped as a single PR. Items 3 and 7 should be separate branches.
