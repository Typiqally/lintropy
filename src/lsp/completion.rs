//! Completion provider for lintropy rule buffers.
//!
//! Scope: `lintropy.yaml` / `*.rule.yaml` / `*.rules.yaml`. Three
//! contexts are recognised:
//!
//! 1. `language:` scalar value — yields the registered languages.
//! 2. Inside a `query: |` block body — yields tree-sitter node kinds
//!    for the rule's language, plus the fixed predicate vocabulary
//!    and any `@captures` already introduced in the buffer.
//! 3. Inside a `{{…}}` interpolation inside any scalar — yields
//!    capture names scraped from nearby `query:` blocks.
//!
//! The context detection is line-oriented and tolerates half-typed
//! input: it never parses the whole YAML and stays fast enough to
//! run on every keystroke.

use std::collections::BTreeSet;
use std::path::Path;

use tower_lsp::lsp_types::{
    CompletionItem, CompletionItemKind, Documentation, InsertTextFormat, MarkupContent, MarkupKind,
    Position,
};

use crate::langs::Language;

use super::position::position_to_byte;

/// Entry point called from the LSP `textDocument/completion` handler.
pub fn complete(path: &Path, src: &str, pos: Position) -> Vec<CompletionItem> {
    if !is_rule_file(path) {
        return Vec::new();
    }
    let cursor = position_to_byte(src, pos);
    let rule = rule_range_containing(src, cursor);
    let rule_src = &src[rule.0..rule.1];
    match detect_context(src, pos) {
        Context::LanguageValue => language_items(),
        Context::Template => template_items(rule_src),
        Context::QueryBody { language } => query_items(language, rule_src),
        Context::None => Vec::new(),
    }
}

/// Return the `[start, end)` byte range of the rule containing `cursor`.
///
/// Single-rule files (`lintropy.yaml`, `*.rule.yaml`) have one implicit
/// rule spanning the whole buffer. `*.rules.yaml` groups rules under a
/// `rules:` list — each `- …` list item is one rule and its body runs
/// until the next sibling `- …` or EOF. Used to scope capture scans so
/// `{{capture}}` templates don't pick up names from a different rule.
fn rule_range_containing(src: &str, cursor: usize) -> (usize, usize) {
    let mut starts: Vec<usize> = Vec::new();
    let mut list_indent: Option<usize> = None;
    let mut byte = 0usize;
    for line in src.split_inclusive('\n') {
        let without_nl = line.strip_suffix('\n').unwrap_or(line);
        let trimmed = without_nl.trim_start();
        let indent = without_nl.len() - trimmed.len();
        let is_item = trimmed.starts_with("- ") || trimmed == "-";
        if is_item {
            match list_indent {
                None => {
                    list_indent = Some(indent);
                    starts.push(byte);
                }
                Some(li) if li == indent => starts.push(byte),
                _ => {} // nested list item; ignore
            }
        }
        byte += line.len();
    }
    if starts.is_empty() {
        return (0, src.len());
    }
    let mut start = 0usize;
    let mut end = src.len();
    for (i, &s) in starts.iter().enumerate() {
        if s <= cursor {
            start = s;
            end = starts.get(i + 1).copied().unwrap_or(src.len());
        }
    }
    (start, end)
}

fn is_rule_file(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
        return false;
    };
    name == "lintropy.yaml"
        || name.ends_with(".rule.yaml")
        || name.ends_with(".rule.yml")
        || name.ends_with(".rules.yaml")
        || name.ends_with(".rules.yml")
}

enum Context {
    None,
    LanguageValue,
    Template,
    QueryBody { language: Option<Language> },
}

fn detect_context(src: &str, pos: Position) -> Context {
    let cursor = position_to_byte(src, pos);
    let line_start = src[..cursor].rfind('\n').map(|i| i + 1).unwrap_or(0);
    let line_prefix = &src[line_start..cursor];

    if line_prefix.trim_start().starts_with("language:") {
        return Context::LanguageValue;
    }

    if inside_open_template(src, cursor) {
        return Context::Template;
    }

    if cursor_in_query_body(src, cursor) {
        return Context::QueryBody {
            language: nearest_language_before(src, cursor),
        };
    }

    Context::None
}

fn inside_open_template(src: &str, cursor: usize) -> bool {
    // `{{…}}` templates live inside a single YAML scalar. Restrict the
    // search to the current line so an earlier line's unclosed brace
    // doesn't bleed through.
    let line_start = src[..cursor].rfind('\n').map(|i| i + 1).unwrap_or(0);
    let region = &src[line_start..cursor];
    let Some(open) = region.rfind("{{") else {
        return false;
    };
    !region[open..].contains("}}")
}

