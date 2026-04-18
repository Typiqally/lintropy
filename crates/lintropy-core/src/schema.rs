//! Thin wrapper around [`Config`] JSON Schema helpers for the `lintropy schema`
//! subcommand (§10.1 of the merged spec).
//!
//! Kept as its own module so callers can `use lintropy_core::schema;`
//! without dragging the entire config API into scope.

use crate::config::Config;

/// JSON Schema describing a root `lintropy.yaml` file.
///
/// Delegates to [`Config::json_schema`]. Returns `serde_json::Value` so the
/// CLI can pretty-print it without re-importing `schemars` types.
pub fn json_schema() -> serde_json::Value {
    Config::json_schema()
}

/// JSON Schema describing a root `lintropy.yaml` file.
pub fn root_json_schema() -> serde_json::Value {
    Config::root_json_schema()
}

/// JSON Schema describing a single `.lintropy/*.rule.yaml` file.
pub fn rule_json_schema() -> serde_json::Value {
    Config::rule_json_schema()
}

/// JSON Schema describing a grouped `.lintropy/*.rules.yaml` file.
pub fn rules_file_json_schema() -> serde_json::Value {
    Config::rules_file_json_schema()
}
