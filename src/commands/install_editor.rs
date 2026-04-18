//! `lintropy install-editor <editor>` — one-command install for every
//! lintropy editor asset that makes sense for that editor.
//!
//! - **vscode / cursor**: delegates to `install-lsp-extension`. The
//!   extension bundles both the LSP client and the `query` grammar
//!   injection, so one `.vsix` covers everything VS Code / Cursor need.
//! - **jetbrains**: unpacks *both* the TextMate bundle (query DSL
//!   highlighting inside YAML files) and the LSP4IJ custom template
//!   (live diagnostics over LSP). The two serve orthogonal purposes and
//!   can't be merged, but one command installs both.

use crate::cli::{
    EditorFamily, InstallEditorArgs, InstallLspExtensionArgs, InstallLspTemplateArgs,
    InstallTextmateBundleArgs, LspExtensionEditor, LspTemplateEditor,
};
use crate::commands::{install_lsp_extension, install_lsp_template, install_textmate_bundle};
use crate::exit::{CliError, EXIT_OK};

pub fn run(args: InstallEditorArgs) -> Result<u8, CliError> {
    match args.editor {
        EditorFamily::Vscode => install_lsp_extension::run(InstallLspExtensionArgs {
            editor: Some(LspExtensionEditor::Vscode),
            profile: args.profile,
            version: args.version,
            vsix: args.vsix,
            package_only: false,
            output: None,
        }),
        EditorFamily::Cursor => install_lsp_extension::run(InstallLspExtensionArgs {
            editor: Some(LspExtensionEditor::Cursor),
            profile: args.profile,
            version: args.version,
            vsix: args.vsix,
            package_only: false,
            output: None,
        }),
        EditorFamily::Jetbrains => install_jetbrains(args),
    }
}

fn install_jetbrains(args: InstallEditorArgs) -> Result<u8, CliError> {
    install_textmate_bundle::run(InstallTextmateBundleArgs {
        dir: args.dir.clone(),
        force: args.force,
    })?;
    println!();
    install_lsp_template::run(InstallLspTemplateArgs {
        editor: LspTemplateEditor::Jetbrains,
        dir: args.dir,
        force: args.force,
    })?;
    Ok(EXIT_OK)
}
