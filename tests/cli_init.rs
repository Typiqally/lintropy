//! Integration tests for `lintropy init`.

use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;

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
fn init_with_skill_no_agent_dirs_prints_snippet() {
    let dir = tempfile::tempdir().unwrap();
    Command::cargo_bin("lintropy")
        .unwrap()
        .current_dir(dir.path())
        .args(["init", "--with-skill"])
        .assert()
        .code(0)
        .stdout(predicate::str::contains("no `.claude/` or `.cursor/`"))
        .stdout(predicate::str::contains("PostToolUse"));
    assert!(!dir.path().join(".claude").exists());
    assert!(!dir.path().join(".cursor").exists());
}

#[test]
fn init_with_skill_claude_present_installs_skill_and_merges_settings() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join(".claude")).unwrap();

    Command::cargo_bin("lintropy")
        .unwrap()
        .current_dir(dir.path())
        .args(["init", "--with-skill"])
        .assert()
        .code(0)
        .stdout(predicate::str::contains(".claude/skills/lintropy/SKILL.md"))
        .stdout(predicate::str::contains(".claude/settings.json"));

    let skill = dir.path().join(".claude/skills/lintropy/SKILL.md");
    assert!(skill.is_file());
    let first = std::fs::read_to_string(&skill).unwrap();
    assert!(first.starts_with("# version:"), "missing version header");

    let settings_path = dir.path().join(".claude/settings.json");
    let parsed: Value =
        serde_json::from_str(&std::fs::read_to_string(&settings_path).unwrap()).unwrap();
    let entry = &parsed["hooks"]["PostToolUse"][0];
    assert_eq!(entry["matcher"], "Write|Edit|NotebookEdit");
    assert_eq!(entry["hooks"][0]["type"], "command");
    assert_eq!(
        entry["hooks"][0]["command"],
        "lintropy hook --agent claude-code"
    );
}

#[test]
fn init_with_skill_rerun_is_idempotent() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join(".claude")).unwrap();

    Command::cargo_bin("lintropy")
        .unwrap()
        .current_dir(dir.path())
        .args(["init", "--with-skill"])
        .assert()
        .code(0);

    // Delete scaffolded files so init re-scaffolds only, then force --with-skill again.
    std::fs::remove_file(dir.path().join("lintropy.yaml")).unwrap();
    std::fs::remove_file(dir.path().join(".lintropy/no-unwrap.rule.yaml")).unwrap();

    let settings_before =
        std::fs::read_to_string(dir.path().join(".claude/settings.json")).unwrap();
    let skill_before =
        std::fs::read_to_string(dir.path().join(".claude/skills/lintropy/SKILL.md")).unwrap();

    Command::cargo_bin("lintropy")
        .unwrap()
        .current_dir(dir.path())
        .args(["init", "--with-skill"])
        .assert()
        .code(0)
        .stdout(predicate::str::contains("unchanged"));

    let settings_after = std::fs::read_to_string(dir.path().join(".claude/settings.json")).unwrap();
    let skill_after =
        std::fs::read_to_string(dir.path().join(".claude/skills/lintropy/SKILL.md")).unwrap();
    assert_eq!(settings_before, settings_after);
    assert_eq!(skill_before, skill_after);
}

#[test]
fn init_with_skill_preserves_unrelated_claude_hooks() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join(".claude")).unwrap();
    let settings = serde_json::json!({
        "hooks": {
            "PreToolUse": [
                { "matcher": "Bash", "hooks": [{ "type": "command", "command": "my-pre-hook" }] }
            ],
            "PostToolUse": [
                { "matcher": "Read", "hooks": [{ "type": "command", "command": "some-other-hook" }] }
            ]
        },
        "other_user_setting": "keep-me"
    });
    std::fs::write(
        dir.path().join(".claude/settings.json"),
        serde_json::to_string_pretty(&settings).unwrap(),
    )
    .unwrap();

    Command::cargo_bin("lintropy")
        .unwrap()
        .current_dir(dir.path())
        .args(["init", "--with-skill"])
        .assert()
        .code(0);

    let parsed: Value = serde_json::from_str(
        &std::fs::read_to_string(dir.path().join(".claude/settings.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(parsed["other_user_setting"], "keep-me");
    assert_eq!(parsed["hooks"]["PreToolUse"][0]["matcher"], "Bash");
    let post = parsed["hooks"]["PostToolUse"].as_array().unwrap();
    assert_eq!(post.len(), 2);
    assert_eq!(post[0]["matcher"], "Read");
    assert_eq!(post[1]["matcher"], "Write|Edit|NotebookEdit");
    assert_eq!(
        post[1]["hooks"][0]["command"],
        "lintropy hook --agent claude-code"
    );
}

// Note: WP7's `merge_claude_settings` replaces the full PostToolUse entry
// matching the canonical matcher; it does not splice inner-hook siblings.
// A finer-grained merge (preserving sibling inner hooks) is TODO(post-mvp).

#[test]
fn init_with_skill_dir_override_writes_only_to_that_dir() {
    let dir = tempfile::tempdir().unwrap();
    let custom = dir.path().join("custom-skill");
    Command::cargo_bin("lintropy")
        .unwrap()
        .current_dir(dir.path())
        .args([
            "init",
            "--with-skill",
            "--skill-dir",
            custom.to_str().unwrap(),
        ])
        .assert()
        .code(0);
    assert!(custom.join("SKILL.md").is_file());
    assert!(!dir.path().join(".claude").exists());
}

#[test]
fn init_with_skill_cursor_present_installs_skill_no_settings_merge() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join(".cursor")).unwrap();

    Command::cargo_bin("lintropy")
        .unwrap()
        .current_dir(dir.path())
        .args(["init", "--with-skill"])
        .assert()
        .code(0);

    assert!(dir
        .path()
        .join(".cursor/skills/lintropy/SKILL.md")
        .is_file());
    assert!(!dir.path().join(".claude/settings.json").exists());
}

#[test]
fn init_with_skill_upgrades_older_version() {
    let dir = tempfile::tempdir().unwrap();
    let skill_path = dir.path().join(".claude/skills/lintropy/SKILL.md");
    std::fs::create_dir_all(skill_path.parent().unwrap()).unwrap();
    std::fs::write(&skill_path, "# version: 0.0.1\nstale content\n").unwrap();

    Command::cargo_bin("lintropy")
        .unwrap()
        .current_dir(dir.path())
        .args(["init", "--with-skill"])
        .assert()
        .code(0)
        .stdout(predicate::str::contains("upgraded"));

    let after = std::fs::read_to_string(&skill_path).unwrap();
    assert!(!after.contains("stale content"));
    assert!(after.starts_with("# version:"));
}
