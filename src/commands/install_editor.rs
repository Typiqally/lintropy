//! `lintropy install-editor <editor>` — one-command install for every
//! lintropy editor asset that makes sense for that editor.
//!
//! - **vscode / cursor**: builds the local extension source via
//!   `install-lsp-extension`. The single extension carries the LSP
//!   client; query-DSL syntax colouring is delivered server-side via
//!   `textDocument/semanticTokens` instead of a TextMate grammar, so one
//!   `.vsix` covers everything.
//! - **jetbrains**: unpacks the LSP4IJ custom template. Query-DSL
//!   colouring and diagnostics both flow through the LSP channel once
//!   LSP4IJ is pointed at `lintropy lsp`.

use crate::cli::{
    EditorFamily, InstallEditorArgs, InstallLspExtensionArgs, InstallLspTemplateArgs,
    LspExtensionEditor, LspTemplateEditor,
};
use crate::commands::{install_lsp_extension, install_lsp_template};
use crate::exit::{CliError, EXIT_OK};

pub fn run(args: InstallEditorArgs) -> Result<u8, CliError> {
    match args.editor {
        EditorFamily::Vscode => install_lsp_extension::run(InstallLspExtensionArgs {
            editor: Some(LspExtensionEditor::Vscode),
            profile: args.profile,
            package_only: false,
            output: None,
        }),
        EditorFamily::Cursor => install_lsp_extension::run(InstallLspExtensionArgs {
            editor: Some(LspExtensionEditor::Cursor),
            profile: args.profile,
            package_only: false,
            output: None,
        }),
        EditorFamily::Jetbrains => {
            install_lsp_template::run(InstallLspTemplateArgs {
                editor: LspTemplateEditor::Jetbrains,
                dir: args.dir,
                force: args.force,
            })?;
            Ok(EXIT_OK)
        }
    }
}
