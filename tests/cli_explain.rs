//! Integration tests for `lintropy explain`.

mod common;

use assert_cmd::Command;
use common::Fixture;
use predicates::prelude::*;

#[test]
fn explain_prints_rule_details() {
    let fx = Fixture::new();
    Command::cargo_bin("lintropy")
        .unwrap()
        .current_dir(fx.path())
        .args(["explain", "no-unwrap"])
        .assert()
        .code(0)
        .stdout(predicate::str::contains("rule:"))
        .stdout(predicate::str::contains("no-unwrap"))
        .stdout(predicate::str::contains("query:"));
}

#[test]
fn explain_unknown_rule_exits_two() {
    let fx = Fixture::new();
    Command::cargo_bin("lintropy")
        .unwrap()
        .current_dir(fx.path())
        .args(["explain", "does-not-exist"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("unknown rule id"));
}
