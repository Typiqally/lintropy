//! Thin wrapper around [`Config::json_schema`] for the `lintropy schema`
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
