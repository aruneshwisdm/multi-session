use iced::widget::{column, container, text, text_editor};
use iced::{Element, Font, Length};
use jc_core::todo::{self, TodoDocument};
use std::path::PathBuf;

use super::workspace::Message;

pub struct TodoViewState {
    pub path: PathBuf,
    pub content: text_editor::Content,
    pub raw_text: String,
    pub document: TodoDocument,
    pub dirty: bool,
    pub active_label: Option<String>,
}

impl TodoViewState {
    pub fn new(project_path: PathBuf) -> Self {
        let path = project_path.join("TODO.md");
        let raw_text = std::fs::read_to_string(&path).unwrap_or_default();
        let document = todo::parse(&raw_text);
        let content = text_editor::Content::with_text(&raw_text);
        Self {
            path,
            content,
            raw_text,
            document,
            dirty: false,
            active_label: None,
        }
    }

    pub fn reload(&mut self) {
        self.raw_text = std::fs::read_to_string(&self.path).unwrap_or_default();
        self.document = todo::parse(&self.raw_text);
        self.content = text_editor::Content::with_text(&self.raw_text);
        self.dirty = false;
    }

    pub fn save(&mut self) {
        let text = self.content.text();
        if let Err(e) = std::fs::write(&self.path, &text) {
            eprintln!("failed to save TODO: {e}");
        } else {
            self.raw_text = text;
            self.document = todo::parse(&self.raw_text);
            self.dirty = false;
        }
    }

    pub fn perform_action(&mut self, action: text_editor::Action) {
        let is_edit = action.is_edit();
        self.content.perform(action);
        if is_edit {
            self.dirty = true;
            self.raw_text = self.content.text();
            self.document = todo::parse(&self.raw_text);
        }
    }

    pub fn problems(&self) -> Vec<todo::TodoProblem> {
        todo::validate(&self.document, &self.path, &self.raw_text)
    }

    pub fn view(&self) -> Element<'_, Message> {
        let header = {
            let dirty_marker = if self.dirty { " [+]" } else { "" };
            text(format!("TODO{dirty_marker}")).size(14)
        };

        let editor = text_editor(&self.content)
            .on_action(Message::TodoEditorAction)
            .font(Font::MONOSPACE)
            .size(12)
            .height(Length::Fill)
            .highlight(
                "md",
                iced::highlighter::Theme::SolarizedDark,
            );

        container(column![header, editor].spacing(4))
            .padding(8)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}
