//! `clap`-derived command surface.

use std::path::PathBuf;

use crate::core::Severity;
use clap::{Args, Parser, Subcommand, ValueEnum};

#[derive(Debug, Parser)]
#[command(
    name = "lintropy",
    about = "Structural linter driven by tree-sitter queries.",
    version,
    disable_help_subcommand = true
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Walk PATHS, run every matching rule, and print diagnostics (default).
    Check(CheckArgs),
    /// Process a single post-write hook event for an agent harness.
    Hook(HookArgs),
    /// Describe a loaded rule by id.
    Explain(ExplainArgs),
    /// List every rule loaded from the config.
    Rules(RulesArgs),
    /// Scaffold lintropy.yaml and .lintropy/ in the current directory.
    Init(InitArgs),
    /// Emit the config JSON schema (for LLM grounding and editor plugins).
    Schema(SchemaArgs),
    /// Load and validate config without running the engine.
    #[command(subcommand)]
    Config(ConfigCommand),
    /// Parse a source file with tree-sitter and print the S-expression.
    #[command(name = "ts-parse")]
    TsParse(TsParseArgs),
    /// Install the embedded `query` syntax extension into VS Code / Cursor.
    #[command(name = "install-query-extension")]
    InstallQueryExtension(InstallQueryExtensionArgs),
    /// Unpack the embedded TextMate bundle (JetBrains `query` highlighting).
    #[command(name = "install-textmate-bundle")]
    InstallTextmateBundle(InstallTextmateBundleArgs),
    /// Run the Language Server Protocol backend over stdio.
    Lsp,
}

#[derive(Debug, Subcommand)]
pub enum ConfigCommand {
    /// Load the config, verify queries/predicates, print OK with rule count.
    Validate(ConfigValidateArgs),
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, ValueEnum)]
pub enum OutputFormat {
    /// Human-readable rustc-style diagnostics (default).
    #[default]
    Text,
    /// Canonical JSON envelope (§7.3 of the spec).
    Json,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, ValueEnum)]
pub enum HookAgent {
    /// Detect the harness from environment variables, Claude-first for phase 1.
    #[default]
    Auto,
    /// Claude Code hook payloads and settings merge.
    #[value(name = "claude-code")]
    ClaudeCode,
    /// Codex hook payloads (phase-2 stub for now).
    Codex,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, ValueEnum)]
pub enum HookFormat {
    /// One compact line per diagnostic, plus an optional help line.
    #[default]
    Compact,
    /// Canonical JSON envelope (§7.3).
    Json,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, ValueEnum)]
pub enum HookSeverity {
    Info,
    Warning,
    #[default]
    Error,
}

impl From<HookSeverity> for Severity {
    fn from(value: HookSeverity) -> Self {
        match value {
            HookSeverity::Info => Severity::Info,
            HookSeverity::Warning => Severity::Warning,
            HookSeverity::Error => Severity::Error,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, ValueEnum)]
pub enum SchemaKind {
    /// The repo-root `lintropy.yaml` schema.
    #[default]
    Root,
    /// A single `.lintropy/*.rule.yaml` schema.
    Rule,
    /// A grouped `.lintropy/*.rules.yaml` schema.
    Rules,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, ValueEnum)]
pub enum GroupBy {
    /// Flat list sorted by id (default).
    #[default]
    None,
    /// Group by rule language.
    Language,
    /// Group by the rule's first tag.
    Tag,
}

#[derive(Debug, Default, Args)]
pub struct CheckArgs {
    /// Paths to scan. Defaults to ".".
    pub paths: Vec<PathBuf>,

    /// Override config discovery with an explicit path.
    #[arg(long, value_name = "PATH")]
    pub config: Option<PathBuf>,

    /// Reporter format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub format: OutputFormat,

    /// Write output atomically to PATH instead of stdout.
    #[arg(long, short = 'o', value_name = "PATH")]
    pub output: Option<PathBuf>,

    /// Apply autofixes in place.
    #[arg(long, conflicts_with = "fix_dry_run")]
    pub fix: bool,

