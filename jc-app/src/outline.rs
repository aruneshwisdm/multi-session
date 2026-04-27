use crate::language::Language;
use tree_sitter::{Language as TsLanguage, Parser, Query, QueryCursor, StreamingIterator};

pub struct OutlineItem {
    pub label: String,
    pub name: String,
    pub context: String,
    pub line: u32,
    pub depth: usize,
    pub parent: Option<usize>,
    pub byte_range: std::ops::Range<usize>,
}

pub fn compute_outline(text: &str, language: Language) -> Vec<OutlineItem> {
    let Some((ts_language, query_source)) = language_and_query(language) else {
        return Vec::new();
    };

    let mut parser = Parser::new();
    if parser.set_language(&ts_language).is_err() {
        return Vec::new();
    }

    let Some(tree) = parser.parse(text, None) else {
        return Vec::new();
    };

    let Ok(query) = Query::new(&ts_language, query_source) else {
        return Vec::new();
    };

    let name_idx = query.capture_index_for_name("name");
    let context_idx = query.capture_index_for_name("context");
    let item_idx = query.capture_index_for_name("item");

    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(&query, tree.root_node(), text.as_bytes());

    let mut raw_items: Vec<(String, String, u32, std::ops::Range<usize>)> = Vec::new();

    while let Some(m) = {
        matches.advance();
        matches.get()
    } {
        let mut name_parts = Vec::new();
        let mut context_parts = Vec::new();
        let mut item_range: Option<std::ops::Range<usize>> = None;
        let mut item_line: Option<u32> = None;

        for capture in m.captures {
            let text_slice = capture.node.utf8_text(text.as_bytes()).unwrap_or("");
            if Some(capture.index) == item_idx {
                item_range = Some(capture.node.byte_range());
                item_line = Some(capture.node.start_position().row as u32);
            } else if Some(capture.index) == name_idx {
                name_parts.push(text_slice.to_string());
            } else if Some(capture.index) == context_idx {
                context_parts.push(text_slice.to_string());
            }
        }

        let Some(range) = item_range else { continue };
        let line = item_line.unwrap_or(0);
        let name = name_parts.join(" ");
        let context = context_parts.join(" ");

        if !name.is_empty() {
            raw_items.push((name, context, line, range));
        }
    }

    let mut items: Vec<OutlineItem> = Vec::with_capacity(raw_items.len());
    let mut stack: Vec<usize> = Vec::new();

    for (name, context, line, range) in raw_items {
        while let Some(&top) = stack.last() {
            if items[top].byte_range.end < range.end {
                stack.pop();
            } else {
                break;
            }
        }

        let parent = stack.last().copied();
        let depth = stack.len();
        let label = if context.is_empty() { name.clone() } else { format!("{context} {name}") };

        stack.push(items.len());
        items.push(OutlineItem { label, name, context, line, depth, parent, byte_range: range });
    }

    items
}

