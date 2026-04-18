//! Integration tests for `lintropy init`.

use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn init_scaffolds_root_and_example_rule() {
    let dir = tempfile::tempdir().unwrap();
    Command::cargo_bin("lintropy")
        .unwrap()
        .current_dir(dir.path())
        .arg("init")
        .assert()
        .code(0)
        .stdout(predicate::str::contains("lintropy.yaml"));

    assert!(dir.path().join("lintropy.yaml").is_file());
    assert!(dir.path().join(".lintropy/no-unwrap.rule.yaml").is_file());
}

#[test]
fn init_refuses_to_overwrite_existing_file() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("lintropy.yaml"), "existing").unwrap();
    Command::cargo_bin("lintropy")
        .unwrap()
        .current_dir(dir.path())
        .arg("init")
        .assert()
        .code(2)
        .stderr(predicate::str::contains("refusing to overwrite"));
}

#[test]
fn init_with_skill_merges_claude_settings() {
    let dir = tempfile::tempdir().unwrap();
    Command::cargo_bin("lintropy")
        .unwrap()
        .current_dir(dir.path())
        .args(["init", "--with-skill"])
        .assert()
        .code(0)
        .stdout(predicate::str::contains(".claude/settings.json"));

    let settings = std::fs::read_to_string(dir.path().join(".claude/settings.json")).unwrap();
    assert!(settings.contains("lintropy hook --agent claude-code"));
}
