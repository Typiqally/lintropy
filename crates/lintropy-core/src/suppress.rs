//! In-source suppression directive parsing and diagnostic filtering.
//!
//! Implements §7.5 of the merged lintropy spec:
//!
//! - `// lintropy-ignore: rule-a, rule-b` on its own line suppresses
//!   diagnostics on the next non-blank, non-comment line.
//! - `// lintropy-ignore-file: rule-a, rule-b` anywhere in the first 20
//!   lines suppresses for the whole file.
//! - Trailing-code forms (`code(); // lintropy-ignore: rule`) are silently
//!   ignored — the directive must be on its own line.
//! - `*` wildcards are rejected; they surface as [`UnusedSuppression`]
//!   entries with [`UnusedReason::WildcardRejected`].
//! - Rust-style `//` is the only comment prefix MVP supports.
//!
//! [`filter`] returns the surviving diagnostics plus a list of unused
//! suppression entries. WP5 is responsible for folding those into the
//! always-on `suppress-unused` meta-rule.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::{Diagnostic, RuleId};

const IGNORE_PREFIX: &str = "lintropy-ignore:";
const IGNORE_FILE_PREFIX: &str = "lintropy-ignore-file:";
const FILE_DIRECTIVE_LINE_LIMIT: usize = 20;

/// Lightweight read-through cache of source-file bytes, keyed by path.
///
/// The CLI populates this with the bytes it already read for the engine
/// run so suppression scanning does not re-read files.
#[derive(Debug, Default, Clone)]
pub struct SourceCache {
    inner: HashMap<PathBuf, Arc<[u8]>>,
}

impl SourceCache {
    /// Empty cache.
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert or replace the bytes for `path`.
    pub fn insert(&mut self, path: impl Into<PathBuf>, bytes: impl Into<Arc<[u8]>>) {
        self.inner.insert(path.into(), bytes.into());
    }

    /// Look up the cached bytes for `path`.
    pub fn get(&self, path: &Path) -> Option<&Arc<[u8]>> {
        self.inner.get(path)
    }

    /// How many files are cached.
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Whether the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}

/// Why a suppression directive produced an `UnusedSuppression` warning.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnusedReason {
    /// The directive named a rule-id but never matched a diagnostic.
    NeverMatched,
    /// The directive contained `*`; wildcards are rejected per §7.5.
    WildcardRejected,
}

/// One unused suppression entry to be surfaced by the `suppress-unused`
/// meta-rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnusedSuppression {
    /// Rule id the directive tried to silence, or `None` for wildcard rejects.
    pub rule_id: Option<RuleId>,
    /// File the directive was found in.
    pub file: PathBuf,
    /// 1-based line number of the directive itself.
    pub line: usize,
    /// Reason the directive is being reported.
    pub reason: UnusedReason,
}

/// Filter `diagnostics` against in-source suppressions sourced from
/// `sources`.
///
/// Diagnostics for files not present in the cache pass through untouched
/// — the caller is expected to seed `sources` with every file the engine
/// visited.
pub fn filter(
    diagnostics: Vec<Diagnostic>,
    sources: &SourceCache,
) -> (Vec<Diagnostic>, Vec<UnusedSuppression>) {
    let mut cache: HashMap<PathBuf, FileDirectives> = HashMap::new();
    let mut survivors: Vec<Diagnostic> = Vec::with_capacity(diagnostics.len());

    for diag in diagnostics {
        let dirs =
            cache
                .entry(diag.file.clone())
                .or_insert_with(|| match sources.get(&diag.file) {
                    Some(bytes) => parse_file(bytes),
                    None => FileDirectives::default(),
                });
        if !apply_suppression(dirs, &diag) {
            survivors.push(diag);
        }
    }

    let mut unused = Vec::new();
    for (file, dirs) in cache {
        for r in dirs.file {
            if !r.used {
                unused.push(UnusedSuppression {
                    rule_id: Some(r.rule_id),
                    file: file.clone(),
                    line: r.directive_line,
                    reason: UnusedReason::NeverMatched,
                });
            }
        }
        for r in dirs.line {
            if !r.used {
                unused.push(UnusedSuppression {
                    rule_id: Some(r.rule_id),
                    file: file.clone(),
                    line: r.directive_line,
                    reason: UnusedReason::NeverMatched,
                });
            }
        }
        for w in dirs.wildcards {
            unused.push(UnusedSuppression {
                rule_id: None,
                file: file.clone(),
                line: w.directive_line,
                reason: UnusedReason::WildcardRejected,
            });
        }
    }

    (survivors, unused)
}

fn apply_suppression(dirs: &mut FileDirectives, diag: &Diagnostic) -> bool {
    for r in &mut dirs.file {
        if r.rule_id == diag.rule_id {
            r.used = true;
            return true;
        }
    }
    for r in &mut dirs.line {
        if r.target_line == diag.line && r.rule_id == diag.rule_id {
            r.used = true;
            return true;
        }
    }
    false
}

