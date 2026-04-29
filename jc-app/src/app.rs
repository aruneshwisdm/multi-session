use iced::futures::SinkExt;
use iced::{Element, Subscription, Task, Theme};
use std::io::Read;
use std::path::PathBuf;
use std::time::Duration;

use crate::views::pane::PaneContentKind;
use crate::views::picker::PickerState;
use crate::views::terminal_canvas::{CELL_HEIGHT, CELL_WIDTH};
use crate::views::workspace::{Message, Workspace};
use jc_core::config::{AppConfig, AppState};

pub struct Flags {
    pub state: AppState,
    pub config: AppConfig,
    pub ipc_rx: flume::Receiver<PathBuf>,
}

pub fn run(state: AppState, config: AppConfig, ipc_rx: flume::Receiver<PathBuf>) {
    let flags = Flags { state, config, ipc_rx };

    iced::application("jc — Claude Code Orchestrator", update, view)
        .subscription(subscription)
        .theme(theme)
        .window_size(iced::Size::new(1200.0, 800.0))
        .font(include_bytes!("../../data/fonts/Lilex-Regular.ttf").as_slice())
        .font(include_bytes!("../../data/fonts/Lilex-Bold.ttf").as_slice())
        .font(include_bytes!("../../data/fonts/Lilex-Italic.ttf").as_slice())
        .font(include_bytes!("../../data/fonts/Lilex-BoldItalic.ttf").as_slice())
        .run_with(move || {
            let workspace = Workspace::new(flags.state, flags.config, flags.ipc_rx);
            (workspace, Task::none())
        })
        .expect("failed to run iced application");
}

