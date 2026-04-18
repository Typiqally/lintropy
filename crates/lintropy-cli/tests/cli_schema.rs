//! Integration tests for `lintropy schema`.

use assert_cmd::Command;
use tempfile::tempdir;

#[test]
fn schema_emits_parseable_json_with_properties() {
    let out = Command::cargo_bin("lintropy")
        .unwrap()
        .arg("schema")
        .assert()
        .code(0)
        .get_output()
        .stdout
        .clone();
    let parsed: serde_json::Value = serde_json::from_slice(&out).expect("valid JSON");
    assert!(parsed.as_object().unwrap().contains_key("properties"));
}

#[test]
fn schema_rule_kind_emits_parseable_json() {
    let out = Command::cargo_bin("lintropy")
        .unwrap()
        .args(["schema", "--kind", "rule"])
        .assert()
        .code(0)
        .get_output()
        .stdout
        .clone();
    let parsed: serde_json::Value = serde_json::from_slice(&out).expect("valid JSON");
    assert!(parsed["properties"]
        .as_object()
        .unwrap()
        .contains_key("message"));
}

#[test]
fn schema_can_write_to_file() {
    let dir = tempdir().unwrap();
    let out = dir.path().join("lintropy-rule.schema.json");

    Command::cargo_bin("lintropy")
        .unwrap()
        .args([
            "schema",
            "--kind",
            "rule",
            "--output",
            out.to_str().unwrap(),
        ])
        .assert()
        .code(0);

    let parsed: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(out).unwrap()).expect("valid JSON");
    assert!(parsed["properties"]
        .as_object()
        .unwrap()
        .contains_key("message"));
}
