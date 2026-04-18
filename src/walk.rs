//! File walker: expand the user's PATHS into a concrete file list via
//! `ignore::WalkBuilder` (honours `.gitignore`, hidden, and parent ignores).

use std::path::{Path, PathBuf};

use ignore::WalkBuilder;

use crate::exit::CliError;

/// Expand `paths` into a deduplicated, canonicalized list of files. Each
/// entry in `paths` may be a file (kept as-is) or a directory (walked).
pub fn expand(paths: &[PathBuf]) -> Result<Vec<PathBuf>, CliError> {
    let effective: Vec<PathBuf> = if paths.is_empty() {
        vec![PathBuf::from(".")]
    } else {
        paths.to_vec()
    };

    let mut out = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for entry in &effective {
        if entry.is_file() {
            push_unique(&mut out, &mut seen, entry.clone());
            continue;
        }
        walk_dir(entry, &mut out, &mut seen)?;
    }
    Ok(out)
}

fn walk_dir(
    root: &Path,
    out: &mut Vec<PathBuf>,
    seen: &mut std::collections::BTreeSet<PathBuf>,
) -> Result<(), CliError> {
    let walker = WalkBuilder::new(root).hidden(true).build();
    for entry in walker {
        let entry = entry.map_err(|err| CliError::internal(format!("walk error: {err}")))?;
        if !entry.file_type().is_some_and(|ft| ft.is_file()) {
            continue;
        }
        push_unique(out, seen, entry.into_path());
    }
    Ok(())
}

fn push_unique(
    out: &mut Vec<PathBuf>,
    seen: &mut std::collections::BTreeSet<PathBuf>,
    path: PathBuf,
) {
    let key = path.canonicalize().unwrap_or_else(|_| path.clone());
    if seen.insert(key) {
        out.push(path);
    }
}
