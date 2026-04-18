//! WP9 — `lintropy check --fix` against a copy of the example repo.

use std::fs;
use std::path::{Path, PathBuf};

use assert_cmd::Command;

fn lintropy() -> Command {
    Command::cargo_bin("lintropy").unwrap()
}

fn rust_demo() -> PathBuf {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    PathBuf::from(manifest).join("examples/rust-demo")
}

fn copy_tree(src: &Path, dst: &Path) {
    fs::create_dir_all(dst).unwrap();
    for entry in fs::read_dir(src).unwrap() {
        let entry = entry.unwrap();
        let from = entry.path();
        let to = dst.join(entry.file_name());
        if from.is_dir() {
            copy_tree(&from, &to);
        } else {
            fs::copy(&from, &to).unwrap();
        }
    }
}

#[test]
fn fix_removes_autofixable_no_unwrap_diagnostics() {
    let tempdir = tempfile::tempdir().unwrap();
    let working = tempdir.path().join("rust-demo");
    copy_tree(&rust_demo(), &working);

    let main_rs = fs::read_to_string(working.join("src/main.rs")).unwrap();
    assert!(
        main_rs.contains(".unwrap()"),
        "sanity: fixture main.rs should start with .unwrap() calls"
    );

    lintropy()
        .arg("check")
        .arg(&working)
        .arg("--fix")
        .assert()
        .code(1);

    let after = fs::read_to_string(working.join("src/main.rs")).unwrap();
    assert!(
        after.contains(".expect(\"TODO: handle error\")"),
        "no-unwrap autofix should have rewritten the plain .unwrap() call:\n{after}"
    );
    // The autofix excludes macro-invocation contexts, so `vec![...unwrap()]`
    // must survive unchanged.
    assert!(
        after.contains("vec![") && after.contains(".unwrap()"),
        "vec![x.unwrap()] must survive autofix:\n{after}"
    );

    // Re-running check should no longer report the now-fixed diagnostic.
    let second = lintropy().arg("check").arg(&working).output().unwrap();
    assert_eq!(second.status.code(), Some(1));
    let out = String::from_utf8(second.stdout).unwrap();
    let unwrap_hits = out.matches("no-unwrap").count();
    assert_eq!(
        unwrap_hits, 0,
        "post-fix run should not re-report no-unwrap:\n{out}"
    );
}
