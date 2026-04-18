//! `lintropy explain <rule-id>` — pretty-print a single rule.

use lintropy_core::{RuleConfig, RuleKind, Severity};

use crate::cli::ExplainArgs;
use crate::commands::{load_config, print_warnings};
use crate::exit::{CliError, EXIT_OK};

pub fn run(args: ExplainArgs) -> Result<u8, CliError> {
    let config = load_config(args.config.as_deref())?;
    print_warnings(&config);

    let Some(rule) = config.rules.iter().find(|r| r.id.as_str() == args.rule_id) else {
        return Err(CliError::user(format!(
            "unknown rule id `{}`",
            args.rule_id
        )));
    };

    print_rule(rule);
    Ok(EXIT_OK)
}

fn print_rule(rule: &RuleConfig) {
    println!("rule:     {}", rule.id);
    println!("severity: {}", severity_label(rule.severity));
    if let Some(lang) = rule.language {
        println!("language: {}", lang.name());
    }
    println!("source:   {}", rule.source_path.display());
    if !rule.tags.is_empty() {
        println!("tags:     {}", rule.tags.join(", "));
    }
    if let Some(docs) = &rule.docs_url {
        println!("docs:     {docs}");
    }
    if !rule.include.is_empty() {
        println!("include:  {}", rule.include.join(", "));
    }
    if !rule.exclude.is_empty() {
        println!("exclude:  {}", rule.exclude.join(", "));
    }

    println!();
    println!("message:");
    for line in rule.message.lines() {
        println!("  {line}");
    }

    if let Some(desc) = &rule.description {
        println!();
        println!("description:");
        for line in wrap_for_terminal(desc) {
            println!("  {line}");
        }
    }

    match &rule.kind {
        RuleKind::Query(q) => {
            println!();
            println!("query:");
            for line in q.source.lines() {
                println!("  {line}");
            }
        }
        RuleKind::Match(m) => {
            println!();
            if let Some(forbid) = &m.forbid {
                println!("forbid:   {forbid}");
            }
            if let Some(require) = &m.require {
                println!("require:  {require}");
            }
            if m.multiline {
                println!("multiline: true");
            }
        }
    }

    if let Some(fix) = &rule.fix {
        println!();
        println!("fix:");
        for line in fix.lines() {
            println!("  {line}");
        }
    }
}

fn severity_label(severity: Severity) -> &'static str {
    match severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
        Severity::Info => "info",
    }
}

fn wrap_for_terminal(text: &str) -> Vec<String> {
    const WRAP_WIDTH: usize = 100;
    let mut out = Vec::new();
    for line in text.lines() {
        if line.is_empty() {
            out.push(String::new());
            continue;
        }
        let mut current = String::new();
        for word in line.split_whitespace() {
            if current.is_empty() {
                current.push_str(word);
            } else if current.len() + 1 + word.len() <= WRAP_WIDTH {
                current.push(' ');
                current.push_str(word);
            } else {
                out.push(std::mem::take(&mut current));
                current.push_str(word);
            }
        }
        if !current.is_empty() {
            out.push(current);
        }
    }
    out
}
