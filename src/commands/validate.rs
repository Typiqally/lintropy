//! `lintropy config validate [path]` — load config, report rule count.

use crate::cli::{ConfigCommand, ConfigValidateArgs};
use crate::commands::{load_config, print_warnings};
use crate::exit::{CliError, EXIT_OK};

pub fn run(cmd: ConfigCommand) -> Result<u8, CliError> {
    match cmd {
        ConfigCommand::Validate(args) => validate(args),
    }
}

fn validate(args: ConfigValidateArgs) -> Result<u8, CliError> {
    let config = load_config(args.path.as_deref())?;
    print_warnings(&config);
    println!(
        "OK: {} {} loaded from {}",
        config.rules.len(),
        if config.rules.len() == 1 {
            "rule"
        } else {
            "rules"
        },
        config.root_dir.display()
    );
    Ok(EXIT_OK)
}
