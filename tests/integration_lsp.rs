//! End-to-end test for `lintropy lsp`.
//!
//! Spawns the binary, drives a handshake + `didOpen` against the
//! `examples/rust-demo/` fixture, and asserts that
//! `textDocument/publishDiagnostics` reports the same rule ids as
//! `lintropy check` on the same file.
//!
//! The transport is hand-rolled LSP framing (`Content-Length` header +
//! UTF-8 body). Bringing in a fully-featured LSP client just for this
//! test would dwarf the test itself.

use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::time::{Duration, Instant};

use serde_json::{json, Value};
use tempfile::TempDir;

fn rust_demo() -> PathBuf {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    PathBuf::from(manifest).join("examples/rust-demo")
}

fn write(path: &Path, contents: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create test parent dir");
    }
    fs::write(path, contents).expect("write test file");
}

fn file_uri(path: &Path) -> String {
    format!("file://{}", path.display())
}

struct LspProcess {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

impl LspProcess {
    fn spawn(cwd: &Path) -> Self {
        Self::spawn_with_args(cwd, &[])
    }

    fn spawn_with_args(cwd: &Path, extra_args: &[&str]) -> Self {
        let bin = assert_cmd::cargo::cargo_bin("lintropy");
        let mut command = Command::new(bin);
        command.arg("lsp");
        command.args(extra_args);
        let mut child = command
            .current_dir(cwd)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn lintropy lsp");
        let stdin = child.stdin.take().unwrap();
        let stdout = BufReader::new(child.stdout.take().unwrap());
        Self {
            child,
            stdin,
            stdout,
        }
    }

    fn send(&mut self, value: &Value) {
        let body = serde_json::to_string(value).unwrap();
        let frame = format!("Content-Length: {}\r\n\r\n{}", body.len(), body);
        self.stdin.write_all(frame.as_bytes()).unwrap();
        self.stdin.flush().unwrap();
    }

    /// Read one framed LSP message. Returns the JSON value.
    fn recv(&mut self) -> Value {
        let mut content_length: Option<usize> = None;
        loop {
            let mut line = String::new();
            self.stdout.read_line(&mut line).expect("read header line");
            if line == "\r\n" || line.is_empty() {
                break;
            }
            if let Some(rest) = line.strip_prefix("Content-Length: ") {
                content_length = Some(rest.trim().parse().unwrap());
            }
        }
        let len = content_length.expect("Content-Length header");
        let mut buf = vec![0u8; len];
        self.stdout.read_exact(&mut buf).expect("read body");
        serde_json::from_slice(&buf).unwrap()
    }

    /// Read messages until `predicate` returns a match, or the deadline
    /// elapses. Returns the matched message. Intermediate messages
    /// (log notifications, etc.) are discarded.
    fn recv_until<F: Fn(&Value) -> bool>(&mut self, deadline: Duration, predicate: F) -> Value {
        let start = Instant::now();
        loop {
            if start.elapsed() > deadline {
                panic!("timed out waiting for matching LSP message");
            }
            let msg = self.recv();
            if predicate(&msg) {
                return msg;
            }
        }
    }
}

impl Drop for LspProcess {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[test]
fn accepts_stdio_flag_for_editor_clients() {
    let demo = rust_demo();
    let mut lsp = LspProcess::spawn_with_args(&demo, &["--stdio"]);

    let root_uri = format!("file://{}", demo.display());
    lsp.send(&json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "processId": null,
            "rootUri": root_uri,
            "capabilities": {},
            "workspaceFolders": [{
                "uri": root_uri,
                "name": "rust-demo"
            }]
        }
    }));

    let init_response = lsp.recv_until(Duration::from_secs(5), |msg| {
        msg.get("id") == Some(&json!(1))
    });
    assert!(
        init_response.get("result").is_some(),
        "initialize failed with --stdio: {init_response}"
    );
}

