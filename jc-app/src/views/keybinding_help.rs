use iced::widget::{column, container, row, scrollable, text};
use iced::Element;

use super::workspace::Message;

pub struct KeybindingHelpState {
    pub visible: bool,
}

impl Default for KeybindingHelpState {
    fn default() -> Self {
        Self { visible: false }
    }
}

struct Binding {
    key: &'static str,
    description: &'static str,
}

const BINDINGS: &[Binding] = &[
    Binding { key: "Ctrl+1..9", description: "Switch to session 1-9" },
    Binding { key: "Ctrl+T", description: "New session" },
    Binding { key: "Ctrl+W", description: "Close session" },
    Binding { key: "Ctrl+;", description: "Next problem" },
    Binding { key: "Ctrl+P", description: "File picker" },
    Binding { key: "Ctrl+Shift+P", description: "Command palette" },
    Binding { key: "Ctrl+K", description: "Snippet picker" },
    Binding { key: "Ctrl+O", description: "Project picker" },
    Binding { key: "Ctrl+D", description: "Git diff" },
    Binding { key: "Ctrl+R", description: "Toggle diff reviewed" },
    Binding { key: "Ctrl+J", description: "Toggle pane layout" },
    Binding { key: "Ctrl+/", description: "Move focus between panes" },
    Binding { key: "Ctrl+?", description: "Show this help" },
    Binding { key: "Ctrl+Q", description: "Quit" },
    Binding { key: "Ctrl+N", description: "Rename session" },
    Binding { key: "Ctrl+L", description: "Go to line" },
    Binding { key: "Ctrl+F", description: "Find in view" },
    Binding { key: "Ctrl+S", description: "Save file" },
    Binding { key: "Ctrl+E", description: "Open in external editor" },
    Binding { key: "Ctrl+1", description: "Show Claude terminal" },
    Binding { key: "Ctrl+2", description: "Show general terminal" },
    Binding { key: "Ctrl+3", description: "Show code viewer" },
    Binding { key: "Ctrl+4", description: "Show TODO editor" },
    Binding { key: "Ctrl+5", description: "Show git diff" },
];

impl KeybindingHelpState {
    pub fn view(&self) -> Element<'_, Message> {
        let rows: Vec<Element<Message>> = BINDINGS
            .iter()
            .map(|b| {
                row![
                    text(b.key).size(13).width(iced::Length::Fixed(180.0)),
                    text(b.description).size(13),
                ]
                .spacing(16)
                .into()
            })
            .collect();

        container(
            column![
                text("Keybindings").size(18),
                scrollable(column(rows).spacing(4))
            ]
            .spacing(12),
        )
        .padding(16)
        .max_width(500)
        .into()
    }
}
