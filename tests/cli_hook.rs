//! Integration tests for `lintropy hook`.

mod common;

use std::fs;

use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::json;

#[test]
fn hook_exits_zero_for_clean_file() {
    let fixture = common::Fixture::new();
    let clean = fixture.path().join("clean.rs");
    fs::write(&clean, "fn main() {}\n").unwrap();

    Command::cargo_bin("lintropy")
        .unwrap()
        .current_dir(fixture.path())
        .arg("hook")
        .write_stdin(json!({ "tool_input": { "file_path": clean } }).to_string())
        .assert()
        .code(0)
        .stderr(predicate::str::is_empty());
}

#[test]
fn hook_returns_two_and_writes_compact_diagnostics_to_stderr() {
    let fixture = common::Fixture::new();

    Command::cargo_bin("lintropy")
        .unwrap()
        .current_dir(fixture.path())
        .args(["hook", "--fail-on", "warning"])
        .write_stdin(json!({ "tool_input": { "file_path": fixture.sample() } }).to_string())
        .assert()
        .code(2)
        .stderr(predicate::str::contains("no-unwrap"))
        .stderr(predicate::str::contains("help: replace with"));
}

#[test]
fn hook_json_format_writes_envelope_to_stderr() {
    let fixture = common::Fixture::new();

    Command::cargo_bin("lintropy")
        .unwrap()
        .current_dir(fixture.path())
        .args(["hook", "--format", "json", "--fail-on", "warning"])
        .write_stdin(json!({ "file_path": fixture.sample() }).to_string())
        .assert()
        .code(2)
        .stderr(predicate::str::contains("\"version\": 1"))
        .stderr(predicate::str::contains("\"diagnostics\""));
}

#[test]
fn hook_uses_payload_key_precedence() {
    let fixture = common::Fixture::new();
    let clean = fixture.path().join("clean.rs");
    fs::write(&clean, "fn main() {}\n").unwrap();

    Command::cargo_bin("lintropy")
        .unwrap()
        .current_dir(fixture.path())
        .arg("hook")
        .write_stdin(
            json!({
                "tool_input": { "path": clean },
                "file_path": fixture.sample()
            })
            .to_string(),
        )
        .assert()
        .code(0)
        .stderr(predicate::str::is_empty());
}

#[test]
fn hook_is_silent_for_malformed_json_without_verbose() {
    let fixture = common::Fixture::new();

    Command::cargo_bin("lintropy")
        .unwrap()
        .current_dir(fixture.path())
        .arg("hook")
        .write_stdin("{ not json")
        .assert()
        .code(0)
        .stderr(predicate::str::is_empty());
}

#[test]
fn hook_skips_gitignored_files() {
    let fixture = common::Fixture::new();
    let ignored = fixture.path().join("ignored.rs");
    fs::write(&ignored, common::SAMPLE_SOURCE).unwrap();
    fs::write(fixture.path().join(".gitignore"), "ignored.rs\n").unwrap();

    Command::cargo_bin("lintropy")
        .unwrap()
        .current_dir(fixture.path())
        .arg("hook")
        .write_stdin(json!({ "path": ignored }).to_string())
        .assert()
        .code(0)
        .stderr(predicate::str::is_empty());
}

#[test]
fn hook_skips_files_outside_rule_scope() {
    let fixture = common::Fixture::new();
    fs::write(
        fixture.path().join(".lintropy/no-unwrap.rule.yaml"),
        common::NO_UNWRAP_RULE.replace("**/*.rs", "src/**/*.rs"),
    )
    .unwrap();

    Command::cargo_bin("lintropy")
        .unwrap()
        .current_dir(fixture.path())
        .arg("hook")
        .write_stdin(json!({ "filename": fixture.sample() }).to_string())
        .assert()
        .code(0)
        .stderr(predicate::str::is_empty());
}
