//! `tower_lsp::LanguageServer` implementation — the glue between the
//! protocol and the lintropy engine.
//!
//! State model:
//! - `config` is `Arc<Mutex<Option<Config>>>` — loaded on `initialize`
//!   from the workspace root, refreshed on `didChangeWatchedFiles`.
//!   `None` means config load failed; we log and keep running so the
//!   client doesn't get a hard error on startup for a broken workspace.
//! - `documents` tracks the client's authoritative buffer state.
//! - `PreparedRules` is rebuilt per lint. Glob compile is cheap and the
//!   alternative (self-referential `PreparedRules` borrowing from `Config`)
//!   is not worth the complexity.

use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::Mutex;
use tower_lsp::jsonrpc::Result as JsonRpcResult;
use tower_lsp::lsp_types::{
    CodeActionOrCommand, CodeActionParams, CodeActionProviderCapability, CodeActionResponse,
    DidChangeConfigurationParams, DidChangeTextDocumentParams, DidChangeWatchedFilesParams,
    DidCloseTextDocumentParams, DidOpenTextDocumentParams, DidSaveTextDocumentParams,
    InitializeParams, InitializeResult, InitializedParams, MessageType, ServerCapabilities,
    ServerInfo, TextDocumentSyncCapability, TextDocumentSyncKind, Url,
};
use tower_lsp::{Client, LanguageServer};

use crate::core::{Config, PreparedRules};

use super::actions::{quickfix_for, ranges_intersect};
use super::diagnostics::to_lsp;
use super::document::DocumentStore;

/// Shared LSP backend.
pub struct Backend {
    client: Client,
    state: Arc<Mutex<State>>,
}

struct State {
    /// Workspace root, resolved from `initialize.workspaceFolders` (or
    /// the deprecated `rootUri`). Used as the config-load starting point.
    workspace_root: Option<PathBuf>,
    config: Option<Config>,
    documents: DocumentStore,
}

