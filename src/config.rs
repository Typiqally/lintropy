use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

pub const DEFAULT_CONFIG: &str = r#"# lintropy.toml
# This file is meant to stay simple enough for humans and LLMs to edit directly.

[project]
name = "my-codebase"
root = "."

[discovery]
include = ["src/**/*.rs", "crates/**/*.rs"]
exclude = ["target/**", "dist/**", "node_modules/**"]

[output]
format = "text" # text | json
show_summary = true
fail_on = "error" # error | warning | never

[[tools]]
name = "rustfmt"
command = "cargo fmt --check"
enabled = true
files = ["**/*.rs"]

[[tools]]
name = "clippy"
command = "cargo clippy --all-targets --all-features -- -D warnings"
enabled = true
files = ["**/*.rs"]

[[rules]]
id = "ban-todo"
enabled = true
severity = "warning" # info | warning | error
message = "Remove TODO markers before merging"
include = ["**/*.rs", "**/*.md"]
exclude = ["CHANGELOG.md"]

[rules.settings]
needle = "TODO"
"#;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub project: ProjectConfig,
    #[serde(default)]
    pub discovery: DiscoveryConfig,
    #[serde(default)]
    pub output: OutputConfig,
    #[serde(default)]
    pub tools: Vec<ToolConfig>,
    #[serde(default)]
    pub rules: Vec<RuleConfig>,
}

impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        let raw = fs::read_to_string(path)
            .with_context(|| format!("failed to read config file at {}", path.display()))?;
        let config: Self = toml::from_str(&raw)
            .with_context(|| format!("failed to parse TOML config at {}", path.display()))?;
        Ok(config)
    }

    pub fn write_default(path: &Path) -> Result<()> {
        fs::write(path, DEFAULT_CONFIG)
            .with_context(|| format!("failed to write config file at {}", path.display()))
    }

    pub fn enabled_tools(&self) -> impl Iterator<Item = &ToolConfig> {
        self.tools.iter().filter(|tool| tool.enabled)
    }

    pub fn enabled_rules(&self) -> impl Iterator<Item = &RuleConfig> {
        self.rules.iter().filter(|rule| rule.enabled)
    }
}

impl Default for Config {
    fn default() -> Self {
        toml::from_str(DEFAULT_CONFIG).expect("default config must stay valid")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    #[serde(default = "default_project_name")]
    pub name: String,
    #[serde(default = "default_project_root")]
    pub root: PathBuf,
}

impl Default for ProjectConfig {
    fn default() -> Self {
        Self {
            name: default_project_name(),
            root: default_project_root(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryConfig {
    #[serde(default = "default_include")]
    pub include: Vec<String>,
    #[serde(default = "default_exclude")]
    pub exclude: Vec<String>,
}

impl Default for DiscoveryConfig {
    fn default() -> Self {
        Self {
            include: default_include(),
            exclude: default_exclude(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputConfig {
    #[serde(default = "default_output_format")]
    pub format: String,
    #[serde(default = "default_show_summary")]
    pub show_summary: bool,
    #[serde(default = "default_fail_on")]
    pub fail_on: String,
}

impl Default for OutputConfig {
    fn default() -> Self {
        Self {
            format: default_output_format(),
            show_summary: default_show_summary(),
            fail_on: default_fail_on(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolConfig {
    pub name: String,
    pub command: String,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub files: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleConfig {
    pub id: String,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default = "default_severity")]
    pub severity: String,
    #[serde(default)]
    pub message: String,
    #[serde(default)]
    pub include: Vec<String>,
    #[serde(default)]
    pub exclude: Vec<String>,
    #[serde(default)]
    pub settings: toml::Table,
}

fn default_project_name() -> String {
    "my-codebase".to_owned()
}

fn default_project_root() -> PathBuf {
    PathBuf::from(".")
}

fn default_include() -> Vec<String> {
    vec!["src/**/*.rs".to_owned()]
}

fn default_exclude() -> Vec<String> {
    vec!["target/**".to_owned()]
}

fn default_output_format() -> String {
    "text".to_owned()
}

fn default_show_summary() -> bool {
    true
}

fn default_fail_on() -> String {
    "error".to_owned()
}

fn default_enabled() -> bool {
    true
}

fn default_severity() -> String {
    "warning".to_owned()
}

#[cfg(test)]
mod tests {
    use super::Config;

    #[test]
    fn default_config_is_valid() {
        let config = Config::default();

        assert_eq!(config.project.name, "my-codebase");
        assert_eq!(config.enabled_tools().count(), 2);
        assert_eq!(config.enabled_rules().count(), 1);
    }
}
