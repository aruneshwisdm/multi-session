use iced::keyboard;

use super::{Message, PaneLayoutKind};
use crate::views::pane::PaneContentKind;
use crate::views::picker::PickerKind;

/// Convert an iced keyboard event into a Message, if it matches a keybinding.
pub fn handle_key_event(event: &keyboard::Event) -> Option<Message> {
    match event {
        keyboard::Event::KeyPressed {
            key,
            modifiers,
            ..
        } => handle_key_press(key, modifiers),
        _ => None,
    }
}

fn handle_key_press(
    key: &keyboard::Key,
    modifiers: &keyboard::Modifiers,
) -> Option<Message> {
    let ctrl = modifiers.control();
    let shift = modifiers.shift();

    if !ctrl {
        return None;
    }

    match key.as_ref() {
        // Terminal clipboard (Ctrl+Shift+C/V)
        keyboard::Key::Character("c") if shift => {
            return Some(Message::TerminalCopy);
        }
        keyboard::Key::Character("v") if shift => {
            return Some(Message::TerminalPaste);
        }

        // Session switching: Ctrl+1..9
        keyboard::Key::Character("1") => {
            Some(Message::SwitchSession(0))
        }
        keyboard::Key::Character("2") => {
            Some(Message::SwitchSession(1))
        }
        keyboard::Key::Character("3") => {
            Some(Message::SwitchSession(2))
        }
        keyboard::Key::Character("4") => {
            Some(Message::SwitchSession(3))
        }
        keyboard::Key::Character("5") => {
            Some(Message::SwitchSession(4))
        }
        keyboard::Key::Character("6") => {
            Some(Message::SwitchSession(5))
        }
        keyboard::Key::Character("7") => {
            Some(Message::SwitchSession(6))
        }
        keyboard::Key::Character("8") => {
            Some(Message::SwitchSession(7))
        }
        keyboard::Key::Character("9") => {
            Some(Message::SwitchSession(8))
        }

        // Session management
        keyboard::Key::Character("t") => Some(Message::NewSession),
        keyboard::Key::Character("w") => {
            Some(Message::CloseActiveSession)
        }

        // Problems
        keyboard::Key::Character(";") if !shift => {
            Some(Message::NextProblem)
        }

        // Pickers
        keyboard::Key::Character("p") if shift => {
            Some(Message::OpenPicker(PickerKind::Command))
        }
        keyboard::Key::Character("p") => {
            Some(Message::OpenPicker(PickerKind::File))
        }
        keyboard::Key::Character("k") => {
            Some(Message::OpenPicker(PickerKind::Snippet))
        }
        keyboard::Key::Character("o") => {
            Some(Message::OpenPicker(PickerKind::Project))
        }

        // Pane views
        keyboard::Key::Character("d") => {
            Some(Message::ShowPane(PaneContentKind::GitDiff))
        }

        // Layout
        keyboard::Key::Character("j") => {
            Some(Message::SetLayout(PaneLayoutKind::Three))
        }

        // Focus
        keyboard::Key::Character("/") => Some(Message::CyclePane),

        // Help
        keyboard::Key::Character("?") => {
            Some(Message::ToggleKeybindingHelp)
        }

        // Save
        keyboard::Key::Character("s") => Some(Message::SaveFile),

        // Quit
        keyboard::Key::Character("q") => Some(Message::RequestQuit),

        // Find
        keyboard::Key::Character("l") => {
            Some(Message::OpenPicker(PickerKind::LineSearch))
        }

        _ => None,
    }
}
