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

fn json_output(fx: &DescribeFixture, args: &[&str]) -> serde_json::Value {
    let mut cmd = Command::cargo_bin("lintropy").unwrap();
    cmd.current_dir(fx.path()).arg("rules");
    for a in args {
        cmd.arg(a);
    }
    let out = cmd.assert().code(0).get_output().stdout.clone();
    serde_json::from_slice(&out).expect("valid JSON")
}

#[test]
fn rules_json_description_null_when_absent() {
    let fx = DescribeFixture::new();
    let arr = json_output(&fx, &["--format", "json"]);
    let arr = arr.as_array().unwrap();
    let dbg_rule = arr
        .iter()
        .find(|o| o["id"] == "no-dbg")
        .expect("no-dbg entry");
    assert_eq!(dbg_rule["description"], serde_json::Value::Null);
}

#[test]
fn rules_json_description_string_when_present() {
    let fx = DescribeFixture::new();
    let arr = json_output(&fx, &["--format", "json"]);
    let arr = arr.as_array().unwrap();
    let unwrap_rule = arr
        .iter()
        .find(|o| o["id"] == "no-unwrap")
        .expect("no-unwrap entry");
    assert_eq!(
        unwrap_rule["description"],
        "Flags `.unwrap()` on Result/Option."
    );
}

#[test]
fn rules_text_group_by_language_produces_rust_group() {
    let fx = DescribeFixture::new();
    let out = run_rules(&fx, &["--group-by", "language"])
        .code(0)
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(out).unwrap();

    // There is exactly one language group: rust.
    let rust_header = text
        .find("rust\n----")
        .expect("expected `rust` group header with underline");

    // The `(any)` bucket should not appear — every fixture rule has a language.
    assert!(
        !text.contains("(any)"),
        "unexpected (any) bucket in language-grouped output:\n{text}"
    );

    // Rules that the spec pins as Rust must appear under the rust header.
    // Other fixture rules are Rust today because MVP supports only Rust; when
    // more languages land, `DescribeFixture` and this test should be revisited.
    for id in ["no-dbg", "no-unwrap"] {
        let pos = text.find(id).unwrap_or_else(|| panic!("{id} missing"));
        assert!(
            pos > rust_header,
            "{id} should appear after the rust group header"
        );
    }
}

#[test]
fn rules_text_group_by_tag_first_tag_wins_untagged_last() {
    let fx = DescribeFixture::new();
    let out = run_rules(&fx, &["--group-by", "tag"])
        .code(0)
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(out).unwrap();

    let noise_idx = text.find("noise\n-----").expect("noise group header");
    let reliability_idx = text
        .find("reliability\n-----------")
        .expect("reliability group header");
    let untagged_idx = text
        .find("(untagged)\n----------")
        .expect("(untagged) group header");

    assert!(noise_idx < reliability_idx);
    assert!(reliability_idx < untagged_idx);

    // no-unwrap should appear under reliability group only, not duplicated under another tag.
    // The rule has tags ["reliability", "rust"] but first-tag-wins means it goes under "reliability".
    let reliability_section_end = text[reliability_idx..]
        .find("\n\n(untagged)")
        .unwrap_or_else(|| text[reliability_idx..].len());
    let reliability_section = &text[reliability_idx..reliability_idx + reliability_section_end];
    assert!(
        reliability_section.contains("no-unwrap"),
        "no-unwrap should be in reliability section"
    );

    // No stray `rust` group header — "rust" is a language, not a tag, so it should never
    // appear as a group. This assertion guards against mishandling the "rust" tag value.
    assert!(
        !text.contains("rust\n----"),
        "no rust group header should exist"
    );
}

#[test]
fn rules_rejects_group_by_with_json_format() {
    let fx = DescribeFixture::new();
    Command::cargo_bin("lintropy")
        .unwrap()
        .current_dir(fx.path())
        .args(["rules", "--format", "json", "--group-by", "language"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "--group-by only applies to text format",
        ));
}
