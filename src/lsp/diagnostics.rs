//! Convert lintropy [`core::Diagnostic`]s into LSP [`lsp_types::Diagnostic`]s.

use tower_lsp::lsp_types::{
    Diagnostic as LspDiagnostic, DiagnosticSeverity, NumberOrString, Url,
};

use crate::core::{Diagnostic, Severity};

use super::position::byte_range_to_range;

/// Fixed LSP diagnostic `source` field — editors render this as the
/// diagnostic's producer (e.g. the "(lintropy)" suffix in VS Code).
pub const SOURCE: &str = "lintropy";

/// Map a single lintropy diagnostic to its LSP counterpart.
///
/// `src` is the buffer the diagnostic was produced against; we re-derive
/// the range from byte offsets so the positions are UTF-16 accurate.
pub fn to_lsp(diagnostic: &Diagnostic, src: &str, docs_url_fallback: Option<&Url>) -> LspDiagnostic {
    LspDiagnostic {
        range: byte_range_to_range(src, diagnostic.byte_start, diagnostic.byte_end),
        severity: Some(severity_to_lsp(diagnostic.severity)),
        code: Some(NumberOrString::String(diagnostic.rule_id.as_str().to_string())),
        code_description: diagnostic
            .docs_url
            .as_deref()
            .and_then(|url| Url::parse(url).ok())
            .or_else(|| docs_url_fallback.cloned())
            .map(|href| tower_lsp::lsp_types::CodeDescription { href }),
        source: Some(SOURCE.to_string()),
        message: diagnostic.message.clone(),
        related_information: None,
        tags: None,
        data: None,
    }
}

fn severity_to_lsp(severity: Severity) -> DiagnosticSeverity {
    match severity {
        Severity::Error => DiagnosticSeverity::ERROR,
        Severity::Warning => DiagnosticSeverity::WARNING,
        Severity::Info => DiagnosticSeverity::INFORMATION,
    }
}