fn nearest_language_before(src: &str, cursor: usize) -> Option<Language> {
    for line in src[..cursor].lines().rev() {
        let trimmed = line.trim_start();
        // Under a `.rules.yaml` list the first key of a rule starts with
        // `- ` (`  - language: rust`); peel that off before matching.
        let trimmed = trimmed.strip_prefix("- ").unwrap_or(trimmed);
        if let Some(rest) = trimmed.strip_prefix("language:") {
            let raw = rest.trim();
            let unquoted = raw.trim_matches(|c| c == '"' || c == '\'');
            if unquoted.is_empty() {
                continue;
            }
            return Language::from_name(unquoted);
        }
    }
    None
}

fn cursor_in_query_body(src: &str, cursor: usize) -> bool {
    let cursor_line = src[..cursor].matches('\n').count();
    let mut in_body = false;
    let mut opener_indent = 0usize;

    for (i, line) in src.lines().enumerate() {
        if in_body {
            let trimmed = line.trim_start();
            let indent = line.len() - trimmed.len();
            if trimmed.is_empty() {
                if i == cursor_line {
                    return true;
                }
                continue;
            }
            if indent <= opener_indent {
                in_body = false;
                // fall through — this line might itself open a new query.
            } else {
                if i == cursor_line {
                    return true;
                }
                continue;
            }
        }
        if let Some(indent) = query_opener_indent(line) {
            if i == cursor_line {
                return false; // still on the opener line itself.
            }
            in_body = true;
            opener_indent = indent;
        }
    }
    false
}

fn query_opener_indent(line: &str) -> Option<usize> {
    let trimmed = line.trim_start();
    let indent = line.len() - trimmed.len();
    let rest = trimmed.strip_prefix("query:")?.trim_start();
    matches!(rest, "|" | ">" | "|+" | "|-" | ">+" | ">-").then_some(indent)
}

fn language_items() -> Vec<CompletionItem> {
    ALL_LANGUAGE_NAMES
        .iter()
        .filter_map(|name| Language::from_name(name))
        .map(|lang| CompletionItem {
            label: lang.name().to_string(),
            kind: Some(CompletionItemKind::ENUM_MEMBER),
            detail: Some(format!(".{}", lang.extensions()[0])),
            ..Default::default()
        })
        .collect()
}

const ALL_LANGUAGE_NAMES: &[&str] = &["rust", "go", "python", "typescript"];

fn query_items(language: Option<Language>, src: &str) -> Vec<CompletionItem> {
    let mut out = predicate_items();
    if let Some(lang) = language {
        out.extend(node_kind_items(lang));
        out.extend(field_name_items(lang));
    }
    out.extend(capture_items(src));
    out
}

fn template_items(src: &str) -> Vec<CompletionItem> {
    collect_capture_names(src)
        .into_iter()
        .map(|name| CompletionItem {
            label: name.clone(),
            kind: Some(CompletionItemKind::VARIABLE),
            detail: Some("capture".into()),
            insert_text: Some(name),
            ..Default::default()
        })
        .collect()
}

fn capture_items(src: &str) -> Vec<CompletionItem> {
    collect_capture_names(src)
        .into_iter()
        .map(|name| CompletionItem {
            label: format!("@{name}"),
            kind: Some(CompletionItemKind::VARIABLE),
            detail: Some("capture".into()),
            insert_text: Some(format!("@{name}")),
            ..Default::default()
        })
        .collect()
}

fn collect_capture_names(src: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let bytes = src.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'@' {
            let start = i + 1;
            let mut end = start;
            while end < bytes.len() && is_capture_char(bytes[end]) {
                end += 1;
            }
            if end > start {
                out.insert(src[start..end].to_string());
            }
            i = end.max(i + 1);
            continue;
        }
        i += 1;
    }
    out
}

fn is_capture_char(b: u8) -> bool {
    matches!(b, b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'_' | b'-')
}

