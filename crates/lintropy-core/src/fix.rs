//! Autofix collection, overlap resolution, and atomic application.
//!
//! Implements §8 of the merged lintropy spec. Two public entry points:
//!
//! - [`apply`] mutates files on disk, splicing each accepted [`FixHunk`] into
//!   place in a single descending pass per file and committing via a
//!   `tempfile`-backed atomic rename.
//! - [`dry_run`] produces a concatenated unified-diff string via `similar`
//!   for the caller to print; no files are touched.
//!
//! Only a single pass is performed — cascading fixes require the user to
//! re-run (spec §8).

use std::collections::BTreeMap;
use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};

use similar::TextDiff;
use tempfile::NamedTempFile;

use crate::{Diagnostic, LintropyError, Result, RuleId};

/// Outcome of an [`apply`] call. Reports the files touched, the number of
/// accepted hunks, and any hunks skipped due to overlap.
#[derive(Debug, Clone, Default)]
pub struct FixReport {
    /// Total accepted hunks that were spliced into a file.
    pub applied: usize,
    /// Hunks dropped because they overlapped a higher-priority hunk.
    pub skipped: Vec<OverlapWarning>,
    /// Files that had at least one accepted hunk and were rewritten.
    pub files: Vec<PathBuf>,
}

/// One hunk dropped from [`apply`] because it overlapped a higher-priority
/// hunk in the same file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverlapWarning {
    /// File the skipped hunk targeted.
    pub file: PathBuf,
    /// Rule that produced the skipped hunk.
    pub rule_id: RuleId,
    /// Start byte of the skipped hunk.
    pub byte_start: usize,
    /// End byte of the skipped hunk (exclusive).
    pub byte_end: usize,
}

/// Apply every [`Diagnostic::fix`] present in `diagnostics`, grouping by file
/// and committing each file atomically.
///
/// Hunks that overlap an already-accepted hunk in the same file are skipped
/// and surfaced via [`FixReport::skipped`]. Priority is input order: the
/// first hunk seen for an overlapping byte range wins.
pub fn apply(diagnostics: &[Diagnostic]) -> Result<FixReport> {
    let groups = group_by_file(diagnostics);
    let mut report = FixReport::default();
    for (path, diags) in groups {
        let plan = plan_file(&path, diags);
        report.skipped.extend(plan.skipped);
        if plan.accepted.is_empty() {
            continue;
        }
        let original = fs::read(&path)?;
        let new_bytes = splice_bytes(&original, &plan.accepted);
        atomic_write(&path, &new_bytes)?;
        report.applied += plan.accepted.len();
        report.files.push(path);
    }
    Ok(report)
}

/// Produce a concatenated unified-diff string for every file with at least
/// one accepted hunk. Nothing is written to disk.
pub fn dry_run(diagnostics: &[Diagnostic]) -> Result<String> {
    let groups = group_by_file(diagnostics);
    let mut out = String::new();
    for (path, diags) in groups {
        let plan = plan_file(&path, diags);
        if plan.accepted.is_empty() {
            continue;
        }
        let original = fs::read(&path)?;
        let new_bytes = splice_bytes(&original, &plan.accepted);
        let original_str = String::from_utf8_lossy(&original);
        let new_str = String::from_utf8_lossy(&new_bytes);
        let diff = TextDiff::from_lines(original_str.as_ref(), new_str.as_ref());
        let header = path.display().to_string();
        let unified = diff
            .unified_diff()
            .context_radius(3)
            .header(&header, &header)
            .to_string();
        out.push_str(&unified);
    }
    Ok(out)
}

struct PlannedFile<'a> {
    /// Accepted hunks sorted by `byte_start` descending, safe to splice in order.
    accepted: Vec<&'a Diagnostic>,
    skipped: Vec<OverlapWarning>,
}

fn group_by_file(diagnostics: &[Diagnostic]) -> BTreeMap<PathBuf, Vec<&Diagnostic>> {
    let mut groups: BTreeMap<PathBuf, Vec<&Diagnostic>> = BTreeMap::new();
    for d in diagnostics {
        if d.fix.is_some() {
            groups.entry(d.file.clone()).or_default().push(d);
        }
    }
    groups
}

