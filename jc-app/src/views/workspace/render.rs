use iced::widget::{button, canvas, column, container, horizontal_rule, row, text};
use iced::{Color, Element, Length};

use super::{Message, Workspace};
use crate::views::pane::PaneContentKind;
use crate::views::session_state::SessionActivity;
use crate::views::terminal_canvas::TerminalProgram;

fn activity_indicator(activity: SessionActivity) -> (&'static str, Color) {
    match activity {
        SessionActivity::Busy => ("\u{25cf}", Color::from_rgb(0.2, 0.7, 0.3)),
        SessionActivity::NeedsPermission => ("\u{25cf}", Color::from_rgb(1.0, 0.6, 0.0)),
        SessionActivity::Error => ("\u{25cf}", Color::from_rgb(0.9, 0.2, 0.2)),
        SessionActivity::Idle => ("\u{25cf}", Color::from_rgb(0.5, 0.5, 0.5)),
        SessionActivity::New => ("\u{25cb}", Color::from_rgb(0.7, 0.7, 0.7)),
    }
}

impl Workspace {
    pub fn view_app(&self) -> Element<'_, Message> {
        let title_bar = self.render_title_bar();
        let pane_area = self.render_panes();

        let layout = column![title_bar, horizontal_rule(1), pane_area]
            .spacing(0)
            .width(Length::Fill)
            .height(Length::Fill);

        let base: Element<Message> = layout.into();

