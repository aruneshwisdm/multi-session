use iced::keyboard;
use jc_terminal::{Keystroke, Modifiers};

pub fn iced_event_to_keystroke(event: &keyboard::Event) -> Option<Keystroke> {
    match event {
        keyboard::Event::KeyPressed {
            key, modifiers, ..
        } => {
            let (key_name, key_char) = match key.as_ref() {
                keyboard::Key::Named(n) => {
                    use keyboard::key::Named;
                    let name = match n {
                        Named::Enter => "enter",
                        Named::Escape => "escape",
                        Named::Tab => "tab",
                        Named::Backspace => "backspace",
                        Named::Delete => "delete",
                        Named::ArrowUp => "up",
                        Named::ArrowDown => "down",
                        Named::ArrowLeft => "left",
                        Named::ArrowRight => "right",
                        Named::Home => "home",
                        Named::End => "end",
                        Named::PageUp => "pageup",
                        Named::PageDown => "pagedown",
                        Named::Insert => "insert",
                        Named::Space => "space",
                        Named::F1 => "f1",
                        Named::F2 => "f2",
                        Named::F3 => "f3",
                        Named::F4 => "f4",
                        Named::F5 => "f5",
                        Named::F6 => "f6",
                        Named::F7 => "f7",
                        Named::F8 => "f8",
                        Named::F9 => "f9",
                        Named::F10 => "f10",
                        Named::F11 => "f11",
                        Named::F12 => "f12",
                        _ => return None,
                    };
                    (name.to_string(), None)
                }
                keyboard::Key::Character(c) => {
                    let c_str: &str = c;
                    (
                        c_str.to_lowercase(),
                        Some(c_str.to_string()),
                    )
                }
                _ => return None,
            };

            Some(Keystroke {
                key: key_name,
                key_char,
                modifiers: Modifiers {
                    control: modifiers.control(),
                    alt: modifiers.alt(),
                    shift: modifiers.shift(),
                },
            })
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_event(
        key: keyboard::Key,
        ctrl: bool,
        alt: bool,
        shift: bool,
    ) -> keyboard::Event {
        let mut modifiers = keyboard::Modifiers::empty();
        if ctrl {
            modifiers = modifiers | keyboard::Modifiers::CTRL;
        }
        if alt {
            modifiers = modifiers | keyboard::Modifiers::ALT;
        }
        if shift {
            modifiers = modifiers | keyboard::Modifiers::SHIFT;
        }
        keyboard::Event::KeyPressed {
            key: key.clone(),
            modified_key: key,
            physical_key: keyboard::key::Physical::Unidentified(
                keyboard::key::NativeCode::Unidentified,
            ),
            location: keyboard::Location::Standard,
            modifiers,
            text: None,
        }
    }

    #[test]
    fn character_key_maps_to_keystroke() {
        let event = make_event(
            keyboard::Key::Character("a".into()),
            false, false, false,
        );
        let ks = iced_event_to_keystroke(&event).unwrap();
        assert_eq!(ks.key, "a");
        assert_eq!(ks.key_char, Some("a".to_string()));
        assert!(!ks.modifiers.control);
        assert!(!ks.modifiers.alt);
        assert!(!ks.modifiers.shift);
    }

    #[test]
    fn character_with_modifiers() {
        let event = make_event(
            keyboard::Key::Character("c".into()),
            true, true, false,
        );
        let ks = iced_event_to_keystroke(&event).unwrap();
        assert!(ks.modifiers.control);
        assert!(ks.modifiers.alt);
        assert!(!ks.modifiers.shift);
    }

    #[test]
    fn named_enter() {
        let event = make_event(
            keyboard::Key::Named(keyboard::key::Named::Enter),
            false, false, false,
        );
        let ks = iced_event_to_keystroke(&event).unwrap();
        assert_eq!(ks.key, "enter");
        assert!(ks.key_char.is_none());
    }

    #[test]
    fn named_arrow_keys() {
        for (named, expected) in [
            (keyboard::key::Named::ArrowUp, "up"),
            (keyboard::key::Named::ArrowDown, "down"),
            (keyboard::key::Named::ArrowLeft, "left"),
            (keyboard::key::Named::ArrowRight, "right"),
        ] {
            let event = make_event(keyboard::Key::Named(named), false, false, false);
            let ks = iced_event_to_keystroke(&event).unwrap();
            assert_eq!(ks.key, expected);
        }
    }

    #[test]
    fn named_special_keys() {
        for (named, expected) in [
            (keyboard::key::Named::Escape, "escape"),
            (keyboard::key::Named::Tab, "tab"),
            (keyboard::key::Named::Backspace, "backspace"),
            (keyboard::key::Named::Delete, "delete"),
            (keyboard::key::Named::Home, "home"),
            (keyboard::key::Named::End, "end"),
            (keyboard::key::Named::PageUp, "pageup"),
            (keyboard::key::Named::PageDown, "pagedown"),
            (keyboard::key::Named::Insert, "insert"),
            (keyboard::key::Named::Space, "space"),
        ] {
            let event = make_event(keyboard::Key::Named(named), false, false, false);
            let ks = iced_event_to_keystroke(&event).unwrap();
            assert_eq!(ks.key, expected, "failed for {named:?}");
        }
    }

    #[test]
    fn function_keys() {
        for (named, expected) in [
            (keyboard::key::Named::F1, "f1"),
            (keyboard::key::Named::F6, "f6"),
            (keyboard::key::Named::F12, "f12"),
        ] {
            let event = make_event(keyboard::Key::Named(named), false, false, false);
            let ks = iced_event_to_keystroke(&event).unwrap();
            assert_eq!(ks.key, expected);
        }
    }

    #[test]
    fn unsupported_named_key_returns_none() {
        let event = make_event(
            keyboard::Key::Named(keyboard::key::Named::CapsLock),
            false, false, false,
        );
        assert!(iced_event_to_keystroke(&event).is_none());
    }

    #[test]
    fn key_released_returns_none() {
        let event = keyboard::Event::KeyReleased {
            key: keyboard::Key::Character("a".into()),
            location: keyboard::Location::Standard,
            modifiers: keyboard::Modifiers::empty(),
        };
        assert!(iced_event_to_keystroke(&event).is_none());
    }

    #[test]
    fn character_key_lowercased() {
        let event = make_event(
            keyboard::Key::Character("A".into()),
            false, false, true,
        );
        let ks = iced_event_to_keystroke(&event).unwrap();
        assert_eq!(ks.key, "a");
        assert_eq!(ks.key_char, Some("A".to_string()));
    }
}