impl Backend {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            state: Arc::new(Mutex::new(State {
                workspace_root: None,
                config: None,
                documents: DocumentStore::new(),
            })),
        }
    }

    async fn log(&self, ty: MessageType, message: impl Into<String>) {
        self.client.log_message(ty, message.into()).await;
    }

    /// Re-load config from the workspace root (or cwd as fallback) and
    /// swap it into `state`. Logs failures; never errors out the server.
    async fn reload_config(&self) {
        let root = {
            let state = self.state.lock().await;
            state.workspace_root.clone()
        };
        let start = root
            .clone()
            .or_else(|| std::env::current_dir().ok())
            .unwrap_or_else(|| PathBuf::from("."));
        match Config::load_from_root(&start) {
            Ok(config) => {
                self.log(
                    MessageType::INFO,
                    format!(
                        "lintropy: loaded {} rules from {}",
                        config.rules.len(),
                        start.display()
                    ),
                )
                .await;
                let mut state = self.state.lock().await;
                state.config = Some(config);
            }
            Err(err) => {
                self.log(
                    MessageType::ERROR,
                    format!("lintropy: config load failed: {err}"),
                )
                .await;
                let mut state = self.state.lock().await;
                state.config = None;
            }
        }
    }

    /// Re-lint every open buffer and publish diagnostics. Used after
    /// config reload; for single-buffer updates call [`publish_for`].
    async fn republish_all(&self) {
        let snapshot: Vec<(Url, i32, String, PathBuf)> = {
            let state = self.state.lock().await;
            state
                .documents
                .iter()
                .map(|(uri, doc)| (uri.clone(), doc.version, doc.text.clone(), doc.path.clone()))
                .collect()
        };
        for (uri, version, text, path) in snapshot {
            self.publish_with(uri, version, &text, &path).await;
        }
    }

    /// Lint `uri`'s current buffer and publish diagnostics.
    async fn publish_for(&self, uri: &Url) {
        let doc = {
            let state = self.state.lock().await;
            state
                .documents
                .get(uri)
                .map(|d| (d.version, d.text.clone(), d.path.clone()))
        };
        if let Some((version, text, path)) = doc {
            self.publish_with(uri.clone(), version, &text, &path).await;
        }
    }

    async fn publish_with(&self, uri: Url, version: i32, text: &str, path: &std::path::Path) {
        let lsp_diags = match self.lint(text, path).await {
            Some(diags) => diags
                .iter()
                .map(|d| to_lsp(d, text, None))
                .collect::<Vec<_>>(),
            None => Vec::new(),
        };
        self.client
            .publish_diagnostics(uri, lsp_diags, Some(version))
            .await;
    }

    /// Core engine invocation: build `PreparedRules` from the current
    /// config and lint the buffer. Returns `None` when no config is
    /// loaded (client gets an empty diagnostic list — explicit signal
    /// rather than stale data).
    async fn lint(&self, text: &str, path: &std::path::Path) -> Option<Vec<crate::core::Diagnostic>> {
        let state = self.state.lock().await;
        let config = state.config.as_ref()?;
        let prepared = PreparedRules::prepare(config).ok()?;
        prepared.lint_buffer(path, text.as_bytes()).ok()
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, params: InitializeParams) -> JsonRpcResult<InitializeResult> {
        let workspace_root = params
            .workspace_folders
            .as_ref()
            .and_then(|folders| folders.first())
            .and_then(|f| f.uri.to_file_path().ok())
            .or_else(|| {
                #[allow(deprecated)]
                params
                    .root_uri
                    .as_ref()
                    .and_then(|uri| uri.to_file_path().ok())
            });

        {
            let mut state = self.state.lock().await;
            state.workspace_root = workspace_root;
        }

        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                code_action_provider: Some(CodeActionProviderCapability::Simple(true)),
                ..ServerCapabilities::default()
            },
            server_info: Some(ServerInfo {
                name: "lintropy".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.reload_config().await;
    }

    async fn shutdown(&self) -> JsonRpcResult<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let doc = params.text_document;
        {
            let mut state = self.state.lock().await;
            state.documents.set(doc.uri.clone(), doc.text, doc.version);
        }
        self.publish_for(&doc.uri).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        // We advertised `TextDocumentSyncKind::FULL`, so there is exactly
        // one content-change entry carrying the whole new buffer.
        if let Some(change) = params.content_changes.into_iter().next() {
            let mut state = self.state.lock().await;
            state
                .documents
                .set(uri.clone(), change.text, params.text_document.version);
        }
        self.publish_for(&uri).await;
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        // Nothing to do — we lint on didChange. Republish just in case
        // the client flushed a partial state between changes.
        self.publish_for(&params.text_document.uri).await;
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri;
        {
            let mut state = self.state.lock().await;
            state.documents.remove(&uri);
        }
        // Clear diagnostics so the editor doesn't keep them after close.
        self.client.publish_diagnostics(uri, Vec::new(), None).await;
    }

    async fn did_change_configuration(&self, _: DidChangeConfigurationParams) {
        self.reload_config().await;
        self.republish_all().await;
    }

    async fn did_change_watched_files(&self, _params: DidChangeWatchedFilesParams) {
        self.reload_config().await;
        self.republish_all().await;
    }

    async fn code_action(
        &self,
        params: CodeActionParams,
    ) -> JsonRpcResult<Option<CodeActionResponse>> {
        let uri = params.text_document.uri.clone();
        let (text, path) = {
            let state = self.state.lock().await;
            match state.documents.get(&uri) {
                Some(doc) => (doc.text.clone(), doc.path.clone()),
                None => return Ok(None),
            }
        };

        let diagnostics = match self.lint(&text, &path).await {
            Some(d) => d,
            None => return Ok(None),
        };

        let requested_range = params.range;
        let mut actions: Vec<CodeActionOrCommand> = Vec::new();
        for diag in &diagnostics {
            let Some(action) = quickfix_for(&uri, &text, diag) else {
                continue;
            };
            let diag_range = action
                .diagnostics
                .as_ref()
                .and_then(|d| d.first())
                .map(|d| d.range)
                .unwrap_or(requested_range);
            if !ranges_intersect(diag_range, requested_range) {
                continue;
            }
            actions.push(CodeActionOrCommand::CodeAction(action));
        }

        if actions.is_empty() {
            Ok(None)
        } else {
            Ok(Some(actions))
        }
    }
}
