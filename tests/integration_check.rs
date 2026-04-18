//! WP9 — End-to-end `lintropy check` against the canonical example repo.
//!
//! Asserts the stable diagnostic set from `examples/rust-demo/README.md`
//! via the exit-code envelope and a handful of content predicates. A full
//! `insta` snapshot is deliberately avoided: rustc-style reporter output
//! is touchy to column arithmetic and pinning it to a snapshot turns
//! every legitimate phrasing tweak into a test break. Content predicates
//! track the same signal (rules + paths) without the churn.

use assert_cmd::Command;
use predicates::prelude::*;

fn lintropy() -> Command {
    Command::cargo_bin("lintropy").expect("lintropy binary built by workspace")
}

fn rust_demo() -> std::path::PathBuf {
    let manifest =
        std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR set by cargo test");
    std::path::PathBuf::from(manifest).join("examples/rust-demo")
}

#[test]
fn check_rust_demo_reports_expected_rules() {
    let demo = rust_demo();
    let out = lintropy()
        .current_dir(&demo)
        .arg("check")
        .arg(".")
        .assert()
        .code(1)
        .get_output()
        .stdout
        .clone();
    let stdout = String::from_utf8(out).expect("utf-8 stdout");

    // Every advertised rule must fire.
    for rule in [
        "domain-no-infra",
        "no-dbg",
        "todo-needs-ticket",
        "old-config-removed-2026Q2",
        "use-tracing-not-log",
        "safety-comment-required",
        "metric-naming",
        "no-stray-ignore",
        "test-name-prefix",
        "no-unwrap",
        "no-println",
        "user-use-builder",
        "no-todo",
    ] {
        assert!(
            stdout.contains(rule),
            "expected rule `{rule}` to fire in stdout:\n{stdout}"
        );
    }
    // Every advertised file must be referenced (paths may print as
    // `./src/...` depending on the walker).
    for fragment in [
        "domain_violation.rs",
        "dbg_usage.rs",
        "todo_ticket.rs",
        "old_config.rs",
        "log_usage.rs",
        "missing_comment.rs",
        "metrics.rs",
        "stray_ignore.rs",
        "tokio_naming.rs",
        "main.rs",
        "user.rs",
        "smoke.rs",
    ] {
        assert!(
            stdout.contains(fragment),
            "expected `{fragment}` in stdout:\n{stdout}"
        );
    }
    // `no-unwrap` must NOT fire inside `vec![...unwrap()]` (macro-invocation
    // suppression via the `#not-has-ancestor?` predicate).
    let unwrap_fires = stdout.matches("warning[no-unwrap]:").count();
    assert_eq!(
        unwrap_fires, 1,
        "expected exactly one no-unwrap fire (macro-context suppression):\n{stdout}"
    );
}

#[test]
fn check_rust_demo_json_envelope_is_valid() {
    let demo = rust_demo();
    lintropy()
        .current_dir(&demo)
        .arg("check")
        .arg(".")
        .args(["--format", "json"])
        .assert()
        .code(1)
        .stdout(predicate::function(|raw: &str| {
            let v: serde_json::Value = match serde_json::from_str(raw) {
                Ok(v) => v,
                Err(_) => return false,
            };
            v["version"] == 1
                && v["diagnostics"]
                    .as_array()
                    .is_some_and(|items| items.len() == 13)
                && v["summary"]["errors"] == 6
                && v["summary"]["warnings"] == 7
        }));
}