        if let Some(picker) = &self.picker {
            column![base, picker.view()].into()
        } else if self.keybinding_help.visible {
            column![base, self.keybinding_help.view()].into()
        } else if let Some(confirm) = &self.close_confirm {
            let dialog = Self::render_close_confirm(confirm);
            column![base, dialog].into()
        } else {
            base
        }
    }

    fn render_title_bar(&self) -> Element<'_, Message> {
        let project = self.active_project();
        let mut title = project.name.clone();
        if let Some(session) = project.active_session() {
            title = format!("{} > {}", title, session.label);
        }

        let problem_count: usize = project
            .active_session()
            .map(|s| s.problems.len())
            .unwrap_or(0)
            + project.problems.len();

        let title_text = if problem_count > 0 {
            text(format!("! {} ({})", title, problem_count)).size(14)
        } else {
            text(title).size(14)
        };

        let tabs: Vec<Element<Message>> = project
            .sessions
            .iter()
            .map(|(&id, session)| {
                let is_active = project.active_session == Some(id);
                let label = if is_active {
                    format!("[{}]", session.label)
                } else {
                    session.label.clone()
                };
                let (dot, dot_color) = activity_indicator(session.activity());
                button(
                    row![
                        text(dot).size(10).color(dot_color),
                        text(label).size(12),
                    ]
                    .spacing(4)
                    .align_y(iced::Alignment::Center),
                )
                .on_press(Message::SwitchSession(id))
                .padding(4)
                .into()
            })
            .collect();

        let new_session_btn = button(text("+").size(12))
            .on_press(Message::NewSession)
            .padding(4);

        // Status bar badges: cross-session problem counts per layer
        let layer_counts = self.layer_problem_sessions();
        let badge_defs: [(&str, Color, &Vec<String>); 4] = [
            ("!", Color::from_rgb(0.9, 0.2, 0.2), &layer_counts[0]),
            ("?", Color::from_rgb(1.0, 0.6, 0.0), &layer_counts[1]),
            ("~", Color::from_rgb(0.3, 0.5, 0.9), &layer_counts[2]),
            ("\u{25cb}", Color::from_rgb(0.5, 0.5, 0.5), &layer_counts[3]),
        ];
        let badges: Vec<Element<Message>> = badge_defs
            .iter()
            .filter(|(_, _, sessions)| !sessions.is_empty())
            .map(|(icon, color, sessions)| {
                text(format!("{icon}{}", sessions.len()))
                    .size(11)
                    .color(*color)
                    .into()
            })
            .collect();

        let mut title_row = row![
            title_text,
            row(tabs).spacing(4),
            new_session_btn,
        ]
        .spacing(12)
        .align_y(iced::Alignment::Center);

        if !badges.is_empty() {
            title_row = title_row.push(
                row(badges).spacing(8).align_y(iced::Alignment::Center),
            );
        }

        container(title_row)
            .padding([4, 8])
            .width(Length::Fill)
            .into()
    }

    fn render_panes(&self) -> Element<'_, Message> {
        let visible = self.visible_pane_count();
        let pane_elements: Vec<Element<Message>> = (0..visible)
            .map(|i| self.render_pane(i))
            .collect();

        row(pane_elements)
            .spacing(1)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn render_pane(&self, index: usize) -> Element<'_, Message> {
        let pane = &self.panes[index];
        let is_active = self.active_pane_index == index;
        let pi = self.active_project_index;

        let header = {
            let title = self.pane_title(pane.kind);
            let active_marker = if is_active { "▸ " } else { "  " };
            text(format!("{active_marker}{title}")).size(12)
        };

        let content: Element<Message> = match pane.kind {
            PaneContentKind::ClaudeTerminal => {
                self.render_terminal_pane(pi, true)
            }
            PaneContentKind::GeneralTerminal => {
                self.render_terminal_pane(pi, false)
            }
            PaneContentKind::CodeViewer => self.code_views[pi].view(),
            PaneContentKind::GitDiff => self.diff_views[pi].view(),
            PaneContentKind::TodoEditor => self.todo_views[pi].view(),
            PaneContentKind::GlobalTodo => self.global_todo.view(),
        };

        container(
            column![
                container(header).padding([2, 8]).width(Length::Fill),
                horizontal_rule(1),
                container(content)
                    .width(Length::Fill)
                    .height(Length::Fill),
            ]
            .spacing(0),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }

    fn render_terminal_pane(&self, project_index: usize, is_claude: bool) -> Element<'_, Message> {
        let project = &self.projects[project_index];
        let Some(session) = project.active_session() else {
            return text("No active session").size(13).into();
        };

        let terminal = if is_claude {
            &session.claude_terminal
        } else {
            &session.general_terminal
        };

        let grid = terminal.state.with_term(|term| {
            jc_terminal::render_grid(term, &self.palette)
        });

        let bg_color = Color::from_rgba(
            self.palette.background.r,
            self.palette.background.g,
            self.palette.background.b,
            self.palette.background.a,
        );

        let program = TerminalProgram {
            grid,
            bg_color,
            is_claude,
            busy: session.busy,
        };

        canvas(program)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn pane_title(&self, kind: PaneContentKind) -> String {
        let pi = self.active_project_index;
        match kind {
            PaneContentKind::CodeViewer => {
                let cv = &self.code_views[pi];
                let dirty = if cv.dirty { " [+]" } else { "" };
                if let Some(path) = &cv.file_path {
                    let name = path
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| "untitled".to_string());
                    format!("Code: {name}{dirty}")
                } else {
                    format!("Code{dirty}")
                }
            }
            PaneContentKind::TodoEditor => {
                let dirty = if self.todo_views[pi].dirty { " [+]" } else { "" };
                format!("TODO{dirty}")
            }
            PaneContentKind::GitDiff => {
                let dv = &self.diff_views[pi];
                let reviewed = dv.reviewed_count();
                let total = dv.file_count();
                format!("Diff ({reviewed}/{total})")
            }
            PaneContentKind::GlobalTodo => {
                let dirty = if self.global_todo.dirty { " [+]" } else { "" };
                format!("Global TODO{dirty}")
            }
            other => other.label().to_string(),
        }
    }

    fn render_close_confirm(confirm: &super::CloseConfirmState) -> Element<'_, Message> {
        let msg = if confirm.session_count > 0 {
            format!(
                "{} active session(s) still running.",
                confirm.session_count
            )
        } else {
            "Unsaved changes exist.".to_string()
        };

        let conflicts_text = if !confirm.conflicts.is_empty() {
            format!(
                "\nMerge conflicts in: {}",
                confirm.conflicts.join(", ")
            )
        } else {
            String::new()
        };

        let action = if confirm.is_quit { "Quit" } else { "Close" };

        container(
            column![
                text(format!("{msg}{conflicts_text}")).size(14),
                row![
                    button(text(action).size(13))
                        .on_press(Message::ConfirmClose),
                    button(text("Cancel").size(13))
                        .on_press(Message::CancelClose),
                ]
                .spacing(8),
            ]
            .spacing(12),
        )
        .padding(16)
        .max_width(400)
        .into()
    }
}