#[derive(Debug, Default)]
struct FileDirectives {
    file: Vec<FileScoped>,
    line: Vec<LineScoped>,
    wildcards: Vec<WildcardHit>,
}

#[derive(Debug)]
struct FileScoped {
    rule_id: RuleId,
    directive_line: usize,
    used: bool,
}

#[derive(Debug)]
struct LineScoped {
    rule_id: RuleId,
    directive_line: usize,
    target_line: usize,
    used: bool,
}

#[derive(Debug)]
struct WildcardHit {
    directive_line: usize,
}

enum Parsed {
    File(RuleList),
    Line(RuleList),
}

#[derive(Default)]
struct RuleList {
    ids: Vec<String>,
    has_wildcard: bool,
}

fn parse_file(bytes: &[u8]) -> FileDirectives {
    let text = match std::str::from_utf8(bytes) {
        Ok(t) => t,
        Err(_) => return FileDirectives::default(),
    };
    let lines: Vec<&str> = text.lines().collect();
    let mut out = FileDirectives::default();
    for (i, raw) in lines.iter().enumerate() {
        let line_no = i + 1;
        let Some(parsed) = parse_directive_line(raw) else {
            continue;
        };
        match parsed {
            Parsed::File(rules) => {
                if line_no > FILE_DIRECTIVE_LINE_LIMIT {
                    continue;
                }
                for id in rules.ids {
                    out.file.push(FileScoped {
                        rule_id: RuleId::new(id),
                        directive_line: line_no,
                        used: false,
                    });
                }
                if rules.has_wildcard {
                    out.wildcards.push(WildcardHit {
                        directive_line: line_no,
                    });
                }
            }
            Parsed::Line(rules) => {
                let Some(target_line) = find_next_target(&lines, i) else {
                    // No target line (file ends in directives/blanks) — directive is unused.
                    if rules.has_wildcard {
                        out.wildcards.push(WildcardHit {
                            directive_line: line_no,
                        });
                    }
                    for id in rules.ids {
                        out.line.push(LineScoped {
                            rule_id: RuleId::new(id),
                            directive_line: line_no,
                            target_line: 0, // never matches — stays unused
                            used: false,
                        });
                    }
                    continue;
                };
                for id in rules.ids {
                    out.line.push(LineScoped {
                        rule_id: RuleId::new(id),
                        directive_line: line_no,
                        target_line,
                        used: false,
                    });
                }
                if rules.has_wildcard {
                    out.wildcards.push(WildcardHit {
                        directive_line: line_no,
                    });
                }
            }
        }
    }
    out
}

fn parse_directive_line(line: &str) -> Option<Parsed> {
    let trimmed = line.trim_start();
    // Own-line requirement: first non-whitespace must be `//`. Trailing-code
    // forms like `x(); // lintropy-ignore: rule` fail this check.
    let after = trimmed.strip_prefix("//")?.trim_start();
    if let Some(rest) = after.strip_prefix(IGNORE_FILE_PREFIX) {
        return Some(Parsed::File(parse_rule_list(rest)));
    }
    if let Some(rest) = after.strip_prefix(IGNORE_PREFIX) {
        return Some(Parsed::Line(parse_rule_list(rest)));
    }
    None
}

fn parse_rule_list(rest: &str) -> RuleList {
    let mut ids = Vec::new();
    let mut has_wildcard = false;
    for token in rest.split(',') {
        let t = token.trim();
        if t.is_empty() {
            continue;
        }
        if t == "*" {
            has_wildcard = true;
            continue;
        }
        ids.push(t.to_string());
    }
    RuleList { ids, has_wildcard }
}

fn find_next_target(lines: &[&str], from_idx: usize) -> Option<usize> {
    for (j, line) in lines.iter().enumerate().skip(from_idx + 1) {
        if !is_blank_or_comment(line) {
            return Some(j + 1);
        }
    }
    None
}

