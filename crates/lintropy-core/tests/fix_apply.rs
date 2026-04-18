//! Integration tests for `lintropy_core::fix::{apply, dry_run}`.

use std::path::PathBuf;

use lintropy_core::{fix, Diagnostic, FixHunk, RuleId, Severity};
use tempfile::tempdir;

fn diag(
    file: &std::path::Path,
    byte_start: usize,
    byte_end: usize,
    replacement: &str,
) -> Diagnostic {
    Diagnostic {
        rule_id: RuleId::new("test-rule"),
        severity: Severity::Warning,
        message: "m".into(),
        file: file.to_path_buf(),
        line: 1,
        column: 1,
        end_line: 1,
        end_column: 1,
        byte_start,
        byte_end,
        rule_source: PathBuf::from(".lintropy/test-rule.rule.yaml"),
        docs_url: None,
        fix: Some(FixHunk {
            replacement: replacement.to_string(),
            byte_start,
            byte_end,
        }),
    }
}

#[test]
fn apply_single_hunk_rewrites_file() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("x.rs");
    std::fs::write(&path, "fn foo() { todo!() }\n").unwrap();
    let start = "fn foo() { ".len();
    let end = start + "todo!()".len();
    let d = diag(&path, start, end, "panic!(\"unimplemented\")");
    let report = fix::apply(&[d]).unwrap();
    assert_eq!(report.applied, 1);
    assert_eq!(report.files, vec![path.clone()]);
    assert!(report.skipped.is_empty());
    let out = std::fs::read_to_string(&path).unwrap();
    assert!(out.contains("panic!(\"unimplemented\")"));
}

#[test]
fn apply_multiple_non_overlapping_in_same_file() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("x.rs");
    std::fs::write(&path, "let a = 1; let b = 2;").unwrap();
    let a_start = "let a = ".len();
    let a_end = a_start + "1".len();
    let b_start = "let a = 1; let b = ".len();
    let b_end = b_start + "2".len();
    let report = fix::apply(&[
        diag(&path, a_start, a_end, "10"),
        diag(&path, b_start, b_end, "20"),
    ])
    .unwrap();
    assert_eq!(report.applied, 2);
    assert_eq!(
        std::fs::read_to_string(&path).unwrap(),
        "let a = 10; let b = 20;"
    );
}

#[test]
fn apply_drops_overlap_and_reports() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("x.rs");
    std::fs::write(&path, "abcdefghij").unwrap();
    let first = diag(&path, 0, 5, "XXXXX");
    let second = diag(&path, 3, 8, "YYYYY");
    let report = fix::apply(&[first, second]).unwrap();
    assert_eq!(report.applied, 1);
    assert_eq!(report.skipped.len(), 1);
    assert_eq!(report.skipped[0].byte_start, 3);
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "XXXXXfghij");
}

#[test]
fn dry_run_produces_diff_without_touching_disk() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("x.rs");
    std::fs::write(&path, "hello world\n").unwrap();
    let start = "hello ".len();
    let end = start + "world".len();
    let d = diag(&path, start, end, "RUST");
    let diff = fix::dry_run(&[d]).unwrap();
    assert!(diff.contains("-hello world"));
    assert!(diff.contains("+hello RUST"));
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "hello world\n");
}

#[test]
fn multibyte_splice_preserves_trailing_bytes() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("x.rs");
    let src = "α-start-β-end-γ";
    std::fs::write(&path, src).unwrap();
    // replace "start" and "end"
    let start_begin = "α-".len();
    let start_end = start_begin + "start".len();
    let end_begin = "α-start-β-".len();
    let end_end = end_begin + "end".len();
    let report = fix::apply(&[
        diag(&path, start_begin, start_end, "START"),
        diag(&path, end_begin, end_end, "END"),
    ])
    .unwrap();
    assert_eq!(report.applied, 2);
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "α-START-β-END-γ");
}

#[test]
fn diagnostics_without_fix_are_ignored() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("x.rs");
    std::fs::write(&path, "noop").unwrap();
    let no_fix = Diagnostic {
        fix: None,
        ..diag(&path, 0, 0, "")
    };
    let report = fix::apply(&[no_fix]).unwrap();
    assert_eq!(report.applied, 0);
    assert!(report.files.is_empty());
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "noop");
}
