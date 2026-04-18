//! Shared types, errors, and traits that other lintropy crates program against.
//!
//! This crate owns:
//!
//! - the canonical diagnostic shape (§7.1 of the merged spec)
//! - the `LintropyError` enum and `Result<T>` alias
//! - the config loader ([`config`]) + discovery helpers ([`discovery`])
//! - the JSON Schema wrapper for `lintropy schema` ([`schema`])
//! - the [`predicates`] seam that WP2 implements
//!
//! Heavier grammar dependencies live in `lintropy-langs`.

pub mod config;
pub mod discovery;
pub mod error;
pub mod predicates;
pub mod schema;
pub mod types;

pub use config::{Config, ConfigWarning, MatchRule, QueryRule, RuleConfig, RuleKind, Settings};
pub use error::{LintropyError, Result};
pub use types::{Diagnostic, FixHunk, RuleId, Severity, Span, Summary};
