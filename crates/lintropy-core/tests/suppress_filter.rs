//! Integration tests for `lintropy_core::suppress::filter`.

use std::path::PathBuf;
use std::sync::Arc;

use lintropy_core::{suppress, Diagnostic, RuleId, Severity, SourceCache, UnusedReason};

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
        fix: None,
    }
}

fn cache(path: &str, src: &str) -> SourceCache {
    let mut c = SourceCache::new();
    c.insert(PathBuf::from(path), Arc::<[u8]>::from(src.as_bytes()));
    c
}

#[test]
fn ignore_next_line_suppresses_single_diagnostic() {
    let src = "ok1\n// lintropy-ignore: foo\nbad\nok2\n";
    let cache = cache("f.rs", src);
    let d = diag("f.rs", "foo", 3);
    let (survivors, unused) = suppress::filter(vec![d], &cache);
    assert!(survivors.is_empty());
    assert!(unused.is_empty());
}

#[test]
fn ignore_file_suppresses_everywhere() {
    let src = "// lintropy-ignore-file: foo\nbad();\nbad();\nbad();\n";
    let cache = cache("f.rs", src);
    let d1 = diag("f.rs", "foo", 2);
    let d2 = diag("f.rs", "foo", 3);
    let d3 = diag("f.rs", "foo", 4);
    let (survivors, _) = suppress::filter(vec![d1, d2, d3], &cache);
    assert!(survivors.is_empty());
}

#[test]
fn ignore_file_on_line_21_has_no_effect() {
    let mut src = String::new();
    for _ in 0..20 {
        src.push_str("//\n");
    }
    src.push_str("// lintropy-ignore-file: foo\n");
    src.push_str("bad();\n");
    let cache = cache("f.rs", &src);
    let d = diag("f.rs", "foo", 22);
    let (survivors, _) = suppress::filter(vec![d], &cache);
    assert_eq!(survivors.len(), 1);
}

#[test]
fn wildcard_is_rejected_with_warning() {
    let src = "// lintropy-ignore: *\nbad();\n";
    let cache = cache("f.rs", src);
    let d = diag("f.rs", "foo", 2);
    let (survivors, unused) = suppress::filter(vec![d], &cache);
    assert_eq!(survivors.len(), 1, "wildcard must not suppress anything");
    assert!(unused
        .iter()
        .any(|u| u.reason == UnusedReason::WildcardRejected && u.rule_id.is_none()));
}

#[test]
fn trailing_code_directive_ignored() {
    let src = "bad(); // lintropy-ignore: foo\n";
    let cache = cache("f.rs", src);
    let d = diag("f.rs", "foo", 1);
    let (survivors, unused) = suppress::filter(vec![d], &cache);
    assert_eq!(survivors.len(), 1);
    assert!(unused.is_empty());
}

#[test]
fn unused_directive_reported_as_never_matched() {
    let src = "// lintropy-ignore: foo\nok();\n";
    let cache = cache("f.rs", src);
    // prime the cache by filtering an unrelated diagnostic on the same file
    let primer = diag("f.rs", "other", 2);
    let (survivors, unused) = suppress::filter(vec![primer], &cache);
    assert_eq!(survivors.len(), 1);
    let hit = unused
        .iter()
        .find(|u| u.rule_id.as_ref().map(|r| r.as_str()) == Some("foo"))
        .expect("unused entry for foo");
    assert_eq!(hit.reason, UnusedReason::NeverMatched);
    assert_eq!(hit.line, 1);
    assert_eq!(hit.file, PathBuf::from("f.rs"));
}

#[test]
fn missing_cache_entry_passes_diagnostic_through() {
    let cache = SourceCache::new();
    let d = diag("f.rs", "foo", 1);
    let (survivors, unused) = suppress::filter(vec![d], &cache);
    assert_eq!(survivors.len(), 1);
    assert!(unused.is_empty());
}

#[test]
fn multiple_rules_in_directive_suppress_independently() {
    let src = "// lintropy-ignore: foo, bar\nbad();\n";
    let cache = cache("f.rs", src);
    let d1 = diag("f.rs", "foo", 2);
    let d2 = diag("f.rs", "bar", 2);
    let d3 = diag("f.rs", "baz", 2);
    let (survivors, _) = suppress::filter(vec![d1, d2, d3], &cache);
    assert_eq!(survivors.len(), 1);
    assert_eq!(survivors[0].rule_id.as_str(), "baz");
}

#[test]
fn directive_scoped_to_target_line_only() {
    let src = "// lintropy-ignore: foo\nbad();\nbad();\n";
    let cache = cache("f.rs", src);
    let on_target = diag("f.rs", "foo", 2);
    let next_line = diag("f.rs", "foo", 3);
    let (survivors, _) = suppress::filter(vec![on_target, next_line], &cache);
    assert_eq!(survivors.len(), 1);
    assert_eq!(survivors[0].line, 3);
}
