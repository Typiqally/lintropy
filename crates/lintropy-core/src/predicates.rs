//! Custom tree-sitter predicates executed after `QueryCursor` filtering.

use std::path::PathBuf;

use regex::Regex;
use tree_sitter::{Node, Query, QueryMatch, QueryPredicateArg};

use crate::{LintropyError, Result};

#[derive(Debug, Clone)]
pub enum CustomPredicate {
    HasAncestor { capture: u32, kinds: Vec<String> },
    NotHasAncestor { capture: u32, kinds: Vec<String> },
    HasParent { capture: u32, kinds: Vec<String> },
    NotHasParent { capture: u32, kinds: Vec<String> },
    HasSibling { capture: u32, kinds: Vec<String> },
    NotHasSibling { capture: u32, kinds: Vec<String> },
    HasPrecedingComment { capture: u32, pattern: Regex },
    NotHasPrecedingComment { capture: u32, pattern: Regex },
}

impl CustomPredicate {
    pub fn apply(
        &self,
        query_match: &QueryMatch<'_, '_>,
        _query: &Query,
        _root: &Node<'_>,
        src: &[u8],
    ) -> bool {
        match self {
            CustomPredicate::HasAncestor { capture, kinds } => {
                capture_nodes(query_match, *capture).any(|node| has_ancestor(node, kinds))
            }
            CustomPredicate::NotHasAncestor { capture, kinds } => {
                capture_nodes(query_match, *capture).all(|node| !has_ancestor(node, kinds))
            }
            CustomPredicate::HasParent { capture, kinds } => {
                capture_nodes(query_match, *capture).any(|node| has_parent(node, kinds))
            }
            CustomPredicate::NotHasParent { capture, kinds } => {
                capture_nodes(query_match, *capture).all(|node| !has_parent(node, kinds))
            }
            CustomPredicate::HasSibling { capture, kinds } => {
                capture_nodes(query_match, *capture).any(|node| has_sibling(node, kinds))
            }
            CustomPredicate::NotHasSibling { capture, kinds } => {
                capture_nodes(query_match, *capture).all(|node| !has_sibling(node, kinds))
            }
            CustomPredicate::HasPrecedingComment { capture, pattern } => {
                capture_nodes(query_match, *capture)
                    .any(|node| has_preceding_comment(node, pattern, src))
            }
            CustomPredicate::NotHasPrecedingComment { capture, pattern } => {
                capture_nodes(query_match, *capture)
                    .all(|node| !has_preceding_comment(node, pattern, src))
            }
        }
    }
}

pub fn parse_general_predicates(query: &Query) -> Result<Vec<CustomPredicate>> {
    Ok(parse_general_predicates_by_pattern(query)?
        .into_iter()
        .flatten()
        .collect())
}

pub(crate) fn parse_general_predicates_by_pattern(
    query: &Query,
) -> Result<Vec<Vec<CustomPredicate>>> {
    let mut all = Vec::with_capacity(query.pattern_count());
    for pattern_index in 0..query.pattern_count() {
        let mut predicates = Vec::new();
        for predicate in query.general_predicates(pattern_index) {
            let custom = match predicate.operator.as_ref() {
                "has-ancestor?" => CustomPredicate::HasAncestor {
                    capture: expect_capture(predicate, 0)?,
                    kinds: expect_strings(predicate, 1)?,
                },
                "not-has-ancestor?" => CustomPredicate::NotHasAncestor {
                    capture: expect_capture(predicate, 0)?,
                    kinds: expect_strings(predicate, 1)?,
                },
                "has-parent?" => CustomPredicate::HasParent {
                    capture: expect_capture(predicate, 0)?,
                    kinds: expect_strings(predicate, 1)?,
                },
                "not-has-parent?" => CustomPredicate::NotHasParent {
                    capture: expect_capture(predicate, 0)?,
                    kinds: expect_strings(predicate, 1)?,
                },
                "has-sibling?" => CustomPredicate::HasSibling {
                    capture: expect_capture(predicate, 0)?,
                    kinds: expect_strings(predicate, 1)?,
                },
                "not-has-sibling?" => CustomPredicate::NotHasSibling {
                    capture: expect_capture(predicate, 0)?,
                    kinds: expect_strings(predicate, 1)?,
                },
                "has-preceding-comment?" => CustomPredicate::HasPrecedingComment {
                    capture: expect_capture(predicate, 0)?,
                    pattern: compile_regex(expect_string(predicate, 1)?)?,
                },
                "not-has-preceding-comment?" => CustomPredicate::NotHasPrecedingComment {
                    capture: expect_capture(predicate, 0)?,
                    pattern: compile_regex(expect_string(predicate, 1)?)?,
                },
                other => {
                    return Err(LintropyError::UnknownPredicate {
                        rule_id: "<unknown>".into(),
                        source_path: PathBuf::from("<unknown>"),
                        predicate: other.trim_end_matches('?').to_string(),
                    });
                }
            };
            predicates.push(custom);
        }
        all.push(predicates);
    }
    Ok(all)
}

