//! Thin wrapper around [`Config`] JSON Schema helpers for the `lintropy schema`
//! subcommand (§10.1 of the merged spec).
//!
//! Kept as its own module so callers can `use super::core::schema;`
//! without dragging the entire config API into scope.

use serde_json::{json, Value};

use super::config::Config;

/// JSON Schema describing a root `lintropy.yaml` file.
///
/// Delegates to [`Config::json_schema`]. Returns `serde_json::Value` so the
/// CLI can pretty-print it without re-importing `schemars` types.
pub fn json_schema() -> Value {
    flatten_severity(Config::json_schema())
}

/// JSON Schema describing a root `lintropy.yaml` file.
pub fn root_json_schema() -> Value {
    flatten_severity(Config::root_json_schema())
}

/// JSON Schema describing a single `.lintropy/*.rule.yaml` file.
pub fn rule_json_schema() -> Value {
    flatten_severity(Config::rule_json_schema())
}

/// JSON Schema describing a grouped `.lintropy/*.rules.yaml` file.
pub fn rules_file_json_schema() -> Value {
    flatten_severity(Config::rules_file_json_schema())
}

/// Rewrite the `Severity` definition into a single flat string enum so
/// YAML editors surface `info | warning | error` as direct autocomplete
/// candidates. schemars emits a `oneOf` of per-variant constants by
/// default, which is technically valid but most YAML LSPs don't unpack
/// it into completion items. `enumDescriptions` keeps the per-variant
/// docs (JetBrains and redhat.vscode-yaml both render them).
fn flatten_severity(mut schema: Value) -> Value {
    if let Some(defs) = schema.get_mut("definitions").and_then(Value::as_object_mut) {
        if defs.contains_key("Severity") {
            defs.insert("Severity".into(), severity_enum_schema());
        }
    }
    schema
}

fn severity_enum_schema() -> Value {
    json!({
        "type": "string",
        "description": "Severity of a diagnostic emitted when the rule matches.",
        "enum": ["info", "warning", "error"],
        "enumDescriptions": [
            "Advisory; informational only.",
            "Soft failure; visible but does not fail the build by default.",
            "Build-breaking diagnostic."
        ]
    })
}
