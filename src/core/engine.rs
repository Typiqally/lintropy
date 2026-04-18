//! Query execution engine.

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};

use globset::{Glob, GlobSet, GlobSetBuilder};
use rayon::prelude::*;
use tree_sitter::{Parser, QueryCursor};

use super::{
    config::{Config, RuleConfig},
    template::{interpolate, CaptureMap},
    Diagnostic, FixHunk, LintropyError, Result,
};
use crate::langs::Language;

/// Walk `files`, read each from disk, and lint in parallel.
///
/// Thin wrapper over [`PreparedRules::lint_buffer`]: builds the per-language
/// rule index once, then for each path reads bytes and delegates to the
/// buffer entry point. Used by `lintropy check`.
pub fn run(config: &Config, files: &[PathBuf]) -> Result<Vec<Diagnostic>> {
    let prepared = PreparedRules::prepare(config)?;
    let diagnostics = files
        .par_iter()
        .map(|path| run_file_from_disk(&prepared, path))
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

/// Precompiled per-language rule index.
///
/// Constructing this compiles all include/exclude globs once so that
/// subsequent [`PreparedRules::lint_buffer`] calls can skip the work.
/// The LSP server builds this on `initialize` and reuses it across
/// `didChange` notifications; `lintropy check` builds it once per run.
pub struct PreparedRules<'a> {
    by_language: HashMap<Language, Vec<ScopedRule<'a>>>,
}

struct ScopedRule<'a> {
    rule: &'a RuleConfig,
    root_dir: &'a Path,
    include: Option<Arc<GlobSet>>,
    exclude: Option<Arc<GlobSet>>,
}

impl<'a> PreparedRules<'a> {
    /// Compile every query rule in `config` into a language-indexed table.
    pub fn prepare(config: &'a Config) -> Result<Self> {
        let mut by_language: HashMap<Language, Vec<ScopedRule<'a>>> = HashMap::new();
        for rule in &config.rules {
            let Some(language) = rule.language else {
                continue;
            };
            if rule.query_rule().is_none() {
                continue;
            }
            let scoped = ScopedRule {
                rule,
                root_dir: &config.root_dir,
                include: compile_globs(&rule.include)?,
                exclude: compile_globs(&rule.exclude)?,
            };
            by_language.entry(language).or_default().push(scoped);
        }
        Ok(Self { by_language })
    }

    /// Lint an in-memory buffer attributed to `path`.
    ///
    /// Does not touch the filesystem. The path is used only for language
    /// detection (via extension) and include/exclude glob matching, and is
    /// propagated into each emitted [`Diagnostic`].
    pub fn lint_buffer(&self, path: &Path, src: &[u8]) -> Result<Vec<Diagnostic>> {
        let Some(language) = crate::langs::language_from_path(path) else {
            return Ok(Vec::new());
        };

        let Some(scoped_rules) = self.by_language.get(&language) else {
            return Ok(Vec::new());
        };
        if scoped_rules.is_empty() {
            return Ok(Vec::new());
        }

        let mut parser = Parser::new();
        parser
            .set_language(&language.ts_language(path))
            .map_err(|err| {
                LintropyError::Internal(format!(
                    "failed to load parser for {}: {err}",
                    language.name()
                ))
            })?;
        let tree = parser.parse(src, None).ok_or_else(|| {
            LintropyError::Internal(format!("failed to parse {}", path.display()))
        })?;
        let root = tree.root_node();

        let mut diagnostics = Vec::new();
        for scoped_rule in scoped_rules {
            if !scoped_rule.matches(path) {
                continue;
            }
            let query_rule = scoped_rule.rule.query_rule().expect("query rule");
            let compiled = pick_compiled(scoped_rule.rule, path);
            let mut cursor = QueryCursor::new();
            for query_match in cursor.matches(compiled, root, src) {
                let pattern_predicates = query_rule
                    .predicates_by_pattern
                    .get(query_match.pattern_index)
                    .map(Vec::as_slice)
                    .unwrap_or(&[]);
                if !pattern_predicates
                    .iter()
                    .all(|predicate| predicate.apply(&query_match, compiled, &root, src))
                {
                    continue;
                }
                diagnostics.push(build_diagnostic(
                    path,
                    src,
                    scoped_rule.rule,
                    compiled,
                    query_match,
                )?);
            }
        }

        Ok(diagnostics)
    }
}

fn run_file_from_disk(prepared: &PreparedRules<'_>, path: &Path) -> Result<Vec<Diagnostic>> {
    let src = std::fs::read(path)?;
    prepared.lint_buffer(path, &src)
}

/// Pick the correct precompiled query for `path`.
///
/// TypeScript rules are dual-compiled against the `typescript` and `tsx`
/// grammars (different symbol IDs), so a query compiled against one grammar
/// won't match a parse tree produced by the other. For `.tsx` paths we return
/// the tsx-grammar compilation; for everything else (including `.ts`,
/// `.d.ts`, and all non-TypeScript languages) we return the primary
/// compilation.
fn pick_compiled<'a>(rule: &'a RuleConfig, _path: &Path) -> &'a tree_sitter::Query {
    let query_rule = rule
        .query_rule()
        .expect("lint_buffer only processes query rules");
    #[cfg(feature = "lang-typescript")]
    if _path.extension().and_then(|e| e.to_str()) == Some("tsx") {
        if let Some(tsx) = &query_rule.compiled_tsx {
            return tsx;
        }
    }
    &query_rule.compiled
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

impl ScopedRule<'_> {
    fn matches(&self, path: &Path) -> bool {
        let owned;
        let candidate = if let Ok(stripped) = path.strip_prefix(self.root_dir) {
            stripped
        } else if let Ok(canonical) = path.canonicalize() {
            owned = canonical;
            owned.strip_prefix(self.root_dir).unwrap_or(path)
        } else {
            path
        };
        let candidate = candidate.to_string_lossy();
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
    compiled: &tree_sitter::Query,
    query_match: tree_sitter::QueryMatch<'_, '_>,
) -> Result<Diagnostic> {
    let captures = capture_map(query_match.captures, compiled, src)?;
    let span_node = compiled
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
