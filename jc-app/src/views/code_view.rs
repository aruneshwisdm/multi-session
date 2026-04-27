use crate::language::Language;
use crate::outline;
use crate::views::compute_checksum;
use crate::views::terminal_canvas::LILEX;
use iced::widget::{column, container, text, text_editor};
use iced::{Element, Length};
use std::path::PathBuf;

use super::workspace::Message;

pub struct CodeViewState {
    pub file_path: Option<PathBuf>,
    pub content: text_editor::Content,
    pub raw_text: String,
    pub language: Language,
    pub dirty: bool,
    pub disk_checksum: u64,
    pub breadcrumb: Vec<String>,
    pub scroll_offset: f32,
    pub externally_modified: bool,
}

impl Default for CodeViewState {
    fn default() -> Self {
        Self {
            file_path: None,
            content: text_editor::Content::new(),
            raw_text: String::new(),
            language: Language::Text,
            dirty: false,
            disk_checksum: 0,
            breadcrumb: Vec::new(),
            scroll_offset: 0.0,
            externally_modified: false,
        }
    }
}

impl CodeViewState {
    pub fn open_file(&mut self, path: PathBuf) {
        let text = std::fs::read_to_string(&path).unwrap_or_default();
        self.language = Language::from_path(&path);
        self.disk_checksum = compute_checksum(&text);
        self.content = text_editor::Content::with_text(&text);
        self.raw_text = text;
        self.file_path = Some(path);
        self.dirty = false;
        self.externally_modified = false;
        self.update_breadcrumb(0);
    }

    pub fn save(&mut self) {
        if let Some(path) = &self.file_path {
            let text = self.content.text();
            if let Err(e) = std::fs::write(path, &text) {
                eprintln!("failed to save {}: {e}", path.display());
            } else {
                self.disk_checksum = compute_checksum(&text);
                self.raw_text = text;
                self.dirty = false;
            }
        }
    }

    pub fn goto_line(&mut self, line: usize) {
        self.content.perform(text_editor::Action::Move(
            text_editor::Motion::DocumentStart,
        ));
        for _ in 0..line.saturating_sub(1) {
            self.content.perform(text_editor::Action::Move(
                text_editor::Motion::Down,
            ));
        }
    }

    pub fn perform_action(&mut self, action: text_editor::Action) {
        let is_edit = action.is_edit();
        self.content.perform(action);
        if is_edit {
            self.dirty = true;
            self.raw_text = self.content.text();
        }
    }

    fn update_breadcrumb(&mut self, byte_offset: usize) {
        let outline_items = outline::compute_outline(&self.raw_text, self.language);
        self.breadcrumb = outline::breadcrumb_at_byte(&outline_items, byte_offset)
            .into_iter()
            .map(|item| item.label.clone())
            .collect();
    }

    pub fn view(&self) -> Element<'_, Message> {
        let header = if let Some(path) = &self.file_path {
            let dirty_marker = if self.dirty { " [+]" } else { "" };
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "untitled".to_string());
            text(format!("{name}{dirty_marker}")).size(14)
        } else {
            text("No file open").size(14)
        };

        let editor = text_editor(&self.content)
            .on_action(Message::CodeEditorAction)
            .font(LILEX)
            .size(13)
            .height(Length::Fill)
            .highlight(
                self.language.extension(),
                iced::highlighter::Theme::SolarizedDark,
            );

        container(column![header, editor].spacing(4))
            .padding(8)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}
