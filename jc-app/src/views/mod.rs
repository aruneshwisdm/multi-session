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
