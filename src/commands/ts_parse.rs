//! `lintropy ts-parse <file>` — thin tree-sitter dump for agent iteration.

use std::fs;

use crate::langs::Language;
use tree_sitter::Parser;

use crate::cli::TsParseArgs;
use crate::exit::{CliError, EXIT_OK};

pub fn run(args: TsParseArgs) -> Result<u8, CliError> {
    let language = resolve_language(&args)?;
    let source = fs::read(&args.file)
        .map_err(|err| CliError::user(format!("cannot read {}: {err}", args.file.display())))?;

    let mut parser = Parser::new();
    parser
        .set_language(&language.ts_language(&args.file))
        .map_err(|err| CliError::internal(format!("set_language: {err}")))?;
    let tree = parser
        .parse(&source, None)
        .ok_or_else(|| CliError::internal(format!("parse failed for {}", args.file.display())))?;
    println!("{}", tree.root_node().to_sexp());
    Ok(EXIT_OK)
}

fn resolve_language(args: &TsParseArgs) -> Result<Language, CliError> {
    if let Some(name) = &args.lang {
        return Language::from_name(name)
            .ok_or_else(|| CliError::user(format!("unknown language `{name}`")));
    }
    let ext = args
        .file
        .extension()
        .and_then(|e| e.to_str())
        .ok_or_else(|| {
            CliError::user(format!(
                "cannot infer language from extension of {}; pass --lang",
                args.file.display()
            ))
        })?;
    Language::from_extension(ext).ok_or_else(|| {
        CliError::user(format!(
            "unknown file extension `.{ext}`; pass --lang to override"
        ))
    })
}
