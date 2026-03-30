# Platform code only in jc-platform

All platform-specific code (`#[cfg(target_os = "...")]`, OS-specific crate usage) must live in the `jc-platform` crate. The three platform subsystems are:

1. **Notifications** — `notify-rust` on Linux/Windows, `objc2` on macOS
2. **IPC** — Unix sockets on Linux/macOS, Named Pipes on Windows
3. **Signals** — `ctrlc` crate (cross-platform)

Rules:
- `jc-core` has zero `#[cfg]` blocks — ever
- `jc-terminal` and `jc-app` should not import platform-specific crates directly
- Use the traits defined in `jc-platform` (`Notifier`, `IpcTransport`) instead
- Platform deps go in `[target.'cfg(...)'.dependencies]` sections of `jc-platform/Cargo.toml`