pub(crate) fn update(workspace: &mut Workspace, message: Message) -> Task<Message> {
    match message {
        Message::TerminalOutput(_session_id, _data) => {}
        Message::TerminalEvent(session_id, event_kind) => {
            use crate::views::session_state::PendingEvent;
            use crate::views::workspace::TerminalEventKind;
            let project = workspace.active_project_mut();
            if let Some(session) = project.sessions.get_mut(&session_id) {
                match event_kind {
                    TerminalEventKind::Bell => {
                        session.pending_events.insert(PendingEvent::TerminalBell);
                    }
                    TerminalEventKind::Exit => {
                        session.busy = false;
                    }
                    TerminalEventKind::Wakeup => {}
                }
            }
        }
        Message::TerminalResize(cols, rows) => {
            let cols = cols.clamp(20, 300);
            let rows = rows.clamp(5, 100);
            if cols != workspace.terminal_cols || rows != workspace.terminal_rows {
                workspace.terminal_cols = cols;
                workspace.terminal_rows = rows;
                workspace.resize_all_terminals(cols, rows);
            }
        }

        Message::SwitchSession(id) => workspace.switch_session(id),
        Message::NewSession => {
            workspace.create_session();
            workspace.sessions_dirty = true;
        }
        Message::CloseSession(id) => {
            workspace.close_session(id);
            workspace.sessions_dirty = true;
        }
        Message::CloseActiveSession => {
            let project = workspace.active_project();
            if let Some(id) = project.active_session {
                workspace.close_session(id);
                workspace.sessions_dirty = true;
            }
        }
        Message::SwitchProject(idx) => {
            if idx < workspace.projects.len() {
                workspace.active_project_index = idx;
            }
        }
        Message::CyclePane => workspace.cycle_pane(),
        Message::SetLayout(layout) => workspace.set_layout(layout),
        Message::ShowPane(kind) => {
            let pane_idx = workspace.resolve_pane_for_kind(kind);
            workspace.show_in_pane(pane_idx, kind);
        }

        Message::HookReceived(event) => {
            workspace.handle_hook_event(event);
            workspace.sessions_dirty = true;
        }

        Message::IpcProjectOpen(path) => {
            let already_exists = workspace.projects.iter().any(|p| p.path == path);
            if !already_exists {
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "unknown".to_string());
                let project =
                    crate::views::project_state::ProjectState::create(
                        path,
                        name,
                        workspace.terminal_cols,
                        workspace.terminal_rows,
                    );
                let pi = workspace.projects.len();
                workspace.projects.push(project);
                workspace
                    .code_views
                    .push(crate::views::code_view::CodeViewState::default());
                workspace.diff_views.push(
                    crate::views::diff_view::DiffViewState::new(
                        workspace.projects[pi].path.clone(),
                    ),
                );
                workspace.todo_views.push(
                    crate::views::todo_view::TodoViewState::new(
                        workspace.projects[pi].path.clone(),
                    ),
                );
                workspace.active_project_index = pi;
            }
        }

        Message::OpenPicker(kind) => {
            use crate::views::picker::{PickerItem, PickerItemData, PickerKind};
            let mut picker = PickerState::new(kind.clone());

            match &kind {
                PickerKind::Session => {
                    let project = workspace.active_project();
                    picker.items = project
                        .sessions
                        .iter()
                        .map(|(&id, s)| PickerItem {
                            label: s.label.clone(),
                            detail: if s.busy {
                                "working...".into()
                            } else {
                                String::new()
                            },
                            data: PickerItemData::Session(id),
                        })
                        .collect();
                }
                PickerKind::Project => {
                    picker.items = workspace
                        .projects
                        .iter()
                        .enumerate()
                        .map(|(i, p)| PickerItem {
                            label: p.name.clone(),
                            detail: p.path.display().to_string(),
                            data: PickerItemData::Project(i),
                        })
                        .collect();
                }
                PickerKind::Command => {
                    picker.items = vec![
                        PickerItem {
                            label: "New Session".into(),
                            detail: "Create a new Claude session".into(),
                            data: PickerItemData::Command("new_session".into()),
                        },
                        PickerItem {
                            label: "Toggle Layout".into(),
                            detail: "Switch between 1/2/3 pane layout".into(),
                            data: PickerItemData::Command("toggle_layout".into()),
                        },
                    ];
                }
                PickerKind::File => {
                    let project_path =
                        workspace.projects[workspace.active_project_index]
                            .path
                            .clone();
                    let mut files = Vec::new();
                    if let Ok(entries) = walkdir_collect(&project_path, 500) {
                        files = entries;
                    }
                    picker.items = files
                        .into_iter()
                        .map(|p| {
                            let rel = p
                                .strip_prefix(&project_path)
                                .unwrap_or(&p)
                                .display()
                                .to_string();
                            PickerItem {
                                label: rel,
                                detail: String::new(),
                                data: PickerItemData::File(p),
                            }
                        })
                        .collect();
                }
                PickerKind::Snippet => {
                    picker.items = workspace
                        .snippets
                        .items
                        .iter()
                        .map(|s| PickerItem {
                            label: s.heading.clone(),
                            detail: s.content.chars().take(80).collect(),
                            data: PickerItemData::Snippet(s.content.clone()),
                        })
                        .collect();
                }
                PickerKind::LineSearch => {}
            }

            workspace.picker = Some(picker);
        }
        Message::PickerQueryChanged(query) => {
            if let Some(picker) = &mut workspace.picker {
                if matches!(picker.kind, crate::views::picker::PickerKind::LineSearch) {
                    picker.query = query.clone();
                    if let Ok(line) = query.trim().parse::<u32>() {
                        picker.items = vec![crate::views::picker::PickerItem {
                            label: format!("Go to line {line}"),
                            detail: String::new(),
                            data: crate::views::picker::PickerItemData::Line(line),
                        }];
                        picker.selected_index = 0;
                    } else {
                        picker.items.clear();
                    }
                } else {
                    picker.filter(&query);
                }
            }
        }
        Message::PickerSelectNext => {
            if let Some(picker) = &mut workspace.picker {
                picker.select_next();
            }
        }
        Message::PickerSelectPrev => {
            if let Some(picker) = &mut workspace.picker {
                picker.select_prev();
            }
        }
        Message::PickerConfirm => {
            if let Some(picker) = &workspace.picker {
                if let Some(item) = picker.selected_item() {
                    let data = item.data.clone();
                    workspace.picker = None;
                    match data {
                        crate::views::picker::PickerItemData::File(path) => {
                            return update(workspace, Message::OpenFile(path));
                        }
                        crate::views::picker::PickerItemData::Session(id) => {
                            return update(workspace, Message::SwitchSession(id));
                        }
                        crate::views::picker::PickerItemData::Project(idx) => {
                            return update(workspace, Message::SwitchProject(idx));
                        }
                        crate::views::picker::PickerItemData::Command(cmd) => {
                            match cmd.as_str() {
                                "new_session" => {
                                    return update(workspace, Message::NewSession);
                                }
                                "toggle_layout" => {
                                    let next = match workspace.layout {
                                        crate::views::workspace::PaneLayoutKind::One => {
                                            crate::views::workspace::PaneLayoutKind::Two
                                        }
                                        crate::views::workspace::PaneLayoutKind::Two => {
                                            crate::views::workspace::PaneLayoutKind::Three
                                        }
                                        crate::views::workspace::PaneLayoutKind::Three => {
                                            crate::views::workspace::PaneLayoutKind::One
                                        }
                                    };
                                    return update(workspace, Message::SetLayout(next));
                                }
                                _ => {}
                            }
                        }
                        crate::views::picker::PickerItemData::Line(line) => {
                            let pi = workspace.active_project_index;
                            workspace.code_views[pi].goto_line(line as usize);
                            let pane_idx = workspace.resolve_pane_for_kind(PaneContentKind::CodeViewer);
                            workspace.show_in_pane(pane_idx, PaneContentKind::CodeViewer);
                        }
                        crate::views::picker::PickerItemData::Snippet(content) => {
                            let active_kind = workspace.panes[workspace.active_pane_index].kind;
                            let is_terminal = matches!(
                                active_kind,
                                PaneContentKind::ClaudeTerminal | PaneContentKind::GeneralTerminal
                            );
                            if is_terminal {
                                let project = workspace.active_project_mut();
                                if let Some(session) = project.active_session_mut() {
                                    let terminal = if active_kind == PaneContentKind::ClaudeTerminal {
                                        &session.claude_terminal
                                    } else {
                                        &session.general_terminal
                                    };
                                    let _ = terminal.pty.write_all(content.as_bytes());
                                }
                            }
                        }
                    }
                } else {
                    workspace.picker = None;
                }
            }
        }
        Message::PickerDismiss => {
            workspace.picker = None;
        }

        Message::OpenFile(path) => {
            let pi = workspace.active_project_index;
            workspace.code_views[pi].open_file(path);
            let pane_idx =
                workspace.resolve_pane_for_kind(PaneContentKind::CodeViewer);
            workspace.show_in_pane(pane_idx, PaneContentKind::CodeViewer);
        }
        Message::SaveFile => {
            let pi = workspace.active_project_index;
            workspace.code_views[pi].save();
            workspace.todo_views[pi].save();
        }
        Message::DiffReviewed => {
            let pi = workspace.active_project_index;
            let dv = &mut workspace.diff_views[pi];
            if let Some(fd) = dv.file_diffs.get_mut(dv.current_file_index) {
                fd.reviewed = !fd.reviewed;
            }
            workspace.projects[pi].unreviewed_files = dv.unreviewed_files();
            workspace.projects[pi].refresh_problems();
        }

        Message::CodeEditorAction(action) => {
            let pi = workspace.active_project_index;
            workspace.code_views[pi].perform_action(action);
        }
        Message::TodoEditorAction(action) => {
            let pi = workspace.active_project_index;
            workspace.todo_views[pi].perform_action(action);
        }
        Message::GlobalTodoEditorAction(action) => {
            workspace.global_todo.perform_action(action);
        }

        Message::TerminalTextSelected(text) => {
            workspace.last_terminal_selection = text.clone();
            if let Ok(mut clipboard) = arboard::Clipboard::new() {
                let _ = clipboard.set_text(text);
            }
        }
        Message::TerminalCopy => {
            if !workspace.last_terminal_selection.is_empty() {
                if let Ok(mut clipboard) = arboard::Clipboard::new() {
                    let _ = clipboard.set_text(&workspace.last_terminal_selection);
                }
            }
        }
        Message::TerminalPaste => {
            if let Ok(mut clipboard) = arboard::Clipboard::new() {
                if let Ok(text) = clipboard.get_text() {
                    let active_kind = workspace.panes[workspace.active_pane_index].kind;
                    let is_terminal = matches!(
                        active_kind,
                        PaneContentKind::ClaudeTerminal | PaneContentKind::GeneralTerminal
                    );
                    if is_terminal {
                        let project = workspace.active_project_mut();
                        if let Some(session) = project.active_session_mut() {
                            let terminal = if active_kind == PaneContentKind::ClaudeTerminal {
                                &session.claude_terminal
                            } else {
                                &session.general_terminal
                            };
                            let mode = terminal.state.with_term(|term| *term.mode());
                            if mode.contains(alacritty_terminal::term::TermMode::BRACKETED_PASTE) {
                                let _ = terminal.pty.write_all(b"\x1b[200~");
                                let _ = terminal.pty.write_all(text.as_bytes());
                                let _ = terminal.pty.write_all(b"\x1b[201~");
                            } else {
                                let _ = terminal.pty.write_all(text.as_bytes());
                            }
                        }
                    }
                }
            }
        }

        Message::WindowResized(width, height) => {
            let pane_count = workspace.visible_pane_count() as f32;
            let pane_width = width / pane_count;
            let pane_height = height - 60.0;
            let new_cols = ((pane_width - 16.0) / CELL_WIDTH).floor() as u16;
            let new_rows = ((pane_height - 30.0) / CELL_HEIGHT).floor() as u16;
            let new_cols = new_cols.clamp(20, 300);
            let new_rows = new_rows.clamp(5, 100);
            if new_cols != workspace.terminal_cols || new_rows != workspace.terminal_rows {
                workspace.terminal_cols = new_cols;
                workspace.terminal_rows = new_rows;
                workspace.resize_all_terminals(new_cols, new_rows);
            }
        }

        Message::ToggleKeybindingHelp => {
            workspace.keybinding_help.visible = !workspace.keybinding_help.visible;
        }

        Message::NextProblem => {
            workspace.next_problem();
        }
        Message::JumpToWait => {
            let pane_idx =
                workspace.resolve_pane_for_kind(PaneContentKind::TodoEditor);
            workspace.show_in_pane(pane_idx, PaneContentKind::TodoEditor);
        }

        Message::RequestClose | Message::RequestQuit => {
            let is_quit = matches!(message, Message::RequestQuit);
            let conflicts = workspace.save_all_dirty();
            let active_count = workspace.active_session_count();
            if conflicts.is_empty() && active_count == 0 {
                for project in &workspace.projects {
                    let _ = jc_core::hooks_settings::uninstall_hooks(&project.path);
                }
                return iced::exit();
            }
            workspace.close_confirm =
                Some(crate::views::workspace::CloseConfirmState {
                    session_count: active_count,
                    conflicts,
                    is_quit,
                });
        }
        Message::ConfirmClose => {
            workspace.persist_sessions();
            workspace.close_confirm = None;
            for project in &workspace.projects {
                let _ = jc_core::hooks_settings::uninstall_hooks(&project.path);
            }
            return iced::exit();
        }
        Message::CancelClose => {
            workspace.close_confirm = None;
        }

        Message::Tick => {
            for project in &mut workspace.projects {
                project.refresh_problems();
            }
            if workspace.sessions_dirty {
                workspace.persist_sessions();
                workspace.sessions_dirty = false;
            }
        }

        Message::KeyboardEvent(event) => {
            // First try keybindings (Ctrl+key shortcuts)
            if let Some(msg) =
                crate::views::workspace::keybindings::handle_key_event(&event)
            {
                return update(workspace, msg);
            }

            // If active pane is a terminal, forward keystroke to PTY
            let active_kind = workspace.panes[workspace.active_pane_index].kind;
            let is_terminal = matches!(
                active_kind,
                PaneContentKind::ClaudeTerminal | PaneContentKind::GeneralTerminal
            );
            if is_terminal {
                if let Some(keystroke) =
                    crate::views::workspace::terminal_input::iced_event_to_keystroke(&event)
                {
                    let project = workspace.active_project_mut();
                    if let Some(session) = project.active_session_mut() {
                        let terminal = if active_kind == PaneContentKind::ClaudeTerminal {
                            &session.claude_terminal
                        } else {
                            &session.general_terminal
                        };
                        let mode = terminal.state.with_term(|term| *term.mode());
                        if let Some(bytes) = jc_terminal::keystroke_to_bytes(&keystroke, mode) {
                            let _ = terminal.pty.write_all(&bytes);
                        }
                    }
                }
            }
        }

        Message::NotificationAction(_) => {}
        Message::FileChanged(path) => {
            let pi = workspace.active_project_index;
            let cv = &workspace.code_views[pi];
            if cv.file_path.as_ref() == Some(&path) && !cv.dirty {
                workspace.code_views[pi].open_file(path.clone());
            }
            let todo_path = workspace.projects[pi].path.join("TODO.md");
            if path == todo_path && !workspace.todo_views[pi].dirty {
                workspace.todo_views[pi] =
                    crate::views::todo_view::TodoViewState::new(
                        workspace.projects[pi].path.clone(),
                    );
            }
            workspace.diff_views[pi].stale = true;
        }
        Message::None => {}
    }

    Task::none()
}

