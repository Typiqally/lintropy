//! Semantic-token provider for lintropy YAML rule files.
//!
//! `.lintropy/*.{yaml,yml}` and `lintropy.yaml` embed tree-sitter `query`
//! DSL inside `query: |` block scalars. This module scans the YAML for
//! those blocks and tokenises the embedded DSL — captures (`@foo`),
//! predicates (`#has-ancestor?`), strings, numbers, comments, and node
//! kinds — then emits the LSP SemanticTokens delta-encoding so any
//! LSP-aware editor (VS Code / Cursor / JetBrains LSP4IJ / Neovim /
//! Helix / Zed) colours them identically.
//!
//! Replaces the standalone TextMate grammar that used to ship as an
//! extension contribution (VS Code) or a `.tmbundle` (JetBrains) — one
//! Rust tokeniser, one coloured experience, everywhere.

use regex::Regex;
use std::sync::OnceLock;

use tower_lsp::lsp_types::{
    SemanticToken, SemanticTokenModifier, SemanticTokenType, SemanticTokens, SemanticTokensLegend,
};

/// Token legend advertised in `InitializeResult.capabilities`. The
/// ordering is load-bearing: every emitted token indexes into this
/// array via `token_type`.
pub fn legend() -> SemanticTokensLegend {
    SemanticTokensLegend {
        token_types: vec![
            // Captures — picked `decorator` because `@foo` plays the
            // same role as a Python decorator or Java annotation and
            // every default JetBrains/VS Code scheme paints decorators
            // in a distinctive gold/yellow.
            SemanticTokenType::DECORATOR, // 0: capture `@foo`
            // Predicates behave like tree-sitter's built-in "macros".
            // `macro` maps to the Rust macro colour in default schemes
            // (purple / magenta), which reads as "meta-level" — matches
            // the role of `#eq?` / `#has-ancestor?` inside a query.
            SemanticTokenType::MACRO, // 1: predicate `#eq?`, `#not-has-ancestor?`
            // Node kinds identify AST *classes* — using `class` rather
            // than the generic `type` makes the default scheme paint
            // them in the class colour (turquoise / teal in Darcula)
            // instead of the dim built-in-type colour.
            SemanticTokenType::CLASS,   // 2: node kind `call_expression`
            SemanticTokenType::STRING,  // 3: `"literal"`
            SemanticTokenType::NUMBER,  // 4: `-1`, `42`
            SemanticTokenType::COMMENT, // 5: `; …`
            // Field names are literally properties of the parent node:
            // `(call_expression function: (…))`. Property colour
            // (italic purple by default) reads as field-access.
            SemanticTokenType::PROPERTY, // 6: field name `function:`
            SemanticTokenType::OPERATOR, // 7: `(`, `)`, `:`, quantifiers `+ * ?`
            // Wildcard `_` is a query keyword, not a node kind —
            // keyword colour (bold orange in Darcula) flags it as
            // special syntax.
            SemanticTokenType::KEYWORD, // 8: bare `_` wildcard
        ],
        token_modifiers: vec![
            // `definition` on the `@foo` that introduces a capture so
            // schemes that distinguish declaration vs. reference can
            // paint them differently (default scheme bolds definitions).
            SemanticTokenModifier::DEFINITION,
        ],
    }
}

const TOKEN_DECORATOR: u32 = 0;
const TOKEN_MACRO: u32 = 1;
const TOKEN_CLASS: u32 = 2;
const TOKEN_STRING: u32 = 3;
const TOKEN_NUMBER: u32 = 4;
const TOKEN_COMMENT: u32 = 5;
const TOKEN_PROPERTY: u32 = 6;
const TOKEN_OPERATOR: u32 = 7;
const TOKEN_KEYWORD: u32 = 8;

const MOD_DEFINITION: u32 = 1 << 0;

/// Tokenise `src` (a full YAML document) and return LSP semantic tokens
/// for every recognisable element inside every `query: |` block scalar.
///
/// Returns `None` when there are no tokens — the LSP client treats that
/// the same as an empty result but saves us the envelope allocation.
pub fn tokenize(src: &str) -> Option<SemanticTokens> {
    let tokens = collect_absolute_tokens(src);
    if tokens.is_empty() {
        return None;
    }
    Some(SemanticTokens {
        result_id: None,
        data: encode_delta(tokens),
    })
}

