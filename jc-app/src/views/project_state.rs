use crate::views::session_state::{SessionId, SessionState};
use jc_core::problem::{DiffProblem, ProjectProblem, ScriptProblem};
use jc_core::status_script;
use jc_core::todo::{self, TodoDocument};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

pub struct ProjectState {
    pub path: PathBuf,
    pub name: String,
    pub sessions: HashMap<SessionId, SessionState>,
    pub active_session: Option<SessionId>,
    pub next_session_id: SessionId,
    pub todo_document: TodoDocument,
    pub todo_text: String,
    pub todo_dirty: bool,
    pub diff_text: String,
    pub diff_stale: bool,
    pub unreviewed_files: Vec<PathBuf>,
    pub problems: Vec<ProjectProblem>,
    pub script_problems: Vec<ScriptProblem>,
    pub last_script_run: Option<Instant>,
}

impl ProjectState {
    pub fn create(path: PathBuf, name: String, cols: u16, rows: u16) -> Self {
        let todo_path = path.join("TODO.md");
        let todo_text = std::fs::read_to_string(&todo_path).unwrap_or_default();
        let todo_document = todo::parse(&todo_text);

        // Find best session to resume.
        let session_dir = Self::session_dir(&path);
        let best = todo_document
            .sessions
            .iter()
            .filter(|s| s.status == todo::SessionStatus::Active)
            .find(|s| {
                !s.uuid.is_empty()
                    && session_dir.join(format!("{}.jsonl", s.uuid)).exists()
            })
            .or_else(|| {
                todo_document
                    .sessions
                    .iter()
                    .find(|s| s.status == todo::SessionStatus::Active)
            });

        let mut sessions = HashMap::new();
        let mut next_session_id: SessionId = 0;

        if let Some(todo_session) = best {
            let uuid = if todo_session.uuid.is_empty() {
                None
            } else {
                Some(todo_session.uuid.clone())
            };
            let id = next_session_id;
            next_session_id += 1;
            let state =
                SessionState::create(id, uuid, todo_session.label.clone(), &path, cols, rows);
            sessions.insert(id, state);
        }

        let active_session = sessions.keys().next().copied();

        Self {
            path,
            name,
            sessions,
            active_session,
            next_session_id,
            todo_document,
            todo_text,
            todo_dirty: false,
            diff_text: String::new(),
            diff_stale: true,
            unreviewed_files: Vec::new(),
            problems: Vec::new(),
            script_problems: Vec::new(),
            last_script_run: None,
        }
    }

    pub fn create_bare(path: PathBuf, name: String) -> Self {
        let todo_path = path.join("TODO.md");
        let todo_text = std::fs::read_to_string(&todo_path).unwrap_or_default();
        let todo_document = todo::parse(&todo_text);

        Self {
            path,
            name,
            sessions: HashMap::new(),
            active_session: None,
            next_session_id: 0,
            todo_document,
            todo_text,
            todo_dirty: false,
            diff_text: String::new(),
            diff_stale: true,
            unreviewed_files: Vec::new(),
            problems: Vec::new(),
            script_problems: Vec::new(),
            last_script_run: None,
        }
    }

    pub fn session_dir(project_path: &Path) -> PathBuf {
        let encoded: String = project_path
            .to_string_lossy()
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() || c == '-' { c } else { '-' })
            .collect();
        let home = std::env::var("HOME").expect("HOME not set");
        PathBuf::from(home).join(".claude/projects").join(encoded)
    }

    pub fn active_session(&self) -> Option<&SessionState> {
        self.active_session.and_then(|id| self.sessions.get(&id))
    }

    pub fn active_session_mut(&mut self) -> Option<&mut SessionState> {
        self.active_session.and_then(|id| self.sessions.get_mut(&id))
    }

    pub fn active_label(&self) -> Option<&str> {
        self.active_session().map(|s| s.label.as_str())
    }

    pub fn refresh_problems(&mut self) -> bool {
        let todo_problems = todo::validate(&self.todo_document, &self.path, &self.todo_text);
        let mut changed = false;

        for session in self.sessions.values_mut() {
            changed |= session.refresh_problems(&todo_problems);
        }

        // Run status.sh at most once every 10 seconds.
        let script_interval = std::time::Duration::from_secs(10);
        let should_run_script =
            self.last_script_run.is_none_or(|t| t.elapsed() >= script_interval);
        if should_run_script {
            self.script_problems = status_script::run_status_script(&self.path);
            self.last_script_run = Some(Instant::now());
        }

        let mut problems: Vec<ProjectProblem> = self
            .unreviewed_files
            .iter()
            .map(|path| ProjectProblem::Diff(DiffProblem::UnreviewedFile(path.clone())))
            .chain(
                self.script_problems
                    .iter()
                    .map(|sp| ProjectProblem::Script(sp.clone())),
            )
            .collect();
        problems.sort_by_key(|p| p.rank());
        changed |= self.problems != problems;
        self.problems = problems;
        changed
    }

    pub fn session_by_label(&self, label: &str) -> Option<(SessionId, &SessionState)> {
        self.sessions
            .iter()
            .find(|(_, s)| s.label == label)
            .map(|(&id, s)| (id, s))
    }

    #[cfg(test)]
    pub fn for_testing(path: PathBuf, name: String) -> Self {
        Self {
            path,
            name,
            sessions: HashMap::new(),
            active_session: None,
            next_session_id: 0,
            todo_document: TodoDocument::default(),
            todo_text: String::new(),
            todo_dirty: false,
            diff_text: String::new(),
            diff_stale: true,
            unreviewed_files: Vec::new(),
            problems: Vec::new(),
            script_problems: Vec::new(),
            last_script_run: None,
        }
    }
}