#[test]
fn publishes_diagnostics_for_open_rust_file() {
    let demo = rust_demo();
    let mut lsp = LspProcess::spawn(&demo);

    let root_uri = format!("file://{}", demo.display());
    lsp.send(&json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "processId": null,
            "rootUri": root_uri,
            "capabilities": {},
            "workspaceFolders": [{
                "uri": root_uri,
                "name": "rust-demo"
            }]
        }
    }));

    let init_response = lsp.recv_until(Duration::from_secs(5), |msg| {
        msg.get("id") == Some(&json!(1))
    });
    assert!(
        init_response.get("result").is_some(),
        "initialize failed: {init_response}"
    );

    lsp.send(&json!({
        "jsonrpc": "2.0",
        "method": "initialized",
        "params": {}
    }));

    let main_path = demo.join("src/main.rs");
    let main_uri = format!("file://{}", main_path.display());
    let main_text = std::fs::read_to_string(&main_path).unwrap();

    lsp.send(&json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didOpen",
        "params": {
            "textDocument": {
                "uri": main_uri,
                "languageId": "rust",
                "version": 1,
                "text": main_text
            }
        }
    }));

    let publish = lsp.recv_until(Duration::from_secs(10), |msg| {
        msg.get("method") == Some(&json!("textDocument/publishDiagnostics"))
            && msg.pointer("/params/uri") == Some(&json!(main_uri))
    });

    let diags = publish
        .pointer("/params/diagnostics")
        .and_then(|v| v.as_array())
        .expect("diagnostics array");

    let codes: Vec<&str> = diags
        .iter()
        .filter_map(|d| d.pointer("/code").and_then(|c| c.as_str()))
        .collect();
    assert!(
        codes.contains(&"no-unwrap"),
        "expected no-unwrap, got {codes:?}"
    );
    assert!(
        codes.contains(&"no-println"),
        "expected no-println, got {codes:?}"
    );

    for diag in diags {
        assert_eq!(diag.pointer("/source"), Some(&json!("lintropy")));
    }
}

#[test]
fn incremental_edit_updates_diagnostics() {
    let demo = rust_demo();
    let mut lsp = LspProcess::spawn(&demo);

    let root_uri = format!("file://{}", demo.display());
    lsp.send(&json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "processId": null,
            "rootUri": root_uri,
            "capabilities": {},
            "workspaceFolders": [{"uri": root_uri, "name": "rust-demo"}]
        }
    }));
    let _ = lsp.recv_until(Duration::from_secs(5), |m| m.get("id") == Some(&json!(1)));
    lsp.send(&json!({"jsonrpc": "2.0", "method": "initialized", "params": {}}));

    let main_path = demo.join("src/main.rs");
    let main_uri = format!("file://{}", main_path.display());
    let main_text = std::fs::read_to_string(&main_path).unwrap();

    lsp.send(&json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didOpen",
        "params": {
            "textDocument": {
                "uri": main_uri,
                "languageId": "rust",
                "version": 1,
                "text": main_text
            }
        }
    }));
    let _ = lsp.recv_until(Duration::from_secs(10), |msg| {
        msg.get("method") == Some(&json!("textDocument/publishDiagnostics"))
            && msg.pointer("/params/uri") == Some(&json!(main_uri))
    });

    // main.rs line 10 (0-based line 9): `    println!("lintropy rust-demo");`.
    // Replace the identifier `println` with `print` (chars 4..11) so the
    // `no-println` match disappears. Sent as a range edit — the server must
    // splice the patch into the stored buffer (not do a full replace).
    lsp.send(&json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didChange",
        "params": {
            "textDocument": {"uri": main_uri, "version": 2},
            "contentChanges": [{
                "range": {
                    "start": {"line": 9, "character": 4},
                    "end": {"line": 9, "character": 11}
                },
                "text": "print"
            }]
        }
    }));

    let publish = lsp.recv_until(Duration::from_secs(10), |msg| {
        msg.get("method") == Some(&json!("textDocument/publishDiagnostics"))
            && msg.pointer("/params/uri") == Some(&json!(main_uri))
            && msg.pointer("/params/version") == Some(&json!(2))
    });

    let codes: Vec<&str> = publish
        .pointer("/params/diagnostics")
        .and_then(|v| v.as_array())
        .unwrap()
        .iter()
        .filter_map(|d| d.pointer("/code").and_then(|c| c.as_str()))
        .collect();
    assert!(
        !codes.contains(&"no-println"),
        "no-println should be gone after edit, got {codes:?}"
    );
    assert!(
        codes.contains(&"no-unwrap"),
        "no-unwrap should still fire, got {codes:?}"
    );
}

