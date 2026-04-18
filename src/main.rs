mod cli;
mod config;
mod engine;

use anyhow::Result;
use clap::Parser;

use crate::cli::{Cli, Commands};

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Lint(args) => engine::run_lint(args),
        Commands::InitConfig(args) => engine::init_config(args),
    }
}
