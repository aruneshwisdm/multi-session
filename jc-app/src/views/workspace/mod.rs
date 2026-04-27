mod problems;
mod render;
pub(crate) mod keybindings;
pub(crate) mod terminal_input;

use crate::views::code_view::CodeViewState;
use crate::views::diff_view::DiffViewState;
use crate::views::keybinding_help::KeybindingHelpState;
use crate::views::pane::{PaneContentKind, PaneState};
use crate::views::picker::{PickerKind, PickerState};
use crate::views::project_state::ProjectState;
use crate::views::session_state::{PendingEvent, SessionId, SessionState};
use crate::views::todo_view::TodoViewState;
use iced::widget::text_editor;
use jc_core::config::{AppConfig, AppState};
use jc_core::hooks::{HookEvent, HookEventKind, HookServer};
use jc_core::snippets::{self, SnippetDocument};
use jc_core::theme::Appearance;
use jc_terminal::Palette;
use std::path::PathBuf;

/// All events that flow through the iced application.
#[derive(Debug, Clone)]
pub enum Message {
    // Terminal
    TerminalOutput(SessionId, Vec<u8>),
    TerminalEvent(SessionId, TerminalEventKind),
    TerminalResize(u16, u16),

    // Workspace navigation
    SwitchSession(SessionId),
    NewSession,
    CloseSession(SessionId),
    SwitchProject(usize),
    CyclePane,
    SetLayout(PaneLayoutKind),
    ShowPane(PaneContentKind),

    // Hooks
    HookReceived(HookEvent),

    // IPC
    IpcProjectOpen(PathBuf),

    // Notifications
    NotificationAction(String),

    // File watcher
    FileChanged(PathBuf),

    // Pickers
    OpenPicker(PickerKind),
    PickerQueryChanged(String),
    PickerSelectNext,
    PickerSelectPrev,
    PickerConfirm,
    PickerDismiss,

    // Code/Diff
    OpenFile(PathBuf),
    SaveFile,
    DiffReviewed,

    // Text editor actions
    CodeEditorAction(text_editor::Action),
    TodoEditorAction(text_editor::Action),
    GlobalTodoEditorAction(text_editor::Action),

    // Terminal clipboard / selection
    TerminalTextSelected(String),
    TerminalCopy,
    TerminalPaste,

    // Window
    WindowResized(f32, f32),

    // Keybinding help
    ToggleKeybindingHelp,

    // Problems
    NextProblem,
    JumpToWait,

    // Close/Quit
    RequestClose,
    RequestQuit,
    ConfirmClose,
    CancelClose,

    // Tick for polling
    Tick,

    // Keyboard event from iced
    KeyboardEvent(iced::keyboard::Event),

    // No-op
    None,
}

#[derive(Debug, Clone)]
pub enum TerminalEventKind {
    Bell,
    Wakeup,
    Exit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PaneLayoutKind {
    One,
    Two,
    #[default]
    Three,
}

pub struct Workspace {
    pub panes: Vec<PaneState>,
    pub active_pane_index: usize,
    pub layout: PaneLayoutKind,
    pub projects: Vec<ProjectState>,
    pub active_project_index: usize,
    pub config: AppConfig,
    pub palette: Palette,

    // Picker state
    pub picker: Option<PickerState>,

    // Keybinding help overlay
    pub keybinding_help: KeybindingHelpState,

    // Code view per project — maps project_index to CodeViewState
    pub code_views: Vec<CodeViewState>,

    // Diff views per project
    pub diff_views: Vec<DiffViewState>,

    // TODO views per project
    pub todo_views: Vec<TodoViewState>,

    // Global TODO view
    pub global_todo: CodeViewState,

    // Hook server
    pub hook_server: Option<HookServer>,

    // IPC channel
    pub ipc_rx: Option<flume::Receiver<PathBuf>>,

    // Snippets
    pub snippets: SnippetDocument,

    // Close confirm state
    pub close_confirm: Option<CloseConfirmState>,

    // Problem cycling
    pub problem_cycle: Option<problems::ProblemCycleState>,
    pub pre_layer0_home: Option<(usize, SessionId)>,

    // Terminal dimensions
    pub terminal_cols: u16,
    pub terminal_rows: u16,
    pub last_terminal_selection: String,