fn plan_file<'a>(path: &Path, diags: Vec<&'a Diagnostic>) -> PlannedFile<'a> {
    let mut accepted: Vec<&'a Diagnostic> = Vec::new();
    let mut skipped: Vec<OverlapWarning> = Vec::new();
    for d in diags {
        let fix = d
            .fix
            .as_ref()
            .expect("group_by_file only retains diagnostics with Some(fix)");
        let overlaps = accepted.iter().any(|prev| {
            let p = prev.fix.as_ref().expect("invariant: accepted have fix");
            ranges_overlap(fix.byte_start, fix.byte_end, p.byte_start, p.byte_end)
        });
        if overlaps {
            skipped.push(OverlapWarning {
                file: path.to_path_buf(),
                rule_id: d.rule_id.clone(),
                byte_start: fix.byte_start,
                byte_end: fix.byte_end,
            });
        } else {
            accepted.push(d);
        }
    }
    accepted.sort_by(|a, b| {
        let a_start = a.fix.as_ref().unwrap().byte_start;
        let b_start = b.fix.as_ref().unwrap().byte_start;
        b_start.cmp(&a_start)
    });
    PlannedFile { accepted, skipped }
}

fn ranges_overlap(a_start: usize, a_end: usize, b_start: usize, b_end: usize) -> bool {
    a_start < b_end && b_start < a_end
}

fn splice_bytes(src: &[u8], accepted: &[&Diagnostic]) -> Vec<u8> {
    let mut out = src.to_vec();
    for d in accepted {
        let fix = d.fix.as_ref().unwrap();
        out.splice(
            fix.byte_start..fix.byte_end,
            fix.replacement.as_bytes().iter().copied(),
        );
    }
    out
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path.parent().filter(|p| !p.as_os_str().is_empty());
    let mut tmp = match parent {
        Some(dir) => NamedTempFile::new_in(dir)?,
        None => NamedTempFile::new_in(".")?,
    };
    tmp.write_all(bytes)?;
    tmp.as_file_mut().sync_all()?;
    tmp.persist(path).map_err(|e| LintropyError::Io(e.error))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{FixHunk, Severity};
    use std::path::PathBuf;

    fn diag_with_fix(
        file: &str,
        byte_start: usize,
        byte_end: usize,
        replacement: &str,
    ) -> Diagnostic {
        Diagnostic {
            rule_id: RuleId::new("test-rule"),
            severity: Severity::Warning,
            message: "m".into(),
            file: PathBuf::from(file),
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
    fn ranges_overlap_basics() {
        assert!(ranges_overlap(0, 5, 3, 7));
        assert!(ranges_overlap(0, 5, 0, 5));
        assert!(!ranges_overlap(0, 5, 5, 7));
        assert!(!ranges_overlap(5, 7, 0, 5));
    }

    #[test]
    fn splice_descends_and_preserves_multibyte() {
        // α/β are 2 bytes each in UTF-8; replacement must respect byte offsets.
        let src = "α=foo;β=bar".as_bytes();
        let foo_start = "α=".len();
        let foo_end = foo_start + "foo".len();
        let bar_start = "α=foo;β=".len();
        let bar_end = bar_start + "bar".len();
        let diags = vec![
            diag_with_fix("x.rs", foo_start, foo_end, "FOO"),
            diag_with_fix("x.rs", bar_start, bar_end, "BAR"),
        ];
        let plan = plan_file(Path::new("x.rs"), diags.iter().collect());
        let out = splice_bytes(src, &plan.accepted);
        assert_eq!(String::from_utf8(out).unwrap(), "α=FOO;β=BAR");
    }

    #[test]
    fn overlapping_hunks_dropped() {
        let diags = vec![
            diag_with_fix("x.rs", 0, 10, "first"),
            diag_with_fix("x.rs", 5, 15, "second"),
        ];
        let plan = plan_file(Path::new("x.rs"), diags.iter().collect());
        assert_eq!(plan.accepted.len(), 1);
        assert_eq!(plan.skipped.len(), 1);
        assert_eq!(plan.skipped[0].byte_start, 5);
    }

    #[test]
    fn apply_writes_file_atomically() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("f.rs");
        std::fs::write(&path, b"hello world").unwrap();
        let diag = diag_with_fix(path.to_str().unwrap(), 6, 11, "RUST!");
        let report = apply(&[diag]).unwrap();
        assert_eq!(report.applied, 1);
        assert_eq!(report.files, vec![path.clone()]);
        assert!(report.skipped.is_empty());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "hello RUST!");
    }

    #[test]
    fn dry_run_emits_unified_diff() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("f.rs");
        std::fs::write(&path, "line one\nline two\n").unwrap();
        let target = "line two";
        let start = "line one\n".len();
        let end = start + target.len();
        let diag = diag_with_fix(path.to_str().unwrap(), start, end, "line TWO");
        let diff = dry_run(&[diag]).unwrap();
        assert!(diff.contains("-line two"));
        assert!(diff.contains("+line TWO"));
    }
}
