//! Integration tests for `lintropy check`.

mod common;

use assert_cmd::Command;
use common::Fixture;
use predicates::prelude::*;

fn bin(fixture: &Fixture) -> Command {
    let mut cmd = Command::cargo_bin("lintropy").unwrap();
    cmd.current_dir(fixture.path());
    cmd
}

#[test]
fn check_text_reports_diagnostics_and_exits_zero_with_warning_fail_on_error() {
    let fx = Fixture::new();
    bin(&fx)
        .args(["check", "sample.rs"])
        .assert()
        .code(0)
        .stdout(predicate::str::contains("no-unwrap"))
        .stdout(predicate::str::contains("avoid .unwrap()"));
}

#[test]
fn check_json_format_emits_diagnostic_envelope() {
    let fx = Fixture::new();
    let output = bin(&fx)
        .args(["check", "--format", "json", "sample.rs"])
        .assert()
        .code(0)
        .get_output()
        .stdout
        .clone();
    let parsed: serde_json::Value = serde_json::from_slice(&output).expect("valid JSON");
    assert_eq!(parsed["version"], 1);
    let diags = parsed["diagnostics"].as_array().expect("diagnostics array");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0]["rule_id"], "no-unwrap");
}

#[test]
fn check_fix_rewrites_source_and_drops_diagnostic() {
    let fx = Fixture::new();
    bin(&fx)
        .args(["check", "--fix", "sample.rs"])
        .assert()
        .code(0)
        .stdout(predicate::str::contains("Applied 1 fix"));
    let after = std::fs::read_to_string(fx.sample()).unwrap();
    assert!(
        after.contains(".expect(\"TODO: handle error\")"),
        "expected fix to be spliced in, got:\n{after}"
    );
    // Re-run: the diagnostic is gone, so --fix should report 0.
    bin(&fx)
        .args(["check", "--fix", "sample.rs"])
        .assert()
        .code(0)
        .stdout(predicate::str::contains("Applied 0 fixes"));
}

#[test]
fn check_fail_on_warning_exits_one_when_warning_present() {
    let fx = Fixture::new();
    // Rewrite config to fail on warning.
    std::fs::write(
        fx.path().join("lintropy.yaml"),
        "version: 1\nsettings:\n  fail_on: warning\n  default_severity: warning\n",
    )
    .unwrap();
    bin(&fx).args(["check", "sample.rs"]).assert().code(1);
}
