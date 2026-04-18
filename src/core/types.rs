//! Shared data types for diagnostics, fixes, and summaries.
//!
//! These shapes mirror §7.1 and §7.3 of the merged lintropy spec
//! (`specs/merged/2026-04-18-lintropy-merged.md`).

use std::path::PathBuf;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Severity of a diagnostic, mirrored from §4.5 of the spec.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// Hardest level; by default causes a non-zero exit.
    Info,
    /// Softer advisory diagnostic.
    Warning,
    /// Build-breaking diagnostic.
    Error,
}

impl Severity {
    /// All severities ordered from least to most severe.
    pub const ALL: [Severity; 3] = [Severity::Info, Severity::Warning, Severity::Error];
}

/// User-visible rule identifier (e.g. `"no-unwrap"`).
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(transparent)]
pub struct RuleId(pub String);

impl RuleId {
    /// Construct a [`RuleId`] from any string-like value.
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    /// Borrow the inner string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for RuleId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<&str> for RuleId {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

impl From<String> for RuleId {
    fn from(value: String) -> Self {
        Self(value)
    }
}

/// Byte, line, and column span inside a source file.
///
/// Lines and columns are 1-based, matching rustc's convention and §7.1
/// of the spec. Byte offsets are 0-based and refer to the source file's
/// UTF-8 bytes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Span {
    /// Path of the file the diagnostic points at.
    pub file: PathBuf,
    /// 1-based line of the span start.
    pub line: usize,
    /// 1-based column of the span start.
    pub column: usize,
    /// 1-based line of the span end (inclusive).
    pub end_line: usize,
    /// 1-based column of the span end (exclusive).
    pub end_column: usize,
    /// 0-based byte offset of the span start.
    pub byte_start: usize,
    /// 0-based byte offset of the span end.
    pub byte_end: usize,
}

/// A single replacement hunk attached to a diagnostic.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct FixHunk {
    /// UTF-8 replacement text that overwrites the byte range.
    pub replacement: String,
    /// 0-based byte offset where the replacement starts.
    pub byte_start: usize,
    /// 0-based byte offset where the replacement ends.
    pub byte_end: usize,
}

/// One emitted diagnostic. Matches §7.1 of the merged spec byte-for-byte.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Diagnostic {
    /// Rule that produced this diagnostic.
    pub rule_id: RuleId,
    /// Severity of the diagnostic.
    pub severity: Severity,
    /// Rendered, capture-interpolated message.
    pub message: String,
    /// Path of the source file the diagnostic points at.
    pub file: PathBuf,
    /// 1-based start line.
    pub line: usize,
    /// 1-based start column.
    pub column: usize,
    /// 1-based end line.
    pub end_line: usize,
    /// 1-based end column.
    pub end_column: usize,
    /// 0-based start byte.
    pub byte_start: usize,
    /// 0-based end byte.
    pub byte_end: usize,
    /// Path of the YAML file that defined the rule.
    pub rule_source: PathBuf,
    /// Optional external documentation URL copied from the rule stanza.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub docs_url: Option<String>,
    /// Optional autofix hunk; present only for query rules with a `fix:` field.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fix: Option<FixHunk>,
}

/// Aggregate run statistics. Matches §7.3 of the merged spec.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Summary {
    /// Count of `error` diagnostics after suppression.
    pub errors: usize,
    /// Count of `warning` diagnostics after suppression.
    pub warnings: usize,
    /// Count of `info` diagnostics after suppression.
    pub infos: usize,
    /// Total files visited by the walker.
    pub files_checked: usize,
    /// Wall-clock run duration in milliseconds.
    pub duration_ms: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn severity_ordering_is_info_warning_error() {
        assert!(Severity::Info < Severity::Warning);
        assert!(Severity::Warning < Severity::Error);
    }

    #[test]
    fn severity_serde_roundtrip() {
        let json = serde_json::to_string(&Severity::Warning).unwrap();
        assert_eq!(json, "\"warning\"");
        let back: Severity = serde_json::from_str(&json).unwrap();
        assert_eq!(back, Severity::Warning);
    }

    #[test]
    fn rule_id_display_matches_inner() {
        let id = RuleId::new("no-unwrap");
        assert_eq!(id.to_string(), "no-unwrap");
        assert_eq!(id.as_str(), "no-unwrap");
    }

    #[test]
    fn diagnostic_serializes_without_optional_fields() {
        let diag = Diagnostic {
            rule_id: RuleId::new("no-unwrap"),
            severity: Severity::Warning,
            message: "avoid .unwrap()".into(),
            file: PathBuf::from("src/main.rs"),
            line: 1,
            column: 1,
            end_line: 1,
            end_column: 2,
            byte_start: 0,
            byte_end: 1,
            rule_source: PathBuf::from(".lintropy/no-unwrap.rule.yaml"),
            docs_url: None,
            fix: None,
        };
        let json = serde_json::to_string(&diag).unwrap();
        assert!(!json.contains("docs_url"));
        assert!(!json.contains("\"fix\""));
    }
}
