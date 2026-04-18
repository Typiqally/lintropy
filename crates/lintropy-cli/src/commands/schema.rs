//! `lintropy schema` — emit the config JSON schema (§10.1).

use std::fs;

use lintropy_core::schema;

use crate::cli::{SchemaArgs, SchemaKind};
use crate::exit::{CliError, EXIT_OK};

pub fn run(args: SchemaArgs) -> Result<u8, CliError> {
    let schema = match args.kind {
        SchemaKind::Root => schema::root_json_schema(),
        SchemaKind::Rule => schema::rule_json_schema(),
        SchemaKind::Rules => schema::rules_file_json_schema(),
    };
    let rendered = serde_json::to_string_pretty(&schema)
        .map_err(|err| CliError::internal(format!("schema: {err}")))?;

    if let Some(path) = args.output {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|err| CliError::user(format!("{}: {err}", parent.display())))?;
        }
        fs::write(&path, rendered)
            .map_err(|err| CliError::user(format!("{}: {err}", path.display())))?;
    } else {
        println!("{rendered}");
    }

    Ok(EXIT_OK)
}
