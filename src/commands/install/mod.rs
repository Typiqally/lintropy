//! `lintropy install <target>` — one-command install for every editor
//! or agent lintropy knows how to integrate with.
//!
//! - **vscode / cursor**: builds the local extension into a `.vsix` and
//!   hands it to the editor's `--install-extension` flag. With
//!   `--package-only -o <path>` the `.vsix` is written without running
//!   the editor CLI.
//! - **jetbrains**: unpacks the LSP4IJ custom template into `--dir`
//!   (defaults to cwd). Still needs one IDE-side import step.
//! - **claude-code**: generates the plugin manifest fresh (version,
//!   feature-gated extension map, absolute `command` path), bundles the
//!   lintropy skill at `<plugin>/skills/lintropy/SKILL.md`, and prints
//!   the `claude --plugin-dir <dir>` invocation the user should run to
//!   pick it up.

pub(crate) mod claude_code;
pub(crate) mod lsp_extension;
pub(crate) mod lsp_template;

use crate::cli::{InstallArgs, InstallTarget};
use crate::exit::CliError;

pub fn run(args: InstallArgs) -> Result<u8, CliError> {
    match args.target {
        InstallTarget::Vscode => install_vsix(args, lsp_extension::VsixEditor::Vscode),
        InstallTarget::Cursor => install_vsix(args, lsp_extension::VsixEditor::Cursor),
        InstallTarget::Jetbrains => lsp_template::install_jetbrains(args.dir, args.force),
        InstallTarget::ClaudeCode => claude_code::run(claude_code::ClaudeCodeInstall {
            dir: args.dir,
            force: args.force,
        }),
    }
}

fn install_vsix(args: InstallArgs, editor: lsp_extension::VsixEditor) -> Result<u8, CliError> {
    lsp_extension::run(lsp_extension::VsixBuild {
        editor: Some(editor),
        profile: args.profile,
        package_only: args.package_only,
        output: args.output,
    })
}