#[test]
fn code_action_returns_autofix_workspace_edit() {
    let demo = rust_demo();
    let mut lsp = LspProcess::spawn(&demo);

    let root_uri = format!("file://{}", demo.display());
    lsp.send(&json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "processId": null,
            "rootUri": root_uri,
            "capabilities": {},
            "workspaceFolders": [{"uri": root_uri, "name": "rust-demo"}]
        }
    }));
    let _ = lsp.recv_until(Duration::from_secs(5), |m| m.get("id") == Some(&json!(1)));
    lsp.send(&json!({"jsonrpc": "2.0", "method": "initialized", "params": {}}));

    let main_path = demo.join("src/main.rs");
    let main_uri = format!("file://{}", main_path.display());
    let main_text = std::fs::read_to_string(&main_path).unwrap();

    lsp.send(&json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didOpen",
        "params": {
            "textDocument": {
                "uri": main_uri,
                "languageId": "rust",
                "version": 1,
                "text": main_text
            }
        }
    }));

    let publish = lsp.recv_until(Duration::from_secs(10), |msg| {
        msg.get("method") == Some(&json!("textDocument/publishDiagnostics"))
            && msg.pointer("/params/uri") == Some(&json!(main_uri))
    });
    let unwrap_diag = publish
        .pointer("/params/diagnostics")
        .and_then(|d| d.as_array())
        .unwrap()
        .iter()
        .find(|d| d.pointer("/code") == Some(&json!("no-unwrap")))
        .expect("no-unwrap diagnostic")
        .clone();
    let range = unwrap_diag.get("range").cloned().unwrap();

    lsp.send(&json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "textDocument/codeAction",
        "params": {
            "textDocument": {"uri": main_uri},
            "range": range,
            "context": {"diagnostics": [unwrap_diag]}
        }
    }));

    let resp = lsp.recv_until(Duration::from_secs(5), |m| m.get("id") == Some(&json!(2)));
    let actions = resp
        .pointer("/result")
        .and_then(|r| r.as_array())
        .expect("actions array");
    assert!(
        actions
            .iter()
            .any(|a| a.pointer("/kind") == Some(&json!("quickfix"))
                && a.pointer("/edit/changes").is_some()),
        "expected quickfix with WorkspaceEdit, got {actions:?}"
    );
}

#[test]
fn semantic_tokens_for_query_block_in_yaml_rule_file() {
    let demo = rust_demo();
    let mut lsp = LspProcess::spawn(&demo);

    let root_uri = format!("file://{}", demo.display());
    lsp.send(&json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "processId": null,
            "rootUri": root_uri,
            "capabilities": {},
            "workspaceFolders": [{"uri": root_uri, "name": "rust-demo"}]
        }
    }));
    let init = lsp.recv_until(Duration::from_secs(5), |m| m.get("id") == Some(&json!(1)));
    // Capability advertised.
    assert!(
        init.pointer("/result/capabilities/semanticTokensProvider")
            .is_some(),
        "server must advertise semanticTokensProvider: {init}"
    );
    lsp.send(&json!({"jsonrpc": "2.0", "method": "initialized", "params": {}}));

    let rule_path = demo.join(".lintropy/no-unwrap.rule.yaml");
    let rule_uri = format!("file://{}", rule_path.display());
    let rule_text = std::fs::read_to_string(&rule_path).unwrap();

    lsp.send(&json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didOpen",
        "params": {
            "textDocument": {
                "uri": rule_uri,
                "languageId": "yaml",
                "version": 1,
                "text": rule_text
            }
        }
    }));

    // YAML files should not produce lint diagnostics (no rules target
    // the rule files themselves) — drain the empty publish so it
    // doesn't confuse the next recv.
    let _ = lsp.recv_until(Duration::from_secs(5), |msg| {
        msg.get("method") == Some(&json!("textDocument/publishDiagnostics"))
            && msg.pointer("/params/uri") == Some(&json!(rule_uri))
    });

    lsp.send(&json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "textDocument/semanticTokens/full",
        "params": {
            "textDocument": {"uri": rule_uri}
        }
    }));

    let resp = lsp.recv_until(Duration::from_secs(5), |m| m.get("id") == Some(&json!(2)));
    let data = resp
        .pointer("/result/data")
        .and_then(|v| v.as_array())
        .expect("tokens array");
    assert!(
        !data.is_empty(),
        "expected semantic tokens for the embedded query DSL, got none"
    );
    // Each token is a group of 5 u32s.
    assert_eq!(
        data.len() % 5,
        0,
        "token array length must be a multiple of 5"
    );

    // Some token must have type index matching FUNCTION (1) — the
    // `#eq?` predicate in the no-unwrap query — or VARIABLE (0) for
    // `@recv` / `@method` / `@match` captures.
    let token_types: Vec<u64> = data
        .chunks(5)
        .map(|chunk| chunk[3].as_u64().unwrap())
        .collect();
    assert!(
        token_types.contains(&0),
        "expected at least one VARIABLE token (@capture): {token_types:?}"
    );
}

