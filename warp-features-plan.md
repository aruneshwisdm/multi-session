# Warp-Inspired Features: Implementation Plan

## Context

The warp-features-analysis.md identifies 8 features inspired by Warp's terminal. Items 2 (problem cycling) and 6 (IPC singleton) are already implemented. Item 7 (Vim mode) is a separate ~24h project. This plan covers the 5 actionable items: **AppConfig + Feature Flags** (items 5+8), **Session State Machine** (item 1), **Diff Review wiring** (item 4 phases 1+2), **Status Bar Badges** (item 2 enhancement), and **Session Persistence / Event Replay** (item 3, Option A).

All code goes in `jc-app` (jc-core is read-only per project rules).

---

## Dependency Order

```
  Step 1: JcAppConfig + Feature Flags ──┐
          (items 5+8)                    │
                                         ▼
  Step 2: SessionActivity enum ────► Step 4: Status Bar Badges
          (item 1)                        (item 2 enhancement)
                                         
  Step 3: Diff Review wiring ────► Step 3b: Review persistence
          (item 4 phase 1)               (item 4 phase 2)
                                         
  Step 5: Session Persistence + Restore
          (item 3, Option A)
```

Steps 1-4 are independent enough to do in sequence within one PR. Step 5 (session persistence) is a separate PR due to higher complexity and risk.

---

## Step 1: JcAppConfig + Feature Flags (~2h)

**Goal**: Create a jc-app-owned config struct stored at `~/.config/jc/jc-app.toml`, separate from upstream's `config.toml`.

### New file: `jc-app/src/app_config.rs`

```rust
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct JcAppConfig {
    pub attribution: AttributionMode,
    pub vim_mode: bool,
    #[serde(default)]
    pub features: HashMap<String, bool>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AttributionMode {
    #[default]
    CoAuthor,
    Author,
    NoAttribution,
}

impl JcAppConfig {
    pub fn is_enabled(&self, feature: &str) -> bool {
        self.features.get(feature).copied().unwrap_or(false)
    }
}

fn config_path() -> PathBuf {
    dirs::home_dir()
        .expect("could not determine home directory")
        .join(".config/jc/jc-app.toml")
}

pub fn load() -> Result<JcAppConfig> {
    let path = config_path();
    match std::fs::read_to_string(&path) {
        Ok(contents) => toml::from_str(&contents)
            .with_context(|| format!("failed to parse {}", path.display())),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(JcAppConfig::default()),
        Err(e) => Err(anyhow::anyhow!("failed to read {}: {e}", path.display())),
    }
}

pub fn save(config: &JcAppConfig) -> Result<()> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let contents = toml::to_string_pretty(config)
        .context("failed to serialize jc-app config")?;
    std::fs::write(&path, contents)
        .with_context(|| format!("failed to write {}", path.display()))
}
```

### Changes to existing files

**`jc-app/Cargo.toml`**: Add `toml.workspace = true` to `[dependencies]`.

**`jc-app/src/views/workspace/mod.rs`**: Add `pub app_config: JcAppConfig` field to `Workspace`. Initialize via `app_config::load().unwrap_or_default()` in `Workspace::new()` and `Workspace::for_testing()`.

**`jc-app/src/main.rs`**: No changes needed — config is loaded inside Workspace::new.

**`jc-app/src/views/mod.rs`**: Add `pub mod app_config;` to `jc-app/src/` (at the crate root, alongside `app.rs`). Actually, since this is app-level config, place in crate root: **new module `jc-app/src/app_config.rs`**, add `mod app_config;` to either `main.rs` or make it pub from `views/mod.rs`. Cleaner: add `pub mod app_config;` at the top of `main.rs` alongside the existing `mod app;`.

### Tests

- Unit tests in `app_config.rs`: roundtrip serialize/deserialize, `is_enabled` returns false for missing keys, default values.
- Forward-compatibility test: a TOML string with unknown keys (e.g., `future_setting = true`) deserializes without error thanks to `#[serde(default)]`. Verify this explicitly.

