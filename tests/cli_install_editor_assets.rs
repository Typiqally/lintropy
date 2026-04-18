//! Integration tests for the `install-*` subcommand family.

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;

#[test]
fn install_textmate_bundle_extracts_expected_files() {
    let dir = tempfile::tempdir().unwrap();
    Command::cargo_bin("lintropy")
        .unwrap()
        .arg("install-textmate-bundle")
        .arg("--dir")
        .arg(dir.path())
        .assert()
        .code(0)
        .stdout(predicate::str::contains("extracted"));

    let bundle = dir.path().join("Lintropy Query.tmbundle");
    assert!(bundle.join("info.plist").is_file());
    assert!(bundle
        .join("Syntaxes/lintropy-query.tmLanguage.json")
        .is_file());
}

#[test]
fn install_textmate_bundle_refuses_existing_dir_without_force() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(dir.path().join("Lintropy Query.tmbundle")).unwrap();
    Command::cargo_bin("lintropy")
        .unwrap()
        .arg("install-textmate-bundle")
        .arg("--dir")
        .arg(dir.path())
        .assert()
        .code(2)
        .stderr(predicate::str::contains("refusing to overwrite"));
}

#[test]
fn install_lsp_template_jetbrains_extracts_template_files() {
    let dir = tempfile::tempdir().unwrap();
    Command::cargo_bin("lintropy")
        .unwrap()
        .arg("install-lsp-template")
        .arg("jetbrains")
        .arg("--dir")
        .arg(dir.path())
        .assert()
        .code(0)
        .stdout(predicate::str::contains("extracted"))
        .stdout(predicate::str::contains("Import from directory"));

    let template = dir.path().join("lsp4ij-template").join("template.json");
    assert!(template.is_file());
    let parsed: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&template).unwrap()).unwrap();
    assert_eq!(parsed["id"], "lintropy");
    assert_eq!(parsed["programArgs"]["default"], "lintropy lsp");
    let patterns = parsed["fileTypeMappings"][0]["fileType"]["patterns"]
        .as_array()
        .unwrap();
    assert!(patterns.iter().any(|p| p == "*.rs"));
}

#[test]
fn install_lsp_template_refuses_existing_dir_without_force() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(dir.path().join("lsp4ij-template")).unwrap();
    Command::cargo_bin("lintropy")
        .unwrap()
        .arg("install-lsp-template")
        .arg("jetbrains")
        .arg("--dir")
        .arg(dir.path())
        .assert()
        .code(2)
        .stderr(predicate::str::contains("refusing to overwrite"));
}

#[test]
fn install_lsp_extension_package_only_copies_vsix() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("source.vsix");
    fs::write(&source, b"PK\x03\x04fake vsix").unwrap();
    let out = dir.path().join("out.vsix");
    Command::cargo_bin("lintropy")
        .unwrap()
        .arg("install-lsp-extension")
        .arg("vscode")
        .arg("--vsix")
        .arg(&source)
        .arg("--package-only")
        .arg("-o")
        .arg(&out)
        .assert()
        .code(0)
        .stdout(predicate::str::contains("packaged"));
    assert_eq!(fs::read(&out).unwrap(), fs::read(&source).unwrap());
}

#[test]
fn install_lsp_extension_rejects_missing_vsix() {
    Command::cargo_bin("lintropy")
        .unwrap()
        .arg("install-lsp-extension")
        .arg("vscode")
        .arg("--vsix")
        .arg("/does/not/exist.vsix")
        .arg("--package-only")
        .arg("-o")
        .arg("/tmp/unused.vsix")
        .assert()
        .code(2)
        .stderr(predicate::str::contains("does not exist"));
}

#[test]
fn install_editor_jetbrains_unpacks_bundle_and_template() {
    let dir = tempfile::tempdir().unwrap();
    Command::cargo_bin("lintropy")
        .unwrap()
        .arg("install-editor")
        .arg("jetbrains")
        .arg("--dir")
        .arg(dir.path())
        .assert()
        .code(0)
        .stdout(predicate::str::contains("Lintropy Query.tmbundle"))
        .stdout(predicate::str::contains("lsp4ij-template"));

    assert!(dir
        .path()
        .join("Lintropy Query.tmbundle")
        .join("info.plist")
        .is_file());
    assert!(dir
        .path()
        .join("lsp4ij-template")
        .join("template.json")
        .is_file());
}