#[test]
fn rule_file_with_broken_query_publishes_inline_diagnostic() {
    let demo = rust_demo();
    let mut lsp = LspProcess::spawn(&demo);

    let root_uri = format!("file://{}", demo.display());
    lsp.send(&json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "processId": null,
            "rootUri": root_uri,
            "capabilities": {},
            "workspaceFolders": [{"uri": root_uri, "name": "rust-demo"}]
        }
    }));
    let _ = lsp.recv_until(Duration::from_secs(5), |m| m.get("id") == Some(&json!(1)));
    lsp.send(&json!({"jsonrpc": "2.0", "method": "initialized", "params": {}}));

    let fake_path = demo.join(".lintropy/_scratch-broken.rule.yaml");
    let fake_uri = format!("file://{}", fake_path.display());
    let broken = "language: rust\nseverity: warning\nmessage: \"hi\"\nquery: |\n  (call_expression\n    function: (identifier\n";
    lsp.send(&json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didOpen",
        "params": {
            "textDocument": {
                "uri": fake_uri,
                "languageId": "yaml",
                "version": 1,
                "text": broken
            }
        }
    }));

    let publish = lsp.recv_until(Duration::from_secs(5), |msg| {
        msg.get("method") == Some(&json!("textDocument/publishDiagnostics"))
            && msg.pointer("/params/uri") == Some(&json!(fake_uri))
    });

    let diags = publish
        .pointer("/params/diagnostics")
        .and_then(|v| v.as_array())
        .expect("diagnostics array");
    assert!(
        !diags.is_empty(),
        "expected at least one query-compile diagnostic, got {diags:?}"
    );
    assert_eq!(
        diags[0].pointer("/code"),
        Some(&json!("query-compile")),
        "{diags:?}"
    );
    let line = diags[0]
        .pointer("/range/start/line")
        .and_then(|v| v.as_u64())
        .unwrap();
    assert!(
        line >= 4,
        "diagnostic should land inside the query block: {diags:?}"
    );
}

