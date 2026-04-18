//! Shared fixture builder for the CLI integration tests.

#![allow(dead_code)]

pub mod describe;

use std::fs;
use std::path::{Path, PathBuf};

use tempfile::TempDir;

pub const ROOT_CONFIG: &str = r#"version: 1
settings:
  fail_on: error
  default_severity: warning
"#;

pub const NO_UNWRAP_RULE: &str = r#"language: rust
severity: warning
message: "avoid .unwrap() on `{{recv}}`"
include: ["**/*.rs"]
query: |
  (call_expression
    function: (field_expression
      value: (_) @recv
      field: (field_identifier) @method)
    (#eq? @method "unwrap")) @match
fix: '{{recv}}.expect("TODO: handle error")'
"#;

pub const SAMPLE_SOURCE: &str =
    "fn main() {\n    let x: Option<i32> = Some(1);\n    let _ = x.unwrap();\n}\n";

/// A populated fixture with `lintropy.yaml`, a single rule file, and a
/// sample Rust source that triggers the rule.
pub struct Fixture {
    pub dir: TempDir,
}

impl Fixture {
    pub fn new() -> Self {
        let dir = tempfile::tempdir().expect("create tempdir");
        fs::write(dir.path().join("lintropy.yaml"), ROOT_CONFIG).unwrap();
        fs::create_dir_all(dir.path().join(".lintropy")).unwrap();
        fs::write(
            dir.path().join(".lintropy/no-unwrap.rule.yaml"),
            NO_UNWRAP_RULE,
        )
        .unwrap();
        fs::write(dir.path().join("sample.rs"), SAMPLE_SOURCE).unwrap();
        Self { dir }
    }

    pub fn path(&self) -> &Path {
        self.dir.path()
    }

    pub fn sample(&self) -> PathBuf {
        self.dir.path().join("sample.rs")
    }
}
