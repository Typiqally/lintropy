//! Integration tests for the `install` subcommand.

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::path::Path;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

#[test]
fn install_jetbrains_extracts_template_files() {
    let dir = tempfile::tempdir().unwrap();
    Command::cargo_bin("lintropy")
        .unwrap()
        .arg("install")
        .arg("jetbrains")
        .arg("--dir")
        .arg(dir.path())
        .assert()
        .code(0)
        .stdout(predicate::str::contains("extracted"))
        .stdout(predicate::str::contains("Import from directory"));

    let template = dir.path().join("lsp4ij-template").join("template.json");
    assert!(template.is_file());
    let parsed: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&template).unwrap()).unwrap();
    assert_eq!(parsed["id"], "lintropy");
    assert_eq!(parsed["programArgs"]["default"], "lintropy lsp");
    let rust_patterns = parsed["fileTypeMappings"][0]["fileType"]["patterns"]
        .as_array()
        .unwrap();
    assert!(rust_patterns.iter().any(|p| p == "*.rs"));
    let yaml_patterns = parsed["fileTypeMappings"][1]["fileType"]["patterns"]
        .as_array()
        .unwrap();
    assert!(yaml_patterns.iter().any(|p| p == "lintropy.yaml"));
    assert!(yaml_patterns.iter().any(|p| p == "*.rule.yaml"));
}

#[test]
fn install_jetbrains_refuses_existing_dir_without_force() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(dir.path().join("lsp4ij-template")).unwrap();
    Command::cargo_bin("lintropy")
        .unwrap()
        .arg("install")
        .arg("jetbrains")
        .arg("--dir")
        .arg(dir.path())
        .assert()
        .code(2)
        .stderr(predicate::str::contains("refusing to overwrite"));
}

#[test]
fn install_claude_code_writes_manifest() {
    let dir = tempfile::tempdir().unwrap();
    let bin_dir = dir.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();
    Command::cargo_bin("lintropy")
        .unwrap()
        .arg("install")
        .arg("claude-code")
        .arg("--dir")
        .arg(dir.path())
        .env("PATH", &bin_dir)
        .assert()
        .code(0)
        .stdout(predicate::str::contains("extracted"))
        .stdout(predicate::str::contains("claude --plugin-dir"));

    let manifest = dir
        .path()
        .join("lintropy-claude-code-plugin")
        .join(".claude-plugin")
        .join("plugin.json");
    assert!(manifest.is_file());
    let parsed: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&manifest).unwrap()).unwrap();
    assert_eq!(parsed["name"], "lintropy-lsp");
    assert_eq!(parsed["version"], env!("CARGO_PKG_VERSION"));
    let server = &parsed["lspServers"]["lintropy"];
    let command = server["command"].as_str().unwrap();
    assert!(
        command.ends_with("lintropy") || command.ends_with("lintropy.exe"),
        "command should point at the lintropy binary: {command}"
    );
    assert_eq!(server["args"][0], "lsp");
    let ext_map = server["extensionToLanguage"].as_object().unwrap();
    assert_eq!(ext_map.get(".rs").and_then(|v| v.as_str()), Some("rust"));
    assert_eq!(ext_map.get(".yaml").and_then(|v| v.as_str()), Some("yaml"));
}

#[test]
fn install_claude_code_prints_plugin_dir_invocation_and_bundles_skill() {
    let dir = tempfile::tempdir().unwrap();
    let bin_dir = dir.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();

    Command::cargo_bin("lintropy")
        .unwrap()
        .arg("install")
        .arg("claude-code")
        .arg("--dir")
        .arg(dir.path())
        .env("PATH", &bin_dir)
        .assert()
        .code(0)
        .stdout(predicate::str::contains("claude --plugin-dir"))
        .stdout(predicate::str::contains("lintropy-claude-code-plugin"));

    let skill = dir
        .path()
        .join("lintropy-claude-code-plugin")
        .join("skills")
        .join("lintropy")
        .join("SKILL.md");
    assert!(skill.is_file(), "SKILL.md must be bundled inside plugin");
    let first_line = fs::read_to_string(&skill)
        .unwrap()
        .lines()
        .next()
        .unwrap()
        .to_string();
    assert!(
        first_line.starts_with("# version:"),
        "first line must carry `# version:` header, got: {first_line}"
    );
}

