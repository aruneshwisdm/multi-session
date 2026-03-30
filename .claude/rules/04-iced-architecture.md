# iced Elm architecture enforcement

Follow iced's Elm pattern strictly. All state lives in the `JcApp` struct, all mutations go through `update()`.

## Rules

1. **State changes only in `update()`** — Never mutate application state from `view()`, `subscription()`, or callbacks. Return a `Message` instead.

2. **`view()` is a pure function** — It reads state and returns `Element`. No side effects, no I/O, no spawning threads.

3. **Async work via `Subscription`** — PTY reads, hook events, IPC connections, file watching, and signal handling all go through `iced::Subscription` returning `Message` variants.

4. **Side effects via `Command`** — File saves, clipboard writes, notification sends, and IPC sends return `iced::Command` from `update()`.

5. **Use the `Message` enum** — Every event type gets a variant. Don't use closures or callbacks that bypass the message loop.

6. **Keybindings via `subscription()`** — Handle `iced::keyboard::Event` in the subscription, map to `Message` variants. Map macOS `Cmd` to `Ctrl` on Windows/Linux.
