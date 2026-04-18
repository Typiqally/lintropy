//! `lintropy init` — scaffold `lintropy.yaml` and `.lintropy/`.
//!
//! `--with-skill` is a stub pending WP6's SKILL.md authoring; it prints a
//! notice to stderr and proceeds without writing the skill file.

use std::fs;
use std::path::{Path, PathBuf};

use crate::agent_settings;
use crate::cli::InitArgs;
use crate::commands::current_dir;
use crate::exit::{CliError, EXIT_OK};

const ROOT_CONFIG: &str = "lintropy.yaml";
const EXAMPLE_RULE_DIR: &str = ".lintropy";
const EXAMPLE_RULE_FILE: &str = "no-unwrap.rule.yaml";

const ROOT_CONFIG_TEMPLATE: &str = r#"version: 1
settings:
  fail_on: error
  default_severity: warning
# Rules are typically defined as one file per rule under `.lintropy/`.
# See `.lintropy/no-unwrap.rule.yaml` for a starter query rule.
"#;

const EXAMPLE_RULE_TEMPLATE: &str = r#"language: rust
severity: warning
message: "avoid .unwrap() on `{{recv}}` — handle the error explicitly"
include: ["**/*.rs"]
query: |
  (call_expression
    function: (field_expression
      value: (_) @recv
      field: (field_identifier) @method)
    (#eq? @method "unwrap")) @match
fix: '{{recv}}.expect("TODO: handle error")'
"#;

pub fn run(args: InitArgs) -> Result<u8, CliError> {
    let root = match &args.root {
        Some(p) => p.clone(),
        None => current_dir()?,
    };

    let config_path = root.join(ROOT_CONFIG);
    write_once(&config_path, ROOT_CONFIG_TEMPLATE)?;
    println!("created {}", config_path.display());

    let rules_dir = root.join(EXAMPLE_RULE_DIR);
    fs::create_dir_all(&rules_dir)?;
    let rule_path = rules_dir.join(EXAMPLE_RULE_FILE);
    write_once(&rule_path, EXAMPLE_RULE_TEMPLATE)?;
    println!("created {}", rule_path.display());

    if args.with_skill {
        agent_settings::merge_claude_settings(&root)?;
        println!("updated {}", root.join(".claude/settings.json").display());
        let _ = args.skill_dir; // placeholder for WP6 wire-up
    }

    Ok(EXIT_OK)
}

fn write_once(path: &Path, contents: &str) -> Result<(), CliError> {
    if path.exists() {
        return Err(CliError::user(format!(
            "refusing to overwrite existing {}",
            path.display()
        )));
    }
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent)?;
    }
    atomic_write(path, contents.as_bytes())
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), CliError> {
    let parent: PathBuf = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let mut tmp = tempfile::NamedTempFile::new_in(&parent)
        .map_err(|err| CliError::internal(format!("tempfile: {err}")))?;
    std::io::Write::write_all(&mut tmp, bytes)?;
    tmp.as_file_mut().sync_all()?;
    tmp.persist(path)
        .map_err(|err| CliError::internal(format!("persist: {err}")))?;
    Ok(())
}
