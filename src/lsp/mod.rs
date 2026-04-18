//! Language Server Protocol front-end for lintropy.
//!
//! Entry point invoked by `lintropy lsp`. Spins up a `tower-lsp` server
//! over stdio, wires the shared engine into `textDocument/*` handlers,
//! and publishes diagnostics as open buffers change. Editors (VS Code,
//! Cursor, JetBrains via LSP4IJ, Neovim, Helix) spawn `lintropy lsp` as
//! a subprocess and talk to it via LSP.

mod actions;
mod diagnostics;
mod document;
mod position;
mod server;

use tower_lsp::{LspService, Server};

use crate::exit::{CliError, EXIT_OK};

/// Run the LSP server to completion over stdio.
///
/// Loads config from the current working directory on initialize;
/// after that all state lives in memory (open docs + prepared rules)
/// and is refreshed by `workspace/didChangeWatchedFiles` notifications
/// from the client.
pub fn run() -> Result<u8, CliError> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|err| CliError::internal(format!("failed to start tokio runtime: {err}")))?;

    runtime.block_on(async {
        let stdin = tokio::io::stdin();
        let stdout = tokio::io::stdout();
        let (service, socket) = LspService::new(server::Backend::new);
        Server::new(stdin, stdout, socket).serve(service).await;
    });

    Ok(EXIT_OK)
}
