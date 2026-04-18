//! Integration tests for the config loader (WP1).
//!
//! Every test scaffolds a throwaway project tree via [`tempfile::TempDir`]
//! rather than shipping on-disk fixtures so the tests stay self-describing
//! and portable across platforms.

use std::fs;
use std::path::Path;

use lintropy_core::{Config, LintropyError, RuleKind, Severity};
use tempfile::TempDir;

fn write(path: &Path, contents: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, contents).unwrap();
}

fn project(root_yaml: &str) -> TempDir {
    let dir = TempDir::new().unwrap();
    write(&dir.path().join("lintropy.yaml"), root_yaml);
    dir
}

const BASE_ROOT: &str = "version: 1\n";

const INLINE_NO_DBG: &str = r#"version: 1
settings:
  default_severity: warning
  fail_on: error
rules:
  - id: no-dbg
    severity: error
    message: "stray dbg! at {{macro}}"
    language: rust
    query: |
      (macro_invocation
        macro: (identifier) @macro
        (#eq? @macro "dbg")) @match
"#;

#[test]
fn loads_happy_path_inline_rule() {
    let dir = project(INLINE_NO_DBG);
    let config = Config::load_from_root(dir.path()).expect("load");
    assert_eq!(config.version, 1);
    assert_eq!(config.settings.fail_on, Severity::Error);
    assert_eq!(config.settings.default_severity, Severity::Warning);
    assert_eq!(config.rules.len(), 1);

    let rule = &config.rules[0];
    assert_eq!(rule.id.as_str(), "no-dbg");
    assert_eq!(rule.severity, Severity::Error);
    assert!(matches!(rule.kind, RuleKind::Query(_)));
    assert_eq!(rule.source_path, config.root_config);
    assert!(config.warnings.is_empty());
}

#[test]
fn discovers_rule_files_under_lintropy_dir() {
    let dir = project(BASE_ROOT);
    write(
        &dir.path().join(".lintropy/no-unwrap.rule.yaml"),
        r#"severity: warning
message: "avoid .unwrap() on `{{recv}}`"
language: rust
query: |
  (call_expression
    function: (field_expression
      value: (_) @recv
      field: (field_identifier) @method)
    (#eq? @method "unwrap")) @match
"#,
    );
    write(
        &dir.path()
            .join(".lintropy/architecture/no-println.rule.yaml"),
        r#"severity: info
message: "no println in domain"
language: rust
query: |
  (macro_invocation
    macro: (identifier) @n
    (#eq? @n "println")) @match
"#,
    );

    let config = Config::load_from_root(dir.path()).unwrap();
    let ids: Vec<&str> = config.rules.iter().map(|r| r.id.as_str()).collect();
    assert!(ids.contains(&"no-unwrap"));
    assert!(ids.contains(&"no-println"));
    assert_eq!(config.rules.len(), 2);
}

#[test]
fn loads_multi_rule_rules_yaml() {
    let dir = project(BASE_ROOT);
    write(
        &dir.path().join(".lintropy/style.rules.yaml"),
        r#"rules:
  - id: no-todo
    severity: warning
    message: "TODO comment leaked into {{match}}"
    language: rust
    query: |
      (line_comment) @match
  - id: no-println
    severity: error
    message: "println not allowed"
    language: rust
    query: |
      (macro_invocation
        macro: (identifier) @n
        (#eq? @n "println")) @match
"#,
    );
    let config = Config::load_from_root(dir.path()).unwrap();
    assert_eq!(config.rules.len(), 2);
    let ids: Vec<&str> = config.rules.iter().map(|r| r.id.as_str()).collect();
    assert!(ids.contains(&"no-todo"));
    assert!(ids.contains(&"no-println"));
}

#[test]
fn rule_yaml_defaults_id_to_file_stem() {
    let dir = project(BASE_ROOT);
    write(
        &dir.path().join(".lintropy/my-rule.rule.yaml"),
        r#"severity: warning
message: "stub"
language: rust
query: "(identifier) @match"
"#,
    );
    let config = Config::load_from_root(dir.path()).unwrap();
    assert_eq!(config.rules[0].id.as_str(), "my-rule");
}

#[test]
fn rules_yaml_requires_explicit_id() {
    let dir = project(BASE_ROOT);
    write(
        &dir.path().join(".lintropy/group.rules.yaml"),
        r#"rules:
  - severity: warning
    message: "stub"
    language: rust
    query: "(identifier) @match"
"#,
    );
    let err = Config::load_from_root(dir.path()).unwrap_err();
    assert!(
        matches!(err, LintropyError::ConfigLoad(ref m) if m.contains("missing required `id`")),
        "got {err:?}"
    );
}

#[test]
fn errors_on_duplicate_rule_id() {
    let dir = project(BASE_ROOT);
    write(
        &dir.path().join(".lintropy/a.rule.yaml"),
        r#"id: same-id
severity: warning
message: "a"
language: rust
query: "(identifier) @match"
"#,
    );
    write(
        &dir.path().join(".lintropy/b.rule.yaml"),
        r#"id: same-id
severity: warning
message: "b"
language: rust
query: "(identifier) @match"
"#,
    );
    let err = Config::load_from_root(dir.path()).unwrap_err();
    match err {
        LintropyError::DuplicateRuleId { rule_id, .. } => assert_eq!(rule_id, "same-id"),
        other => panic!("expected DuplicateRuleId, got {other:?}"),
    }
}

#[test]
fn errors_on_unknown_capture_in_message() {
    let dir = project(BASE_ROOT);
    write(
        &dir.path().join(".lintropy/bad.rule.yaml"),
        r#"severity: warning
message: "refers to {{never_captured}}"
language: rust
query: "(identifier) @match"
"#,
    );
    let err = Config::load_from_root(dir.path()).unwrap_err();
    match err {
        LintropyError::UnknownCapture {
            capture, rule_id, ..
        } => {
            assert_eq!(capture, "never_captured");
            assert_eq!(rule_id, "bad");
        }
        other => panic!("expected UnknownCapture, got {other:?}"),
    }
}

#[test]
fn errors_on_unknown_capture_in_fix() {
    let dir = project(BASE_ROOT);
    write(
        &dir.path().join(".lintropy/bad-fix.rule.yaml"),
        r#"severity: warning
message: "ok"
fix: "{{missing}}"
language: rust
query: "(identifier) @match"
"#,
    );
    let err = Config::load_from_root(dir.path()).unwrap_err();
    assert!(
        matches!(&err, LintropyError::UnknownCapture { capture, .. } if capture == "missing"),
        "got {err:?}"
    );
}

#[test]
fn errors_on_query_compile_failure() {
    let dir = project(BASE_ROOT);
    write(
        &dir.path().join(".lintropy/broken.rule.yaml"),
        r#"severity: warning
message: "broken"
language: rust
query: "(not_a_real_node_kind @match"
"#,
    );
    let err = Config::load_from_root(dir.path()).unwrap_err();
    match err {
        LintropyError::QueryCompile { rule_id, .. } => assert_eq!(rule_id, "broken"),
        other => panic!("expected QueryCompile, got {other:?}"),
    }
}

#[test]
fn load_from_path_overrides_root_discovery() {
    let dir = TempDir::new().unwrap();
    let custom_path = dir.path().join("custom.yaml");
    write(&custom_path, INLINE_NO_DBG);
    let config = Config::load_from_path(&custom_path).unwrap();
    assert_eq!(config.rules.len(), 1);
    assert_eq!(config.rules[0].id.as_str(), "no-dbg");
}

#[test]
fn warns_when_query_rule_omits_match_capture() {
    let dir = project(BASE_ROOT);
    write(
        &dir.path().join(".lintropy/no-match.rule.yaml"),
        r#"severity: warning
message: "loose span"
language: rust
query: "(identifier) @name"
"#,
    );
    let config = Config::load_from_root(dir.path()).unwrap();
    assert_eq!(config.warnings.len(), 1);
    let w = &config.warnings[0];
    assert_eq!(w.rule_id.as_ref().unwrap().as_str(), "no-match");
    assert!(w.message.contains("@match"));
}

#[test]
fn query_rule_with_match_capture_emits_no_warning() {
    let dir = project(BASE_ROOT);
    write(
        &dir.path().join(".lintropy/with-match.rule.yaml"),
        r#"severity: warning
message: "ok"
language: rust
query: "(identifier) @match"
"#,
    );
    let config = Config::load_from_root(dir.path()).unwrap();
    assert!(config.warnings.is_empty());
}

#[test]
fn errors_on_mixed_query_and_match_keys() {
    let dir = project(BASE_ROOT);
    write(
        &dir.path().join(".lintropy/mixed.rule.yaml"),
        r#"severity: warning
message: "cannot mix"
language: rust
query: "(identifier) @match"
forbid: "TODO"
"#,
    );
    let err = Config::load_from_root(dir.path()).unwrap_err();
    assert!(matches!(err, LintropyError::ConfigLoad(ref m) if m.contains("both")));
}

#[test]
fn errors_on_query_rule_without_language() {
    let dir = project(BASE_ROOT);
    write(
        &dir.path().join(".lintropy/no-lang.rule.yaml"),
        r#"severity: warning
message: "missing language"
query: "(identifier) @match"
"#,
    );
    let err = Config::load_from_root(dir.path()).unwrap_err();
    assert!(matches!(err, LintropyError::ConfigLoad(ref m) if m.contains("language")));
}

#[test]
fn match_rules_rejected_until_phase_two() {
    let dir = project(BASE_ROOT);
    write(
        &dir.path().join(".lintropy/forbid-only.rule.yaml"),
        r#"severity: error
message: "no console.log"
forbid: 'console\.log'
"#,
    );
    let err = Config::load_from_root(dir.path()).unwrap_err();
    assert!(matches!(err, LintropyError::Unsupported(_)), "got {err:?}");
}

#[test]
fn version_other_than_one_is_rejected() {
    let dir = project("version: 2\n");
    let err = Config::load_from_root(dir.path()).unwrap_err();
    assert!(matches!(err, LintropyError::ConfigLoad(ref m) if m.contains("version 2")));
}

#[test]
fn missing_root_config_errors_with_configload() {
    let dir = TempDir::new().unwrap();
    let err = Config::load_from_root(dir.path()).unwrap_err();
    assert!(matches!(err, LintropyError::ConfigLoad(_)));
}

#[test]
fn severity_defaults_to_settings_default() {
    let dir = project(
        r#"version: 1
settings:
  default_severity: info
"#,
    );
    write(
        &dir.path().join(".lintropy/defaulted.rule.yaml"),
        r#"message: "defaulted"
language: rust
query: "(identifier) @match"
"#,
    );
    let config = Config::load_from_root(dir.path()).unwrap();
    assert_eq!(config.rules[0].severity, Severity::Info);
}

#[test]
fn json_schema_is_emittable_and_nontrivial() {
    let schema = Config::json_schema();
    let rendered = serde_json::to_string(&schema).unwrap();
    assert!(rendered.contains("version"));
    assert!(rendered.contains("rules"));
    assert!(rendered.contains("settings"));
}
