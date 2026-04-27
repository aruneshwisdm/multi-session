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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_contains_six_variants() {
        assert_eq!(PaneContentKind::ALL.len(), 6);
    }

    #[test]
    fn labels_are_non_empty() {
        for kind in PaneContentKind::ALL {
            assert!(!kind.label().is_empty(), "{kind:?} has empty label");
        }
    }

    #[test]
    fn specific_labels() {
        assert_eq!(PaneContentKind::ClaudeTerminal.label(), "Claude");
        assert_eq!(PaneContentKind::GeneralTerminal.label(), "Terminal");
        assert_eq!(PaneContentKind::GitDiff.label(), "Diff");
        assert_eq!(PaneContentKind::CodeViewer.label(), "Code");
        assert_eq!(PaneContentKind::TodoEditor.label(), "TODO");
        assert_eq!(PaneContentKind::GlobalTodo.label(), "Global TODO");
    }

    #[test]
    fn pane_state_stores_kind() {
        let pane = PaneState::new(PaneContentKind::CodeViewer);
        assert_eq!(pane.kind, PaneContentKind::CodeViewer);
    }
}
