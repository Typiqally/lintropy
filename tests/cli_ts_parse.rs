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
