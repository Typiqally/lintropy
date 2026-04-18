//! Shared types and core implementation for lintropy.

pub mod config;
pub mod discovery;
pub mod engine;
pub mod error;
pub mod predicates;
pub mod schema;
pub mod template;
pub mod types;

pub use config::{Config, ConfigWarning, MatchRule, QueryRule, RuleConfig, RuleKind, Settings};
pub use error::{LintropyError, Result};
pub use types::{Diagnostic, FixHunk, RuleId, Severity, Span, Summary};
