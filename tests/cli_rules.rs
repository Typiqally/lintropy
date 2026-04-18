//! Integration tests for `lintropy rules`.

mod common;

use assert_cmd::Command;
use common::Fixture;
use predicates::prelude::*;

#[test]
fn rules_text_lists_every_rule() {
    let fx = Fixture::new();
    Command::cargo_bin("lintropy")
        .unwrap()
        .current_dir(fx.path())
        .arg("rules")
        .assert()
        .code(0)
        .stdout(predicate::str::contains("no-unwrap"))
        .stdout(predicate::str::contains("[warning]"));
}

#[test]
fn rules_json_format_is_valid_array() {
    let fx = Fixture::new();
    let out = Command::cargo_bin("lintropy")
        .unwrap()
        .current_dir(fx.path())
        .args(["rules", "--format", "json"])
        .assert()
        .code(0)
        .get_output()
        .stdout
        .clone();
    let parsed: serde_json::Value = serde_json::from_slice(&out).expect("valid JSON");
    let arr = parsed.as_array().expect("array");
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["id"], "no-unwrap");
    assert_eq!(arr[0]["severity"], "warning");
    assert_eq!(arr[0]["language"], "rust");
    assert_eq!(arr[0]["kind"], "query");
}
