//! Integration tests for `lintropy rules` description + grouping.

mod common;

use assert_cmd::Command;
use common::describe::DescribeFixture;
use predicates::prelude::*;

fn run_rules(fx: &DescribeFixture, args: &[&str]) -> assert_cmd::assert::Assert {
    let mut cmd = Command::cargo_bin("lintropy").unwrap();
    cmd.current_dir(fx.path()).arg("rules");
    for a in args {
        cmd.arg(a);
    }
    cmd.assert()
}

#[test]
fn rules_text_default_shows_description_line() {
    let fx = DescribeFixture::new();
    run_rules(&fx, &[])
        .code(0)
        .stdout(predicate::str::contains("no-unwrap"))
        .stdout(predicate::str::contains(
            "Flags `.unwrap()` on Result/Option.",
        ))
        .stdout(predicate::str::contains("tags: reliability, rust"))
        .stdout(predicate::str::contains(
            "source: .lintropy/no-unwrap.rule.yaml",
        ));
}

#[test]
fn rules_text_hides_description_when_absent() {
    let fx = DescribeFixture::new();
    let out = run_rules(&fx, &[]).code(0).get_output().stdout.clone();
    let text = String::from_utf8(out).unwrap();

    // The no-dbg rule has no description.
    let idx = text
        .find("no-dbg")
        .expect("no-dbg rule should appear in output");
    let after = &text[idx..];
    let next_blank = after.find("\n\n").unwrap_or(after.len());
    let block = &after[..next_blank];
    assert!(
        !block.contains("Flags"),
        "no-dbg block should have no description, got:\n{block}"
    );
}
