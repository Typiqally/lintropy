//! lintropy CLI entry point.
//!
//! Dispatches every §9 subcommand. Exit codes follow
//! §7.6 of the merged spec:
//!
//! * `0` — no diagnostics at or above `settings.fail_on`
//! * `1` — diagnostics present at or above `fail_on`
//! * `2` — config load / schema / parse failure, or user-facing invalid input
//! * `3` — internal error (caught panic, invariant violation)

use std::process::ExitCode;

use clap::Parser;

use lintropy::cli::{self, Cli, Command};
use lintropy::commands;
use lintropy::exit::CliError;

fn main() -> ExitCode {
    let cli = Cli::parse();
    match dispatch(cli) {
        Ok(code) => ExitCode::from(code),
        Err(err) => {
            eprintln!("error: {}", err.message());
            ExitCode::from(err.exit_code())
        }
    }
}

fn dispatch(cli: Cli) -> Result<u8, CliError> {
    match cli.command {
        Some(Command::Check(args)) => commands::check::run(args),
        Some(Command::Hook(args)) => commands::hook::run(args),
        Some(Command::Explain(args)) => commands::explain::run(args),
        Some(Command::Rules(args)) => commands::rules::run(args),
        Some(Command::Init(args)) => commands::init::run(args),
        Some(Command::Schema(args)) => commands::schema::run(args),
        Some(Command::Config(args)) => commands::validate::run(args),
        Some(Command::TsParse(args)) => commands::ts_parse::run(args),
        Some(Command::InstallQueryExtension(args)) => commands::install_query_extension::run(args),
        Some(Command::InstallTextmateBundle(args)) => commands::install_textmate_bundle::run(args),
        None => commands::check::run(cli::CheckArgs::default()),
    }
}
