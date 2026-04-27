use jc_core::problem::{AppTodoProblem, ClaudeProblem, SessionProblem, TerminalProblem};
use jc_core::todo::TodoProblem;
use jc_terminal::{PtyHandle, TerminalEvent, TerminalState};
use parking_lot::Mutex;
use std::collections::HashSet;
use std::io::Read;
use std::path::Path;
use std::sync::Arc;

use super::pane::PaneContentKind;

pub type SessionId = usize;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PendingEvent {
    ClaudePermission,
    ClaudeStopFailure,
    TerminalBell,
}

#[derive(Debug, Clone)]
pub struct SavedPaneLayout {
    pub pane_kinds: [Option<PaneContentKind>; 3],
    pub active_pane_index: usize,
}

/// Per-terminal PTY + emulator state.
pub struct Terminal {
    pub pty: PtyHandle,
    pub reader: Arc<Mutex<Option<Box<dyn Read + Send>>>>,
    pub state: TerminalState,
    pub event_rx: flume::Receiver<TerminalEvent>,
}

impl Terminal {
    fn spawn_shell(project_path: &Path, cols: u16, rows: u16) -> Self {
        let (event_tx, event_rx) = flume::unbounded();
        let (pty, reader) =
            PtyHandle::spawn_shell(cols, rows, Some(project_path)).expect("failed to spawn shell PTY");
        let state = TerminalState::new(cols as usize, rows as usize, event_tx);
        Self { pty, reader: Arc::new(Mutex::new(Some(reader))), state, event_rx }
    }

    fn spawn_command(command: &str, project_path: &Path, cols: u16, rows: u16) -> Self {
        let (event_tx, event_rx) = flume::unbounded();
        match PtyHandle::spawn_command(command, cols, rows, Some(project_path)) {
            Ok((pty, reader)) => {
                let state = TerminalState::new(cols as usize, rows as usize, event_tx);
                Self { pty, reader: Arc::new(Mutex::new(Some(reader))), state, event_rx }
            }
            Err(e) => {
                eprintln!("failed to spawn command '{command}': {e}");
                eprintln!("falling back to shell — is 'claude' in PATH?");
                Self::spawn_shell(project_path, cols, rows)
            }
        }
    }
}

pub struct SessionState {
    pub id: SessionId,
    pub uuid: Option<String>,
    pub label: String,
    pub claude_terminal: Terminal,
    pub general_terminal: Terminal,
    pub pending_events: HashSet<PendingEvent>,
    pub problems: Vec<SessionProblem>,
    pub busy: bool,
    pub has_ever_been_busy: bool,
    pub code_file: Option<std::path::PathBuf>,
    pub saved_layout: Option<SavedPaneLayout>,
}

impl SessionState {
    pub fn create(
        id: SessionId,
        uuid: Option<String>,
        label: String,
        project_path: &Path,
        cols: u16,
        rows: u16,
    ) -> Self {
        let command = uuid
            .as_ref()
            .filter(|u| !u.is_empty())
            .map(|u| format!("claude --resume {u}"))
            .unwrap_or_else(|| "claude".to_string());

        let claude_terminal = Terminal::spawn_command(&command, project_path, cols, rows);
        let general_terminal = Terminal::spawn_shell(project_path, cols, rows);

        Self {
            id,
            uuid,
            label,
            claude_terminal,
            general_terminal,
            pending_events: HashSet::default(),
            problems: Vec::new(),
            busy: false,
            has_ever_been_busy: false,
            code_file: None,
            saved_layout: None,
        }
    }

    pub fn refresh_problems(&mut self, todo_problems: &[TodoProblem]) -> bool {
        let mut problems = Vec::new();

        for event in &self.pending_events {
            let sp = match event {
                PendingEvent::ClaudePermission => SessionProblem::Claude(ClaudeProblem::Permission),
                PendingEvent::ClaudeStopFailure => {
                    SessionProblem::Claude(ClaudeProblem::StopFailure)
                }
                PendingEvent::TerminalBell => SessionProblem::Terminal(TerminalProblem::Bell),
            };
            problems.push(sp);
        }

        for tp in todo_problems {
            match tp {
                TodoProblem::UnsentWait { label } if label == &self.label => {
                    problems.push(SessionProblem::Todo(AppTodoProblem::UnsentWait {
                        label: label.clone(),
                    }));
                }
                _ => {}
            }
        }

        problems.sort_by_key(|p| p.rank());
        let changed = self.problems != problems;
        self.problems = problems;
        changed
    }

    pub fn acknowledge(&mut self) {
        self.pending_events.clear();
    }
}
