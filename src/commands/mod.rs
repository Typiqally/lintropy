//! Per-subcommand handlers. Each returns `Result<u8, CliError>` where the
//! `u8` is the process exit code on success.

pub mod check;
pub mod explain;
pub mod hook;
pub mod init;
pub mod install_editor;
pub mod install_lsp_extension;
pub mod install_lsp_template;
pub mod install_textmate_bundle;
pub mod rules;
pub mod schema;
pub mod ts_parse;
pub mod validate;

use std::path::{Path, PathBuf};

use crate::core::Config;

use crate::exit::CliError;

/// Resolve a config either from `--config <path>` or by walking up from
/// the current directory.
pub(crate) fn load_config(explicit: Option<&Path>) -> Result<Config, CliError> {
    match explicit {
        Some(path) => Ok(Config::load_from_path(path)?),
        None => {
            let cwd =
                std::env::current_dir().map_err(|err| CliError::internal(format!("cwd: {err}")))?;
            Ok(Config::load_from_root(&cwd)?)
        }
    }
}

/// Print any non-fatal config warnings to stderr.
pub(crate) fn print_warnings(cfg: &Config) {
    for w in &cfg.warnings {
        let where_ = w
            .rule_id
            .as_ref()
            .map(|id| format!("rule `{id}` ({}): ", w.source_path.display()))
            .unwrap_or_else(|| format!("{}: ", w.source_path.display()));
        eprintln!("warning: {where_}{}", w.message);
    }
}

pub(crate) fn current_dir() -> Result<PathBuf, CliError> {
    std::env::current_dir().map_err(|err| CliError::internal(format!("cwd: {err}")))
}
