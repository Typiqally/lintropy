//! Query execution engine.

use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use globset::{Glob, GlobSet, GlobSetBuilder};
use rayon::prelude::*;
use tree_sitter::{Parser, QueryCursor};

use crate::{
    config::{Config, RuleConfig},
    template::{interpolate, CaptureMap},
    Diagnostic, FixHunk, LintropyError, Result,
};

pub fn run(config: &Config, files: &[PathBuf]) -> Result<Vec<Diagnostic>> {
    let rules_by_language = RulesByLanguage::new(config)?;
    let diagnostics = files
        .par_iter()
        .map(|path| run_file(path, &rules_by_language))
        .collect::<Result<Vec<_>>>()?;

    let mut flattened: Vec<_> = diagnostics.into_iter().flatten().collect();
    flattened.sort_by(|left, right| {
        left.file
            .cmp(&right.file)
            .then(left.byte_start.cmp(&right.byte_start))
            .then(left.rule_id.as_str().cmp(right.rule_id.as_str()))
    });
    Ok(flattened)
}

struct RulesByLanguage<'a> {
    rust: Vec<ScopedRule<'a>>,
}

struct ScopedRule<'a> {
    rule: &'a RuleConfig,
    include: Option<Arc<GlobSet>>,
    exclude: Option<Arc<GlobSet>>,
}

impl<'a> RulesByLanguage<'a> {
    fn new(config: &'a Config) -> Result<Self> {
        let mut rust = Vec::new();
        for rule in &config.rules {
            let Some(language) = rule.language else {
                continue;
            };
            if rule.query_rule().is_none() {
                continue;
            }
            let scoped = ScopedRule {
                rule,
                include: compile_globs(&rule.include)?,
                exclude: compile_globs(&rule.exclude)?,
            };
            if language == lintropy_langs::Language::Rust {
                rust.push(scoped);
            }
        }
        Ok(Self { rust })
    }
}

fn compile_globs(patterns: &[String]) -> Result<Option<Arc<GlobSet>>> {
    if patterns.is_empty() {
        return Ok(None);
    }

    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        let glob = Glob::new(pattern)
            .map_err(|err| LintropyError::Internal(format!("invalid glob `{pattern}`: {err}")))?;
        builder.add(glob);
    }

    let set = builder
        .build()
        .map_err(|err| LintropyError::Internal(format!("invalid glob set: {err}")))?;
    Ok(Some(Arc::new(set)))
}

fn run_file(path: &Path, rules_by_language: &RulesByLanguage<'_>) -> Result<Vec<Diagnostic>> {
    let Some(language) = path
        .extension()
        .and_then(|ext| ext.to_str())
        .and_then(lintropy_langs::Language::from_extension)
    else {
        return Ok(Vec::new());
    };

    let scoped_rules = match language {
        lintropy_langs::Language::Rust => &rules_by_language.rust,
    };
    if scoped_rules.is_empty() {
        return Ok(Vec::new());
    }

    let src = std::fs::read(path)?;
    let mut parser = Parser::new();
    parser
        .set_language(&language.ts_language())
        .map_err(|err| LintropyError::Internal(format!("failed to load parser for {}: {err}", language.name())))?;
    let tree = parser
        .parse(&src, None)
        .ok_or_else(|| LintropyError::Internal(format!("failed to parse {}", path.display())))?;
    let root = tree.root_node();

    let mut diagnostics = Vec::new();
    for scoped_rule in scoped_rules {
        if !scoped_rule.matches(path) {
            continue;
        }
        let query_rule = scoped_rule.rule.query_rule().expect("query rule");
        let mut cursor = QueryCursor::new();
        for query_match in cursor.matches(query_rule.compiled.as_ref(), root, src.as_slice()) {
            let pattern_predicates = query_rule
                .predicates_by_pattern
                .get(query_match.pattern_index)
                .map(Vec::as_slice)
                .unwrap_or(&[]);
            if !pattern_predicates
                .iter()
                .all(|predicate| predicate.apply(&query_match, query_rule.compiled.as_ref(), &root, &src))
            {
                continue;
            }
            diagnostics.push(build_diagnostic(path, &src, scoped_rule.rule, query_rule, query_match)?);
        }
    }

    Ok(diagnostics)
}

impl ScopedRule<'_> {
    fn matches(&self, path: &Path) -> bool {
        let candidate = path.to_string_lossy();
        let include_ok = self
            .include
            .as_ref()
            .map(|set| set.is_match(candidate.as_ref()))
            .unwrap_or(true);
        let exclude_ok = self
            .exclude
            .as_ref()
            .map(|set| !set.is_match(candidate.as_ref()))
            .unwrap_or(true);
        include_ok && exclude_ok
    }
}

fn build_diagnostic(
    path: &Path,
    src: &[u8],
    rule: &RuleConfig,
    query_rule: &crate::config::QueryRule,
    query_match: tree_sitter::QueryMatch<'_, '_>,
) -> Result<Diagnostic> {
    let captures = capture_map(query_match.captures, query_rule.compiled.as_ref(), src)?;
    let span_node = query_rule
        .compiled
        .capture_index_for_name("match")
        .and_then(|match_capture| {
            query_match
                .captures
                .iter()
                .find(|capture| capture.index == match_capture)
                .map(|capture| capture.node)
        })
        .unwrap_or(query_match.captures[0].node);
    let start = span_node.start_position();
    let end = span_node.end_position();

    let fix = rule.fix.as_ref().map(|template| FixHunk {
        replacement: interpolate(template, &captures),
        byte_start: span_node.start_byte(),
        byte_end: span_node.end_byte(),
    });

    Ok(Diagnostic {
        rule_id: rule.id.clone(),
        severity: rule.severity,
        message: interpolate(&rule.message, &captures),
        file: path.to_path_buf(),
        line: start.row + 1,
        column: start.column + 1,
        end_line: end.row + 1,
        end_column: end.column + 1,
        byte_start: span_node.start_byte(),
        byte_end: span_node.end_byte(),
        rule_source: rule.source_path.clone(),
        docs_url: rule.docs_url.clone(),
        fix,
    })
}

fn capture_map(
    captures: &[tree_sitter::QueryCapture<'_>],
    query: &tree_sitter::Query,
    src: &[u8],
) -> Result<CaptureMap> {
    let mut map = CaptureMap::new();
    for capture in captures {
        let name = query.capture_names()[capture.index as usize].to_string();
        let text = capture
            .node
            .utf8_text(src)
            .map_err(|err| LintropyError::Internal(format!("capture text is not utf-8: {err}")))?;
        map.insert(name, text.to_string());
    }
    Ok(map)
}