fn predicate_items() -> Vec<CompletionItem> {
    PREDICATES
        .iter()
        .map(|p| CompletionItem {
            label: format!("#{}", p.name),
            kind: Some(CompletionItemKind::FUNCTION),
            detail: Some("predicate".into()),
            documentation: Some(Documentation::MarkupContent(MarkupContent {
                kind: MarkupKind::Markdown,
                value: p.doc.to_string(),
            })),
            insert_text: Some(p.snippet.to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        })
        .collect()
}

struct Predicate {
    name: &'static str,
    doc: &'static str,
    /// LSP snippet body (no leading `#` — clients insert what the label
    /// starts with, so the snippet repeats the whole thing). `$1`, `$2`,
    /// `$0` are tabstops in LSP snippet format.
    snippet: &'static str,
}

/// Built-in tree-sitter predicates plus lintropy's custom vocabulary.
/// Kept in sync with `core::predicates::parse_general_predicates_by_pattern`.
const PREDICATES: &[Predicate] = &[
    Predicate {
        name: "eq?",
        doc: "Match if the capture equals the given string.",
        snippet: "#eq? @${1:capture} \"${2:value}\"",
    },
    Predicate {
        name: "not-eq?",
        doc: "Negation of `#eq?`.",
        snippet: "#not-eq? @${1:capture} \"${2:value}\"",
    },
    Predicate {
        name: "match?",
        doc: "Match if the capture matches the given regex.",
        snippet: "#match? @${1:capture} \"${2:regex}\"",
    },
    Predicate {
        name: "not-match?",
        doc: "Negation of `#match?`.",
        snippet: "#not-match? @${1:capture} \"${2:regex}\"",
    },
    Predicate {
        name: "any-of?",
        doc: "Match if the capture equals any listed string.",
        snippet: "#any-of? @${1:capture} \"${2:a}\" \"${3:b}\"",
    },
    Predicate {
        name: "not-any-of?",
        doc: "Negation of `#any-of?`.",
        snippet: "#not-any-of? @${1:capture} \"${2:a}\" \"${3:b}\"",
    },
    Predicate {
        name: "has-ancestor?",
        doc: "lintropy: capture has an ancestor of one of the given kinds.",
        snippet: "#has-ancestor? @${1:capture} \"${2:kind}\"",
    },
    Predicate {
        name: "not-has-ancestor?",
        doc: "Negation of `#has-ancestor?`.",
        snippet: "#not-has-ancestor? @${1:capture} \"${2:kind}\"",
    },
    Predicate {
        name: "has-parent?",
        doc: "lintropy: capture's immediate parent is one of the given kinds.",
        snippet: "#has-parent? @${1:capture} \"${2:kind}\"",
    },
    Predicate {
        name: "not-has-parent?",
        doc: "Negation of `#has-parent?`.",
        snippet: "#not-has-parent? @${1:capture} \"${2:kind}\"",
    },
    Predicate {
        name: "has-sibling?",
        doc: "lintropy: capture has a sibling of one of the given kinds.",
        snippet: "#has-sibling? @${1:capture} \"${2:kind}\"",
    },
    Predicate {
        name: "not-has-sibling?",
        doc: "Negation of `#has-sibling?`.",
        snippet: "#not-has-sibling? @${1:capture} \"${2:kind}\"",
    },
    Predicate {
        name: "has-preceding-comment?",
        doc: "lintropy: capture is preceded by a comment matching the regex.",
        snippet: "#has-preceding-comment? @${1:capture} \"${2:regex}\"",
    },
    Predicate {
        name: "not-has-preceding-comment?",
        doc: "Negation of `#has-preceding-comment?`.",
        snippet: "#not-has-preceding-comment? @${1:capture} \"${2:regex}\"",
    },
];

fn field_name_items(lang: Language) -> Vec<CompletionItem> {
    let fake = std::path::PathBuf::from(format!(
        "_.{}",
        lang.extensions().first().copied().unwrap_or("src")
    ));
    let ts = lang.ts_language(&fake);
    let count = ts.field_count();
    let mut seen = BTreeSet::new();
    let mut out = Vec::with_capacity(count);
    // tree-sitter field ids are 1-based; id 0 is the invalid sentinel.
    for id in 1..=count {
        let Some(name) = ts.field_name_for_id(id as u16) else {
            continue;
        };
        if name.is_empty() || !seen.insert(name.to_string()) {
            continue;
        }
        out.push(CompletionItem {
            label: format!("{name}:"),
            kind: Some(CompletionItemKind::FIELD),
            detail: Some(format!("{} field", lang.name())),
            insert_text: Some(format!("{name}: ")),
            ..Default::default()
        });
    }
    out
}

fn node_kind_items(lang: Language) -> Vec<CompletionItem> {
    let fake = std::path::PathBuf::from(format!(
        "_.{}",
        lang.extensions().first().copied().unwrap_or("src")
    ));
    let ts = lang.ts_language(&fake);
    let count = ts.node_kind_count();
    let mut out = Vec::with_capacity(count);
    let mut seen = BTreeSet::new();
    for id in 0..count {
        let id = id as u16;
        if !ts.node_kind_is_named(id) || !ts.node_kind_is_visible(id) {
            continue;
        }
        let Some(kind) = ts.node_kind_for_id(id) else {
            continue;
        };
        if kind.is_empty() || kind == "ERROR" {
            continue;
        }
        if !kind.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            continue;
        }
        if !seen.insert(kind.to_string()) {
            continue;
        }
        out.push(CompletionItem {
            label: kind.to_string(),
            kind: Some(CompletionItemKind::CLASS),
            detail: Some(format!("{} node", lang.name())),
            ..Default::default()
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pos(line: u32, character: u32) -> Position {
        Position { line, character }
    }

    fn p(name: &str) -> std::path::PathBuf {
        std::path::PathBuf::from(name)
    }

    #[test]
    fn ignores_non_rule_files() {
        let src = "language: rust\n";
        let items = complete(&p("README.md"), src, pos(0, 10));
        assert!(items.is_empty());
    }

    #[test]
    fn language_value_completes_known_languages() {
        let src = "language: \nquery: |\n  (identifier) @x\n";
        let items = complete(&p("r.rule.yaml"), src, pos(0, 10));
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        assert!(labels.contains(&"rust"), "labels: {labels:?}");
    }

    #[test]
    fn inside_query_body_offers_node_kinds_and_predicates() {
        let src = "language: rust\nquery: |\n  (\n";
        let items = complete(&p("r.rule.yaml"), src, pos(2, 3));
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        assert!(labels.contains(&"#eq?"), "predicate missing: {labels:?}");
        assert!(
            labels.contains(&"call_expression"),
            "rust node kind missing: {labels:?}"
        );
    }

    #[test]
    fn inside_query_body_includes_existing_captures() {
        let src = "language: rust\nquery: |\n  (identifier) @needle\n  \n";
        let items = complete(&p("r.rule.yaml"), src, pos(3, 2));
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        assert!(labels.contains(&"@needle"), "labels: {labels:?}");
    }

    #[test]
    fn inside_template_braces_yields_captures() {
        let src = "language: rust\nmessage: \"use {{\"\nquery: |\n  (identifier) @foo\n";
        // cursor right after the `{{` on line 1
        let items = complete(&p("r.rule.yaml"), src, pos(1, 16));
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        assert!(labels.contains(&"foo"), "labels: {labels:?}");
    }

    #[test]
    fn outside_known_contexts_returns_nothing() {
        let src = "language: rust\nseverity: warning\n";
        let items = complete(&p("r.rule.yaml"), src, pos(1, 17));
        assert!(items.is_empty());
    }

    #[test]
    fn query_body_includes_grammar_field_names() {
        let src = "language: rust\nquery: |\n  (call_expression\n    \n";
        let items = complete(&p("r.rule.yaml"), src, pos(3, 4));
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        assert!(
            labels.contains(&"function:"),
            "expected function: field, got {labels:?}"
        );
    }

    #[test]
    fn predicates_insert_as_snippets() {
        let src = "language: rust\nquery: |\n  (\n";
        let items = complete(&p("r.rule.yaml"), src, pos(2, 3));
        let eq = items.iter().find(|i| i.label == "#eq?").expect("eq? item");
        assert_eq!(eq.insert_text_format, Some(InsertTextFormat::SNIPPET));
        let snippet = eq.insert_text.as_deref().unwrap_or("");
        assert!(
            snippet.contains("$1") || snippet.contains("${1"),
            "{snippet}"
        );
    }

    #[test]
    fn template_captures_scoped_to_containing_rule() {
        let src = "rules:\n\
                   \x20 - language: rust\n\
                   \x20   query: |\n\
                   \x20     (identifier) @first\n\
                   \x20   message: \"hit {{\"\n\
                   \x20 - language: rust\n\
                   \x20   query: |\n\
                   \x20     (identifier) @second\n\
                   \x20   message: \"hit\"\n";
        // cursor right after `{{` on line 4 (rule A's message)
        let items = complete(&p("r.rules.yaml"), src, pos(4, 20));
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        assert!(labels.contains(&"first"), "labels: {labels:?}");
        assert!(
            !labels.contains(&"second"),
            "rule-B capture leaked: {labels:?}"
        );
    }

    #[test]
    fn detects_query_body_under_rules_list() {
        let src = "rules:\n  - language: rust\n    query: |\n      (\n";
        let items = complete(&p("r.rules.yaml"), src, pos(3, 7));
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        assert!(labels.contains(&"call_expression"), "labels: {labels:?}");
    }
}