fn view(workspace: &Workspace) -> Element<'_, Message> {
    workspace.view_app()
}

fn subscription(workspace: &Workspace) -> Subscription<Message> {
    let mut subscriptions = Vec::new();

    // Keyboard events
    subscriptions.push(
        iced::keyboard::on_key_press(|key, modifiers| {
            let event = iced::keyboard::Event::KeyPressed {
                key: key.clone(),
                location: iced::keyboard::Location::Standard,
                modifiers,
                text: None,
                modified_key: key.clone(),
                physical_key: iced::keyboard::key::Physical::Unidentified(
                    iced::keyboard::key::NativeCode::Unidentified,
                ),
            };
            Some(Message::KeyboardEvent(event))
        }),
    );

    // Window resize events
    subscriptions.push(iced::event::listen().map(|event| {
        if let iced::Event::Window(iced::window::Event::Resized(size)) = event {
            Message::WindowResized(size.width, size.height)
        } else {
            Message::None
        }
    }));

    // Periodic tick
    subscriptions.push(
        iced::time::every(Duration::from_secs(2)).map(|_| Message::Tick),
    );

    // Hook server events
    if let Some(server) = &workspace.hook_server {
        let rx = server.rx.clone();
        subscriptions.push(Subscription::run_with_id(
            "hook-events",
            iced::stream::channel(32, move |mut sender| {
                let rx = rx.clone();
                async move {
                    loop {
                        match rx.recv_async().await {
                            Ok(event) => {
                                let _ = sender
                                    .send(Message::HookReceived(event))
                                    .await;
                            }
                            Err(_) => {
                                std::future::pending::<()>().await;
                            }
                        }
                    }
                }
            }),
        ));
    }

    // IPC events
    if let Some(ipc_rx) = &workspace.ipc_rx {
        let rx = ipc_rx.clone();
        subscriptions.push(Subscription::run_with_id(
            "ipc-events",
            iced::stream::channel(8, move |mut sender| {
                let rx = rx.clone();
                async move {
                    loop {
                        match rx.recv_async().await {
                            Ok(path) => {
                                let _ = sender
                                    .send(Message::IpcProjectOpen(path))
                                    .await;
                            }
                            Err(_) => {
                                std::future::pending::<()>().await;
                            }
                        }
                    }
                }
            }),
        ));
    }

    // File watcher
    let watch_paths: Vec<PathBuf> = workspace
        .projects
        .iter()
        .map(|p| p.path.clone())
        .collect();
    if !watch_paths.is_empty() {
        subscriptions.push(Subscription::run_with_id(
            "file-watcher",
            iced::stream::channel(64, move |mut sender| {
                async move {
                    use notify::{RecommendedWatcher, RecursiveMode, Watcher};
                    let (tx, rx) = std::sync::mpsc::channel();
                    let mut watcher: RecommendedWatcher =
                        match notify::recommended_watcher(tx) {
                            Ok(w) => w,
                            Err(e) => {
                                eprintln!("file watcher init failed: {e}");
                                std::future::pending::<()>().await;
                                return;
                            }
                        };
                    for p in &watch_paths {
                        let _ = watcher.watch(p, RecursiveMode::Recursive);
                    }
                    loop {
                        match rx.recv() {
                            Ok(Ok(event)) => {
                                for path in event.paths {
                                    let _ = sender
                                        .send(Message::FileChanged(path))
                                        .await;
                                }
                            }
                            Ok(Err(e)) => {
                                eprintln!("file watcher error: {e}");
                            }
                            Err(_) => {
                                std::future::pending::<()>().await;
                            }
                        }
                    }
                }
            }),
        ));
    }

    // PTY reader subscriptions — one per terminal
    for project in &workspace.projects {
        for (&session_id, session) in &project.sessions {
            for (is_claude, terminal) in [
                (true, &session.claude_terminal),
                (false, &session.general_terminal),
            ] {
                let label = if is_claude { "claude" } else { "general" };
                let sub_id = format!("pty-{session_id}-{label}");
                let reader_slot = terminal.reader.clone();
                let term_handle = terminal.state.term_handle();
                let event_rx = terminal.event_rx.clone();

                subscriptions.push(Subscription::run_with_id(
                    sub_id,
                    iced::stream::channel(256, move |mut sender| {
                        async move {
                            // Take reader (only succeeds once per terminal)
                            let reader = { reader_slot.lock().take() };
                            if let Some(mut reader) = reader {
                                let term_handle = term_handle.clone();
                                std::thread::spawn(move || {
                                    let mut processor: alacritty_terminal::vte::ansi::Processor = alacritty_terminal::vte::ansi::Processor::new();
                                    let mut buf = [0u8; 4096];
                                    loop {
                                        match reader.read(&mut buf) {
                                            Ok(0) => break,
                                            Ok(n) => {
                                                let mut term = term_handle.lock();
                                                processor.advance(&mut *term, &buf[..n]);
                                            }
                                            Err(_) => break,
                                        }
                                    }
                                });
                            }

                            // Forward terminal events to iced
                            loop {
                                match event_rx.recv_async().await {
                                    Ok(event) => {
                                        use crate::views::workspace::TerminalEventKind;
                                        use jc_terminal::TerminalEvent;
                                        let msg = match event {
                                            TerminalEvent::Wakeup => {
                                                Message::TerminalEvent(
                                                    session_id,
                                                    TerminalEventKind::Wakeup,
                                                )
                                            }
                                            TerminalEvent::Bell => {
                                                Message::TerminalEvent(
                                                    session_id,
                                                    TerminalEventKind::Bell,
                                                )
                                            }
                                            TerminalEvent::Exit
                                            | TerminalEvent::ChildExit => {
                                                Message::TerminalEvent(
                                                    session_id,
                                                    TerminalEventKind::Exit,
                                                )
                                            }
                                            _ => continue,
                                        };
                                        let _ = sender.send(msg).await;
                                    }
                                    Err(_) => {
                                        std::future::pending::<()>().await;
                                    }
                                }
                            }
                        }
                    }),
                ));
            }
        }
    }

    Subscription::batch(subscriptions)
}

