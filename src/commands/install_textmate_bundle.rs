//! `lintropy install-textmate-bundle` — unpack the embedded TextMate
//! bundle (`Lintropy Query.tmbundle`) into a user-chosen directory so
//! JetBrains IDEs can import it via `Editor | TextMate Bundles`.

use std::fs;
use std::path::Path;

use crate::cli::InstallTextmateBundleArgs;
use crate::commands::current_dir;
use crate::editor_assets::{TMBUNDLE_DIR, TMBUNDLE_DIR_NAME};
use crate::exit::{CliError, EXIT_OK};

pub fn run(args: InstallTextmateBundleArgs) -> Result<u8, CliError> {
    let parent = match args.dir {
        Some(p) => p,
        None => current_dir()?,
    };
    let target = parent.join(TMBUNDLE_DIR_NAME);

    if target.exists() {
        if args.force {
            fs::remove_dir_all(&target)?;
        } else {
            return Err(CliError::user(format!(
                "refusing to overwrite existing {} (pass --force)",
                target.display()
            )));
        }
    }

    extract_dir(&target)?;

    println!("extracted {}", target.display());
    println!(
        "JetBrains IDEs: Settings → Editor → TextMate Bundles → + → {}",
        target.display()
    );
    Ok(EXIT_OK)
}

fn extract_dir(target: &Path) -> Result<(), CliError> {
    fs::create_dir_all(target)?;
    for file in TMBUNDLE_DIR.files() {
        write_embedded_file(target, file.path(), file.contents())?;
    }
    for dir in TMBUNDLE_DIR.dirs() {
        walk_dir(target, dir)?;
    }
    Ok(())
}

fn walk_dir(target: &Path, dir: &include_dir::Dir<'_>) -> Result<(), CliError> {
    for file in dir.files() {
        write_embedded_file(target, file.path(), file.contents())?;
    }
    for sub in dir.dirs() {
        walk_dir(target, sub)?;
    }
    Ok(())
}

fn write_embedded_file(target: &Path, rel: &Path, bytes: &[u8]) -> Result<(), CliError> {
    let stripped = rel.strip_prefix(TMBUNDLE_DIR_NAME).unwrap_or(rel);
    let out = target.join(stripped);
    if let Some(parent) = out.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent)?;
    }
    fs::write(&out, bytes)?;
    Ok(())
}
