use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

use insta::{assert_json_snapshot, assert_snapshot};
use lintropy_core::{Diagnostic, FixHunk, RuleId, Severity, Summary};
use lintropy_output::{ColorChoice, JsonReporter, OutputFormat, OutputSink, Reporter, TextReporter};
use tempfile::TempDir;

#[test]
fn text_reporter_zero_diagnostics() {
    let mut out = Vec::new();
    let summary = Summary {
        errors: 0,
        warnings: 0,
        infos: 0,
        files_checked: 0,
        duration_ms: 0,
    };

    TextReporter::new(Box::new(&mut out), ColorChoice::Never)
        .report(&[], &summary)
        .unwrap();

    assert_snapshot!(String::from_utf8(out).unwrap(), @"Summary: 0 diagnostics across 0 files.
");
}

#[test]
fn text_reporter_warning_with_fix() {
    let fixture = FixtureDir::new();
    let file = fixture.write(
        "src/handlers/users.rs",
        "fn run() {\n    let user = client.unwrap().get(id).await?;\n}\n",
    );

    let diagnostic = diagnostic(&file, Severity::Warning, 2, 23, 2, 38)
        .with_message("avoid .unwrap() on `client`");
    let diagnostic = with_fix(diagnostic, "client.expect(\"TODO: handle error\")");

    let mut out = Vec::new();
    TextReporter::new(Box::new(&mut out), ColorChoice::Never)
        .report(&[diagnostic], &summary(0, 1, 0, 1))
        .unwrap();

    let rendered = normalize_temp_paths(String::from_utf8(out).unwrap(), fixture.root());
    assert_snapshot!(rendered, @r#"
warning[no-unwrap]: avoid .unwrap() on `client`
  --> [TEMP]/src/handlers/users.rs:2:23
   |
2 |     let user = client.unwrap().get(id).await?;
   |                       ^^^^^^^^^^^^^^^ help: replace with `client.expect("TODO: handle error")`
   |
   = rule defined in: .lintropy/no-unwrap.rule.yaml
   = see: lintropy explain no-unwrap

Summary: 1 warning across 1 file. 1 autofix available — re-run with --fix.
"#);
}

#[test]
fn text_reporter_mixed_severities_multiple_files() {
    let fixture = FixtureDir::new();
    let file_a = fixture.write("src/main.rs", "println!(\"debug\");\n");
    let file_b = fixture.write("src/lib.rs", "let _ = config.unwrap();\n");

    let diagnostics = vec![
        diagnostic(&file_a, Severity::Info, 1, 1, 1, 8).with_message("debug output left in place"),
        with_fix(
            diagnostic(&file_b, Severity::Warning, 1, 16, 1, 24)
                .with_message("avoid .unwrap() on `config`"),
            "config.expect(\"TODO\")",
        ),
        diagnostic(&file_b, Severity::Error, 1, 5, 1, 11).with_message("blocking rule violation"),
    ];

    let mut out = Vec::new();
    TextReporter::new(Box::new(&mut out), ColorChoice::Never)
        .report(&diagnostics, &summary(1, 1, 1, 2))
        .unwrap();

    let rendered = normalize_temp_paths(String::from_utf8(out).unwrap(), fixture.root());
    assert_snapshot!(rendered, @r#"
info[no-unwrap]: debug output left in place
  --> [TEMP]/src/main.rs:1:1
   |
1 | println!("debug");
   | ^^^^^^^
   |
   = rule defined in: .lintropy/no-unwrap.rule.yaml
   = see: lintropy explain no-unwrap

warning[no-unwrap]: avoid .unwrap() on `config`
  --> [TEMP]/src/lib.rs:1:16
   |
1 | let _ = config.unwrap();
   |                ^^^^^^^^ help: replace with `config.expect("TODO")`
   |
   = rule defined in: .lintropy/no-unwrap.rule.yaml
   = see: lintropy explain no-unwrap

error[no-unwrap]: blocking rule violation
  --> [TEMP]/src/lib.rs:1:5
   |
1 | let _ = config.unwrap();
   |     ^^^^^^
   |
   = rule defined in: .lintropy/no-unwrap.rule.yaml
   = see: lintropy explain no-unwrap

Summary: 1 error, 1 warning, 1 info across 2 files. 1 autofix available — re-run with --fix.
"#);
}

#[test]
fn text_reporter_color_on_and_off() {
    let fixture = FixtureDir::new();
    let file = fixture.write("src/main.rs", "let _ = thing.unwrap();\n");
    let diagnostic = with_docs_url(
        diagnostic(&file, Severity::Warning, 1, 15, 1, 23).with_message("avoid .unwrap()"),
        "https://example.com/no-unwrap",
    );

    let mut off = Vec::new();
    TextReporter::new(Box::new(&mut off), ColorChoice::Never)
        .report(std::slice::from_ref(&diagnostic), &summary(0, 1, 0, 1))
        .unwrap();

    let mut on = Vec::new();
    TextReporter::new(Box::new(&mut on), ColorChoice::Always)
        .report(&[diagnostic], &summary(0, 1, 0, 1))
        .unwrap();

    let off = normalize_temp_paths(String::from_utf8(off).unwrap(), fixture.root());
    let on = normalize_temp_paths(String::from_utf8(on).unwrap(), fixture.root());

    assert_snapshot!(&off, @r"
warning[no-unwrap]: avoid .unwrap()
  --> [TEMP]/src/main.rs:1:15
   |
1 | let _ = thing.unwrap();
   |               ^^^^^^^^
   |
   = rule defined in: .lintropy/no-unwrap.rule.yaml
   = see: lintropy explain no-unwrap
   = docs: https://example.com/no-unwrap

Summary: 1 warning across 1 file.
");
    assert_snapshot!(&on, @r"
[33mwarning[39m[no-unwrap]: avoid .unwrap()
  --> [TEMP]/src/main.rs:1:15
   |
1 | let _ = thing.unwrap();
   |               ^^^^^^^^
   |
   = rule defined in: .lintropy/no-unwrap.rule.yaml
   = see: lintropy explain no-unwrap
   = docs: https://example.com/no-unwrap

Summary: 1 warning across 1 file.
");
}

#[test]
fn json_reporter_emits_envelope() {
    let fixture = FixtureDir::new();
    let file = fixture.write("src/main.rs", "let _ = thing.unwrap();\n");
    let diagnostics = vec![with_docs_url(
        diagnostic(&file, Severity::Warning, 1, 15, 1, 23).with_message("avoid .unwrap()"),
        "https://example.com/no-unwrap",
    )];
    let summary = summary(0, 1, 0, 1);

    let mut out = Vec::new();
    JsonReporter::new(Box::new(&mut out))
        .report(&diagnostics, &summary)
        .unwrap();

    let rendered = normalize_temp_paths(String::from_utf8(out).unwrap(), fixture.root());
    let value: serde_json::Value = serde_json::from_str(&rendered).unwrap();
    assert_json_snapshot!(value, @r#"
{
  "diagnostics": [
    {
      "byte_end": 22,
      "byte_start": 14,
      "column": 15,
      "docs_url": "https://example.com/no-unwrap",
      "end_column": 23,
      "end_line": 1,
      "file": "[TEMP]/src/main.rs",
      "line": 1,
      "message": "avoid .unwrap()",
      "rule_id": "no-unwrap",
      "rule_source": ".lintropy/no-unwrap.rule.yaml",
      "severity": "warning"
    }
  ],
  "summary": {
    "duration_ms": 123,
    "errors": 0,
    "files_checked": 1,
    "infos": 0,
    "warnings": 1
  },
  "version": 1
}
"#);
}

#[test]
fn output_sink_writes_atomically() {
    let fixture = FixtureDir::new();
    let destination = fixture.root().join("report.txt");
    let mut sink = OutputSink::open(Some(&destination)).unwrap();

    {
        let mut writer = sink.writer();
        writer.write_all(b"hello world\n").unwrap();
    }

    assert!(!destination.exists());
    sink.commit().unwrap();
    assert_eq!(fs::read_to_string(destination).unwrap(), "hello world\n");
}

#[test]
fn output_format_color_policy_matches_spec() {
    assert_eq!(
        OutputFormat::Text.color_choice(false, false, true),
        ColorChoice::Always
    );
    assert_eq!(
        OutputFormat::Text.color_choice(true, false, true),
        ColorChoice::Never
    );
    assert_eq!(
        OutputFormat::Text.color_choice(false, true, true),
        ColorChoice::Never
    );
    assert_eq!(
        OutputFormat::Json.color_choice(false, false, true),
        ColorChoice::Never
    );
    assert_eq!(
        OutputFormat::Text.color_choice(false, false, false),
        ColorChoice::Never
    );
}

fn summary(errors: usize, warnings: usize, infos: usize, files_checked: usize) -> Summary {
    Summary {
        errors,
        warnings,
        infos,
        files_checked,
        duration_ms: 123,
    }
}

struct DiagnosticBuilder {
    diagnostic: Diagnostic,
}

impl DiagnosticBuilder {
    fn with_message(mut self, message: &str) -> Diagnostic {
        self.diagnostic.message = message.to_string();
        self.diagnostic
    }

}

fn with_fix(mut diagnostic: Diagnostic, replacement: &str) -> Diagnostic {
    diagnostic.fix = Some(FixHunk {
        replacement: replacement.to_string(),
        byte_start: diagnostic.byte_start,
        byte_end: diagnostic.byte_end,
    });
    diagnostic
}

fn with_docs_url(mut diagnostic: Diagnostic, docs_url: &str) -> Diagnostic {
    diagnostic.docs_url = Some(docs_url.to_string());
    diagnostic
}

fn diagnostic(
    file: &Path,
    severity: Severity,
    line: usize,
    column: usize,
    end_line: usize,
    end_column: usize,
) -> DiagnosticBuilder {
    DiagnosticBuilder {
        diagnostic: Diagnostic {
            rule_id: RuleId::new("no-unwrap"),
            severity,
            message: String::new(),
            file: file.to_path_buf(),
            line,
            column,
            end_line,
            end_column,
            byte_start: column.saturating_sub(1),
            byte_end: end_column.saturating_sub(1),
            rule_source: PathBuf::from(".lintropy/no-unwrap.rule.yaml"),
            docs_url: None,
            fix: None,
        },
    }
}

fn normalize_temp_paths(value: String, root: &Path) -> String {
    value.replace(&root.display().to_string(), "[TEMP]")
}

struct FixtureDir {
    temp_dir: TempDir,
}

impl FixtureDir {
    fn new() -> Self {
        let root = PathBuf::from("/tmp/lintropy-output-tests");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        Self {
            temp_dir: TempDir::new_in("/tmp").unwrap(),
        }
    }

    fn root(&self) -> &Path {
        self.temp_dir.path()
    }

    fn write(&self, relative: &str, contents: &str) -> PathBuf {
        let path = self.root().join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&path, contents).unwrap();
        path
    }
}
