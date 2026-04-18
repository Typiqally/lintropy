//! Integration tests for `lintropy explain` description handling.

mod common;

use assert_cmd::Command;
use common::describe::DescribeFixture;
use predicates::prelude::*;

#[test]
fn explain_prints_description_block_when_present() {
    let fx = DescribeFixture::new();
    Command::cargo_bin("lintropy")
        .unwrap()
        .current_dir(fx.path())
        .args(["explain", "no-unwrap"])
        .assert()
        .code(0)
        .stdout(predicate::str::contains("description:"))
        .stdout(predicate::str::contains(
            "Flags `.unwrap()` on Result/Option.",
        ));
}

#[test]
fn explain_omits_description_block_when_absent() {
    let fx = DescribeFixture::new();
    let out = Command::cargo_bin("lintropy")
        .unwrap()
        .current_dir(fx.path())
        .args(["explain", "no-dbg"])
        .assert()
        .code(0)
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(out).unwrap();
    assert!(
        !text.contains("description:"),
        "no-dbg has no description, but `description:` header appeared:\n{text}"
    );
}
