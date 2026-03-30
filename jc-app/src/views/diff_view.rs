use iced::widget::{column, container, scrollable, text};
use iced::Element;
use std::path::{Path, PathBuf};

use super::workspace::Message;

#[derive(Debug, Clone)]
pub struct FileDiff {
    pub name: String,
    pub content: String,
    pub reviewed: bool,
}

#[derive(Debug, Clone)]
pub enum DiffSource {
    WorkingTree,
    Commit { oid: String, message: String },
}

impl DiffSource {
    pub fn label(&self) -> &str {
        match self {
            Self::WorkingTree => "working tree",
            Self::Commit { message, .. } => message.as_str(),
        }
    }
}

pub struct DiffViewState {
    pub project_path: PathBuf,
    pub source: DiffSource,
    pub file_diffs: Vec<FileDiff>,
    pub current_file_index: usize,
    pub stale: bool,
}

impl DiffViewState {
    pub fn new(project_path: PathBuf) -> Self {
        Self {
            project_path,
            source: DiffSource::WorkingTree,
            file_diffs: Vec::new(),
            current_file_index: 0,
            stale: true,
        }
    }

    pub fn generate_diff(project_path: &Path) -> String {
        let output = std::process::Command::new("git")
            .args(["diff", "--no-color"])
            .current_dir(project_path)
            .output();

        match output {
            Ok(out) => String::from_utf8_lossy(&out.stdout).to_string(),
            Err(e) => format!("Failed to run git diff: {e}"),
        }
    }

    pub fn apply_diff_text(&mut self, diff_text: String) -> bool {
        let file_diffs = parse_diff_files(&diff_text);
        let changed = self.file_diffs.len() != file_diffs.len();
        self.file_diffs = file_diffs;
        self.stale = false;
        changed
    }

    pub fn reviewed_count(&self) -> usize {
        self.file_diffs.iter().filter(|f| f.reviewed).count()
    }

    pub fn file_count(&self) -> usize {
        self.file_diffs.len()
    }

    pub fn current_file_name(&self) -> Option<&str> {
        self.file_diffs
            .get(self.current_file_index)
            .map(|f| f.name.as_str())
    }

    pub fn unreviewed_files(&self) -> Vec<PathBuf> {
        self.file_diffs
            .iter()
            .filter(|f| !f.reviewed)
            .map(|f| PathBuf::from(&f.name))
            .collect()
    }

    pub fn view(&self) -> Element<'_, Message> {
        let header = {
            let reviewed = self.reviewed_count();
            let total = self.file_count();
            let source_label = self.source.label();
            text(format!(
                "Diff [{source_label}] ({reviewed}/{total} reviewed)"
            ))
            .size(14)
        };

        let body: Element<Message> = if self.file_diffs.is_empty() {
            text("No changes").size(13).into()
        } else if let Some(file_diff) = self.file_diffs.get(self.current_file_index) {
            let lines: Vec<Element<Message>> = file_diff
                .content
                .lines()
                .map(|line| text(line).size(12).into())
                .collect();
            scrollable(column(lines).spacing(0)).into()
        } else {
            text("No file selected").size(13).into()
        };

        container(column![header, body].spacing(4))
            .padding(8)
            .into()
    }
}

fn parse_diff_files(diff_text: &str) -> Vec<FileDiff> {
    let mut files = Vec::new();
    let mut current_name = String::new();
    let mut current_content = String::new();

    for line in diff_text.lines() {
        if line.starts_with("diff --git") {
            if !current_name.is_empty() {
                files.push(FileDiff {
                    name: current_name.clone(),
                    content: current_content.clone(),
                    reviewed: false,
                });
            }
            // Extract file name from "diff --git a/foo b/foo"
            current_name = line
                .split(" b/")
                .nth(1)
                .unwrap_or("unknown")
                .to_string();
            current_content = line.to_string();
            current_content.push('\n');
        } else {
            current_content.push_str(line);
            current_content.push('\n');
        }
    }

    if !current_name.is_empty() {
        files.push(FileDiff {
            name: current_name,
            content: current_content,
            reviewed: false,
        });
    }

    files
}
