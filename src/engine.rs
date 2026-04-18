use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail};
use walkdir::WalkDir;

use crate::cli::{InitConfigArgs, LintArgs};
use crate::config::Config;

pub fn init_config(args: InitConfigArgs) -> Result<()> {
    if args.output.exists() && !args.force {
        bail!(
            "refusing to overwrite existing config at {} (pass --force to replace it)",
            args.output.display()
        );
    }

    if let Some(parent) = args.output.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).with_context(|| {
                format!("failed to create config directory {}", parent.display())
            })?;
        }
    }

    Config::write_default(&args.output)?;
    println!("Wrote starter config to {}", args.output.display());
    Ok(())
}

pub fn run_lint(args: LintArgs) -> Result<()> {
    if !args.target.exists() {
        bail!("target path does not exist: {}", args.target.display());
    }

    let config = Config::load(&args.config)?;
    let files_scanned = count_files(&args.target)?;

    println!("Lintropy scaffold run");
    println!("project: {}", config.project.name);
    println!("target: {}", args.target.display());
    println!("config: {}", args.config.display());
    println!("files discovered: {files_scanned}");
    println!("output format: {}", config.output.format);
    println!("fail policy: {}", config.output.fail_on);
    println!();

    println!("Configured external tools:");
    for tool in config.enabled_tools() {
        println!("- {} => {}", tool.name, tool.command);
    }

    println!();
    println!("Configured opinionated rules:");
    for rule in config.enabled_rules() {
        let message = if rule.message.is_empty() {
            "no message configured"
        } else {
            &rule.message
        };
        println!("- {} [{}] => {}", rule.id, rule.severity, message);
    }

    println!();
    println!("Next implementation step: map configured tools/rules onto a real lint engine.");
    Ok(())
}

fn count_files(root: &Path) -> Result<usize> {
    let mut count = 0usize;

    for entry in WalkDir::new(root) {
        let entry = entry.with_context(|| format!("failed while walking {}", root.display()))?;
        if entry.file_type().is_file() {
            count += 1;
        }
    }

    Ok(count)
}

#[cfg(test)]
mod tests {
    use crate::config::Config;

    #[test]
    fn config_iterators_skip_disabled_entries() {
        let mut config = Config::default();
        config.tools[0].enabled = false;
        config.rules[0].enabled = false;

        assert_eq!(config.enabled_tools().count(), 1);
        assert_eq!(config.enabled_rules().count(), 0);
    }
}