#[test]
fn committed_claude_code_plugin_matches_generated_manifest() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let committed_path = repo_root
        .join("editors")
        .join("claude-code")
        .join(".claude-plugin")
        .join("plugin.json");
    let committed: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&committed_path).unwrap()).unwrap();

    let dir = tempfile::tempdir().unwrap();
    let bin_dir = dir.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();
    let stub_bin = bin_dir.join("lintropy");
    write_executable(&stub_bin, "#!/bin/sh\nexit 0\n");

    Command::cargo_bin("lintropy")
        .unwrap()
        .arg("install")
        .arg("claude-code")
        .arg("--dir")
        .arg(dir.path())
        .env("PATH", &bin_dir)
        .assert()
        .code(0);

    let generated_path = dir
        .path()
        .join("lintropy-claude-code-plugin")
        .join(".claude-plugin")
        .join("plugin.json");
    let mut generated: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&generated_path).unwrap()).unwrap();
    // Runtime emits an absolute path; the committed reference uses the PATH
    // fallback "lintropy". Normalise the dynamic field before comparing.
    generated["lspServers"]["lintropy"]["command"] =
        serde_json::Value::String("lintropy".to_string());
    assert_eq!(
        generated, committed,
        "editors/claude-code/.claude-plugin/plugin.json is out of sync with build_manifest(). \
         Regenerate: `lintropy install claude-code` then copy the file over."
    );
}

#[test]
fn committed_claude_code_skill_matches_canonical() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let canonical = fs::read_to_string(repo_root.join("skill").join("SKILL.md")).unwrap();
    let bundled = fs::read_to_string(
        repo_root
            .join("editors")
            .join("claude-code")
            .join("skills")
            .join("lintropy")
            .join("SKILL.md"),
    )
    .unwrap();
    assert_eq!(
        canonical, bundled,
        "editors/claude-code/skills/lintropy/SKILL.md is out of sync with skill/SKILL.md. \
         Run `cp skill/SKILL.md editors/claude-code/skills/lintropy/SKILL.md`."
    );
}

#[test]
fn marketplace_manifest_points_at_claude_code_plugin() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let marketplace_path = repo_root.join(".claude-plugin").join("marketplace.json");
    let marketplace: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&marketplace_path).unwrap()).unwrap();
    assert_eq!(marketplace["name"], "lintropy");
    let plugins = marketplace["plugins"].as_array().unwrap();
    let entry = plugins
        .iter()
        .find(|p| p["name"] == "lintropy-lsp")
        .expect("marketplace.json should list the lintropy-lsp plugin");
    let source = entry["source"].as_str().unwrap();
    let plugin_root = repo_root.join(source.trim_start_matches("./"));
    assert!(
        plugin_root
            .join(".claude-plugin")
            .join("plugin.json")
            .is_file(),
        "marketplace source {source} does not contain .claude-plugin/plugin.json"
    );
}

