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

        // Diff review toggle
        keyboard::Key::Character("r") if !shift => {
            Some(Message::DiffReviewed)
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

#[cfg(test)]
mod tests {
    use super::*;

    fn key_event(ch: &str, ctrl: bool, shift: bool) -> keyboard::Event {
        let mut modifiers = keyboard::Modifiers::empty();
        if ctrl {
            modifiers = modifiers | keyboard::Modifiers::CTRL;
        }
        if shift {
            modifiers = modifiers | keyboard::Modifiers::SHIFT;
        }
        keyboard::Event::KeyPressed {
            key: keyboard::Key::Character(ch.into()),
            modified_key: keyboard::Key::Character(ch.into()),
            physical_key: keyboard::key::Physical::Unidentified(
                keyboard::key::NativeCode::Unidentified,
            ),
            location: keyboard::Location::Standard,
            modifiers,
            text: None,
        }
    }

    #[test]
    fn no_ctrl_returns_none() {
        let event = key_event("t", false, false);
        assert!(handle_key_event(&event).is_none());
    }

    #[test]
    fn ctrl_t_creates_new_session() {
        let event = key_event("t", true, false);
        assert!(matches!(handle_key_event(&event), Some(Message::NewSession)));
    }

    #[test]
    fn ctrl_w_closes_active_session() {
        let event = key_event("w", true, false);
        assert!(matches!(handle_key_event(&event), Some(Message::CloseActiveSession)));
    }

    #[test]
    fn ctrl_1_through_9_switch_sessions() {
        for (ch, expected_id) in [
            ("1", 0), ("2", 1), ("3", 2), ("4", 3), ("5", 4),
            ("6", 5), ("7", 6), ("8", 7), ("9", 8),
        ] {
            let event = key_event(ch, true, false);
            match handle_key_event(&event) {
                Some(Message::SwitchSession(id)) => assert_eq!(id, expected_id, "Ctrl+{ch}"),
                other => panic!("Ctrl+{ch}: expected SwitchSession({expected_id}), got {other:?}"),
            }
        }
    }

    #[test]
    fn ctrl_shift_c_terminal_copy() {
        let event = key_event("c", true, true);
        assert!(matches!(handle_key_event(&event), Some(Message::TerminalCopy)));
    }

    #[test]
    fn ctrl_shift_v_terminal_paste() {
        let event = key_event("v", true, true);
        assert!(matches!(handle_key_event(&event), Some(Message::TerminalPaste)));
    }

    #[test]
    fn ctrl_p_opens_file_picker() {
        let event = key_event("p", true, false);
        assert!(matches!(
            handle_key_event(&event),
            Some(Message::OpenPicker(PickerKind::File))
        ));
    }

    #[test]
    fn ctrl_shift_p_opens_command_palette() {
        let event = key_event("p", true, true);
        assert!(matches!(
            handle_key_event(&event),
            Some(Message::OpenPicker(PickerKind::Command))
        ));
    }

    #[test]
    fn ctrl_k_opens_snippet_picker() {
        let event = key_event("k", true, false);
        assert!(matches!(
            handle_key_event(&event),
            Some(Message::OpenPicker(PickerKind::Snippet))
        ));
    }

    #[test]
    fn ctrl_o_opens_project_picker() {
        let event = key_event("o", true, false);
        assert!(matches!(
            handle_key_event(&event),
            Some(Message::OpenPicker(PickerKind::Project))
        ));
    }

    #[test]
    fn ctrl_l_opens_line_search() {
        let event = key_event("l", true, false);
        assert!(matches!(
            handle_key_event(&event),
            Some(Message::OpenPicker(PickerKind::LineSearch))
        ));
    }

    #[test]
    fn ctrl_d_shows_git_diff() {
        let event = key_event("d", true, false);
        assert!(matches!(
            handle_key_event(&event),
            Some(Message::ShowPane(PaneContentKind::GitDiff))
        ));
    }

    #[test]
    fn ctrl_j_sets_three_pane_layout() {
        let event = key_event("j", true, false);
        assert!(matches!(
            handle_key_event(&event),
            Some(Message::SetLayout(PaneLayoutKind::Three))
        ));
    }

    #[test]
    fn ctrl_slash_cycles_pane() {
        let event = key_event("/", true, false);
        assert!(matches!(handle_key_event(&event), Some(Message::CyclePane)));
    }

    #[test]
    fn ctrl_s_saves_file() {
        let event = key_event("s", true, false);
        assert!(matches!(handle_key_event(&event), Some(Message::SaveFile)));
    }

    #[test]
    fn ctrl_q_quits() {
        let event = key_event("q", true, false);
        assert!(matches!(handle_key_event(&event), Some(Message::RequestQuit)));
    }

    #[test]
    fn ctrl_question_toggles_help() {
        let event = key_event("?", true, false);
        assert!(matches!(handle_key_event(&event), Some(Message::ToggleKeybindingHelp)));
    }

    #[test]
    fn ctrl_r_toggles_diff_reviewed() {
        let event = key_event("r", true, false);
        assert!(matches!(handle_key_event(&event), Some(Message::DiffReviewed)));
    }

    #[test]
    fn ctrl_semicolon_next_problem() {
        let event = key_event(";", true, false);
        assert!(matches!(handle_key_event(&event), Some(Message::NextProblem)));
    }

    #[test]
    fn unbound_ctrl_key_returns_none() {
        let event = key_event("z", true, false);
        assert!(handle_key_event(&event).is_none());
    }

    #[test]
    fn key_released_returns_none() {
        let event = keyboard::Event::KeyReleased {
            key: keyboard::Key::Character("t".into()),
            location: keyboard::Location::Standard,
            modifiers: keyboard::Modifiers::CTRL,
        };
        assert!(handle_key_event(&event).is_none());
    }
}
