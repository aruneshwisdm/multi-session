use iced::widget::{column, container, text, text_input};
use iced::{Element, Length};

use super::workspace::Message;

#[derive(Debug, Clone)]
pub enum PickerKind {
    File,
    Session,
    Project,
    Snippet,
    Command,
    LineSearch,
}

pub struct PickerState {
    pub kind: PickerKind,
    pub query: String,
    pub items: Vec<PickerItem>,
    pub selected_index: usize,
}

#[derive(Debug, Clone)]
pub struct PickerItem {
    pub label: String,
    pub detail: String,
    pub data: PickerItemData,
}

#[derive(Debug, Clone)]
pub enum PickerItemData {
    File(std::path::PathBuf),
    Session(usize),
    Project(usize),
    Snippet(String),
    Command(String),
    Line(u32),
}

impl PickerState {
    pub fn new(kind: PickerKind) -> Self {
        Self {
            kind,
            query: String::new(),
            items: Vec::new(),
            selected_index: 0,
        }
    }

    pub fn filter(&mut self, query: &str) {
        self.query = query.to_string();
        // Fuzzy filtering is applied when items are populated.
        self.selected_index = 0;
    }

    pub fn select_next(&mut self) {
        if self.selected_index + 1 < self.items.len() {
            self.selected_index += 1;
        }
    }

    pub fn select_prev(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    pub fn selected_item(&self) -> Option<&PickerItem> {
        self.items.get(self.selected_index)
    }

    pub fn view(&self) -> Element<'_, Message> {
        let title = match &self.kind {
            PickerKind::File => "Open File",
            PickerKind::Session => "Switch Session",
            PickerKind::Project => "Project Actions",
            PickerKind::Snippet => "Insert Snippet",
            PickerKind::Command => "Command Palette",
            PickerKind::LineSearch => "Go to Line",
        };

        let input = text_input(title, &self.query)
            .on_input(Message::PickerQueryChanged)
            .size(14);

        let items: Vec<Element<Message>> = self
            .items
            .iter()
            .enumerate()
            .take(20)
            .map(|(i, item)| {
                let style = if i == self.selected_index {
                    text(&item.label).size(13)
                } else {
                    text(&item.label).size(13)
                };
                style.into()
            })
            .collect();

        container(
            column![input, column(items).spacing(2)]
                .spacing(8)
                .width(Length::Fill),
        )
        .padding(12)
        .max_width(500)
        .into()
    }
}
