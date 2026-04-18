//! Integration tests for `install-query-extension --package-only`
//! and `install-textmate-bundle`.

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;

#[test]
fn install_query_extension_package_only_writes_vsix() {
    let dir = tempfile::tempdir().unwrap();
    let vsix = dir.path().join("out.vsix");
    Command::cargo_bin("lintropy")
        .unwrap()
        .arg("install-query-extension")
        .arg("--package-only")
        .arg("-o")
        .arg(&vsix)
        .assert()
        .code(0)
        .stdout(predicate::str::contains("packaged"));

    let bytes = fs::read(&vsix).unwrap();
    // `.vsix` is a zip — local file header magic is "PK\x03\x04".
    assert_eq!(&bytes[..4], b"PK\x03\x04");
}

#[test]
fn install_query_extension_rejects_missing_editor() {
    Command::cargo_bin("lintropy")
        .unwrap()
        .arg("install-query-extension")
        .assert()
        .code(2)
        .stderr(predicate::str::contains("editor is required"));
}

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