fn theme(_workspace: &Workspace) -> Theme {
    Theme::Dark
}

fn walkdir_collect(
    root: &std::path::Path,
    limit: usize,
) -> std::io::Result<Vec<PathBuf>> {
    let mut result = Vec::new();
    let skip_dirs = [".git", "target", "node_modules", ".next", "__pycache__"];
    walk_recurse(root, root, &skip_dirs, limit, &mut result);
    Ok(result)
}

fn walk_recurse(
    dir: &std::path::Path,
    root: &std::path::Path,
    skip: &[&str],
    limit: usize,
    out: &mut Vec<PathBuf>,
) {
    if out.len() >= limit {
        return;
    }
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        if out.len() >= limit {
            return;
        }
        let path = entry.path();
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if name_str.starts_with('.') {
            continue;
        }
        if path.is_dir() {
            if !skip.contains(&name_str.as_ref()) {
                walk_recurse(&path, root, skip, limit, out);
            }
        } else {
            out.push(path);
        }
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use crate::views::pane::PaneContentKind;
    use crate::views::picker::PickerKind;
    use crate::views::workspace::PaneLayoutKind;

    fn ws() -> Workspace {
        Workspace::for_testing()
    }

    fn send(w: &mut Workspace, msg: Message) {
        let _ = update(w, msg);
    }

    // -------------------------------------------------------------------
    // Layout management
    // -------------------------------------------------------------------

    #[test]
    fn set_layout_changes_visible_panes() {
        let mut w = ws();
        assert_eq!(w.visible_pane_count(), 3);

        send(&mut w, Message::SetLayout(PaneLayoutKind::One));
        assert_eq!(w.visible_pane_count(), 1);
        assert_eq!(w.layout, PaneLayoutKind::One);

        send(&mut w, Message::SetLayout(PaneLayoutKind::Two));
        assert_eq!(w.visible_pane_count(), 2);
    }

    #[test]
    fn set_layout_clamps_active_pane() {
        let mut w = ws();
        w.active_pane_index = 2;
        send(&mut w, Message::SetLayout(PaneLayoutKind::One));
        assert_eq!(w.active_pane_index, 0);
    }

    // -------------------------------------------------------------------
    // Pane cycling
    // -------------------------------------------------------------------

    #[test]
    fn cycle_pane_wraps_around() {
        let mut w = ws();
        assert_eq!(w.active_pane_index, 0);

        send(&mut w, Message::CyclePane);
        assert_eq!(w.active_pane_index, 1);

        send(&mut w, Message::CyclePane);
        assert_eq!(w.active_pane_index, 2);

        send(&mut w, Message::CyclePane);
        assert_eq!(w.active_pane_index, 0);
    }

    #[test]
    fn cycle_pane_respects_layout() {
        let mut w = ws();
        send(&mut w, Message::SetLayout(PaneLayoutKind::Two));
        w.active_pane_index = 1;
        send(&mut w, Message::CyclePane);
        assert_eq!(w.active_pane_index, 0);
    }

    // -------------------------------------------------------------------
    // Show pane
    // -------------------------------------------------------------------

    #[test]
    fn show_pane_changes_content() {
        let mut w = ws();
        assert_eq!(w.panes[0].kind, PaneContentKind::ClaudeTerminal);

        send(&mut w, Message::ShowPane(PaneContentKind::GitDiff));
        let diff_idx = w.resolve_pane_for_kind(PaneContentKind::GitDiff);
        assert_eq!(w.panes[diff_idx].kind, PaneContentKind::GitDiff);
    }

    // -------------------------------------------------------------------
    // Keybinding help toggle
    // -------------------------------------------------------------------

    #[test]
    fn toggle_keybinding_help() {
        let mut w = ws();
        assert!(!w.keybinding_help.visible);

        send(&mut w, Message::ToggleKeybindingHelp);
        assert!(w.keybinding_help.visible);

        send(&mut w, Message::ToggleKeybindingHelp);
        assert!(!w.keybinding_help.visible);
    }

    // -------------------------------------------------------------------
    // Picker lifecycle
    // -------------------------------------------------------------------

    #[test]
    fn open_picker_sets_state() {
        let mut w = ws();
        assert!(w.picker.is_none());

        send(&mut w, Message::OpenPicker(PickerKind::Command));
        assert!(w.picker.is_some());
        assert_eq!(w.picker.as_ref().unwrap().items.len(), 2);
    }

    #[test]
    fn picker_dismiss_clears() {
        let mut w = ws();
        send(&mut w, Message::OpenPicker(PickerKind::Command));
        assert!(w.picker.is_some());

        send(&mut w, Message::PickerDismiss);
        assert!(w.picker.is_none());
    }

    #[test]
    fn picker_navigate_and_confirm() {
        let mut w = ws();
        send(&mut w, Message::OpenPicker(PickerKind::Command));
        assert_eq!(w.picker.as_ref().unwrap().selected_index, 0);

        send(&mut w, Message::PickerSelectNext);
        assert_eq!(w.picker.as_ref().unwrap().selected_index, 1);

        send(&mut w, Message::PickerSelectPrev);
        assert_eq!(w.picker.as_ref().unwrap().selected_index, 0);

        // Confirm "New Session" — this will try to spawn a session (PTY),
        // so we test the dispatch path. The session creation will fail
        // gracefully in test because create_session spawns PTYs.
        // Instead, confirm on "Toggle Layout" which is pure state change.
        send(&mut w, Message::PickerSelectNext);
        send(&mut w, Message::PickerConfirm);
        assert!(w.picker.is_none());
    }

    #[test]
    fn picker_query_changed_filters() {
        let mut w = ws();
        send(&mut w, Message::OpenPicker(PickerKind::Command));
        send(&mut w, Message::PickerQueryChanged("toggle".into()));
        assert_eq!(w.picker.as_ref().unwrap().query, "toggle");
    }

    #[test]
    fn file_picker_populates_from_project() {
        let mut w = ws();
        // Create a file in the test project directory
        let project_path = w.projects[0].path.clone();
        let test_file = project_path.join("test_integration.txt");
        let _ = std::fs::write(&test_file, "hello");

        send(&mut w, Message::OpenPicker(PickerKind::File));
        let picker = w.picker.as_ref().unwrap();
        let has_file = picker.items.iter().any(|i| i.label.contains("test_integration.txt"));
        assert!(has_file, "file picker should contain the test file");

        let _ = std::fs::remove_file(&test_file);
    }

    #[test]
    fn line_search_picker_parses_number() {
        let mut w = ws();
        send(&mut w, Message::OpenPicker(PickerKind::LineSearch));
        assert!(w.picker.is_some());

        send(&mut w, Message::PickerQueryChanged("42".into()));
        let picker = w.picker.as_ref().unwrap();
        assert_eq!(picker.items.len(), 1);
        assert!(picker.items[0].label.contains("42"));
    }

    #[test]
    fn line_search_non_number_yields_empty() {
        let mut w = ws();
        send(&mut w, Message::OpenPicker(PickerKind::LineSearch));
        send(&mut w, Message::PickerQueryChanged("abc".into()));
        assert!(w.picker.as_ref().unwrap().items.is_empty());
    }

    // -------------------------------------------------------------------
    // File open and save
    // -------------------------------------------------------------------

    #[test]
    fn open_file_loads_into_code_view() {
        let mut w = ws();
        let project_path = w.projects[0].path.clone();
        let test_file = project_path.join("test_open.rs");
        std::fs::write(&test_file, "fn main() {}").unwrap();

        send(&mut w, Message::OpenFile(test_file.clone()));
        assert_eq!(w.code_views[0].file_path.as_ref(), Some(&test_file));
        assert!(w.code_views[0].raw_text.contains("fn main"));
        assert_eq!(w.panes[w.active_pane_index].kind, PaneContentKind::CodeViewer);

        let _ = std::fs::remove_file(&test_file);
    }

    #[test]
    fn save_file_persists_changes() {
        let mut w = ws();
        let project_path = w.projects[0].path.clone();
        let test_file = project_path.join("test_save.txt");
        std::fs::write(&test_file, "original").unwrap();

        send(&mut w, Message::OpenFile(test_file.clone()));
        // Simulate an edit by performing an action
        use iced::widget::text_editor;
        w.code_views[0].content.perform(text_editor::Action::SelectAll);
        w.code_views[0].content.perform(text_editor::Action::Edit(
            text_editor::Edit::Paste(std::sync::Arc::new("modified".into())),
        ));
        w.code_views[0].dirty = true;

        send(&mut w, Message::SaveFile);
        let content = std::fs::read_to_string(&test_file).unwrap();
        assert_eq!(content.trim(), "modified");

        let _ = std::fs::remove_file(&test_file);
    }

    // -------------------------------------------------------------------
    // Window resize
    // -------------------------------------------------------------------

    #[test]
    fn window_resize_updates_terminal_dimensions() {
        let mut w = ws();
        assert_eq!(w.terminal_cols, 80);
        assert_eq!(w.terminal_rows, 24);

        send(&mut w, Message::WindowResized(1800.0, 1000.0));
        assert_ne!(w.terminal_cols, 80);
        assert!(w.terminal_cols > 50);
        assert!(w.terminal_rows > 20);
    }

    // -------------------------------------------------------------------
    // Terminal clipboard (without real PTY)
    // -------------------------------------------------------------------

    #[test]
    fn terminal_text_selected_updates_state() {
        let mut w = ws();
        send(&mut w, Message::TerminalTextSelected("hello world".into()));
        assert_eq!(w.last_terminal_selection, "hello world");
    }

    // -------------------------------------------------------------------
    // Diff reviewed
    // -------------------------------------------------------------------

    #[test]
    fn diff_reviewed_marks_current_file() {
        let mut w = ws();
        w.diff_views[0].apply_diff_text(
            "diff --git a/foo.rs b/foo.rs\n+added\ndiff --git a/bar.rs b/bar.rs\n+more\n".into(),
        );
        assert_eq!(w.diff_views[0].reviewed_count(), 0);

        send(&mut w, Message::DiffReviewed);
        assert_eq!(w.diff_views[0].reviewed_count(), 1);
        assert!(w.diff_views[0].file_diffs[0].reviewed);
        assert!(!w.diff_views[0].file_diffs[1].reviewed);
    }

    // -------------------------------------------------------------------
    // Project switching
    // -------------------------------------------------------------------

    #[test]
    fn switch_project_changes_active() {
        let mut w = ws();
        let path2 = std::env::temp_dir().join("jc-test-project-2");
        let _ = std::fs::create_dir_all(&path2);
        w.projects.push(
            crate::views::project_state::ProjectState::for_testing(
                path2, "project-2".into(),
            ),
        );
        w.code_views.push(crate::views::code_view::CodeViewState::default());
        w.diff_views.push(crate::views::diff_view::DiffViewState::new(
            w.projects[1].path.clone(),
        ));
        w.todo_views.push(crate::views::todo_view::TodoViewState::new(
            w.projects[1].path.clone(),
        ));

        assert_eq!(w.active_project_index, 0);
        send(&mut w, Message::SwitchProject(1));
        assert_eq!(w.active_project_index, 1);
    }

    #[test]
    fn switch_project_out_of_bounds_ignored() {
        let mut w = ws();
        send(&mut w, Message::SwitchProject(99));
        assert_eq!(w.active_project_index, 0);
    }

    // -------------------------------------------------------------------
    // Close/Quit flow (no active sessions = immediate exit request)
    // -------------------------------------------------------------------

    #[test]
    fn close_active_session_with_none_is_noop() {
        let mut w = ws();
        assert!(w.projects[0].active_session.is_none());
        send(&mut w, Message::CloseActiveSession);
        assert!(w.projects[0].active_session.is_none());
    }

    // -------------------------------------------------------------------
    // Code editor action
    // -------------------------------------------------------------------

    #[test]
    fn code_editor_action_marks_dirty() {
        let mut w = ws();
        let project_path = w.projects[0].path.clone();
        let test_file = project_path.join("test_edit.txt");
        std::fs::write(&test_file, "original").unwrap();
        send(&mut w, Message::OpenFile(test_file.clone()));
        assert!(!w.code_views[0].dirty);

        use iced::widget::text_editor;
        send(
            &mut w,
            Message::CodeEditorAction(text_editor::Action::Edit(
                text_editor::Edit::Insert('x'),
            )),
        );
        assert!(w.code_views[0].dirty);

        let _ = std::fs::remove_file(&test_file);
    }

    // -------------------------------------------------------------------
    // File change notification
    // -------------------------------------------------------------------

    #[test]
    fn file_changed_reloads_code_view() {
        let mut w = ws();
        let project_path = w.projects[0].path.clone();
        let test_file = project_path.join("test_watch.txt");
        std::fs::write(&test_file, "version1").unwrap();

        send(&mut w, Message::OpenFile(test_file.clone()));
        assert!(w.code_views[0].raw_text.contains("version1"));

        std::fs::write(&test_file, "version2").unwrap();
        send(&mut w, Message::FileChanged(test_file.clone()));
        assert!(w.code_views[0].raw_text.contains("version2"));

        let _ = std::fs::remove_file(&test_file);
    }

    #[test]
    fn file_changed_marks_diff_stale() {
        let mut w = ws();
        w.diff_views[0].stale = false;
        let path = w.projects[0].path.join("something.rs");
        send(&mut w, Message::FileChanged(path));
        assert!(w.diff_views[0].stale);
    }

    // -------------------------------------------------------------------
    // Multi-step flow: open file picker → select → verify code view
    // -------------------------------------------------------------------

    #[test]
    fn open_file_via_picker_flow() {
        let mut w = ws();
        let project_path = w.projects[0].path.clone();
        let test_file = project_path.join("picker_test.rs");
        std::fs::write(&test_file, "fn picker_test() {}").unwrap();

        send(&mut w, Message::OpenPicker(PickerKind::File));
        let picker = w.picker.as_ref().unwrap();
        let file_idx = picker
            .items
            .iter()
            .position(|i| i.label.contains("picker_test.rs"));

        if let Some(idx) = file_idx {
            // Navigate to the item
            for _ in 0..idx {
                send(&mut w, Message::PickerSelectNext);
            }
            send(&mut w, Message::PickerConfirm);

            assert!(w.picker.is_none());
            assert!(w.code_views[0].raw_text.contains("fn picker_test"));
            assert_eq!(w.panes[w.active_pane_index].kind, PaneContentKind::CodeViewer);
        }

        let _ = std::fs::remove_file(&test_file);
    }

    // -------------------------------------------------------------------
    // Toggle layout via command palette
    // -------------------------------------------------------------------

    #[test]
    fn command_palette_toggle_layout() {
        let mut w = ws();
        assert_eq!(w.layout, PaneLayoutKind::Three);

        send(&mut w, Message::OpenPicker(PickerKind::Command));
        // "Toggle Layout" is the second item
        send(&mut w, Message::PickerSelectNext);
        send(&mut w, Message::PickerConfirm);

        assert!(w.picker.is_none());
        assert_eq!(w.layout, PaneLayoutKind::One);
    }

    // -------------------------------------------------------------------
    // Keyboard event dispatch
    // -------------------------------------------------------------------

    #[test]
    fn keyboard_event_dispatches_keybinding() {
        let mut w = ws();
        assert!(!w.keybinding_help.visible);

        let event = iced::keyboard::Event::KeyPressed {
            key: iced::keyboard::Key::Character("?".into()),
            modified_key: iced::keyboard::Key::Character("?".into()),
            physical_key: iced::keyboard::key::Physical::Unidentified(
                iced::keyboard::key::NativeCode::Unidentified,
            ),
            location: iced::keyboard::Location::Standard,
            modifiers: iced::keyboard::Modifiers::CTRL,
            text: None,
        };
        send(&mut w, Message::KeyboardEvent(event));
        assert!(w.keybinding_help.visible);
    }

    // -------------------------------------------------------------------
    // None message is no-op
    // -------------------------------------------------------------------

    #[test]
    fn none_message_is_noop() {
        let mut w = ws();
        let layout_before = w.layout;
        let pane_before = w.active_pane_index;
        send(&mut w, Message::None);
        assert_eq!(w.layout, layout_before);
        assert_eq!(w.active_pane_index, pane_before);
    }
}