    /// Print unified diff of autofixes instead of applying them.
    #[arg(long = "fix-dry-run")]
    pub fix_dry_run: bool,

    /// Force color off even on TTY.
    #[arg(long = "no-color")]
    pub no_color: bool,

    /// Suppress reporter output (exit code still reflects fail_on).
    #[arg(long)]
    pub quiet: bool,
}

#[derive(Debug, Args)]
pub struct HookArgs {
    /// Override config discovery with an explicit path.
    #[arg(long, value_name = "PATH")]
    pub config: Option<PathBuf>,

    /// Agent harness to target.
    #[arg(long, value_enum, default_value_t = HookAgent::Auto)]
    pub agent: HookAgent,

    /// Output format for diagnostics written to stderr.
    #[arg(long, value_enum, default_value_t = HookFormat::Compact)]
    pub format: HookFormat,

    /// Minimum severity that causes an exit status of 2.
    #[arg(long = "fail-on", value_enum, default_value_t = HookSeverity::Error)]
    pub fail_on: HookSeverity,

    /// Emit non-blocking hook warnings to stderr.
    #[arg(long, hide = true)]
    pub verbose: bool,
}

#[derive(Debug, Args)]
pub struct ExplainArgs {
    /// Rule id to describe (e.g. `no-unwrap`).
    pub rule_id: String,

    /// Override config discovery.
    #[arg(long, value_name = "PATH")]
    pub config: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub struct RulesArgs {
    /// Emit the list as JSON.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub format: OutputFormat,

    /// Group text output by language or first tag. Text format only.
    #[arg(long = "group-by", value_enum, default_value_t = GroupBy::None)]
    pub group_by: GroupBy,

    /// Override config discovery.
    #[arg(long, value_name = "PATH")]
    pub config: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub struct InitArgs {
    /// Root directory to scaffold into. Defaults to ".".
    #[arg(value_name = "DIR")]
    pub root: Option<PathBuf>,

    /// Also install SKILL.md into agent skill directories (stub until WP6).
    #[arg(long = "with-skill")]
    pub with_skill: bool,

    /// Override skill directory target.
    #[arg(long = "skill-dir", value_name = "PATH")]
    pub skill_dir: Option<PathBuf>,
}

#[derive(Debug, Default, Args)]
pub struct SchemaArgs {
    /// Schema shape to emit.
    #[arg(long, value_enum, default_value_t = SchemaKind::Root)]
    pub kind: SchemaKind,

    /// Write the schema to PATH instead of stdout.
    #[arg(long, short = 'o', value_name = "PATH")]
    pub output: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub struct ConfigValidateArgs {
    /// Explicit config file to validate. Falls back to root discovery.
    pub path: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub struct TsParseArgs {
    /// Source file to parse.
    pub file: PathBuf,

    /// Override the language derived from the extension.
    #[arg(long, value_name = "NAME")]
    pub lang: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum QueryEditor {
    /// Install into VS Code via the `code` CLI.
    Vscode,
    /// Install into Cursor via the `cursor` CLI.
    Cursor,
}

#[derive(Debug, Args)]
pub struct InstallQueryExtensionArgs {
    /// Target editor. Required unless `--package-only` is set.
    #[arg(value_enum)]
    pub editor: Option<QueryEditor>,

    /// Install into a named editor profile.
    #[arg(long, value_name = "NAME")]
    pub profile: Option<String>,

    /// Write the embedded `.vsix` to disk instead of invoking the editor.
    #[arg(long = "package-only")]
    pub package_only: bool,

    /// Output path for `--package-only`. Defaults to
    /// `./lintropy-query-syntax.vsix`.
    #[arg(long, short = 'o', value_name = "PATH", requires = "package_only")]
    pub output: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub struct InstallTextmateBundleArgs {
    /// Parent directory to unpack the bundle into. Defaults to the
    /// current working directory; the bundle dir is created beneath it.
    #[arg(long, value_name = "PATH")]
    pub dir: Option<PathBuf>,

    /// Overwrite an existing bundle dir in place.
    #[arg(long)]
    pub force: bool,
}
