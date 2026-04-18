//! Map lintropy [`FixHunk`]s into LSP [`CodeAction`]s (quickfixes).

use std::collections::HashMap;

use tower_lsp::lsp_types::{
    CodeAction, CodeActionKind, Diagnostic as LspDiagnostic, Range, TextEdit, Url, WorkspaceEdit,
};

use crate::core::Diagnostic;

use super::{diagnostics::to_lsp, position::byte_range_to_range};

/// Build a quickfix [`CodeAction`] for a diagnostic that carries a fix.
///
/// Returns `None` if the diagnostic has no `fix` payload or if the fix
/// range is empty / past-end of the buffer.
pub fn quickfix_for(uri: &Url, src: &str, diagnostic: &Diagnostic) -> Option<CodeAction> {
    let fix = diagnostic.fix.as_ref()?;

    let range = byte_range_to_range(src, fix.byte_start, fix.byte_end);
    let edit = TextEdit {
        range,
        new_text: fix.replacement.clone(),
    };

    let mut changes = HashMap::new();
    changes.insert(uri.clone(), vec![edit]);

    Some(CodeAction {
        title: format!("Autofix: {}", diagnostic.rule_id.as_str()),
        kind: Some(CodeActionKind::QUICKFIX),
        diagnostics: Some(vec![to_lsp(diagnostic, src, None)]),
        edit: Some(WorkspaceEdit {
            changes: Some(changes),
            document_changes: None,
            change_annotations: None,
        }),
        command: None,
        is_preferred: Some(true),
        disabled: None,
        data: None,
    })
}

/// True if `haystack` overlaps `needle` at all (inclusive on both ends).
pub fn ranges_intersect(haystack: Range, needle: Range) -> bool {
    !(haystack.end.line < needle.start.line
        || (haystack.end.line == needle.start.line
            && haystack.end.character < needle.start.character)
        || haystack.start.line > needle.end.line
        || (haystack.start.line == needle.end.line
            && haystack.start.character > needle.end.character))
}
