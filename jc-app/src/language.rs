use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Language {
    Rust,
    JavaScript,
    TypeScript,
    Tsx,
    Python,
    Ruby,
    Go,
    Markdown,
    Toml,
    Json,
    Yaml,
    Html,
    Css,
    C,
    Cpp,
    Java,
    Bash,
    #[default]
    Text,
}

impl Language {
    pub fn name(self) -> &'static str {
        match self {
            Self::Rust => "rust",
            Self::JavaScript => "javascript",
            Self::TypeScript => "typescript",
            Self::Tsx => "tsx",
            Self::Python => "python",
            Self::Ruby => "ruby",
            Self::Go => "go",
            Self::Markdown => "markdown",
            Self::Toml => "toml",
            Self::Json => "json",
            Self::Yaml => "yaml",
            Self::Html => "html",
            Self::Css => "css",
            Self::C => "c",
            Self::Cpp => "cpp",
            Self::Java => "java",
            Self::Bash => "bash",
            Self::Text => "text",
        }
    }

    pub fn from_extension(ext: &str) -> Self {
        match ext {
            "rs" => Self::Rust,
            "js" => Self::JavaScript,
            "ts" => Self::TypeScript,
            "tsx" => Self::Tsx,
            "py" => Self::Python,
            "rb" => Self::Ruby,
            "go" => Self::Go,
            "c" | "h" => Self::C,
            "cpp" | "cc" | "cxx" | "hpp" => Self::Cpp,
            "java" => Self::Java,
            "md" => Self::Markdown,
            "toml" => Self::Toml,
            "json" => Self::Json,
            "yaml" | "yml" => Self::Yaml,
            "html" => Self::Html,
            "css" => Self::Css,
            "sh" | "bash" | "zsh" => Self::Bash,
            _ => Self::Text,
        }
    }

    pub fn from_path(path: &Path) -> Self {
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        Self::from_extension(ext)
    }

    pub fn extension(self) -> &'static str {
        match self {
            Self::Rust => "rs",
            Self::JavaScript => "js",
            Self::TypeScript => "ts",
            Self::Tsx => "tsx",
            Self::Python => "py",
            Self::Ruby => "rb",
            Self::Go => "go",
            Self::Markdown => "md",
            Self::Toml => "toml",
            Self::Json => "json",
            Self::Yaml => "yml",
            Self::Html => "html",
            Self::Css => "css",
            Self::C => "c",
            Self::Cpp => "cpp",
            Self::Java => "java",
            Self::Bash => "sh",
            Self::Text => "txt",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn from_extension_rust() {
        assert_eq!(Language::from_extension("rs"), Language::Rust);
    }

    #[test]
    fn from_extension_javascript() {
        assert_eq!(Language::from_extension("js"), Language::JavaScript);
    }

    #[test]
    fn from_extension_typescript() {
        assert_eq!(Language::from_extension("ts"), Language::TypeScript);
    }

    #[test]
    fn from_extension_tsx() {
        assert_eq!(Language::from_extension("tsx"), Language::Tsx);
    }

    #[test]
    fn from_extension_python() {
        assert_eq!(Language::from_extension("py"), Language::Python);
    }

    #[test]
    fn from_extension_c_header() {
        assert_eq!(Language::from_extension("h"), Language::C);
    }

    #[test]
    fn from_extension_cpp_variants() {
        assert_eq!(Language::from_extension("cpp"), Language::Cpp);
        assert_eq!(Language::from_extension("cc"), Language::Cpp);
        assert_eq!(Language::from_extension("cxx"), Language::Cpp);
        assert_eq!(Language::from_extension("hpp"), Language::Cpp);
    }

    #[test]
    fn from_extension_yaml_variants() {
        assert_eq!(Language::from_extension("yaml"), Language::Yaml);
        assert_eq!(Language::from_extension("yml"), Language::Yaml);
    }

    #[test]
    fn from_extension_bash_variants() {
        assert_eq!(Language::from_extension("sh"), Language::Bash);
        assert_eq!(Language::from_extension("bash"), Language::Bash);
        assert_eq!(Language::from_extension("zsh"), Language::Bash);
    }

    #[test]
    fn from_extension_unknown_is_text() {
        assert_eq!(Language::from_extension("xyz"), Language::Text);
        assert_eq!(Language::from_extension(""), Language::Text);
    }

    #[test]
    fn from_path_extracts_extension() {
        assert_eq!(Language::from_path(Path::new("src/main.rs")), Language::Rust);
        assert_eq!(Language::from_path(Path::new("app.tsx")), Language::Tsx);
        assert_eq!(Language::from_path(Path::new("Makefile")), Language::Text);
    }

    #[test]
    fn name_returns_lowercase_string() {
        assert_eq!(Language::Rust.name(), "rust");
        assert_eq!(Language::JavaScript.name(), "javascript");
        assert_eq!(Language::Text.name(), "text");
    }

    #[test]
    fn extension_roundtrip() {
        for lang in [
            Language::Rust, Language::JavaScript, Language::TypeScript,
            Language::Python, Language::Go, Language::Markdown,
            Language::Toml, Language::Json, Language::Html, Language::Css,
            Language::C, Language::Java, Language::Bash, Language::Text,
        ] {
            let ext = lang.extension();
            assert_eq!(Language::from_extension(ext), lang, "roundtrip failed for {lang:?}");
        }
    }

    #[test]
    fn default_is_text() {
        assert_eq!(Language::default(), Language::Text);
    }
}