/// A semantic token in absolute (line, utf16-column) coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct AbsToken {
    line: u32,
    character: u32,
    length: u32,
    token_type: u32,
    token_modifiers: u32,
}

impl AbsToken {
    fn plain(line: u32, character: u32, length: u32, token_type: u32) -> Self {
        Self {
            line,
            character,
            length,
            token_type,
            token_modifiers: 0,
        }
    }
}

fn collect_absolute_tokens(src: &str) -> Vec<AbsToken> {
    let mut tokens = Vec::new();

    let mut lines = src.lines().enumerate().peekable();
    while let Some((_, line)) = lines.next() {
        let Some(block_indent) = query_block_indent(line) else {
            continue;
        };

        // Walk the block body. YAML terminates the block at the first
        // non-empty line whose leading whitespace is <= block_indent.
        while let Some(&(next_idx, next_line)) = lines.peek() {
            if next_line.trim().is_empty() {
                lines.next();
                continue;
            }
            let indent = leading_spaces(next_line);
            if indent <= block_indent {
                break;
            }
            let (_, body) = lines.next().unwrap();
            tokenize_query_line(next_idx as u32, body, &mut tokens);
        }
    }

    tokens
}

/// If `line` is a YAML `query: |`/`query: >` opener, return the column
/// of the `q` in `query` (this is the indent the body must exceed).
fn query_block_indent(line: &str) -> Option<usize> {
    static QUERY_RE: OnceLock<Regex> = OnceLock::new();
    let re = QUERY_RE.get_or_init(|| {
        Regex::new(r"^(?P<indent>\s*)query:\s*[|>][+\-]?\s*$").expect("valid regex")
    });
    re.captures(line).map(|c| c["indent"].len())
}

fn leading_spaces(line: &str) -> usize {
    line.chars().take_while(|c| *c == ' ').count()
}

/// Append tokens found in one body line of a query block.
fn tokenize_query_line(line: u32, body: &str, out: &mut Vec<AbsToken>) {
    // Walk byte-by-byte, emitting tokens by shape. Regex-per-category
    // would work too, but a single pass keeps the ordering obvious and
    // avoids regex crate fighting over overlapping matches.
    let bytes = body.as_bytes();
    let mut idx = 0;
    while idx < bytes.len() {
        let byte = bytes[idx];

        if byte == b';' {
            let start_char = utf16_col(body, idx);
            let length = body[idx..].encode_utf16().count() as u32;
            out.push(AbsToken::plain(line, start_char, length, TOKEN_COMMENT));
            return;
        }

        if byte == b'"' {
            let end = find_string_end(bytes, idx + 1);
            let slice = &body[idx..end];
            let start_char = utf16_col(body, idx);
            let length = slice.encode_utf16().count() as u32;
            out.push(AbsToken::plain(line, start_char, length, TOKEN_STRING));
            idx = end;
            continue;
        }

        if byte == b'@' {
            let end = consume_ident(bytes, idx + 1);
            if end > idx + 1 {
                let slice = &body[idx..end];
                let start_char = utf16_col(body, idx);
                let length = slice.encode_utf16().count() as u32;
                // Every capture inside a query string is a definition —
                // references (`#eq? @foo "…"`) also carry the same
                // decorator flavour, but the definition modifier lets
                // themes bold/italicise the introducing occurrence.
                out.push(AbsToken {
                    line,
                    character: start_char,
                    length,
                    token_type: TOKEN_DECORATOR,
                    token_modifiers: MOD_DEFINITION,
                });
                idx = end;
                continue;
            }
        }

        if byte == b'#' {
            let end = consume_predicate(bytes, idx + 1);
            if end > idx + 1 {
                let slice = &body[idx..end];
                let start_char = utf16_col(body, idx);
                let length = slice.encode_utf16().count() as u32;
                out.push(AbsToken::plain(line, start_char, length, TOKEN_MACRO));
                idx = end;
                continue;
            }
        }

        if byte == b'-' && idx + 1 < bytes.len() && bytes[idx + 1].is_ascii_digit() {
            let end = consume_digits(bytes, idx + 1);
            let slice = &body[idx..end];
            let start_char = utf16_col(body, idx);
            let length = slice.encode_utf16().count() as u32;
            out.push(AbsToken::plain(line, start_char, length, TOKEN_NUMBER));
            idx = end;
            continue;
        }

        if byte.is_ascii_digit() {
            let end = consume_digits(bytes, idx);
            let slice = &body[idx..end];
            let start_char = utf16_col(body, idx);
            let length = slice.encode_utf16().count() as u32;
            out.push(AbsToken::plain(line, start_char, length, TOKEN_NUMBER));
            idx = end;
            continue;
        }

        if byte == b'(' || byte == b')' || byte == b'[' || byte == b']' {
            let start_char = utf16_col(body, idx);
            out.push(AbsToken::plain(line, start_char, 1, TOKEN_OPERATOR));
            idx += 1;
            continue;
        }

        // Tree-sitter quantifiers on a preceding node/capture.
        if byte == b'+' || byte == b'*' || byte == b'?' || byte == b'!' || byte == b'.' {
            let start_char = utf16_col(body, idx);
            out.push(AbsToken::plain(line, start_char, 1, TOKEN_OPERATOR));
            idx += 1;
            continue;
        }

        if byte == b'_' && !is_ident_tail(bytes.get(idx + 1).copied()) {
            // Bare `_` wildcard — tree-sitter keyword, not a node kind.
            let start_char = utf16_col(body, idx);
            out.push(AbsToken::plain(line, start_char, 1, TOKEN_KEYWORD));
            idx += 1;
            continue;
        }

        if byte == b'_' || byte.is_ascii_alphabetic() {
            let end = consume_ident(bytes, idx);
            let slice = &body[idx..end];
            let start_char = utf16_col(body, idx);
            let length = slice.encode_utf16().count() as u32;
            // An ident followed by `:` is a field name, not a node kind.
            let token_type = if bytes.get(end).copied() == Some(b':') {
                TOKEN_PROPERTY
            } else {
                TOKEN_CLASS
            };
            out.push(AbsToken::plain(line, start_char, length, token_type));
            idx = end;
            continue;
        }

        idx += 1;
    }
}