fn expect_capture(predicate: &tree_sitter::QueryPredicate, index: usize) -> Result<u32> {
    match predicate.args.get(index) {
        Some(QueryPredicateArg::Capture(index)) => Ok(*index),
        other => Err(LintropyError::Internal(format!(
            "invalid predicate arguments: expected capture at arg {index}, got {other:?}"
        ))),
    }
}

fn expect_string(predicate: &tree_sitter::QueryPredicate, index: usize) -> Result<&str> {
    match predicate.args.get(index) {
        Some(QueryPredicateArg::String(value)) => Ok(value),
        other => Err(LintropyError::Internal(format!(
            "invalid predicate arguments: expected string at arg {index}, got {other:?}"
        ))),
    }
}

fn expect_strings(predicate: &tree_sitter::QueryPredicate, start: usize) -> Result<Vec<String>> {
    predicate.args[start..]
        .iter()
        .map(|arg| match arg {
            QueryPredicateArg::String(value) => Ok(value.to_string()),
            other => Err(LintropyError::Internal(format!(
                "invalid predicate arguments: expected string list, got {other:?}"
            ))),
        })
        .collect()
}

fn compile_regex(pattern: &str) -> Result<Regex> {
    Regex::new(pattern)
        .map_err(|err| LintropyError::Internal(format!("invalid predicate regex `{pattern}`: {err}")))
}

fn capture_nodes<'a>(
    query_match: &'a QueryMatch<'a, 'a>,
    capture: u32,
) -> impl Iterator<Item = Node<'a>> + 'a {
    query_match
        .captures
        .iter()
        .filter(move |entry| entry.index == capture)
        .map(|entry| entry.node)
}

fn has_ancestor(node: Node<'_>, kinds: &[String]) -> bool {
    let mut current = node.parent();
    while let Some(parent) = current {
        if kinds.iter().any(|kind| kind == parent.kind()) {
            return true;
        }
        current = parent.parent();
    }
    false
}

fn has_parent(node: Node<'_>, kinds: &[String]) -> bool {
    node.parent()
        .map(|parent| kinds.iter().any(|kind| kind == parent.kind()))
        .unwrap_or(false)
}

fn has_sibling(node: Node<'_>, kinds: &[String]) -> bool {
    let Some(parent) = node.parent() else {
        return false;
    };

    let mut cursor = parent.walk();
    let has_match = parent.children(&mut cursor).any(|child| {
        child != node && kinds.iter().any(|kind| kind == child.kind())
    });
    has_match
}

fn has_preceding_comment(node: Node<'_>, pattern: &Regex, src: &[u8]) -> bool {
    let Ok(source) = std::str::from_utf8(src) else {
        return false;
    };
    let lines: Vec<&str> = source.lines().collect();
    let mut row = node.start_position().row;
    if row == 0 {
        return false;
    }

    while row > 0 {
        row -= 1;
        let candidate = lines.get(row).copied().unwrap_or("").trim();
        if candidate.is_empty() {
            continue;
        }

        if candidate.starts_with("//") {
            return pattern.is_match(candidate);
        }

        if candidate.ends_with("*/") {
            let mut block = String::from(candidate);
            let mut block_row = row;
            while block_row > 0 {
                block_row -= 1;
                let line = lines.get(block_row).copied().unwrap_or("").trim();
                block = format!("{line}\n{block}");
                if line.starts_with("/*") {
                    return pattern.is_match(&block);
                }
            }
        }

        return false;
    }

    false
}

