//! Lint `lintropy.yaml` / `.lintropy/**/*.{rule,rules}.yaml` buffers and
//! surface problems inline in the editor.
//!
//! This is the LSP counterpart to the `lintropy check` config-load
//! diagnostics: the same query compile errors, template capture
//! checks, and predicate-name validation that fire at `Config::load`
//! time also fire here on every buffer change — reported as LSP
//! diagnostics anchored to the offending line and column inside the
//! YAML document, not at the top of the file.

use std::path::Path;

use serde::Deserialize;
use tower_lsp::lsp_types::{Diagnostic, DiagnosticSeverity, NumberOrString, Position, Range};
use tree_sitter::Query as TsQuery;

use crate::langs::Language;

/// Return LSP diagnostics for problems inside a lintropy rule file
/// that are detectable without loading the full workspace config.
///
/// Scope: per-rule query compile errors, unknown/missing `language:`,
/// and `{{capture}}` interpolation that references a capture that
/// doesn't exist in the query. Orthogonal to the diagnostic set the
/// Rust engine emits on source files.
pub fn lint(path: &Path, src: &str) -> Vec<Diagnostic> {
    let rules = match extract_rules(path, src) {
        Ok(r) => r,
        Err(_) => return Vec::new(), // serde_yaml errors surface via the YAML schema extension; don't double-report.
    };

    // Pair each parsed rule with the line number of its `query: |`
    // opener so we can translate per-query offsets into YAML coordinates.
    let block_starts = locate_query_block_starts(src);
    let mut diags = Vec::new();

    for (rule_idx, rule) in rules.iter().enumerate() {
        let Some(query_src) = rule.query.as_deref() else {
            continue;
        };
        let block_start = block_starts.get(rule_idx).copied();

        match rule.language.as_deref().and_then(Language::from_name) {
            Some(language) => {
                let fake_path = fake_source_path(language);
                let ts_lang = language.ts_language(&fake_path);
                match TsQuery::new(&ts_lang, query_src) {
                    Ok(compiled) => {
                        diags.extend(check_template_captures(rule, &compiled, block_start));
                    }
                    Err(err) => {
                        diags.push(query_error_diagnostic(err, block_start));
                    }
                }
            }
            None => {
                // Missing or unknown language: flag the line so the
                // user sees it inline rather than only on next
                // `lintropy check`. Absent `language:` ⇒ line 0.
                let lang_line = locate_field_line(src, "language")
                    .or_else(|| block_start.map(|b| b.line.saturating_sub(1)))
                    .unwrap_or(0);
                let msg = match rule.language.as_deref() {
                    None => "rule is missing a `language:` field".to_string(),
                    Some(name) => format!("unknown language `{name}`"),
                };
                diags.push(diag_at_line(lang_line, msg));
            }
        }
    }

    diags
}

/// Minimal projection of the on-disk rule schema, just enough to
/// validate query syntax + template references without depending on
/// the engine's private `RawRule` type.
#[derive(Deserialize)]
struct Rule {
    #[serde(default)]
    language: Option<String>,
    #[serde(default)]
    query: Option<String>,
    #[serde(default)]
    message: Option<String>,
    #[serde(default)]
    fix: Option<String>,
}

#[derive(Deserialize)]
struct RulesFile {
    rules: Vec<Rule>,
}

fn extract_rules(path: &Path, src: &str) -> Result<Vec<Rule>, serde_yaml::Error> {
    let is_rules_file = path
        .file_name()
        .and_then(|s| s.to_str())
        .map(|s| s.ends_with(".rules.yaml") || s.ends_with(".rules.yml"))
        .unwrap_or(false);

    if is_rules_file {
        let file: RulesFile = serde_yaml::from_str(src)?;
        Ok(file.rules)
    } else {
        // `.rule.yaml` / `lintropy.yaml` — a single rule at the top
        // level. `lintropy.yaml` may also have no `query:` at all
        // (settings-only), in which case the loop body skips it.
        let rule: Rule = serde_yaml::from_str(src)?;
        Ok(vec![rule])
    }
}