fn is_ident_tail(byte: Option<u8>) -> bool {
    matches!(byte, Some(b) if b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

fn consume_ident(bytes: &[u8], mut idx: usize) -> usize {
    while idx < bytes.len() {
        let b = bytes[idx];
        if b.is_ascii_alphanumeric() || b == b'_' || b == b'-' {
            idx += 1;
        } else {
            break;
        }
    }
    idx
}

fn consume_predicate(bytes: &[u8], mut idx: usize) -> usize {
    while idx < bytes.len() {
        let b = bytes[idx];
        if b.is_ascii_alphanumeric() || b == b'_' || b == b'-' || b == b'?' || b == b'!' {
            idx += 1;
        } else {
            break;
        }
    }
    idx
}

fn consume_digits(bytes: &[u8], mut idx: usize) -> usize {
    while idx < bytes.len() && bytes[idx].is_ascii_digit() {
        idx += 1;
    }
    idx
}

fn find_string_end(bytes: &[u8], mut idx: usize) -> usize {
    while idx < bytes.len() {
        match bytes[idx] {
            b'\\' if idx + 1 < bytes.len() => idx += 2,
            b'"' => return idx + 1,
            _ => idx += 1,
        }
    }
    bytes.len()
}

fn utf16_col(line: &str, byte_idx: usize) -> u32 {
    line[..byte_idx].encode_utf16().count() as u32
}

/// LSP SemanticTokens transmits tokens as a delta-encoded flat `u32`
/// array where each group of five is `[deltaLine, deltaStart, length,
/// tokenType, tokenModifiers]`. Same-line `deltaStart` is relative to
/// the previous token; new-line `deltaStart` is absolute.
fn encode_delta(mut tokens: Vec<AbsToken>) -> Vec<SemanticToken> {
    tokens.sort_by_key(|t| (t.line, t.character));
    let mut out = Vec::with_capacity(tokens.len());
    let mut prev_line = 0u32;
    let mut prev_char = 0u32;
    for tok in tokens {
        let delta_line = tok.line - prev_line;
        let delta_start = if delta_line == 0 {
            tok.character - prev_char
        } else {
            tok.character
        };
        out.push(SemanticToken {
            delta_line,
            delta_start,
            length: tok.length,
            token_type: tok.token_type,
            token_modifiers_bitset: tok.token_modifiers,
        });
        prev_line = tok.line;
        prev_char = tok.character;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_query_block_returns_none() {
        assert!(tokenize("id: x\nmessage: hi\n").is_none());
    }

    #[test]
    fn captures_predicates_strings_and_node_kinds_tokenised() {
        let src = "\
query: |
  (call_expression
    function: (field_expression
      value: (_) @recv
      field: (field_identifier) @method)
    (#eq? @method \"unwrap\")) @match
";
        let tokens = tokenize(src).expect("tokens");
        let types: Vec<u32> = tokens.data.iter().map(|t| t.token_type).collect();

        // call_expression (CLASS), function: (PROPERTY), field_expression
        // (CLASS), value: (PROPERTY), (_) wildcard (KEYWORD), @recv
        // (DECORATOR), field: (PROPERTY), field_identifier (CLASS),
        // @method (DECORATOR), #eq? (MACRO), "unwrap" (STRING),
        // @match (DECORATOR), parens (OPERATOR).
        assert!(types.contains(&TOKEN_DECORATOR));
        assert!(types.contains(&TOKEN_MACRO));
        assert!(types.contains(&TOKEN_CLASS));
        assert!(types.contains(&TOKEN_STRING));
        assert!(types.contains(&TOKEN_PROPERTY));
        assert!(types.contains(&TOKEN_OPERATOR));
        assert!(types.contains(&TOKEN_KEYWORD));
    }

    #[test]
    fn captures_carry_definition_modifier() {
        let src = "query: |\n  (identifier) @foo\n";
        let tokens = tokenize(src).expect("tokens");
        let capture = tokens
            .data
            .iter()
            .find(|t| t.token_type == TOKEN_DECORATOR)
            .expect("capture token");
        assert_eq!(capture.token_modifiers_bitset, MOD_DEFINITION);
    }

    #[test]
    fn field_name_followed_by_colon_is_property_not_class() {
        let src = "query: |\n  (call function: (_))\n";
        let tokens = tokenize(src).expect("tokens");
        // `call` (no colon) → CLASS; `function:` → PROPERTY; `_` → KEYWORD.
        let types: Vec<u32> = tokens.data.iter().map(|t| t.token_type).collect();
        assert!(types.contains(&TOKEN_CLASS));
        assert!(types.contains(&TOKEN_PROPERTY));
        assert!(types.contains(&TOKEN_KEYWORD));
    }

    #[test]
    fn semicolon_starts_comment_to_end_of_line() {
        let src = "\
query: |
  (identifier) ; trailing note
";
        let tokens = tokenize(src).expect("tokens");
        assert!(tokens.data.iter().any(|t| t.token_type == TOKEN_COMMENT));
    }

    #[test]
    fn delta_encoding_is_sorted_and_relative() {
        // Two tokens on the same line, 5 chars apart.
        let src = "query: |\n  @a @b\n";
        let tokens = tokenize(src).expect("tokens");
        assert_eq!(tokens.data.len(), 2);
        assert_eq!(tokens.data[0].delta_line, 1);
        // First token on line 1 at column 2 (two-space indent).
        assert_eq!(tokens.data[0].delta_start, 2);
        // Second token same line → delta_start = column diff = 3
        // (@a occupies chars 2..4, space at 4, @b starts at 5, so 5 - 2 = 3).
        assert_eq!(tokens.data[1].delta_line, 0);
        assert_eq!(tokens.data[1].delta_start, 3);
    }

    #[test]
    fn multiple_query_blocks_in_one_file_tokenised() {
        let src = "\
rules:
  - id: a
    query: |
      (identifier) @x
  - id: b
    query: |
      (string) @y
";
        let tokens = tokenize(src).expect("tokens");
        let decorators: Vec<_> = tokens
            .data
            .iter()
            .filter(|t| t.token_type == TOKEN_DECORATOR)
            .collect();
        assert_eq!(decorators.len(), 2);
    }
}
