use iced::{Element, Subscription, Task, Theme};
use iced::futures::SinkExt;
use std::path::PathBuf;
use std::time::Duration;

use crate::views::pane::PaneContentKind;
use crate::views::picker::PickerState;
use crate::views::workspace::{Message, Workspace};
use jc_core::config::{AppConfig, AppState};

/// Flags passed to the iced application on startup.
pub struct Flags {
    pub state: AppState,
    pub config: AppConfig,
    pub ipc_rx: flume::Receiver<PathBuf>,
}

/// Run the iced application.
pub fn run(state: AppState, config: AppConfig, ipc_rx: flume::Receiver<PathBuf>) {
    let flags = Flags { state, config, ipc_rx };

    iced::application("jc — Claude Code Orchestrator", update, view)
        .subscription(subscription)
        .theme(theme)
        .window_size(iced::Size::new(1200.0, 800.0))
        .run_with(move || {
            let workspace = Workspace::new(flags.state, flags.config, flags.ipc_rx);
            (workspace, Task::none())
        })
        .expect("failed to run iced application");
}

fn update(workspace: &mut Workspace, message: Message) -> Task<Message> {
    match message {
        // Terminal
        Message::TerminalOutput(session_id, data) => {
            let project = workspace.active_project_mut();
            if let Some(_session) = project.sessions.get_mut(&session_id) {
                // Feed data to alacritty terminal emulator.
                // Feed PTY output to the terminal emulator.
                // Full VTE parsing will be added when terminal canvas rendering
                // is implemented. For now, data is captured but not parsed.
                let _ = &data;
            }
        }
        Message::TerminalEvent(session_id, event_kind) => {
            use crate::views::workspace::TerminalEventKind;
            use crate::views::session_state::PendingEvent;
            let project = workspace.active_project_mut();
            if let Some(session) = project.sessions.get_mut(&session_id) {
                match event_kind {
                    TerminalEventKind::Bell => {
                        session.pending_events.insert(PendingEvent::TerminalBell);
                    }
                    TerminalEventKind::Exit => {
                        // Terminal process exited — mark session not busy.
                        session.busy = false;
                    }
                    TerminalEventKind::Wakeup => {
                        // Trigger re-render (handled by returning from update).
                    }
                }
            }
        }
        Message::TerminalResize(_cols, _rows) => {
            // Will be implemented when terminal canvas rendering is added.
        }

        // Workspace navigation
        Message::SwitchSession(id) => workspace.switch_session(id),
        Message::NewSession => workspace.create_session(),
        Message::CloseSession(id) => workspace.close_session(id),
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

        // Hooks
        Message::HookReceived(event) => {
            workspace.handle_hook_event(event);
        }

        // IPC
        Message::IpcProjectOpen(path) => {
            // Register new project if needed.
            let already_exists = workspace
                .projects
                .iter()
                .any(|p| p.path == path);
            if !already_exists {
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "unknown".to_string());
                let project =
                    crate::views::project_state::ProjectState::create(path, name);
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

        // Picker
        Message::OpenPicker(kind) => {
            workspace.picker = Some(PickerState::new(kind));
        }
        Message::PickerQueryChanged(query) => {
            if let Some(picker) = &mut workspace.picker {
                picker.filter(&query);
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
            // Handle picker selection
            workspace.picker = None;
        }
        Message::PickerDismiss => {
            workspace.picker = None;
        }

        // Code/Diff
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
            if let Some(fd) =
                dv.file_diffs.get_mut(dv.current_file_index)
            {
                fd.reviewed = true;
            }
        }

        // Keybinding help
        Message::ToggleKeybindingHelp => {
            workspace.keybinding_help.visible =
                !workspace.keybinding_help.visible;
        }

        // Problems
        Message::NextProblem => {
            workspace.next_problem();
        }
        Message::JumpToWait => {
            let pane_idx = workspace
                .resolve_pane_for_kind(PaneContentKind::TodoEditor);
            workspace.show_in_pane(pane_idx, PaneContentKind::TodoEditor);
        }

        // Close/Quit
        Message::RequestClose | Message::RequestQuit => {
            let is_quit = matches!(message, Message::RequestQuit);
            let conflicts = workspace.save_all_dirty();
            let active_count = workspace.active_session_count();
            if conflicts.is_empty() && active_count == 0 {
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
            workspace.close_confirm = None;
            return iced::exit();
        }
        Message::CancelClose => {
            workspace.close_confirm = None;
        }

        // Tick — periodic refresh
        Message::Tick => {
            for project in &mut workspace.projects {
                project.refresh_problems();
            }
        }

        // Keyboard
        Message::KeyboardEvent(event) => {
            if let Some(msg) =
                crate::views::workspace::keybindings::handle_key_event(&event)
            {
                return update(workspace, msg);
            }
        }

        Message::NotificationAction(_) => {}
        Message::FileChanged(_) => {}
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
            // Directly map key presses to messages via keybinding handler.
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

    // Periodic tick for problem refresh (every 2 seconds)
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
                                // Channel closed, sleep forever.
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

    // PTY reader subscriptions for all active sessions
    for project in &workspace.projects {
        for (&session_id, _session) in &project.sessions {
            // PTY reading subscriptions will be added when terminal rendering
            // is implemented. For now, terminal output is not consumed.
            // The reader threads need to be spawned and feed data via channels.
            let _ = session_id;
        }
    }

    Subscription::batch(subscriptions)
}

fn theme(_workspace: &Workspace) -> Theme {
    Theme::Dark
}
