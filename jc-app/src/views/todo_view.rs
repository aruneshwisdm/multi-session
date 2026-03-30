use iced::widget::{column, container, scrollable, text};
use iced::Element;
use jc_core::todo::{self, TodoDocument};
use std::path::PathBuf;

use super::workspace::Message;

pub struct TodoViewState {
    pub path: PathBuf,
    pub content: String,
    pub document: TodoDocument,
    pub dirty: bool,
    pub active_label: Option<String>,
}

impl TodoViewState {
    pub fn new(project_path: PathBuf) -> Self {
        let path = project_path.join("TODO.md");
        let content = std::fs::read_to_string(&path).unwrap_or_default();
        let document = todo::parse(&content);
        Self {
            path,
            content,
            document,
            dirty: false,
            active_label: None,
        }
    }

    pub fn reload(&mut self) {
        self.content = std::fs::read_to_string(&self.path).unwrap_or_default();
        self.document = todo::parse(&self.content);
        self.dirty = false;
    }

    pub fn save(&mut self) {
        if let Err(e) = std::fs::write(&self.path, &self.content) {
            eprintln!("failed to save TODO: {e}", );
        } else {
            self.dirty = false;
        }
    }

    pub fn problems(&self) -> Vec<todo::TodoProblem> {
        todo::validate(&self.document, &self.path, &self.content)
    }

    pub fn view(&self) -> Element<'_, Message> {
        let header = {
            let dirty_marker = if self.dirty { " [+]" } else { "" };
            text(format!("TODO{dirty_marker}")).size(14)
        };

        let body: Element<Message> = if self.content.is_empty() {
            text("No TODO.md found").size(13).into()
        } else {
            let lines: Vec<Element<Message>> = self
                .content
                .lines()
                .map(|line| text(line).size(12).into())
                .collect();
            scrollable(column(lines).spacing(0)).into()
        };

        container(column![header, body].spacing(4))
            .padding(8)
            .into()
    }
}