pub fn breadcrumb_at_byte(outline: &[OutlineItem], byte_offset: usize) -> Vec<&OutlineItem> {
    let mut best: Option<usize> = None;
    for (i, item) in outline.iter().enumerate() {
        if item.byte_range.start <= byte_offset && byte_offset < item.byte_range.end {
            match best {
                Some(b) if outline[b].depth < item.depth => best = Some(i),
                None => best = Some(i),
                _ => {}
            }
        }
    }

    let mut chain = Vec::new();
    let mut idx = best;
    while let Some(i) = idx {
        chain.push(&outline[i]);
        idx = outline[i].parent;
    }
    chain.reverse();
    chain
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unsupported_language_returns_empty() {
        let items = compute_outline("hello world", Language::Text);
        assert!(items.is_empty());
    }

    #[test]
    fn rust_outline_finds_functions() {
        let src = "fn main() {\n    println!(\"hello\");\n}\n\nfn helper() {}\n";
        let items = compute_outline(src, Language::Rust);
        let names: Vec<&str> = items.iter().map(|i| i.name.as_str()).collect();
        assert!(names.contains(&"main"), "should find main: {names:?}");
        assert!(names.contains(&"helper"), "should find helper: {names:?}");
    }

    #[test]
    fn rust_outline_finds_structs_and_impls() {
        let src = "struct Foo {}\nimpl Foo {\n    fn bar(&self) {}\n}\n";
        let items = compute_outline(src, Language::Rust);
        let names: Vec<&str> = items.iter().map(|i| i.name.as_str()).collect();
        assert!(names.contains(&"Foo"), "should find Foo: {names:?}");
        assert!(names.contains(&"bar"), "should find bar: {names:?}");
    }

    #[test]
    fn python_outline_finds_classes_and_functions() {
        let src = "class Foo:\n    def bar(self):\n        pass\n\ndef baz():\n    pass\n";
        let items = compute_outline(src, Language::Python);
        let names: Vec<&str> = items.iter().map(|i| i.name.as_str()).collect();
        assert!(names.contains(&"Foo"), "should find Foo: {names:?}");
        assert!(names.contains(&"bar"), "should find bar: {names:?}");
        assert!(names.contains(&"baz"), "should find baz: {names:?}");
    }

    #[test]
    fn typescript_outline_finds_functions() {
        let src = "function hello(): void {}\nconst world: number = 42;\n";
        let items = compute_outline(src, Language::TypeScript);
        let names: Vec<&str> = items.iter().map(|i| i.name.as_str()).collect();
        assert!(names.contains(&"hello") || names.contains(&"world"),
            "should find at least one item: {names:?}");
    }

    #[test]
    fn markdown_outline_finds_headings() {
        let src = "# Title\n\n## Section A\n\n### Subsection\n\n## Section B\n";
        let items = compute_outline(src, Language::Markdown);
        assert!(items.len() >= 3, "should find at least 3 headings: {}", items.len());
    }

    #[test]
    fn outline_items_have_line_numbers() {
        let src = "fn first() {}\n\nfn second() {}\n";
        let items = compute_outline(src, Language::Rust);
        assert!(items.len() >= 2);
        assert_eq!(items[0].line, 0);
        assert!(items[1].line > 0);
    }

    #[test]
    fn impl_and_method_both_found() {
        let src = "struct Foo {}\nimpl Foo {\n    fn bar(&self) {}\n}\n";
        let items = compute_outline(src, Language::Rust);
        let names: Vec<&str> = items.iter().map(|i| i.name.as_str()).collect();
        assert!(names.contains(&"Foo"), "should find Foo: {names:?}");
        assert!(names.contains(&"bar"), "should find bar: {names:?}");
    }

    #[test]
    fn breadcrumb_at_byte_returns_chain() {
        let src = "impl Foo {\n    fn bar(&self) {\n        let x = 1;\n    }\n}\n";
        let items = compute_outline(src, Language::Rust);
        let offset = src.find("let x").unwrap();
        let crumbs = breadcrumb_at_byte(&items, offset);
        assert!(!crumbs.is_empty());
        let names: Vec<&str> = crumbs.iter().map(|i| i.name.as_str()).collect();
        assert!(names.contains(&"bar"), "breadcrumb should include bar: {names:?}");
    }

    #[test]
    fn breadcrumb_outside_items_empty() {
        let src = "fn main() {}\n";
        let items = compute_outline(src, Language::Rust);
        let crumbs = breadcrumb_at_byte(&items, src.len() + 100);
        assert!(crumbs.is_empty());
    }

    #[test]
    fn empty_source_returns_empty() {
        let items = compute_outline("", Language::Rust);
        assert!(items.is_empty());
    }
}

fn language_and_query(language: Language) -> Option<(TsLanguage, &'static str)> {
    match language {
        Language::Rust => {
            Some((tree_sitter_rust::LANGUAGE.into(), include_str!("outline_queries/rust.scm")))
        }
        Language::Markdown => {
            Some((tree_sitter_md::LANGUAGE.into(), include_str!("outline_queries/markdown.scm")))
        }
        Language::Python => {
            Some((tree_sitter_python::LANGUAGE.into(), include_str!("outline_queries/python.scm")))
        }
        Language::Go => {
            Some((tree_sitter_go::LANGUAGE.into(), include_str!("outline_queries/go.scm")))
        }
        Language::JavaScript => Some((
            tree_sitter_javascript::LANGUAGE.into(),
            include_str!("outline_queries/javascript.scm"),
        )),
        Language::TypeScript => Some((
            tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            include_str!("outline_queries/typescript.scm"),
        )),
        Language::Tsx => Some((
            tree_sitter_typescript::LANGUAGE_TSX.into(),
            include_str!("outline_queries/javascript.scm"),
        )),
        _ => None,
    }
}