    // Window state
    pub window_active: bool,
}

pub struct CloseConfirmState {
    pub session_count: usize,
    pub conflicts: Vec<String>,
    pub is_quit: bool,
}

impl Workspace {
    pub fn new(
        state: AppState,
        config: AppConfig,
        ipc_rx: flume::Receiver<PathBuf>,
    ) -> Self {
        let palette = Palette::for_appearance(Appearance::Dark);

        use crate::views::terminal_canvas::{CELL_WIDTH, CELL_HEIGHT};
        let pane_width = 1200.0f32 / 3.0;
        let pane_height = 800.0f32 - 60.0;
        let terminal_cols = ((pane_width - 16.0) / CELL_WIDTH).floor() as u16;
        let terminal_rows = ((pane_height - 30.0) / CELL_HEIGHT).floor() as u16;
        let terminal_cols = terminal_cols.clamp(20, 300);
        let terminal_rows = terminal_rows.clamp(5, 100);

        let mut projects = Vec::new();
        for project in &state.projects {
            projects.push(ProjectState::create(
                project.path.clone(),
                project.name(),
                terminal_cols,
                terminal_rows,
            ));
        }

        if projects.is_empty() {
            let path = std::env::current_dir().unwrap_or_default();
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| "unknown".into());
            projects.push(ProjectState::create(path, name, terminal_cols, terminal_rows));
        }

        let code_views: Vec<CodeViewState> =
            projects.iter().map(|_| CodeViewState::default()).collect();
        let diff_views: Vec<DiffViewState> = projects
            .iter()
            .map(|p| DiffViewState::new(p.path.clone()))
            .collect();
        let todo_views: Vec<TodoViewState> = projects
            .iter()
            .map(|p| TodoViewState::new(p.path.clone()))
            .collect();

        let global_todo_path = PathBuf::from(
            std::env::var("HOME").expect("HOME not set"),
        )
        .join(".claude/TODO.md");
        let mut global_todo = CodeViewState::default();
        if global_todo_path.exists() {
            global_todo.open_file(global_todo_path);
        }

        // Initial pane layout: Claude, TODO, Global TODO
        let panes = vec![
            PaneState::new(PaneContentKind::ClaudeTerminal),
            PaneState::new(PaneContentKind::TodoEditor),
            PaneState::new(PaneContentKind::GlobalTodo),
        ];

        // Start hook server.
        let project_paths: Vec<PathBuf> =
            projects.iter().map(|p| p.path.clone()).collect();
        let hook_server = match HookServer::start(project_paths.clone()) {
            Ok(server) => {
                let port = server.port;
                for path in &project_paths {
                    let path = path.clone();
                    std::thread::spawn(move || {
                        if let Err(e) =
                            jc_core::hooks_settings::install_hooks(&path, port)
                        {
                            eprintln!(
                                "failed to install hooks for {}: {e}",
                                path.display()
                            );
                        }
                    });
                }
                Some(server)
            }
            Err(e) => {
                eprintln!("failed to start hook server: {e}");
                None
            }
        };

        // Load snippets.
        snippets::ensure_file_exists();
        let snippets = snippets::load();

        Self {
            panes,
            active_pane_index: 0,
            layout: PaneLayoutKind::default(),
            projects,
            active_project_index: 0,
            config,
            palette,
            picker: None,
            keybinding_help: KeybindingHelpState::default(),
            code_views,
            diff_views,
            todo_views,
            global_todo,
            hook_server,
            ipc_rx: Some(ipc_rx),
            snippets,
            close_confirm: None,
            problem_cycle: None,
            pre_layer0_home: None,
            terminal_cols,
            terminal_rows,
            last_terminal_selection: String::new(),
            window_active: true,
        }
    }

    // -------------------------------------------------------------------
    // Accessors
    // -------------------------------------------------------------------

    pub fn active_project(&self) -> &ProjectState {
        &self.projects[self.active_project_index]
    }

    pub fn active_project_mut(&mut self) -> &mut ProjectState {
        &mut self.projects[self.active_project_index]
    }

    pub fn visible_pane_count(&self) -> usize {
        match self.layout {
            PaneLayoutKind::One => 1,
            PaneLayoutKind::Two => 2,
            PaneLayoutKind::Three => 3,
        }
    }

    // -------------------------------------------------------------------
    // Session management
    // -------------------------------------------------------------------

    pub fn create_session(&mut self) {
        let project = &mut self.projects[self.active_project_index];
        let id = project.next_session_id;
        project.next_session_id += 1;

        let label = format!("Session {}", id + 1);
        let cols = self.terminal_cols;
        let rows = self.terminal_rows;
        let session = SessionState::create(id, None, label, &project.path, cols, rows);
        project.sessions.insert(id, session);
        project.active_session = Some(id);
    }

    pub fn switch_session(&mut self, session_id: SessionId) {
        let project = &mut self.projects[self.active_project_index];
        if project.sessions.contains_key(&session_id) {
            if let Some(session) = project.active_session_mut() {
                session.acknowledge();
            }
            project.active_session = Some(session_id);
        }
    }

