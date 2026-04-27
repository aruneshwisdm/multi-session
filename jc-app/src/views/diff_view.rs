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

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_DIFF: &str = "\
diff --git a/src/main.rs b/src/main.rs
index abc123..def456 100644
--- a/src/main.rs
+++ b/src/main.rs
@@ -1,3 +1,4 @@
 fn main() {
+    println!(\"hello\");
 }
diff --git a/src/lib.rs b/src/lib.rs
index 111222..333444 100644
--- a/src/lib.rs
+++ b/src/lib.rs
@@ -1 +1,2 @@
+pub mod utils;
 pub mod core;";

    #[test]
    fn parse_diff_extracts_file_names() {
        let files = parse_diff_files(SAMPLE_DIFF);
        assert_eq!(files.len(), 2);
        assert_eq!(files[0].name, "src/main.rs");
        assert_eq!(files[1].name, "src/lib.rs");
    }

    #[test]
    fn parse_diff_files_not_reviewed_by_default() {
        let files = parse_diff_files(SAMPLE_DIFF);
        assert!(!files[0].reviewed);
        assert!(!files[1].reviewed);
    }

    #[test]
    fn parse_diff_preserves_content() {
        let files = parse_diff_files(SAMPLE_DIFF);
        assert!(files[0].content.contains("+    println!"));
        assert!(files[1].content.contains("+pub mod utils"));
    }

    #[test]
    fn parse_empty_diff() {
        let files = parse_diff_files("");
        assert!(files.is_empty());
    }

    #[test]
    fn parse_single_file_diff() {
        let diff = "diff --git a/foo.rs b/foo.rs\n+added line\n";
        let files = parse_diff_files(diff);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].name, "foo.rs");
    }

    #[test]
    fn diff_view_new_is_stale() {
        let dv = DiffViewState::new(PathBuf::from("/tmp/test"));
        assert!(dv.stale);
        assert!(dv.file_diffs.is_empty());
        assert_eq!(dv.current_file_index, 0);
    }

    #[test]
    fn apply_diff_text_clears_stale() {
        let mut dv = DiffViewState::new(PathBuf::from("/tmp/test"));
        dv.apply_diff_text(SAMPLE_DIFF.to_string());
        assert!(!dv.stale);
        assert_eq!(dv.file_count(), 2);
    }

    #[test]
    fn reviewed_count_tracks_reviews() {
        let mut dv = DiffViewState::new(PathBuf::from("/tmp/test"));
        dv.apply_diff_text(SAMPLE_DIFF.to_string());
        assert_eq!(dv.reviewed_count(), 0);
        dv.file_diffs[0].reviewed = true;
        assert_eq!(dv.reviewed_count(), 1);
    }

    #[test]
    fn unreviewed_files_returns_paths() {
        let mut dv = DiffViewState::new(PathBuf::from("/tmp/test"));
        dv.apply_diff_text(SAMPLE_DIFF.to_string());
        let unreviewed = dv.unreviewed_files();
        assert_eq!(unreviewed.len(), 2);
        dv.file_diffs[0].reviewed = true;
        let unreviewed = dv.unreviewed_files();
        assert_eq!(unreviewed.len(), 1);
        assert_eq!(unreviewed[0], PathBuf::from("src/lib.rs"));
    }

    #[test]
    fn current_file_name_returns_indexed_file() {
        let mut dv = DiffViewState::new(PathBuf::from("/tmp/test"));
        dv.apply_diff_text(SAMPLE_DIFF.to_string());
        assert_eq!(dv.current_file_name(), Some("src/main.rs"));
        dv.current_file_index = 1;
        assert_eq!(dv.current_file_name(), Some("src/lib.rs"));
    }

    #[test]
    fn current_file_name_out_of_bounds_returns_none() {
        let mut dv = DiffViewState::new(PathBuf::from("/tmp/test"));
        dv.apply_diff_text(SAMPLE_DIFF.to_string());
        dv.current_file_index = 99;
        assert!(dv.current_file_name().is_none());
    }

    #[test]
    fn diff_source_label() {
        assert_eq!(DiffSource::WorkingTree.label(), "working tree");
        let commit = DiffSource::Commit {
            oid: "abc".into(),
            message: "fix bug".into(),
        };
        assert_eq!(commit.label(), "fix bug");
    }

    #[test]
    fn apply_diff_returns_changed_on_new_content() {
        let mut dv = DiffViewState::new(PathBuf::from("/tmp/test"));
        let changed = dv.apply_diff_text(SAMPLE_DIFF.to_string());
        assert!(changed);
        let changed = dv.apply_diff_text(SAMPLE_DIFF.to_string());
        assert!(!changed);
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
