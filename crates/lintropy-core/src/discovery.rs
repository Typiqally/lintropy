//! Project root discovery and rule-file enumeration.
//!
//! Implements the discovery algorithm in §4.1 of the merged spec:
//!
//! 1. Walk up from `start` looking for `lintropy.yaml`.
//! 2. From the resolved root, glob `.lintropy/**/*.{rule,rules}.yaml` via
//!    the `ignore` crate so `.gitignore` is honoured.

use std::path::{Path, PathBuf};

use crate::{LintropyError, Result};

/// Anchor filename at the project root.
pub const ROOT_CONFIG_NAME: &str = "lintropy.yaml";

/// Subdirectory that holds discoverable rule files.
pub const RULE_DIR: &str = ".lintropy";

/// Everything [`discover_from`] resolved about a project tree.
#[derive(Debug, Clone)]
pub struct Discovered {
    /// Path to the anchoring `lintropy.yaml`.
    pub root_config: PathBuf,
    /// Path to the project root (parent of `lintropy.yaml`).
    pub root_dir: PathBuf,
    /// Rule files under `.lintropy/`, in stable sorted order.
    pub rule_files: Vec<PathBuf>,
}

/// Discover the project root from `start` and enumerate its rule files.
///
/// Ascends the filesystem hierarchy until a `lintropy.yaml` is found or the
/// filesystem root is hit. Returns [`LintropyError::ConfigLoad`] if no root
/// config is found.
pub fn discover_from(start: &Path) -> Result<Discovered> {
    let start = canonicalize_best_effort(start);
    let root_config = walk_up_for_root(&start)?;
    let root_dir = root_config
        .parent()
        .ok_or_else(|| LintropyError::ConfigLoad("root config has no parent dir".into()))?
        .to_path_buf();
    let rule_files = enumerate_rule_files(&root_dir)?;
    Ok(Discovered {
        root_config,
        root_dir,
        rule_files,
    })
}

/// Enumerate `.lintropy/**/*.{rule,rules}.yaml` under `root_dir`.
///
/// Missing `.lintropy/` directory returns `Ok(vec![])`. Honours
/// `.gitignore` via [`ignore::WalkBuilder`]. Output is sorted so loader
/// behaviour is deterministic across platforms.
pub fn enumerate_rule_files(root_dir: &Path) -> Result<Vec<PathBuf>> {
    let rule_dir = root_dir.join(RULE_DIR);
    if !rule_dir.exists() {
        return Ok(Vec::new());
    }

    let mut files = Vec::new();
    let walker = ignore::WalkBuilder::new(&rule_dir)
        .hidden(false)
        .parents(false)
        .build();

    for entry in walker {
        let entry = entry.map_err(|e| LintropyError::ConfigLoad(format!("walk error: {e}")))?;
        if !entry.file_type().is_some_and(|ft| ft.is_file()) {
            continue;
        }
        let path = entry.into_path();
        if is_rule_file(&path) {
            files.push(path);
        }
    }

    files.sort();
    Ok(files)
}

fn walk_up_for_root(start: &Path) -> Result<PathBuf> {
    let mut cursor: Option<&Path> = Some(start);
    while let Some(dir) = cursor {
        let candidate = dir.join(ROOT_CONFIG_NAME);
        if candidate.is_file() {
            return Ok(candidate);
        }
        cursor = dir.parent();
    }
    Err(LintropyError::ConfigLoad(format!(
        "no {ROOT_CONFIG_NAME} found walking up from {}",
        start.display()
    )))
}

fn canonicalize_best_effort(p: &Path) -> PathBuf {
    p.canonicalize().unwrap_or_else(|_| p.to_path_buf())
}

fn is_rule_file(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return false;
    };
    name.ends_with(".rule.yaml") || name.ends_with(".rules.yaml")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn touch(path: &Path) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, b"").unwrap();
    }

    #[test]
    fn is_rule_file_matches_spec() {
        assert!(is_rule_file(Path::new(".lintropy/no-unwrap.rule.yaml")));
        assert!(is_rule_file(Path::new(".lintropy/2026q2.rules.yaml")));
        assert!(!is_rule_file(Path::new(".lintropy/notes.md")));
        assert!(!is_rule_file(Path::new(".lintropy/rule.yaml")));
    }

    #[test]
    fn walks_up_for_root_config() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        touch(&root.join("lintropy.yaml"));
        let nested = root.join("deep/nested/dir");
        fs::create_dir_all(&nested).unwrap();
        let discovered = discover_from(&nested).unwrap();
        assert_eq!(
            discovered.root_config.canonicalize().unwrap(),
            root.join("lintropy.yaml").canonicalize().unwrap()
        );
        assert!(discovered.rule_files.is_empty());
    }

    #[test]
    fn enumerates_rule_and_rules_files_sorted() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        touch(&root.join("lintropy.yaml"));
        touch(&root.join(".lintropy/z-one.rule.yaml"));
        touch(&root.join(".lintropy/a-two.rules.yaml"));
        touch(&root.join(".lintropy/notes.md"));
        touch(&root.join(".lintropy/sub/nested.rule.yaml"));
        let files = enumerate_rule_files(root).unwrap();
        assert_eq!(files.len(), 3);
        assert!(files.windows(2).all(|w| w[0] <= w[1]));
    }

    #[test]
    fn missing_root_errors() {
        let dir = TempDir::new().unwrap();
        let err = discover_from(dir.path()).unwrap_err();
        assert!(matches!(err, LintropyError::ConfigLoad(_)));
    }
}