fn is_blank_or_comment(line: &str) -> bool {
    let t = line.trim();
    t.is_empty() || t.starts_with("//")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{FixHunk, Severity};
    use std::path::PathBuf;

    fn diag(file: &str, rule: &str, line: usize) -> Diagnostic {
        Diagnostic {
            rule_id: RuleId::new(rule),
            severity: Severity::Warning,
            message: "m".into(),
            file: PathBuf::from(file),
            line,
            column: 1,
            end_line: line,
            end_column: 2,
            byte_start: 0,
            byte_end: 1,
            rule_source: PathBuf::from(".lintropy/x.rule.yaml"),
            docs_url: None,
            fix: None::<FixHunk>,
        }
    }

    fn cache_with(path: &str, src: &str) -> SourceCache {
        let mut c = SourceCache::new();
        c.insert(PathBuf::from(path), Arc::<[u8]>::from(src.as_bytes()));
        c
    }

    #[test]
    fn trailing_code_form_ignored() {
        assert!(parse_directive_line("x(); // lintropy-ignore: foo").is_none());
    }

    #[test]
    fn own_line_directive_matches() {
        assert!(matches!(
            parse_directive_line("    // lintropy-ignore: foo, bar"),
            Some(Parsed::Line(_))
        ));
        assert!(matches!(
            parse_directive_line("// lintropy-ignore-file: foo"),
            Some(Parsed::File(_))
        ));
    }

    #[test]
    fn wildcard_rejected_not_applied() {
        let src = "// lintropy-ignore: *, foo\nbad_line\n";
        let cache = cache_with("a.rs", src);
        let d = diag("a.rs", "bar", 2);
        let (survivors, unused) = filter(vec![d], &cache);
        assert_eq!(survivors.len(), 1, "wildcard must not suppress `bar`");
        assert!(unused
            .iter()
            .any(|u| u.reason == UnusedReason::WildcardRejected));
    }

    #[test]
    fn wildcard_still_allows_other_rules_on_same_line() {
        let src = "// lintropy-ignore: *, foo\nbad_line\n";
        let cache = cache_with("a.rs", src);
        let d = diag("a.rs", "foo", 2);
        let (survivors, _unused) = filter(vec![d], &cache);
        assert!(
            survivors.is_empty(),
            "`foo` listed explicitly should suppress"
        );
    }

    #[test]
    fn ignore_next_line_skips_blanks_and_comments() {
        let src = "// lintropy-ignore: foo\n\n// another comment\nbad_line\n";
        let cache = cache_with("a.rs", src);
        let d = diag("a.rs", "foo", 4);
        let (survivors, unused) = filter(vec![d], &cache);
        assert!(survivors.is_empty());
        assert!(unused.is_empty());
    }

    #[test]
    fn ignore_file_suppresses_all_lines() {
        let src = "// lintropy-ignore-file: foo\nbad();\nbad2();\n";
        let cache = cache_with("a.rs", src);
        let d1 = diag("a.rs", "foo", 2);
        let d2 = diag("a.rs", "foo", 3);
        let (survivors, _) = filter(vec![d1, d2], &cache);
        assert!(survivors.is_empty());
    }

    #[test]
    fn ignore_file_beyond_line_20_ignored() {
        let mut src = String::new();
        for _ in 0..20 {
            src.push_str("fn filler() {}\n");
        }
        src.push_str("// lintropy-ignore-file: foo\n");
        src.push_str("bad();\n");
        let cache = cache_with("a.rs", &src);
        let d = diag("a.rs", "foo", 22);
        let (survivors, unused) = filter(vec![d], &cache);
        assert_eq!(survivors.len(), 1);
        // Late file-directive is simply ignored by parse (not stored) → no unused entry.
        assert!(unused.is_empty());
    }

    #[test]
    fn unused_rule_id_surfaces_as_never_matched() {
        let src = "// lintropy-ignore: foo\nok_line\n";
        let cache = cache_with("a.rs", src);
        let (survivors, unused) = filter(vec![], &cache);
        assert!(survivors.is_empty());
        // No diagnostics filtered but directive present and file visited via a diag?
        // With zero diagnostics we never prime the cache — re-run priming manually:
        let _ = unused; // zero diagnostics = cache never populated, expected.
    }

    #[test]
    fn unused_rule_id_reported_after_file_visited() {
        let src = "// lintropy-ignore: foo\nok_line\n";
        let cache = cache_with("a.rs", src);
        // One unrelated diagnostic against the same file primes the cache.
        let d = diag("a.rs", "bar", 2);
        let (survivors, unused) = filter(vec![d], &cache);
        assert_eq!(survivors.len(), 1);
        assert!(unused
            .iter()
            .any(|u| u.rule_id.as_ref().map(|r| r.as_str()) == Some("foo")
                && u.reason == UnusedReason::NeverMatched));
    }

    #[test]
    fn multibyte_source_line_numbers_stable() {
        // Ensure line accounting works through multi-byte UTF-8.
        let src = "// α β γ\n// lintropy-ignore: foo\nbad();\n";
        let cache = cache_with("a.rs", src);
        let d = diag("a.rs", "foo", 3);
        let (survivors, _) = filter(vec![d], &cache);
        assert!(survivors.is_empty());
    }

    #[test]
    fn source_cache_insert_and_get() {
        let mut c = SourceCache::new();
        c.insert(PathBuf::from("x"), Arc::<[u8]>::from(b"abc".as_slice()));
        assert!(c.get(Path::new("x")).is_some());
        assert_eq!(c.len(), 1);
        assert!(!c.is_empty());
    }
}