---

## Step 2: SessionActivity Enum (~1h)

**Goal**: Derive a single session activity state from existing fields.

### Changes to `jc-app/src/views/session_state.rs`

Add the enum and a method on `SessionState`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionActivity {
    Busy,
    Idle,
    NeedsPermission,
    Error,
    New,
}

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
            SessionActivity::Idle
        }
    }
}
```

Note: `HasUnreviewedDiffs` is deferred (requires per-session diff attribution). The enum has 5 variants for now.

### Changes to `jc-app/src/views/workspace/render.rs`

In `render_title_bar()`, replace the current plain-text tab rendering with colored activity dots. Each session tab gets a colored dot prefix based on `session.activity()`:

```rust
fn activity_indicator(activity: SessionActivity) -> (&'static str, Color) {
    match activity {
        SessionActivity::Busy => ("●", Color::from_rgb(0.2, 0.7, 0.3)),
        SessionActivity::NeedsPermission => ("●", Color::from_rgb(1.0, 0.6, 0.0)),
        SessionActivity::Error => ("●", Color::from_rgb(0.9, 0.2, 0.2)),
        SessionActivity::Idle => ("●", Color::from_rgb(0.5, 0.5, 0.5)),
        SessionActivity::New => ("○", Color::from_rgb(0.7, 0.7, 0.7)),
    }
}
```

Modify the tab rendering closure in `render_title_bar()` (lines 51-72 of `render.rs`) to prepend the colored dot using `text(dot).color(color)` in a `row!` alongside the tab label.

### Tests

- Unit tests in `session_state.rs`: verify `activity()` returns the correct variant for each state combination. Priority ordering: Error > NeedsPermission > Busy > New > Idle.

---

## Step 3: Diff Review Wiring (~3h)

### Phase 1: Complete the toggle + keybinding (~1h)

**`jc-app/src/app.rs` (line 325-331)**: The `DiffReviewed` handler already exists and sets `reviewed = true`. Extend it to:
1. **Toggle** instead of one-way set: `fd.reviewed = !fd.reviewed;`
2. **Update `unreviewed_files`** on the project so problems reflect the change:
   ```rust
   Message::DiffReviewed => {
       let pi = workspace.active_project_index;
       let dv = &mut workspace.diff_views[pi];
       if let Some(fd) = dv.file_diffs.get_mut(dv.current_file_index) {
           fd.reviewed = !fd.reviewed;
       }
       workspace.projects[pi].unreviewed_files = dv.unreviewed_files();
       workspace.projects[pi].refresh_problems();
   }
   ```

**`jc-app/src/views/workspace/keybindings.rs`**: Add `Ctrl+R` binding for `DiffReviewed`. `Ctrl+R` is currently unbound (verified). Note: the global keybinding handler in `keybindings.rs` only fires when the iced keyboard subscription delivers the event. Terminal panes handle input separately via `terminal_input.rs` which writes directly to the PTY — so `Ctrl+R` in the terminal still sends `\x12` to the shell (e.g., reverse-i-search in bash). The global handler does NOT intercept terminal input. Safe to use.

```rust
keyboard::Key::Character("r") if !shift => {
    Some(Message::DiffReviewed)
}
```

**`jc-app/src/views/workspace/keybindings.rs`** (keybinding help): Also update the `KEYBINDINGS` array in `keybinding_help.rs` to include `Ctrl+R — Toggle diff reviewed` so it appears in the `Ctrl+?` help overlay.

**`jc-app/src/views/diff_view.rs`**: In the `view()` method, add both a visual indicator and a clickable button for users who don't know the keyboard shortcut:

```rust
// In view(), after the header text
let review_status = if let Some(file) = self.file_diffs.get(self.current_file_index) {
    if file.reviewed { "✓ reviewed" } else { "○ unreviewed" }
} else {
    ""
};
```

Add a clickable `button(text(review_status)).on_press(Message::DiffReviewed)` next to the file name in the header row, so review state is toggleable via mouse click as well as `Ctrl+R`.

### Phase 2: Preserve review state across refreshes (~1h)

**`jc-app/src/views/diff_view.rs`**: Modify `apply_diff_text()` to carry forward `reviewed` status for files whose content hasn't changed:

```rust
pub fn apply_diff_text(&mut self, diff_text: String) -> bool {
    let mut new_diffs = parse_diff_files(&diff_text);
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

### Tests

- Existing test `diff_reviewed_marks_current_file` needs update: it currently asserts `reviewed_count() == 1` after one DiffReviewed. With toggle behavior, sending DiffReviewed twice should return to 0.
- New test: `apply_diff_text_preserves_reviewed_for_unchanged_files` — mark a file reviewed, re-apply same diff text, verify it stays reviewed.
- New test: `apply_diff_text_resets_reviewed_for_changed_files` — mark reviewed, apply diff with different content for that file, verify reviewed is false.

---

## Step 4: Status Bar Badges (~1h)

**Goal**: Show cross-session problem counts per layer in the title bar.

### Changes to `jc-app/src/views/workspace/render.rs`

In `render_title_bar()`, after the tab row, add layer badges using data from `self.layer_problem_sessions()` (already implemented in `problems.rs:244`):

```rust
let layer_counts = self.layer_problem_sessions();
let badges: Vec<Element<Message>> = [
    ("!", Color::from_rgb(0.9, 0.2, 0.2), &layer_counts[0]),     // L0
    ("?", Color::from_rgb(1.0, 0.6, 0.0), &layer_counts[1]),     // L1
    ("~", Color::from_rgb(0.3, 0.5, 0.9), &layer_counts[2]),     // L2
    ("○", Color::from_rgb(0.5, 0.5, 0.5), &layer_counts[3]),     // L3
]
.iter()
.filter(|(_, _, sessions)| !sessions.is_empty())
.map(|(icon, color, sessions)| {
    text(format!("{icon}{}", sessions.len()))
        .size(11)
        .color(*color)
        .into()
})
.collect();
```

Add these badges to the right side of the title bar row. Only render badges with non-zero counts to avoid clutter.

### Tests

- `layer_problem_sessions()` is already tested via the existing problem cycling tests. The badge rendering is pure view code; verify visually.

---

## Step 5: Session Persistence + Restore (~10h, separate PR)

**Goal**: Persist session state to disk so sessions survive jc restarts. Uses Option A from the analysis (kill old process, `--resume`, restore state).

### New file: `jc-app/src/session_persistence.rs`

```rust
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedSession {
    pub project_path: PathBuf,
    pub label: String,
    pub uuid: Option<String>,
    pub pty_pid: Option<u32>,
    pub busy: bool,
    pub has_ever_been_busy: bool,
    pub saved_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SessionStore {
    pub sessions: Vec<PersistedSession>,
}
```

Provide `load()` / `save()` functions storing to `~/.config/jc/sessions.json` (JSON, not TOML — session state changes on every hook event so JSON's faster serialization matters; config in Step 1 uses TOML for human editability).

### Changes to `jc-app/src/views/session_state.rs`

- Add `pub pty_pid: Option<u32>` to `SessionState` (extracted from `PtyHandle` after spawn).
- After spawning the claude PTY, store the child PID.

### Changes to `jc-app/src/views/workspace/mod.rs`

- On significant state changes (hook event, session create/close), call `session_persistence::save()`.
- In `Workspace::new()`, check for persisted sessions. For each:
  1. Check if the old PID is still alive (`unsafe { libc::kill(pid, 0) }` on Linux, or just skip PID check and always kill).
  2. Kill the old process (SIGTERM).
  3. Create a new session via `SessionState::create()` with the persisted UUID (triggering `claude --resume {uuid}`).
  4. Restore `busy` / `has_ever_been_busy` from persisted state.

### Hook state inference

After restore, the session starts in the persisted `busy` state. If a hook event arrives within ~2 seconds, it corrects the state. If no event arrives and `busy` was true, a timeout (via `Tick`) can flip it to idle. This is imprecise but functional.

### Feature flag gate

Wrap the restore logic behind `app_config.is_enabled("session_restore")` so it can be disabled if it causes problems.

### New dependencies

- Add `chrono.workspace = true` to `jc-app/Cargo.toml` (already declared in workspace `Cargo.toml` line 18, just needs `chrono.workspace = true` in jc-app's `[dependencies]` section).
- On Linux, use `libc::kill` for PID checking (add `libc` to workspace deps, or use `std::process::Command::new("kill")` for simplicity).

### Risk mitigations

- If the persisted PID belongs to a different process (PID reuse), we'd kill the wrong thing. Mitigation: before killing, check `/proc/{pid}/cmdline` contains "claude".
- Race condition between old/new hook servers: the old process may POST to the old port. Mitigation: the new hook server binds a new port and reinstalls hooks, so old POSTs will fail silently (acceptable).

### Tests

- Unit tests for `SessionStore` serialization roundtrip.
- Test that `PersistedSession` with missing fields deserializes with defaults.
- Integration test: create a persisted session file, verify `Workspace::new()` restores it (would need to mock PTY spawning — may be difficult).

---

## Files Modified (Summary)

| File | Steps | Change |
|------|-------|--------|
| `jc-app/Cargo.toml` | 1, 5 | Add `toml`, `chrono` deps |
| `jc-app/src/main.rs` | 1 | Add `mod app_config;` |
| **`jc-app/src/app_config.rs`** (new) | 1 | JcAppConfig struct, load/save, feature flags |
| `jc-app/src/views/session_state.rs` | 2, 5 | SessionActivity enum + activity() method, pty_pid |
| `jc-app/src/views/workspace/mod.rs` | 1, 5 | Add `app_config` field, persistence hooks |
| `jc-app/src/views/workspace/render.rs` | 2, 4 | Activity dots on tabs, layer badges |
| `jc-app/src/views/workspace/keybindings.rs` | 3 | Ctrl+R for DiffReviewed |
| `jc-app/src/views/keybinding_help.rs` | 3 | Add Ctrl+R entry to help overlay |
| `jc-app/src/views/diff_view.rs` | 3 | Toggle reviewed, preserve across refresh, review indicator + clickable button in view |
| `jc-app/src/app.rs` | 3 | Update DiffReviewed handler to toggle + refresh problems |
| **`jc-app/src/session_persistence.rs`** (new) | 5 | PersistedSession, SessionStore, load/save (JSON format) |

---

## Verification

### After Steps 1-4 (first PR)

1. **Build**: `cargo build -p jc-app` — 0 errors, 0 warnings
2. **Tests**: `cargo test --workspace` — all existing tests pass + new tests pass
3. **Config**: Run the app, verify `~/.config/jc/jc-app.toml` is loadable (or defaults gracefully)
4. **Session dots**: Open multiple sessions. Send a prompt — tab dot turns green. Wait for completion — dot turns gray. Trigger a permission prompt — dot turns orange.
5. **Diff review**: Open diff view (`Ctrl+D`), press `Ctrl+R` — current file toggles reviewed. Header shows updated count. Press `Ctrl+R` again — toggles back.
6. **Review persistence**: With a file marked reviewed, modify a different file (triggering diff refresh). Verify the reviewed file stays checked if its diff content is unchanged.
7. **Status badges**: Create multiple sessions, trigger problems in some. Verify colored badges appear in the title bar showing cross-session problem counts.

### After Step 5 (second PR)

1. **Build + tests** as above
2. **Persistence**: Start jc, create sessions, kill jc (`kill -9`). Restart jc — sessions should restore with correct labels and UUIDs.
3. **Feature gate**: Set `session_restore = false` in `jc-app.toml` — sessions should not restore on restart.
