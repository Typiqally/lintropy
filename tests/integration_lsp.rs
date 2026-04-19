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

use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::time::{Duration, Instant};

use serde_json::{json, Value};

fn rust_demo() -> PathBuf {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    PathBuf::from(manifest).join("examples/rust-demo")
}

struct LspProcess {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

impl LspProcess {
    fn spawn(cwd: &Path) -> Self {
        let bin = assert_cmd::cargo::cargo_bin("lintropy");
        let mut child = Command::new(bin)
            .arg("lsp")
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
