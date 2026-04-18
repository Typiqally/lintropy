//! In-memory document store for the LSP server.
//!
//! The client pushes the authoritative buffer state (open, change, close);
//! we keep the latest text + version indexed by URI so the engine can lint
//! the live buffer instead of whatever is on disk.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use tower_lsp::lsp_types::{Range, Url};

use super::position::apply_change;

/// Latest known state of a single open editor buffer.
#[derive(Debug, Clone)]
pub struct Document {
    /// Filesystem path derived from the URI. `lint_buffer` needs it for
    /// language detection (via extension) and include/exclude glob matching.
    pub path: PathBuf,
    /// Buffer contents, UTF-8.
    pub text: String,
    /// Monotonic version from the client.
    pub version: i32,
}

/// Map from `textDocument.uri` to the latest [`Document`].
#[derive(Debug, Default)]
pub struct DocumentStore {
    docs: HashMap<Url, Document>,
}

impl DocumentStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Replace the full buffer for `uri` (`didOpen`).
    pub fn set(&mut self, uri: Url, text: String, version: i32) {
        let path = uri_to_path(&uri).unwrap_or_else(|| PathBuf::from(uri.path()));
        self.docs.insert(
            uri,
            Document {
                path,
                text,
                version,
            },
        );
    }

    /// Apply one incremental edit from a `didChange` notification.
    ///
    /// `range == None` is the full-sync fallback (some clients still send
    /// it even with incremental negotiated); `range == Some(..)` patches
    /// only that UTF-16 range.
    ///
    /// No-op if the URI isn't tracked (shouldn't happen — the client
    /// always opens before changing — but we'd rather drop the edit than
    /// synthesize a partial document from scratch).
    pub fn apply_edit(&mut self, uri: &Url, range: Option<Range>, new_text: &str, version: i32) {
        if let Some(doc) = self.docs.get_mut(uri) {
            apply_change(&mut doc.text, range, new_text);
            doc.version = version;
        }
    }

    pub fn get(&self, uri: &Url) -> Option<&Document> {
        self.docs.get(uri)
    }

    pub fn remove(&mut self, uri: &Url) {
        self.docs.remove(uri);
    }

    /// Iterate over every currently open buffer. Used when the config
    /// reloads and we need to re-lint all buffers.
    pub fn iter(&self) -> impl Iterator<Item = (&Url, &Document)> {
        self.docs.iter()
    }
}

fn uri_to_path(uri: &Url) -> Option<PathBuf> {
    if uri.scheme() != "file" {
        return None;
    }
    uri.to_file_path().ok().map(|p| p as PathBuf)
}

/// Turn a filesystem path back into a `file://` URI. Used when publishing
/// diagnostics for a path the server knows only by filesystem path
/// (e.g. workspace scan triggered by config reload).
#[allow(dead_code)]
pub fn path_to_uri(path: &Path) -> Option<Url> {
    Url::from_file_path(path).ok()
}
