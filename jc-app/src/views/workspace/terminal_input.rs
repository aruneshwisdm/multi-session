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
