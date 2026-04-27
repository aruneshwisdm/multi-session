pub mod pane;
pub mod session_state;
pub mod project_state;
pub mod workspace;
pub mod code_view;
pub mod diff_view;
pub mod todo_view;
pub mod picker;
pub mod keybinding_help;
pub mod terminal_canvas;

use std::hash::{DefaultHasher, Hash, Hasher};

use crate::language::Language;

/// Trait for views that support line search.
pub trait LineSearchable {
    fn editor_text(&self) -> String;
    fn language_name(&self) -> Language;
    fn scroll_to_line(&mut self, line: u32);
}

pub fn compute_checksum(content: &str) -> u64 {
    let mut hasher = DefaultHasher::default();
    content.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::workspace::PaneLayoutKind;

    #[test]
    fn compute_checksum_deterministic() {
        let a = compute_checksum("hello world");
        let b = compute_checksum("hello world");
        assert_eq!(a, b);
    }

    #[test]
    fn compute_checksum_different_for_different_content() {
        let a = compute_checksum("hello");
        let b = compute_checksum("world");
        assert_ne!(a, b);
    }

    #[test]
    fn compute_checksum_empty_string() {
        let a = compute_checksum("");
        let b = compute_checksum("");
        assert_eq!(a, b);
    }

    #[test]
    fn pane_layout_default_is_three() {
        assert_eq!(PaneLayoutKind::default(), PaneLayoutKind::Three);
    }

    #[test]
    fn pane_layout_equality() {
        assert_eq!(PaneLayoutKind::One, PaneLayoutKind::One);
        assert_ne!(PaneLayoutKind::One, PaneLayoutKind::Two);
        assert_ne!(PaneLayoutKind::Two, PaneLayoutKind::Three);
    }
}
