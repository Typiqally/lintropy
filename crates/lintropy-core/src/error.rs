//! Crate-wide error type and [`Result`] alias.

use std::path::PathBuf;

use thiserror::Error;

/// Every fallible lintropy operation returns this.
pub type Result<T> = std::result::Result<T, LintropyError>;

/// Top-level error variant shared across all lintropy crates.
///
/// Individual crates may carry richer context inside the string payloads;
/// the variants themselves are intentionally coarse so downstream code can
/// branch on kind rather than message text.
#[derive(Debug, Error)]
pub enum LintropyError {
    /// Failed to discover, parse, or validate user config.
    #[error("config load error: {0}")]
    ConfigLoad(String),

    /// A rule's tree-sitter query failed to compile.
    #[error("query compile error in rule `{rule_id}` ({source_path}): {message}")]
    QueryCompile {
        /// Offending rule id.
        rule_id: String,
        /// YAML file that defined the rule.
        source_path: PathBuf,
        /// Compiler-provided error message.
        message: String,
    },

    /// A `{{capture}}` in `message` or `fix` names a capture the query does not define.
    #[error("unknown capture `{capture}` in rule `{rule_id}` ({source_path})")]
    UnknownCapture {
        /// Offending rule id.
        rule_id: String,
        /// YAML file that defined the rule.
        source_path: PathBuf,
        /// Name of the unresolved capture.
        capture: String,
    },

    /// A query references a custom predicate that lintropy does not implement.
    #[error("unknown custom predicate `#{predicate}?` in rule `{rule_id}` ({source_path})")]
    UnknownPredicate {
        /// Offending rule id.
        rule_id: String,
        /// YAML file that defined the rule.
        source_path: PathBuf,
        /// Predicate name (without the leading `#` or trailing `?`).
        predicate: String,
    },

    /// Two or more rule files declare the same `id`.
    #[error("duplicate rule id `{rule_id}` defined in {first} and {second}")]
    DuplicateRuleId {
        /// The colliding id.
        rule_id: String,
        /// First source path (in discovery order).
        first: PathBuf,
        /// Second source path.
        second: PathBuf,
    },

    /// Filesystem-level error.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    /// YAML parse error.
    #[error("yaml error: {0}")]
    Yaml(String),

    /// Unexpected internal invariant violation.
    #[error("internal error: {0}")]
    Internal(String),

    /// Rule uses a feature lintropy has not shipped yet (e.g. match rules in Phase 1).
    #[error("unsupported feature: {0}")]
    Unsupported(String),
}
