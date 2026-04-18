use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use crate::exit::CliError;

const CLAUDE_SETTINGS_DIR: &str = ".claude";
const CLAUDE_SETTINGS_FILE: &str = "settings.json";
const MATCHER: &str = "Write|Edit|NotebookEdit";
const COMMAND: &str = "lintropy hook --agent claude-code";

pub fn merge_claude_settings(repo_root: &Path) -> Result<(), CliError> {
    let settings_path = repo_root
        .join(CLAUDE_SETTINGS_DIR)
        .join(CLAUDE_SETTINGS_FILE);
    let mut root = read_settings(&settings_path)?;
    merge_lintropy_hook(&mut root)?;
    write_settings_atomic(&settings_path, &root)?;
    Ok(())
}

fn read_settings(path: &Path) -> Result<Value, CliError> {
    if !path.exists() {
        return Ok(json!({}));
    }

    let contents = fs::read_to_string(path)?;
    if contents.trim().is_empty() {
        return Ok(json!({}));
    }

    serde_json::from_str(&contents)
        .map_err(|err| CliError::user(format!("invalid JSON in {}: {err}", path.display())))
}

fn merge_lintropy_hook(root: &mut Value) -> Result<(), CliError> {
    if !root.is_object() {
        *root = json!({});
    }

    let top = root
        .as_object_mut()
        .ok_or_else(|| CliError::internal("settings root was not an object"))?;
    let hooks = top.entry("hooks").or_insert_with(|| json!({}));
    if !hooks.is_object() {
        *hooks = json!({});
    }

    let post_tool_use = hooks
        .as_object_mut()
        .expect("hooks object")
        .entry("PostToolUse")
        .or_insert_with(|| json!([]));
    if !post_tool_use.is_array() {
        *post_tool_use = json!([]);
    }

    let entries = post_tool_use
        .as_array_mut()
        .ok_or_else(|| CliError::internal("PostToolUse was not an array"))?;
    let replacement = lintropy_entry();

    if let Some(entry) = entries.iter_mut().find(|entry| {
        entry
            .get("matcher")
            .and_then(Value::as_str)
            .is_some_and(|matcher| matcher == MATCHER)
    }) {
        *entry = replacement;
    } else {
        entries.push(replacement);
    }

    Ok(())
}

fn lintropy_entry() -> Value {
    json!({
        "matcher": MATCHER,
        "hooks": [
            {
                "type": "command",
                "command": COMMAND
            }
        ]
    })
}

fn write_settings_atomic(path: &Path, root: &Value) -> Result<(), CliError> {
    let parent: PathBuf = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    fs::create_dir_all(&parent)?;

    let bytes = serde_json::to_vec_pretty(root)
        .map_err(|err| CliError::internal(format!("serialize settings: {err}")))?;

    let mut tmp = tempfile::NamedTempFile::new_in(&parent)
        .map_err(|err| CliError::internal(format!("tempfile: {err}")))?;
    std::io::Write::write_all(&mut tmp, &bytes)?;
    std::io::Write::write_all(&mut tmp, b"\n")?;
    tmp.as_file_mut().sync_all()?;
    tmp.persist(path)
        .map_err(|err| CliError::internal(format!("persist: {err}")))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_creates_empty_settings_file() {
        let dir = tempfile::tempdir().unwrap();
        merge_claude_settings(dir.path()).unwrap();

        let settings: Value = serde_json::from_str(
            &fs::read_to_string(dir.path().join(".claude/settings.json")).unwrap(),
        )
        .unwrap();

        assert_eq!(
            settings["hooks"]["PostToolUse"][0]["hooks"][0]["command"],
            COMMAND
        );
    }

    #[test]
    fn merge_is_idempotent() {
        let dir = tempfile::tempdir().unwrap();

        merge_claude_settings(dir.path()).unwrap();
        let once = fs::read_to_string(dir.path().join(".claude/settings.json")).unwrap();

        merge_claude_settings(dir.path()).unwrap();
        let twice = fs::read_to_string(dir.path().join(".claude/settings.json")).unwrap();

        assert_eq!(once, twice);
    }

    #[test]
    fn merge_preserves_unrelated_hooks() {
        let dir = tempfile::tempdir().unwrap();
        let settings_path = dir.path().join(".claude/settings.json");
        fs::create_dir_all(settings_path.parent().unwrap()).unwrap();
        fs::write(
            &settings_path,
            serde_json::to_vec_pretty(&json!({
                "hooks": {
                    "PostToolUse": [
                        {
                            "matcher": "Read",
                            "hooks": [
                                {
                                    "type": "command",
                                    "command": "echo keep-me"
                                }
                            ]
                        }
                    ]
                }
            }))
            .unwrap(),
        )
        .unwrap();

        merge_claude_settings(dir.path()).unwrap();

        let settings: Value =
            serde_json::from_str(&fs::read_to_string(settings_path).unwrap()).unwrap();
        let entries = settings["hooks"]["PostToolUse"].as_array().unwrap();
        assert_eq!(entries.len(), 2);
        assert!(entries.iter().any(|entry| entry["matcher"] == "Read"));
        assert!(entries.iter().any(|entry| entry["matcher"] == MATCHER));
    }
}
