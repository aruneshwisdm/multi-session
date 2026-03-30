use crate::views::pane::PaneContentKind;
use crate::views::session_state::SessionId;
use jc_core::problem::{ProblemLayer, ProblemTarget};

use super::Workspace;

#[derive(Debug, Clone)]
pub struct ProblemCycleState {
    pub layer: ProblemLayer,
    pub index: usize,
}

struct CrossSessionProblem {
    project_index: usize,
    session_id: SessionId,
    target: ProblemTarget,
}

impl Workspace {
    pub fn next_problem(&mut self) {
        let l0_problems = self.collect_cross_session_l0();

        if !l0_problems.is_empty() {
            if self.pre_layer0_home.is_none() {
                if let Some(active_sid) =
                    self.projects[self.active_project_index].active_session
                {
                    self.pre_layer0_home =
                        Some((self.active_project_index, active_sid));
                }
            }

            let idx = match &self.problem_cycle {
                Some(state) if state.layer == ProblemLayer::L0 => {
                    let next = state.index + 1;
                    if next < l0_problems.len() { next } else { 0 }
                }
                _ => 0,
            };

            self.problem_cycle = Some(ProblemCycleState {
                layer: ProblemLayer::L0,
                index: idx,
            });

            let problem = &l0_problems[idx];
            let target_pi = problem.project_index;
            let target_sid = problem.session_id;
            let target = problem.target.clone();

            let current_sid =
                self.projects[self.active_project_index].active_session;
            if target_pi != self.active_project_index
                || Some(target_sid) != current_sid
            {
                self.active_project_index = target_pi;
                self.projects[target_pi].active_session = Some(target_sid);
            }

            self.jump_to_problem_target(target);
            return;
        }

        // No L0 problems — return home if we were away.
        if let Some((home_pi, home_sid)) = self.pre_layer0_home.take() {
            let home_valid = self
                .projects
                .get(home_pi)
                .map(|p| p.sessions.contains_key(&home_sid))
                .unwrap_or(false);
            if home_valid {
                self.active_project_index = home_pi;
                self.projects[home_pi].active_session = Some(home_sid);
            }
            self.problem_cycle = None;
        }

        // Cycle L1/L2/L3 in current session/project.
        let local_problems = self.collect_local_problems();

        if local_problems.is_empty() {
            self.problem_cycle = None;
            let pane_idx = self.resolve_pane_for_kind(PaneContentKind::TodoEditor);
            self.show_in_pane(pane_idx, PaneContentKind::TodoEditor);
            return;
        }

        let idx = match &self.problem_cycle {
            Some(state) if state.layer >= ProblemLayer::L1 => {
                let same_layer: Vec<usize> = local_problems
                    .iter()
                    .enumerate()
                    .filter(|(_, (l, _, _))| *l == state.layer)
                    .map(|(i, _)| i)
                    .collect();

                if same_layer.is_empty() {
                    local_problems
                        .iter()
                        .position(|(l, _, _)| *l > state.layer)
                } else {
                    let next_in_layer = same_layer
                        .iter()
                        .find(|&&i| {
                            let pos_in_layer = same_layer
                                .iter()
                                .position(|&j| j == i)
                                .unwrap_or(0);
                            pos_in_layer > state.index
                        })
                        .copied();
                    next_in_layer.or_else(|| {
                        local_problems
                            .iter()
                            .position(|(l, _, _)| *l > state.layer)
                    })
                }
            }
            _ => Some(0),
        };

        if let Some(idx) = idx {
            let (layer, _, target) = &local_problems[idx];
            let layer = *layer;
            let target = target.clone();
            let layer_start = local_problems
                .iter()
                .position(|(l, _, _)| *l == layer)
                .unwrap_or(0);
            let index_in_layer = idx - layer_start;

            self.problem_cycle = Some(ProblemCycleState {
                layer,
                index: index_in_layer,
            });
            self.jump_to_problem_target(target);
        } else {
            self.problem_cycle = None;
            let pane_idx = self.resolve_pane_for_kind(PaneContentKind::TodoEditor);
            self.show_in_pane(pane_idx, PaneContentKind::TodoEditor);
        }
    }

