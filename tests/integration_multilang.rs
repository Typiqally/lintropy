#[cfg(any(
    feature = "lang-go",
    feature = "lang-python",
    feature = "lang-typescript"
))]
use assert_cmd::Command;

#[cfg(any(
    feature = "lang-go",
    feature = "lang-python",
    feature = "lang-typescript"
))]
fn fixture_root(lang: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/multilang")
        .join(lang)
}

#[cfg(feature = "lang-typescript")]
use tempfile::TempDir;

#[cfg(feature = "lang-typescript")]
fn write(dir: &std::path::Path, rel: &str, contents: &str) {
    let path = dir.join(rel);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(path, contents).unwrap();
}

#[cfg(feature = "lang-typescript")]
#[test]
fn tsx_jsx_rule_matches_only_in_tsx_files() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    write(root, "lintropy.yaml", "version: 1\n");
    write(
        root,
        ".lintropy/no-raw-div.rule.yaml",
        r#"severity: warning
message: "no raw <div>"
language: typescript
query: |
  (jsx_element
    (jsx_opening_element (identifier) @name)
    (#eq? @name "div")) @m
"#,
    );
    write(root, "src/app.tsx", "const x = <div></div>;\n");
    write(root, "src/lib.ts", "const x: number = 1;\n");

    let mut cmd = Command::cargo_bin("lintropy").unwrap();
    cmd.current_dir(root)
        .arg("check")
        .arg("--format")
        .arg("json");
    let output = cmd.output().unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("app.tsx"), "tsx match missing: {stdout}");
    assert!(
        !stdout.contains("lib.ts"),
        "false positive on lib.ts: {stdout}"
    );
}

#[cfg(feature = "lang-go")]
#[test]
fn go_fixture_flags_fmt_println() {
    let root = fixture_root("go");
    let mut cmd = assert_cmd::Command::cargo_bin("lintropy").unwrap();
    cmd.current_dir(&root)
        .arg("check")
        .arg("--format")
        .arg("json");
    let output = cmd.output().unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("main.go"), "expected go diag: {stdout}");
    assert!(stdout.contains("no-println"), "rule id missing: {stdout}");
}

#[cfg(feature = "lang-python")]
#[test]
fn python_fixture_flags_print_call() {
    let root = fixture_root("python");
    let mut cmd = assert_cmd::Command::cargo_bin("lintropy").unwrap();
    cmd.current_dir(&root)
        .arg("check")
        .arg("--format")
        .arg("json");
    let output = cmd.output().unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("app.py"), "expected python diag: {stdout}");
    assert!(stdout.contains("no-print"), "rule id missing: {stdout}");
}

#[cfg(feature = "lang-typescript")]
#[test]
fn typescript_fixture_flags_console_log() {
    let root = fixture_root("typescript");
    let mut cmd = assert_cmd::Command::cargo_bin("lintropy").unwrap();
    cmd.current_dir(&root)
        .arg("check")
        .arg("--format")
        .arg("json");
    let output = cmd.output().unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("app.ts"), "expected ts diag: {stdout}");
    assert!(
        stdout.contains("no-console-log"),
        "rule id missing: {stdout}"
    );
}
