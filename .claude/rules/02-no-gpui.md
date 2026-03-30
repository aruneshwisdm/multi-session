# No GPUI dependencies

Never add `gpui`, `gpui-component`, or `gpui-component-assets` as dependencies anywhere. The entire purpose of this project is replacing GPUI with `iced`.

When porting code from upstream jc, replace:
- `gpui::*` types → `iced::*` equivalents
- `Entity<T>` → state in `JcApp` struct
- `Render` trait → `iced::Application::view()`
- `EventEmitter` → `Message` enum variants
- `cx.spawn()` → `iced::Subscription`
- `gpui::Hsla` → `iced::Color`
- `gpui::Keystroke` → `iced::keyboard::Key` + modifiers
