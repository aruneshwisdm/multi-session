# Terminal porting rules

## Keep these files unchanged from upstream
- `jc-terminal/src/pty.rs` — PtyHandle via portable-pty, already cross-platform
- `jc-terminal/src/terminal.rs` — TerminalState via alacritty_terminal, already cross-platform

## Terminal widget approach
1. Prefer `iced_term` for terminal rendering — it already integrates with `alacritty_terminal`
2. Only fall back to custom `iced::widget::canvas::Program` if `iced_term` cannot handle a specific requirement (e.g., custom selection rendering, specific cursor styles)
3. If building custom rendering, follow the upstream 3-pass pattern: backgrounds → selection/text → cursor

## PTY considerations
- `portable-pty` handles Unix PTY vs Windows ConPTY automatically
- Default shell: respect `$SHELL` on Linux, use `cmd.exe` or `powershell.exe` on Windows
- PTY resize must propagate: window resize → grid recalculate → `PtyHandle::resize()`