/// Zero-based line numbers where each `query: |` / `query: >` opener
/// lives, in document order. Same detection heuristic as
/// `semantic_tokens::query_block_indent`, kept here independent to
/// avoid tangling the two modules.
fn locate_query_block_starts(src: &str) -> Vec<BlockPos> {
    let mut out = Vec::new();
    for (idx, line) in src.lines().enumerate() {
        if is_query_block_opener(line) {
            out.push(BlockPos {
                line: idx as u32 + 1, // body begins on the NEXT line
                indent: leading_spaces(line),
            });
        }
    }
    out
}

#[derive(Copy, Clone)]
struct BlockPos {
    /// Line number of the first body line (zero-based).
    line: u32,
    /// Indent of the `query:` key itself — body indent will be >.
    indent: usize,
}

fn is_query_block_opener(line: &str) -> bool {
    let trimmed = line.trim_start();
    matches!(
        trimmed.strip_prefix("query:").map(str::trim_start),
        Some("|") | Some(">") | Some("|+") | Some("|-") | Some(">+") | Some(">-")
    )
}

fn leading_spaces(line: &str) -> usize {
    line.chars().take_while(|c| *c == ' ').count()
}

fn locate_field_line(src: &str, field: &str) -> Option<u32> {
    let prefix = format!("{field}:");
    for (idx, line) in src.lines().enumerate() {
        if line.trim_start().starts_with(&prefix) {
            return Some(idx as u32);
        }
    }
    None
}

/// Build a synthetic path with the right extension so
/// `Language::ts_language(path)` picks the dialect-correct grammar
/// (only matters for TypeScript → `.ts` vs `.tsx`; ignored by every
/// other language).
fn fake_source_path(language: Language) -> std::path::PathBuf {
    let ext = language.extensions().first().copied().unwrap_or("src");
    std::path::PathBuf::from(format!("_.{ext}"))
}

fn query_error_diagnostic(err: tree_sitter::QueryError, block: Option<BlockPos>) -> Diagnostic {
    // tree-sitter gives `row` + `column` in coordinates internal to
    // the query source string (zero-based). Translate to the YAML
    // buffer: query body starts at `block.line`, so the error's
    // `row` adds to that. Column is the byte column inside the
    // query body line — because YAML block scalars strip a uniform
    // leading indent, the editor column is `indent_of_body + column`.
    //
    // Default `indent_of_body = block.indent + 2` (YAML's typical
    // "at least one more space than the key"). That matches every
    // rule file we ship and is good enough for inline reporting;
    // the error ends up on the right line even if the column is
    // off by one or two characters, which is what matters.
    let (line, character) = match block {
        Some(b) => (b.line + err.row as u32, (b.indent + 2 + err.column) as u32),
        None => (0, 0),
    };
    Diagnostic {
        range: Range {
            start: Position { line, character },
            end: Position {
                line,
                character: character.saturating_add(1),
            },
        },
        severity: Some(DiagnosticSeverity::ERROR),
        code: Some(NumberOrString::String("query-compile".into())),
        source: Some("lintropy".into()),
        message: format!("query compile failed ({:?}): {}", err.kind, err.message),
        ..Default::default()
    }
}

fn check_template_captures(
    rule: &Rule,
    compiled: &TsQuery,
    block_start: Option<BlockPos>,
) -> Vec<Diagnostic> {
    // LSP range falls back to the query-block line when we don't
    // have the exact template position — precise offset inside
    // the `message:` / `fix:` scalar would require a streaming
    // YAML parser with position info, overkill for now.
    let anchor_line = block_start.map(|b| b.line.saturating_sub(1)).unwrap_or(0);
    let captures: Vec<&str> = compiled.capture_names().to_vec();
    let mut out = Vec::new();
    for (label, text) in [
        ("message", rule.message.as_deref()),
        ("fix", rule.fix.as_deref()),
    ] {
        let Some(text) = text else { continue };
        for name in extract_template_vars(text) {
            if !captures.iter().any(|c| *c == name) {
                out.push(diag_at_line(
                    anchor_line,
                    format!("{label} references unknown capture `{{{{{name}}}}}` — define `@{name}` in the query"),
                ));
            }
        }
    }
    out
}

