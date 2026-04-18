//! Multi-rule fixture for description + grouping tests.

#![allow(dead_code)]

use std::fs;
use std::path::Path;

use tempfile::TempDir;

/// Root config — minimal; rules live in `.lintropy/`.
const ROOT: &str = "version: 1\n";

/// Described + tagged Rust rule.
const NO_UNWRAP: &str = r#"language: rust
severity: warning
description: Flags `.unwrap()` on Result/Option.
tags: ["reliability", "rust"]
message: "no unwrap"
query: |
  (call_expression
    function: (field_expression
      field: (field_identifier) @m)
    (#eq? @m "unwrap")) @match
"#;

/// Described + untagged rule. (Plan calls for Python here, but phase-1
/// loader only supports Rust; swap language to Rust with a harmless
/// Rust query. T5 may restore Python once support lands.)
const NO_PRINT: &str = r#"language: rust
severity: info
description: |
  Bans stray print() calls from shipped modules.
  Leave them to tests and scripts.
message: "no print"
query: |
  (call_expression
    function: (identifier) @f
    (#eq? @f "println")) @match
"#;

/// Undescribed + tagged Rust rule.
const NO_DBG: &str = r#"language: rust
severity: error
tags: ["noise"]
message: "no dbg"
query: |
  (macro_invocation
    macro: (identifier) @n
    (#eq? @n "dbg")) @match
"#;

/// Undescribed + untagged minimal Rust rule. Used to exercise the
/// "(untagged)" bucket and the JSON-null description case.
const BARE: &str = r#"language: rust
severity: info
message: "bare"
query: |
  ((identifier) @match (#eq? @match "zzzz_unlikely"))
"#;

pub struct DescribeFixture {
    pub dir: TempDir,
}

impl DescribeFixture {
    pub fn new() -> Self {
        let dir = tempfile::tempdir().expect("create tempdir");
        fs::write(dir.path().join("lintropy.yaml"), ROOT).unwrap();
        let rules = dir.path().join(".lintropy");
        fs::create_dir_all(&rules).unwrap();
        fs::write(rules.join("no-unwrap.rule.yaml"), NO_UNWRAP).unwrap();
        fs::write(rules.join("no-print.rule.yaml"), NO_PRINT).unwrap();
        fs::write(rules.join("no-dbg.rule.yaml"), NO_DBG).unwrap();
        fs::write(rules.join("bare.rule.yaml"), BARE).unwrap();
        Self { dir }
    }

    pub fn path(&self) -> &Path {
        self.dir.path()
    }
}