    pub fn close_session(&mut self, session_id: SessionId) {
        let project = &mut self.projects[self.active_project_index];
        project.sessions.remove(&session_id);
        if project.active_session == Some(session_id) {
            project.active_session = project.sessions.keys().next().copied();
        }
    }

    // -------------------------------------------------------------------
    // Pane management
    // -------------------------------------------------------------------

    pub fn set_layout(&mut self, layout: PaneLayoutKind) {
        self.layout = layout;
        let count = self.visible_pane_count();
        if self.active_pane_index >= count {
            self.active_pane_index = 0;
        }
    }

    pub fn show_in_pane(&mut self, pane_idx: usize, kind: PaneContentKind) {
        if pane_idx < self.panes.len() {
            self.panes[pane_idx] = PaneState::new(kind);
            self.active_pane_index = pane_idx;
        }
    }

    pub fn cycle_pane(&mut self) {
        let count = self.visible_pane_count();
        self.active_pane_index = (self.active_pane_index + 1) % count;
    }

    pub fn resolve_pane_for_kind(&self, kind: PaneContentKind) -> usize {
        let visible = self.visible_pane_count();
        if visible == 1 {
            return 0;
        }
        if let Some(idx) = (0..visible).find(|&i| self.panes[i].kind == kind) {
            return idx;
        }
        if visible == 2 {
            return if kind == PaneContentKind::ClaudeTerminal { 0 } else { 1 };
        }
        match kind {
            PaneContentKind::ClaudeTerminal => 0,
            PaneContentKind::TodoEditor => 1,
            _ => 2,
        }
    }

    // -------------------------------------------------------------------
    // Hook event handling
    // -------------------------------------------------------------------

    pub fn handle_hook_event(&mut self, event: HookEvent) {
        let project = &mut self.projects[self.active_project_index];
        let sid = &event.session_id;

        // Find the session by its ID string.
        let session_id_opt = project
            .session_by_label(sid)
            .map(|(id, _)| id);

        match &event.kind {
            HookEventKind::PromptSubmit => {
                if let Some(session_id) = session_id_opt {
                    if let Some(s) = project.sessions.get_mut(&session_id) {
                        s.busy = true;
                        s.has_ever_been_busy = true;
                    }
                }
            }
            HookEventKind::Stop => {
                if let Some(session_id) = session_id_opt {
                    if let Some(s) = project.sessions.get_mut(&session_id) {
                        s.busy = false;
                    }
                }
            }
            HookEventKind::StopFailure => {
                if let Some(session_id) = session_id_opt {
                    if let Some(s) = project.sessions.get_mut(&session_id) {
                        s.busy = false;
                        s.pending_events.insert(PendingEvent::ClaudeStopFailure);
                    }
                }
            }
            HookEventKind::IdlePrompt => {
                if let Some(session_id) = session_id_opt {
                    if let Some(s) = project.sessions.get_mut(&session_id) {
                        s.busy = false;
                    }
                }
            }
            HookEventKind::PermissionPrompt => {
                if let Some(session_id) = session_id_opt {
                    if let Some(s) = project.sessions.get_mut(&session_id) {
                        s.pending_events.insert(PendingEvent::ClaudePermission);
                    }
                }
            }
            _ => {}
        }
    }

    // -------------------------------------------------------------------
    // Save
    // -------------------------------------------------------------------

    pub fn save_all_dirty(&mut self) -> Vec<String> {
        let mut conflicts = Vec::new();

        for (i, project) in self.projects.iter().enumerate() {
            if self.todo_views[i].dirty {
                self.todo_views[i].save();
            }
            for _session in project.sessions.values() {
                let cv = &self.code_views[i];
                if cv.dirty {
                    if cv.externally_modified {
                        if let Some(path) = &cv.file_path {
                            let relative = path
                                .strip_prefix(&project.path)
                                .unwrap_or(path);
                            conflicts.push(relative.display().to_string());
                        }
                    }
                    // Note: save is deferred since we can't easily borrow mutably here
                }
            }
        }

        if self.global_todo.dirty && !self.global_todo.externally_modified {
            self.global_todo.save();
        }

        conflicts
    }

    pub fn resize_all_terminals(&self, cols: u16, rows: u16) {
        for project in &self.projects {
            for session in project.sessions.values() {
                let _ = session.claude_terminal.pty.resize(cols, rows, 0, 0);
                session.claude_terminal.state.resize(cols as usize, rows as usize);
                let _ = session.general_terminal.pty.resize(cols, rows, 0, 0);
                session.general_terminal.state.resize(cols as usize, rows as usize);
            }
        }
    }

    pub fn active_session_count(&self) -> usize {
        self.projects
            .iter()
            .flat_map(|p| p.sessions.values())
            .filter(|s| s.busy)
            .count()
    }
}
