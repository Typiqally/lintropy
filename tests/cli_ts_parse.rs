//! Integration tests for `lintropy ts-parse`.

use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn ts_parse_emits_rust_sexp() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("hello.rs");
    std::fs::write(&file, "fn main() {}\n").unwrap();
    Command::cargo_bin("lintropy")
        .unwrap()
        .args(["ts-parse", file.to_str().unwrap()])
        .assert()
        .code(0)
        .stdout(predicate::str::contains("source_file"))
        .stdout(predicate::str::contains("function_item"));
}

#[test]
fn ts_parse_respects_lang_override() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("no_ext");
    std::fs::write(&file, "fn main() {}\n").unwrap();
    Command::cargo_bin("lintropy")
        .unwrap()
        .args(["ts-parse", file.to_str().unwrap(), "--lang", "rust"])
        .assert()
        .code(0)
        .stdout(predicate::str::contains("source_file"));
}

#[test]
fn ts_parse_unknown_extension_lists_available_langs() {
    let tmp = tempfile::tempdir().unwrap();
    let file = tmp.path().join("foo.unknown");
    std::fs::write(&file, "hello").unwrap();
    let mut cmd = assert_cmd::Command::cargo_bin("lintropy").unwrap();
    cmd.arg("ts-parse").arg(&file);
    let assert = cmd.assert().failure();
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr).into_owned();
    assert!(
        stderr.contains("rust"),
        "error should list rust among available langs: {stderr}"
    );
}

#[test]
fn ts_parse_unknown_language_lists_available_langs() {
    let tmp = tempfile::tempdir().unwrap();
    let file = tmp.path().join("foo.txt");
    std::fs::write(&file, "hello").unwrap();
    let mut cmd = assert_cmd::Command::cargo_bin("lintropy").unwrap();
    cmd.arg("ts-parse")
        .arg(&file)
        .arg("--lang")
        .arg("brainfuck");
    let assert = cmd.assert().failure();
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr).into_owned();
    assert!(stderr.contains("brainfuck"), "echo unknown name: {stderr}");
    assert!(stderr.contains("rust"), "list rust: {stderr}");
}

#[cfg(feature = "lang-go")]
#[test]
fn ts_parse_auto_detects_go_from_extension() {
    let tmp = tempfile::tempdir().unwrap();
    let file = tmp.path().join("t.go");
    std::fs::write(&file, "package main\n").unwrap();
    let mut cmd = assert_cmd::Command::cargo_bin("lintropy").unwrap();
    cmd.arg("ts-parse")
        .arg(&file)
        .assert()
        .success()
        .stdout(predicates::str::contains("source_file"));
}

#[cfg(feature = "lang-python")]
#[test]
fn ts_parse_auto_detects_python_from_extension() {
    let tmp = tempfile::tempdir().unwrap();
    let file = tmp.path().join("t.py");
    std::fs::write(&file, "x = 1\n").unwrap();
    let mut cmd = assert_cmd::Command::cargo_bin("lintropy").unwrap();
    cmd.arg("ts-parse")
        .arg(&file)
        .assert()
        .success()
        .stdout(predicates::str::contains("module"));
}

#[cfg(feature = "lang-typescript")]
#[test]
fn ts_parse_auto_detects_typescript_from_ts_extension() {
    let tmp = tempfile::tempdir().unwrap();
    let file = tmp.path().join("t.ts");
    std::fs::write(&file, "const x: number = 1;\n").unwrap();
    let mut cmd = assert_cmd::Command::cargo_bin("lintropy").unwrap();
    cmd.arg("ts-parse")
        .arg(&file)
        .assert()
        .success()
        .stdout(predicates::str::contains("program"));
}

#[cfg(feature = "lang-typescript")]
#[test]
fn ts_parse_auto_detects_typescript_from_tsx_extension() {
    let tmp = tempfile::tempdir().unwrap();
    let file = tmp.path().join("t.tsx");
    std::fs::write(&file, "const x = <div/>;\n").unwrap();
    let mut cmd = assert_cmd::Command::cargo_bin("lintropy").unwrap();
    cmd.arg("ts-parse")
        .arg(&file)
        .assert()
        .success()
        .stdout(predicates::str::contains("program"));
}
