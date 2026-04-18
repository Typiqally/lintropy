//! `lintropy install-query-extension` — materialise the embedded
//! `.vsix` and (optionally) invoke `code` / `cursor` to install it.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::cli::{InstallQueryExtensionArgs, QueryEditor};
use crate::editor_assets::{VSIX_BYTES, VSIX_FILE_NAME};
use crate::exit::{CliError, EXIT_OK};

pub fn run(args: InstallQueryExtensionArgs) -> Result<u8, CliError> {
    if args.package_only {
        let target = args.output.unwrap_or_else(|| PathBuf::from(VSIX_FILE_NAME));
        write_vsix(&target)?;
        println!("packaged {}", target.display());
        return Ok(EXIT_OK);
    }

    let editor = args.editor.ok_or_else(|| {
        CliError::user("editor is required (vscode or cursor) unless --package-only is set")
    })?;
    let cli_bin = match editor {
        QueryEditor::Vscode => "code",
        QueryEditor::Cursor => "cursor",
    };
    if which(cli_bin).is_none() {
        return Err(CliError::user(format!(
            "`{cli_bin}` not found in PATH — install the editor's shell command first"
        )));
    }

    let tmp = tempfile::Builder::new()
        .prefix("lintropy-query-")
        .suffix(".vsix")
        .tempfile()
        .map_err(|err| CliError::internal(format!("tempfile: {err}")))?;
    write_vsix(tmp.path())?;

    let mut cmd = Command::new(cli_bin);
    if let Some(profile) = args.profile.as_deref() {
        cmd.arg("--profile").arg(profile);
    }
    cmd.arg("--install-extension")
        .arg(tmp.path())
        .arg("--force");

    let status = cmd
        .status()
        .map_err(|err| CliError::internal(format!("spawn {cli_bin}: {err}")))?;
    if !status.success() {
        return Err(CliError::user(format!(
            "`{cli_bin} --install-extension` exited with {status}"
        )));
    }
    println!("installed lintropy-query-syntax into {cli_bin}");
    Ok(EXIT_OK)
}

fn write_vsix(path: &Path) -> Result<(), CliError> {
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent)?;
    }
    let mut f = fs::File::create(path)?;
    f.write_all(VSIX_BYTES)?;
    f.sync_all()?;
    Ok(())
}

fn which(bin: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|dir| dir.join(bin))
        .find(|p| p.is_file())
}
