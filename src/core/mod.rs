//! Shared types and core implementation for lintropy.

pub mod config;
pub mod discovery;
pub mod engine;
pub mod error;
pub mod fix;
pub mod predicates;
pub mod schema;
pub mod suppress;
pub mod template;
pub mod types;

pub use config::{Config, ConfigWarning, MatchRule, QueryRule, RuleConfig, RuleKind, Settings};
pub use error::{LintropyError, Result};
pub use fix::{FixReport, OverlapWarning};
pub use suppress::{SourceCache, UnusedReason, UnusedSuppression};
pub use types::{Diagnostic, FixHunk, RuleId, Severity, Span, Summary};
