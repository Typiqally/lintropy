use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "lintropy",
    version,
    about = "Opinionated static linting scaffold for codebases"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Run the linting pipeline using the configured rules.
    Lint(LintArgs),
    /// Write a starter lintropy.toml file.
    InitConfig(InitConfigArgs),
}

#[derive(Debug, Args)]
pub struct LintArgs {
    /// Target path to lint.
    #[arg(default_value = ".")]
    pub target: PathBuf,

    /// Config file to load.
    #[arg(short, long, default_value = "lintropy.toml")]
    pub config: PathBuf,
}

#[derive(Debug, Args)]
pub struct InitConfigArgs {
    /// Output path for the generated config.
    #[arg(short, long, default_value = "lintropy.toml")]
    pub output: PathBuf,

    /// Overwrite an existing config file.
    #[arg(long)]
    pub force: bool,
}