    fn collect_cross_session_l0(&self) -> Vec<CrossSessionProblem> {
        let mut result = Vec::new();
        for (pi, project) in self.projects.iter().enumerate() {
            let mut session_ids: Vec<SessionId> =
                project.sessions.keys().copied().collect();
            session_ids.sort();
            for sid in session_ids {
                let session = &project.sessions[&sid];
                for problem in &session.problems {
                    if problem.layer() == ProblemLayer::L0 {
                        result.push(CrossSessionProblem {
                            project_index: pi,
                            session_id: sid,
                            target: problem.target(),
                        });
                    }
                }
            }
        }
        result
    }

    fn collect_local_problems(
        &self,
    ) -> Vec<(ProblemLayer, i8, ProblemTarget)> {
        let project = &self.projects[self.active_project_index];
        let mut problems: Vec<(ProblemLayer, i8, ProblemTarget)> = Vec::new();

        if let Some(session) = project.active_session() {
            for sp in &session.problems {
                problems.push((sp.layer(), sp.rank(), sp.target()));
            }
            if !session.busy && session.has_ever_been_busy {
                let has_claude_problem = session.problems.iter().any(|p| {
                    matches!(p.target(), ProblemTarget::ClaudeTerminal)
                });
                if !has_claude_problem {
                    problems.push((
                        ProblemLayer::L3,
                        0,
                        ProblemTarget::ClaudeTerminal,
                    ));
                }
            }
        }

        for pp in &project.problems {
            problems.push((pp.layer(), pp.rank(), pp.target()));
        }

        let session_busy = project
            .active_session()
            .map(|s| s.busy)
            .unwrap_or(false);
        let has_l1 = problems.iter().any(|(l, _, _)| *l == ProblemLayer::L1);
        if session_busy || has_l1 {
            problems.retain(|(l, _, _)| *l != ProblemLayer::L2);
        }

        problems.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
        problems
    }

    fn jump_to_problem_target(&mut self, target: ProblemTarget) {
        let kind = match &target {
            ProblemTarget::ClaudeTerminal => PaneContentKind::ClaudeTerminal,
            ProblemTarget::GeneralTerminal => {
                PaneContentKind::GeneralTerminal
            }
            ProblemTarget::TodoEditor => PaneContentKind::TodoEditor,
            ProblemTarget::DiffView { .. } => PaneContentKind::GitDiff,
            ProblemTarget::CodeView { file, .. }
                if file
                    .file_name()
                    .and_then(|f| f.to_str())
                    == Some("TODO.md") =>
            {
                PaneContentKind::TodoEditor
            }
            ProblemTarget::CodeView { .. } => PaneContentKind::CodeViewer,
        };

        let pane_idx = self.resolve_pane_for_kind(kind);
        self.show_in_pane(pane_idx, kind);

        match target {
            ProblemTarget::CodeView { file, line: _ }
                if kind == PaneContentKind::CodeViewer =>
            {
                let pi = self.active_project_index;
                let full_path = self.projects[pi].path.join(&file);
                self.code_views[pi].open_file(full_path);
                // Line scrolling will be added with text editor widget.
            }
            _ => {}
        }
    }

    pub fn layer_problem_sessions(&self) -> [Vec<String>; 4] {
        let mut result: [Vec<String>; 4] = Default::default();
        let layers = [
            ProblemLayer::L0,
            ProblemLayer::L1,
            ProblemLayer::L2,
            ProblemLayer::L3,
        ];

        for (li, layer) in layers.iter().enumerate() {
            for (pi, project) in self.projects.iter().enumerate() {
                for (&sid, session) in &project.sessions {
                    let is_active = pi == self.active_project_index
                        && project.active_session == Some(sid);
                    if is_active {
                        continue;
                    }

                    let mut has_layer = false;
                    for sp in &session.problems {
                        if sp.layer() == *layer {
                            has_layer = true;
                            break;
                        }
                    }

                    if !has_layer
                        && *layer == ProblemLayer::L3
                        && !session.busy
                        && session.has_ever_been_busy
                    {
                        let has_claude_problem =
                            session.problems.iter().any(|p| {
                                matches!(
                                    p.target(),
                                    ProblemTarget::ClaudeTerminal
                                )
                            });
                        if !has_claude_problem {
                            has_layer = true;
                        }
                    }

                    if has_layer {
                        let label = format!(
                            "{} > {}",
                            project.name, session.label
                        );
                        result[li].push(label);
                    }
                }
            }
        }
        result
    }
}
