//! `lintropy install-lsp-extension vscode|cursor` — build the VS Code /
//! Cursor extension from the local checkout, package it into a `.vsix`,
//! and hand that artifact to the editor's `--install-extension` flag.
//!
//! This keeps the installed extension aligned with the current checkout
//! instead of relying on a prebuilt release artifact.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

use crate::cli::{InstallLspExtensionArgs, LspExtensionEditor};
use crate::exit::{CliError, EXIT_OK};

pub fn run(args: InstallLspExtensionArgs) -> Result<u8, CliError> {
    let extension_dir = extension_source_dir()?;
    ensure_tool("pnpm")?;

    let (vsix_path, _owned_tmpdir) = if args.package_only {
        let target = args
            .output
            .unwrap_or_else(|| PathBuf::from("lintropy.vsix"));
        if let Some(parent) = target.parent().filter(|p| !p.as_os_str().is_empty()) {
            fs::create_dir_all(parent)?;
        }
        (target, None)
    } else {
        let tmpdir =
            tempfile::tempdir().map_err(|err| CliError::internal(format!("tempdir: {err}")))?;
        (tmpdir.path().join("lintropy.vsix"), Some(tmpdir))
    };

    build_vsix(&extension_dir, &vsix_path)?;

    if args.package_only {
        println!("packaged {}", vsix_path.display());
        return Ok(EXIT_OK);
    }

    let editor = args
        .editor
        .ok_or_else(|| CliError::user("editor is required (vscode or cursor)"))?;
    let cli_bin = match editor {
        LspExtensionEditor::Vscode => "code",
        LspExtensionEditor::Cursor => "cursor",
    };
    ensure_tool(cli_bin)?;

    let mut cmd = Command::new(cli_bin);
    if let Some(profile) = args.profile.as_deref() {
        cmd.arg("--profile").arg(profile);
    }
    cmd.arg("--install-extension")
        .arg(&vsix_path)
        .arg("--force");

    let status = cmd
        .status()
        .map_err(|err| CliError::internal(format!("spawn {cli_bin}: {err}")))?;
    if !status.success() {
        return Err(CliError::user(format!(
            "`{cli_bin} --install-extension` exited with {status}"
        )));
    }
    println!("installed lintropy into {cli_bin}");
    Ok(EXIT_OK)
}

fn extension_source_dir() -> Result<PathBuf, CliError> {
    let dir = std::env::var_os("LINTROPY_VSCODE_EXTENSION_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("editors")
                .join("vscode")
                .join("lintropy")
        });
    if !dir.join("package.json").is_file() {
        return Err(CliError::user(format!(
            "VS Code extension source not found at {}",
            dir.display()
        )));
    }
    Ok(dir)
}

fn build_vsix(extension_dir: &PathBuf, output: &PathBuf) -> Result<(), CliError> {
    if output.is_file() {
        fs::remove_file(output)?;
    }

    run_in_dir("pnpm", ["install"], extension_dir)?;
    run_in_dir("pnpm", ["run", "compile"], extension_dir)?;

    let output_arg = output.to_str().ok_or_else(|| {
        CliError::user(format!("non-utf8 path not supported: {}", output.display()))
    })?;
    run_in_dir(
        "pnpm",
        [
            "exec",
            "vsce",
            "package",
            "--no-yarn",
            "--no-dependencies",
            "-o",
            output_arg,
        ],
        extension_dir,
    )?;

    if !output.is_file() {
        return Err(CliError::internal(format!(
            "extension build did not produce {}",
            output.display()
        )));
    }
    Ok(())
}

fn run_in_dir<I, S>(bin: &str, args: I, dir: &PathBuf) -> Result<(), CliError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<std::ffi::OsStr>,
{
    let collected: Vec<std::ffi::OsString> = args
        .into_iter()
        .map(|arg| arg.as_ref().to_os_string())
        .collect();
    let status = Command::new(bin)
        .args(&collected)
        .current_dir(dir)
        .status()
        .map_err(|err| CliError::internal(format!("spawn {bin}: {err}")))?;
    if !status.success() {
        let rendered = collected
            .iter()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join(" ");
        return Err(CliError::user(format!(
            "`{bin} {rendered}` exited with {status}"
        )));
    }
    Ok(())
}

fn ensure_tool(bin: &str) -> Result<(), CliError> {
    if which(bin).is_none() {
        return Err(CliError::user(format!(
            "`{bin}` not found in PATH — install it first"
        )));
    }
    Ok(())
}

fn which(bin: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|dir| dir.join(bin))
        .find(|p| p.is_file())
}