#[test]
fn watched_nested_lintropy_yaml_replaces_parent_rule_context() {
    let dir = TempDir::new().expect("create temp dir");
    write(&dir.path().join("lintropy.yaml"), "version: 1\n");
    write(
        &dir.path().join(".lintropy/root-no-unwrap.rule.yaml"),
        r#"severity: warning
include: ["**/*.rs"]
message: "avoid unwrap from root context"
language: rust
query: |
  (call_expression
    function: (field_expression
      value: (_) @recv
      field: (field_identifier) @method)
    arguments: (arguments)
    (#eq? @method "unwrap")
    (#not-has-ancestor? @method "macro_invocation")) @match
"#,
    );

    let nested_root = dir.path().join("packages/demo");
    let main_path = nested_root.join("src/main.rs");
    write(
        &main_path,
        r#"fn main() {
    let value = Some("demo");
    let _ = value.unwrap();
    println!("nested");
}
"#,
    );

    let mut lsp = LspProcess::spawn(dir.path());
    let root_uri = file_uri(dir.path());
    lsp.send(&json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "processId": null,
            "rootUri": root_uri,
            "capabilities": {},
            "workspaceFolders": [{"uri": root_uri, "name": "root"}]
        }
    }));
    let _ = lsp.recv_until(Duration::from_secs(5), |m| m.get("id") == Some(&json!(1)));
    lsp.send(&json!({"jsonrpc": "2.0", "method": "initialized", "params": {}}));

    let main_uri = file_uri(&main_path);
    let main_text = fs::read_to_string(&main_path).expect("read nested main.rs");
    lsp.send(&json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didOpen",
        "params": {
            "textDocument": {
                "uri": main_uri,
                "languageId": "rust",
                "version": 1,
                "text": main_text
            }
        }
    }));

    let initial_publish = lsp.recv_until(Duration::from_secs(10), |msg| {
        msg.get("method") == Some(&json!("textDocument/publishDiagnostics"))
            && msg.pointer("/params/uri") == Some(&json!(main_uri))
    });
    let initial_codes: Vec<&str> = initial_publish
        .pointer("/params/diagnostics")
        .and_then(|v| v.as_array())
        .expect("diagnostics array")
        .iter()
        .filter_map(|d| d.pointer("/code").and_then(|c| c.as_str()))
        .collect();
    assert!(
        initial_codes.contains(&"root-no-unwrap"),
        "expected parent context before nested lintropy.yaml exists, got {initial_codes:?}"
    );

    write(&nested_root.join("lintropy.yaml"), "version: 1\n");
    let nested_rule = nested_root.join(".lintropy/nested-no-println.rule.yaml");
    write(
        &nested_rule,
        r#"severity: warning
include: ["src/**/*.rs"]
message: "avoid println from nested context"
language: rust
query: |
  (macro_invocation
    macro: (identifier) @name
    (#eq? @name "println")) @match
"#,
    );

    lsp.send(&json!({
        "jsonrpc": "2.0",
        "method": "workspace/didChangeWatchedFiles",
        "params": {
            "changes": [
                {"uri": file_uri(&nested_root.join("lintropy.yaml")), "type": 1},
                {"uri": file_uri(&nested_rule), "type": 1}
            ]
        }
    }));

    let republish = lsp.recv_until(Duration::from_secs(10), |msg| {
        msg.get("method") == Some(&json!("textDocument/publishDiagnostics"))
            && msg.pointer("/params/uri") == Some(&json!(main_uri))
    });
    let codes: Vec<&str> = republish
        .pointer("/params/diagnostics")
        .and_then(|v| v.as_array())
        .expect("diagnostics array")
        .iter()
        .filter_map(|d| d.pointer("/code").and_then(|c| c.as_str()))
        .collect();
    assert!(
        codes.contains(&"nested-no-println"),
        "expected nested context after nested lintropy.yaml appears, got {codes:?}"
    );
    assert!(
        !codes.contains(&"root-no-unwrap"),
        "nested context should replace parent rules for child files, got {codes:?}"
    );
}

#[test]
fn watched_lintropy_rule_file_merges_into_existing_context() {
    let dir = TempDir::new().expect("create temp dir");
    write(&dir.path().join("lintropy.yaml"), "version: 1\n");
    let main_path = dir.path().join("src/main.rs");
    write(
        &main_path,
        r#"fn main() {
    println!("hello");
}
"#,
    );

    let mut lsp = LspProcess::spawn(dir.path());
    let root_uri = file_uri(dir.path());
    lsp.send(&json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "processId": null,
            "rootUri": root_uri,
            "capabilities": {},
            "workspaceFolders": [{"uri": root_uri, "name": "root"}]
        }
    }));
    let _ = lsp.recv_until(Duration::from_secs(5), |m| m.get("id") == Some(&json!(1)));
    lsp.send(&json!({"jsonrpc": "2.0", "method": "initialized", "params": {}}));

    let main_uri = file_uri(&main_path);
    let main_text = fs::read_to_string(&main_path).expect("read root main.rs");
    lsp.send(&json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didOpen",
        "params": {
            "textDocument": {
                "uri": main_uri,
                "languageId": "rust",
                "version": 1,
                "text": main_text
            }
        }
    }));

    let initial_publish = lsp.recv_until(Duration::from_secs(10), |msg| {
        msg.get("method") == Some(&json!("textDocument/publishDiagnostics"))
            && msg.pointer("/params/uri") == Some(&json!(main_uri))
    });
    let initial_diags = initial_publish
        .pointer("/params/diagnostics")
        .and_then(|v| v.as_array())
        .expect("diagnostics array");
    assert!(
        initial_diags.is_empty(),
        "expected no diagnostics before adding rules, got {initial_diags:?}"
    );

    let rule_path = dir.path().join(".lintropy/no-println.rule.yaml");
    write(
        &rule_path,
        r#"severity: warning
include: ["src/**/*.rs"]
message: "avoid println after live rule merge"
language: rust
query: |
  (macro_invocation
    macro: (identifier) @name
    (#eq? @name "println")) @match
"#,
    );

    lsp.send(&json!({
        "jsonrpc": "2.0",
        "method": "workspace/didChangeWatchedFiles",
        "params": {
            "changes": [
                {"uri": file_uri(&rule_path), "type": 1}
            ]
        }
    }));

    let republish = lsp.recv_until(Duration::from_secs(10), |msg| {
        msg.get("method") == Some(&json!("textDocument/publishDiagnostics"))
            && msg.pointer("/params/uri") == Some(&json!(main_uri))
    });
    let codes: Vec<&str> = republish
        .pointer("/params/diagnostics")
        .and_then(|v| v.as_array())
        .expect("diagnostics array")
        .iter()
        .filter_map(|d| d.pointer("/code").and_then(|c| c.as_str()))
        .collect();
    assert!(
        codes.contains(&"no-println"),
        "expected live merged rule to republish diagnostics, got {codes:?}"
    );
}
