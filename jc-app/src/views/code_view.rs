use crate::language::Language;
use crate::outline;
use crate::views::compute_checksum;
use iced::widget::{column, container, row, scrollable, text};
use iced::Element;
use std::path::PathBuf;

use super::workspace::Message;

pub struct CodeViewState {
    pub file_path: Option<PathBuf>,
    pub content: String,
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
            content: String::new(),
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
        let content = std::fs::read_to_string(&path).unwrap_or_default();
        self.language = Language::from_path(&path);
        self.disk_checksum = compute_checksum(&content);
        self.content = content;
        self.file_path = Some(path);
        self.dirty = false;
        self.externally_modified = false;
        self.update_breadcrumb(0);
    }

    pub fn save(&mut self) {
        if let Some(path) = &self.file_path {
            if let Err(e) = std::fs::write(path, &self.content) {
                eprintln!("failed to save {}: {e}", path.display());
            } else {
                self.disk_checksum = compute_checksum(&self.content);
                self.dirty = false;
            }
        }
    }

    fn update_breadcrumb(&mut self, byte_offset: usize) {
        let outline_items = outline::compute_outline(&self.content, self.language);
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

        let body: Element<Message> = if self.content.is_empty() {
            text("Empty file").size(13).into()
        } else {
            let lines: Vec<Element<Message>> = self
                .content
                .lines()
                .enumerate()
                .map(|(i, line)| {
                    row![
                        text(format!("{:>4} ", i + 1)).size(12),
                        text(line).size(13),
                    ]
                    .spacing(4)
                    .into()
                })
                .collect();

            scrollable(column(lines).spacing(0)).into()
        };

        container(column![header, body].spacing(4))
            .padding(8)
            .into()
    }
}
