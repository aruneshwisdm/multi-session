#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaneContentKind {
    ClaudeTerminal,
    GeneralTerminal,
    GitDiff,
    CodeViewer,
    TodoEditor,
    GlobalTodo,
}

impl PaneContentKind {
    pub const ALL: [PaneContentKind; 6] = [
        Self::ClaudeTerminal,
        Self::GeneralTerminal,
        Self::GitDiff,
        Self::CodeViewer,
        Self::TodoEditor,
        Self::GlobalTodo,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::ClaudeTerminal => "Claude",
            Self::GeneralTerminal => "Terminal",
            Self::GitDiff => "Diff",
            Self::CodeViewer => "Code",
            Self::TodoEditor => "TODO",
            Self::GlobalTodo => "Global TODO",
        }
    }
}

/// Content stored in a pane — identifies what view is displayed.
#[derive(Debug, Clone)]
pub struct PaneState {
    pub kind: PaneContentKind,
}

impl PaneState {
    pub fn new(kind: PaneContentKind) -> Self {
        Self { kind }
    }
}
