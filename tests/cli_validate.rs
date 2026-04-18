//! Integration tests for `lintropy config validate`.

mod common;

use assert_cmd::Command;
use common::Fixture;
use predicates::prelude::*;

#[test]
fn validate_happy_path_reports_rule_count() {
    let fx = Fixture::new();
    Command::cargo_bin("lintropy")
        .unwrap()
        .current_dir(fx.path())
        .args(["config", "validate"])
        .assert()
        .code(0)
        .stdout(predicate::str::contains("OK:"))
        .stdout(predicate::str::contains("1 rule"));
}

#[test]
fn validate_sad_path_exits_two_on_broken_config() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("lintropy.yaml"), "version: 999\n").unwrap();
    Command::cargo_bin("lintropy")
        .unwrap()
        .current_dir(dir.path())
        .args(["config", "validate"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("unsupported version"));
}