#[test]
fn install_vscode_package_only_builds_vsix_from_source() {
    let dir = tempfile::tempdir().unwrap();
    let extension_dir = dir.path().join("extension");
    fs::create_dir_all(&extension_dir).unwrap();
    fs::write(extension_dir.join("package.json"), "{}").unwrap();
    let bin_dir = dir.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();
    let log = dir.path().join("pnpm.log");
    write_executable(
        &bin_dir.join("pnpm"),
        r#"#!/bin/sh
if [ -n "$LINTROPY_TEST_LOG" ]; then
  echo "$@" >> "$LINTROPY_TEST_LOG"
fi
if [ "$1" = "install" ]; then
  exit 0
fi
if [ "$1" = "run" ] && [ "$2" = "compile" ]; then
  exit 0
fi
if [ "$1" = "exec" ] && [ "$2" = "vsce" ] && [ "$3" = "package" ]; then
  while [ "$1" != "-o" ] && [ $# -gt 0 ]; do
    shift
  done
  shift
  printf 'PK\003\004fake vsix' > "$1"
  exit 0
fi
exit 1
"#,
    );
    let out = dir.path().join("out.vsix");
    Command::cargo_bin("lintropy")
        .unwrap()
        .arg("install")
        .arg("vscode")
        .env("LINTROPY_VSCODE_EXTENSION_DIR", &extension_dir)
        .env("LINTROPY_TEST_LOG", &log)
        .env("PATH", &bin_dir)
        .arg("--package-only")
        .arg("-o")
        .arg(&out)
        .assert()
        .code(0)
        .stdout(predicate::str::contains("packaged"));
    assert_eq!(fs::read(&out).unwrap(), b"PK\x03\x04fake vsix");
    let invocations = fs::read_to_string(&log).unwrap();
    assert!(invocations.contains("install"));
    assert!(invocations.contains("run compile"));
    assert!(invocations.contains("exec vsce package --no-yarn --no-dependencies -o"));
}

#[test]
fn install_vscode_rejects_missing_source_dir() {
    Command::cargo_bin("lintropy")
        .unwrap()
        .arg("install")
        .arg("vscode")
        .env("LINTROPY_VSCODE_EXTENSION_DIR", "/does/not/exist")
        .arg("--package-only")
        .arg("-o")
        .arg("/tmp/unused.vsix")
        .assert()
        .code(2)
        .stderr(predicate::str::contains("source not found"));
}

#[test]
fn install_vscode_builds_and_installs_extension() {
    let dir = tempfile::tempdir().unwrap();
    let extension_dir = dir.path().join("extension");
    fs::create_dir_all(&extension_dir).unwrap();
    fs::write(extension_dir.join("package.json"), "{}").unwrap();

    let bin_dir = dir.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();
    let pnpm_log = dir.path().join("pnpm.log");
    let code_log = dir.path().join("code.log");

    write_executable(
        &bin_dir.join("pnpm"),
        r#"#!/bin/sh
if [ -n "$LINTROPY_TEST_LOG" ]; then
  echo "$@" >> "$LINTROPY_TEST_LOG"
fi
if [ "$1" = "install" ]; then
  exit 0
fi
if [ "$1" = "run" ] && [ "$2" = "compile" ]; then
  exit 0
fi
if [ "$1" = "exec" ] && [ "$2" = "vsce" ] && [ "$3" = "package" ]; then
  while [ "$1" != "-o" ] && [ $# -gt 0 ]; do
    shift
  done
  shift
  printf 'PK\003\004fake vsix' > "$1"
  exit 0
fi
exit 1
"#,
    );
    write_executable(
        &bin_dir.join("code"),
        r#"#!/bin/sh
echo "$@" > "$LINTROPY_CODE_LOG"
exit 0
"#,
    );

    Command::cargo_bin("lintropy")
        .unwrap()
        .arg("install")
        .arg("vscode")
        .arg("--profile")
        .arg("Default")
        .env("LINTROPY_VSCODE_EXTENSION_DIR", &extension_dir)
        .env("LINTROPY_TEST_LOG", &pnpm_log)
        .env("LINTROPY_CODE_LOG", &code_log)
        .env("PATH", &bin_dir)
        .assert()
        .code(0)
        .stdout(predicate::str::contains("installed lintropy into code"));

    let code_args = fs::read_to_string(&code_log).unwrap();
    assert!(code_args.contains("--profile Default"));
    assert!(code_args.contains("--install-extension"));
    assert!(code_args.contains("--force"));

    let invocations = fs::read_to_string(&pnpm_log).unwrap();
    assert!(invocations.contains("install"));
    assert!(invocations.contains("run compile"));
    assert!(invocations.contains("exec vsce package --no-yarn --no-dependencies -o"));
}

#[cfg(unix)]
fn write_executable(path: &Path, body: &str) {
    fs::write(path, body).unwrap();
    let mut perms = fs::metadata(path).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(path, perms).unwrap();
}

#[cfg(not(unix))]
fn write_executable(path: &Path, body: &str) {
    fs::write(path, body).unwrap();
}
