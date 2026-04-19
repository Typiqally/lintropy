//! Integration tests for the `install-*` subcommand family.

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::path::Path;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

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
    let rust_patterns = parsed["fileTypeMappings"][0]["fileType"]["patterns"]
        .as_array()
        .unwrap();
    assert!(rust_patterns.iter().any(|p| p == "*.rs"));
    let yaml_patterns = parsed["fileTypeMappings"][1]["fileType"]["patterns"]
        .as_array()
        .unwrap();
    assert!(yaml_patterns.iter().any(|p| p == "lintropy.yaml"));
    assert!(yaml_patterns.iter().any(|p| p == "*.rule.yaml"));
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
fn install_lsp_extension_package_only_builds_vsix_from_source() {
    let dir = tempfile::tempdir().unwrap();
    let extension_dir = dir.path().join("extension");
    fs::create_dir_all(&extension_dir).unwrap();
    fs::write(extension_dir.join("package.json"), "{}").unwrap();
    let bin_dir = dir.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();
    let log = dir.path().join("pnpm.log");
    write_executable(
        &bin_dir.join("pnpm"),
        r#"#!/bin/sh
if [ -n "$LINTROPY_TEST_LOG" ]; then
  echo "$@" >> "$LINTROPY_TEST_LOG"
fi
if [ "$1" = "install" ]; then
  exit 0
fi
if [ "$1" = "run" ] && [ "$2" = "compile" ]; then
  exit 0
fi
if [ "$1" = "exec" ] && [ "$2" = "vsce" ] && [ "$3" = "package" ]; then
  while [ "$1" != "-o" ] && [ $# -gt 0 ]; do
    shift
  done
  shift
  printf 'PK\003\004fake vsix' > "$1"
  exit 0
fi
exit 1
"#,
    );
    let out = dir.path().join("out.vsix");
    Command::cargo_bin("lintropy")
        .unwrap()
        .arg("install-lsp-extension")
        .arg("vscode")
        .env("LINTROPY_VSCODE_EXTENSION_DIR", &extension_dir)
        .env("LINTROPY_TEST_LOG", &log)
        .env("PATH", &bin_dir)
        .arg("--package-only")
        .arg("-o")
        .arg(&out)
        .assert()
        .code(0)
        .stdout(predicate::str::contains("packaged"));
    assert_eq!(fs::read(&out).unwrap(), b"PK\x03\x04fake vsix");
    let invocations = fs::read_to_string(&log).unwrap();
    assert!(invocations.contains("install"));
    assert!(invocations.contains("run compile"));
    assert!(invocations.contains("exec vsce package --no-yarn --no-dependencies -o"));
}

#[test]
fn install_lsp_extension_rejects_missing_source_dir() {
    Command::cargo_bin("lintropy")
        .unwrap()
        .arg("install-lsp-extension")
        .arg("vscode")
        .env("LINTROPY_VSCODE_EXTENSION_DIR", "/does/not/exist")
        .arg("--package-only")
        .arg("-o")
        .arg("/tmp/unused.vsix")
        .assert()
        .code(2)
        .stderr(predicate::str::contains("source not found"));
}

#[test]
fn install_editor_jetbrains_unpacks_lsp4ij_template() {
    let dir = tempfile::tempdir().unwrap();
    Command::cargo_bin("lintropy")
        .unwrap()
        .arg("install-editor")
        .arg("jetbrains")
        .arg("--dir")
        .arg(dir.path())
        .assert()
        .code(0)
        .stdout(predicate::str::contains("lsp4ij-template"));

    assert!(dir
        .path()
        .join("lsp4ij-template")
        .join("template.json")
        .is_file());
}

#[test]
fn install_editor_vscode_builds_and_installs_extension() {
    let dir = tempfile::tempdir().unwrap();
    let extension_dir = dir.path().join("extension");
    fs::create_dir_all(&extension_dir).unwrap();
    fs::write(extension_dir.join("package.json"), "{}").unwrap();

    let bin_dir = dir.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();
    let pnpm_log = dir.path().join("pnpm.log");
    let code_log = dir.path().join("code.log");

    write_executable(
        &bin_dir.join("pnpm"),
        r#"#!/bin/sh
if [ -n "$LINTROPY_TEST_LOG" ]; then
  echo "$@" >> "$LINTROPY_TEST_LOG"
fi
if [ "$1" = "install" ]; then
  exit 0
fi
if [ "$1" = "run" ] && [ "$2" = "compile" ]; then
  exit 0
fi
if [ "$1" = "exec" ] && [ "$2" = "vsce" ] && [ "$3" = "package" ]; then
  while [ "$1" != "-o" ] && [ $# -gt 0 ]; do
    shift
  done
  shift
  printf 'PK\003\004fake vsix' > "$1"
  exit 0
fi
exit 1
"#,
    );
    write_executable(
        &bin_dir.join("code"),
        r#"#!/bin/sh
echo "$@" > "$LINTROPY_CODE_LOG"
exit 0
"#,
    );

    Command::cargo_bin("lintropy")
        .unwrap()
        .arg("install-editor")
        .arg("vscode")
        .arg("--profile")
        .arg("Default")
        .env("LINTROPY_VSCODE_EXTENSION_DIR", &extension_dir)
        .env("LINTROPY_TEST_LOG", &pnpm_log)
        .env("LINTROPY_CODE_LOG", &code_log)
        .env("PATH", &bin_dir)
        .assert()
        .code(0)
        .stdout(predicate::str::contains("installed lintropy into code"));

    let code_args = fs::read_to_string(&code_log).unwrap();
    assert!(code_args.contains("--profile Default"));
    assert!(code_args.contains("--install-extension"));
    assert!(code_args.contains("--force"));

    let invocations = fs::read_to_string(&pnpm_log).unwrap();
    assert!(invocations.contains("install"));
    assert!(invocations.contains("run compile"));
    assert!(invocations.contains("exec vsce package --no-yarn --no-dependencies -o"));
}

#[cfg(unix)]
fn write_executable(path: &Path, body: &str) {
    fs::write(path, body).unwrap();
    let mut perms = fs::metadata(path).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(path, perms).unwrap();
}

#[cfg(not(unix))]
fn write_executable(path: &Path, body: &str) {
    fs::write(path, body).unwrap();
}