#[cfg(test)]
mod tests {
    use lintropy_langs::Language;
    use tree_sitter::{Parser, Query, QueryCursor};

    use super::{
        has_ancestor, parse_general_predicates, parse_general_predicates_by_pattern, CustomPredicate,
    };

    fn rust_tree(src: &str) -> tree_sitter::Tree {
        let mut parser = Parser::new();
        parser
            .set_language(&Language::Rust.ts_language())
            .expect("rust parser");
        parser.parse(src, None).expect("tree")
    }

    #[test]
    fn parses_general_predicates_per_pattern() {
        let language = Language::Rust.ts_language();
        let query = Query::new(
            &language,
            r#"
            (call_expression
              function: (field_expression value: (_) @recv field: (field_identifier) @method)
              (#has-parent? @method "field_expression")
              (#not-has-ancestor? @recv "function_item")) @match
            "#,
        )
        .unwrap();

        let predicates = parse_general_predicates(&query).unwrap();
        assert_eq!(predicates.len(), 2);

        let by_pattern = parse_general_predicates_by_pattern(&query).unwrap();
        assert_eq!(by_pattern.len(), 1);
        assert_eq!(by_pattern[0].len(), 2);
    }

    #[test]
    fn applies_ancestor_predicate() {
        let src = "fn main() { let value = user.unwrap(); }";
        let language = Language::Rust.ts_language();
        let query = Query::new(
            &language,
            r#"
            (call_expression
              function: (field_expression value: (_) @recv field: (field_identifier) @method)
              (#eq? @method "unwrap")) @match
            "#,
        )
        .unwrap();
        let tree = rust_tree(src);
        let mut cursor = QueryCursor::new();
        let m = cursor
            .matches(&query, tree.root_node(), src.as_bytes())
            .next()
            .unwrap();
        let recv = m.captures
            .iter()
            .find(|capture| capture.index == query.capture_index_for_name("recv").unwrap())
            .unwrap()
            .node;
        assert!(has_ancestor(recv, &[String::from("call_expression")]));
        let predicate = CustomPredicate::HasAncestor {
            capture: query.capture_index_for_name("recv").unwrap(),
            kinds: vec!["call_expression".into()],
        };
        assert!(predicate.apply(&m, &query, &tree.root_node(), src.as_bytes()));
    }

    #[test]
    fn applies_parent_and_sibling_predicates() {
        let src = "fn main() { let value = user.unwrap(); }";
        let language = Language::Rust.ts_language();
        let query = Query::new(
            &language,
            r#"
            (call_expression
              function: (field_expression value: (_) @recv field: (field_identifier) @method)
              (#eq? @method "unwrap")) @match
            "#,
        )
        .unwrap();
        let tree = rust_tree(src);
        let mut cursor = QueryCursor::new();
        let m = cursor
            .matches(&query, tree.root_node(), src.as_bytes())
            .next()
            .unwrap();
        let method = query.capture_index_for_name("method").unwrap();

        let parent = CustomPredicate::HasParent {
            capture: method,
            kinds: vec!["field_expression".into()],
        };
        let sibling = CustomPredicate::HasSibling {
            capture: method,
            kinds: vec!["identifier".into()],
        };

        assert!(parent.apply(&m, &query, &tree.root_node(), src.as_bytes()));
        assert!(sibling.apply(&m, &query, &tree.root_node(), src.as_bytes()));
    }

    #[test]
    fn applies_preceding_comment_predicate() {
        let src = "// SAFETY: checked above\nfn dangerous() {}";
        let language = Language::Rust.ts_language();
        let query = Query::new(&language, r#"(function_item) @match"#).unwrap();
        let tree = rust_tree(src);
        let mut cursor = QueryCursor::new();
        let m = cursor
            .matches(&query, tree.root_node(), src.as_bytes())
            .next()
            .unwrap();
        let node = m.captures[0].node;
        let regex = regex::Regex::new("SAFETY").unwrap();
        assert!(super::has_preceding_comment(node, &regex, src.as_bytes()));
    }
}
