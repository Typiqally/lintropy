//! `lintropy init` — scaffold `lintropy.yaml` and `.lintropy/`.
//!
//! `--with-skill` writes the embedded `SKILL.md` into detected agent skill
//! directories (§10.2) and, when `.claude/` is present, merges the §15.3
//! `PostToolUse` entry into `.claude/settings.json` via
//! `agent_settings::merge_claude_settings`. Idempotent.

use std::fs;
use std::path::{Path, PathBuf};

use crate::agent_settings;
use crate::cli::InitArgs;
use crate::commands::current_dir;
use crate::exit::{CliError, EXIT_OK};
use crate::skill::{EMBEDDED_SKILL, SKILL_VERSION};

const ROOT_CONFIG: &str = "lintropy.yaml";
const EXAMPLE_RULE_DIR: &str = ".lintropy";
const EXAMPLE_RULE_FILE: &str = "no-unwrap.rule.yaml";
const CLAUDE_MATCHER: &str = "Write|Edit|NotebookEdit";
const LINTROPY_COMMAND: &str = "lintropy hook --agent claude-code";
const VSCODE_EXTENSIONS: &str = r#"{
  "recommendations": [
    "lintropy.lintropy",
    "redhat.vscode-yaml"
  ]
}
"#;

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

    scaffold_vscode_recommendations(&root)?;

    if args.with_skill {
        install_skill(&root, args.skill_dir.as_deref())?;
    }

    Ok(EXIT_OK)
}

fn install_skill(root: &Path, override_dir: Option<&Path>) -> Result<(), CliError> {
    if let Some(dir) = override_dir {
        let target = dir.join("SKILL.md");
        let outcome = write_skill(&target)?;
        report_skill(&target, outcome);
        return Ok(());
    }

    let claude = root.join(".claude");
    let cursor = root.join(".cursor");
    let claude_present = claude.is_dir();
    let cursor_present = cursor.is_dir();

    if !claude_present && !cursor_present {
        print_snippets();
        return Ok(());
    }

    if claude_present {
        let target = claude.join("skills").join("lintropy").join("SKILL.md");
        let outcome = write_skill(&target)?;
        report_skill(&target, outcome);
        let settings_path = claude.join("settings.json");
        let before = fs::read(&settings_path).ok();
        agent_settings::merge_claude_settings(root)?;
        let after = fs::read(&settings_path).ok();
        let label = match (before, after) {
            (None, Some(_)) => "created",
            (Some(b), Some(a)) if b == a => "unchanged",
            _ => "updated",
        };
        println!("{label} {}", settings_path.display());
    }
    if cursor_present {
        let target = cursor.join("skills").join("lintropy").join("SKILL.md");
        let outcome = write_skill(&target)?;
        report_skill(&target, outcome);
    }
    Ok(())
}

#[derive(Debug, PartialEq, Eq)]
enum SkillOutcome {
    Created,
    Upgraded,
    Unchanged,
}

fn write_skill(path: &Path) -> Result<SkillOutcome, CliError> {
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent)?;
    }
    let existed = path.exists();
    if existed {
        let current = fs::read_to_string(path)?;
        if version_header(&current) == Some(SKILL_VERSION) {
            return Ok(SkillOutcome::Unchanged);
        }
    }
    atomic_write(path, EMBEDDED_SKILL.as_bytes())?;
    Ok(if existed {
        SkillOutcome::Upgraded
    } else {
        SkillOutcome::Created
    })
}

fn version_header(source: &str) -> Option<&str> {
    let first = source.lines().next()?;
    let rest = first.trim_start().strip_prefix('#')?.trim_start();
    rest.strip_prefix("version:").map(str::trim)
}

fn report_skill(path: &Path, outcome: SkillOutcome) {
    let label = match outcome {
        SkillOutcome::Created => "created",
        SkillOutcome::Upgraded => "upgraded",
        SkillOutcome::Unchanged => "unchanged",
    };
    println!("{label} {}", path.display());
}

fn print_snippets() {
    println!();
    println!("no `.claude/` or `.cursor/` detected — skipping skill install.");
    println!("paste these into your agent config to wire lintropy manually:");
    println!();
    println!("Claude Code (.claude/settings.json):");
    println!(
        r#"{{
  "hooks": {{
    "PostToolUse": [
      {{
        "matcher": "{matcher}",
        "hooks": [
          {{ "type": "command", "command": "{cmd}" }}
        ]
      }}
    ]
  }}
}}"#,
        matcher = CLAUDE_MATCHER,
        cmd = LINTROPY_COMMAND,
    );
    println!();
    println!("Codex: phase-2 — schema TBD.");
}

/// Write `.vscode/extensions.json` recommending the lintropy + YAML
/// extensions. Non-invasive: skips when the file already exists so users
/// who have their own recommendations list aren't clobbered.
fn scaffold_vscode_recommendations(root: &Path) -> Result<(), CliError> {
    let target = root.join(".vscode").join("extensions.json");
    if target.exists() {
        println!("skipped {} (already present)", target.display());
        return Ok(());
    }
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }
    atomic_write(&target, VSCODE_EXTENSIONS.as_bytes())?;
    println!("created {}", target.display());
    Ok(())
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