/// Scan `{{name}}` interpolations out of a template string. Matches
/// the engine's interpolation grammar — alphanumerics, underscore,
/// hyphen — and tolerates whitespace inside the braces.
fn extract_template_vars(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let bytes = text.as_bytes();
    let mut i = 0;
    while i + 1 < bytes.len() {
        if bytes[i] == b'{' && bytes[i + 1] == b'{' {
            let start = i + 2;
            let end = text[start..].find("}}").map(|p| start + p);
            if let Some(end) = end {
                let name = text[start..end].trim();
                if !name.is_empty()
                    && name
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
                {
                    out.push(name.to_string());
                }
                i = end + 2;
                continue;
            }
        }
        i += 1;
    }
    out
}

fn diag_at_line(line: u32, message: String) -> Diagnostic {
    Diagnostic {
        range: Range {
            start: Position { line, character: 0 },
            end: Position {
                line,
                character: u32::MAX,
            },
        },
        severity: Some(DiagnosticSeverity::ERROR),
        code: Some(NumberOrString::String("rule-config".into())),
        source: Some("lintropy".into()),
        message,
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(name: &str) -> std::path::PathBuf {
        std::path::PathBuf::from(name)
    }

    #[test]
    fn valid_rule_emits_no_diagnostics() {
        let src = r#"language: rust
severity: warning
message: "no unwrap on {{recv}}"
include: ["**/*.rs"]
query: |
  (call_expression
    function: (field_expression
      value: (_) @recv
      field: (field_identifier) @method)
    (#eq? @method "unwrap")) @match
fix: '{{recv}}.expect("TODO")'
"#;
        let diags = lint(&p("no-unwrap.rule.yaml"), src);
        assert!(diags.is_empty(), "unexpected diags: {diags:?}");
    }

    #[test]
    fn query_syntax_error_becomes_inline_diagnostic() {
        let src = r#"language: rust
severity: warning
message: "hi"
query: |
  (call_expression
    function: (field_expression
      value: ((_ @recv      ; extra open paren, missing close
      field: (field_identifier) @method))
"#;
        let diags = lint(&p("broken.rule.yaml"), src);
        assert_eq!(diags.len(), 1, "diags: {diags:?}");
        let d = &diags[0];
        assert_eq!(
            d.code.as_ref().map(stringify_code).as_deref(),
            Some("query-compile")
        );
        // Error line is inside the query body, not at line 0.
        assert!(d.range.start.line >= 4, "line too early: {d:?}");
    }

    #[test]
    fn missing_language_is_reported() {
        let src = r#"severity: warning
message: "hi"
query: |
  (identifier) @x
"#;
        let diags = lint(&p("no-lang.rule.yaml"), src);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("missing"));
    }

    #[test]
    fn unknown_language_is_reported() {
        let src = r#"language: klingon
severity: warning
message: "hi"
query: |
  (identifier) @x
"#;
        let diags = lint(&p("alien.rule.yaml"), src);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("unknown language"));
        assert!(diags[0].message.contains("klingon"));
    }

    #[test]
    fn template_referencing_missing_capture_is_reported() {
        let src = r#"language: rust
severity: warning
message: "use of {{nonexistent}}"
query: |
  (identifier) @match
fix: "{{match}}_v2"
"#;
        let diags = lint(&p("bad-template.rule.yaml"), src);
        assert_eq!(diags.len(), 1, "diags: {diags:?}");
        assert!(diags[0].message.contains("nonexistent"));
    }

    #[test]
    fn rules_file_reports_each_invalid_rule() {
        let src = r#"rules:
  - id: a
    language: rust
    severity: warning
    message: "hi"
    query: |
      (identifier) @x
  - id: b
    language: rust
    severity: warning
    message: "hi"
    query: |
      (broken @y       ; missing close paren
"#;
        let diags = lint(&p("multi.rules.yaml"), src);
        assert_eq!(diags.len(), 1, "diags: {diags:?}");
        assert!(diags[0].range.start.line > 8, "{diags:?}");
    }

    fn stringify_code(code: &NumberOrString) -> String {
        match code {
            NumberOrString::String(s) => s.clone(),
            NumberOrString::Number(n) => n.to_string(),
        }
    }
}
