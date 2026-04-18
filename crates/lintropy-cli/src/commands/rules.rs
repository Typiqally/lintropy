//! `lintropy rules` — list every loaded rule.

use lintropy_core::{Config, RuleConfig, RuleKind, Severity};
use serde_json::json;

use crate::cli::{GroupBy, OutputFormat, RulesArgs};
use crate::commands::{load_config, print_warnings};
use crate::exit::{CliError, EXIT_OK};

const WRAP_WIDTH: usize = 100;
const INDENT: &str = "  ";

pub fn run(args: RulesArgs) -> Result<u8, CliError> {
    let config = load_config(args.config.as_deref())?;
    print_warnings(&config);

    match args.format {
        OutputFormat::Text => {
            print_text(&config, args.group_by);
            Ok(EXIT_OK)
        }
        OutputFormat::Json => {
            if !matches!(args.group_by, GroupBy::None) {
                return Err(CliError::user(
                    "--group-by only applies to text format",
                ));
            }
            print_json(&config)?;
            Ok(EXIT_OK)
        }
    }
}

fn print_text(config: &Config, group_by: GroupBy) {
    let mut rules: Vec<&RuleConfig> = config.rules.iter().collect();
    rules.sort_by(|a, b| a.id.as_str().cmp(b.id.as_str()));

    let root = config.root_dir.as_path();

    match group_by {
        GroupBy::None => {
            print_rule_block(&rules, root);
        }
        GroupBy::Language => {
            let groups = group_by_language(&rules);
            for (label, group) in groups {
                print_group_header(&label);
                print_rule_block(&group, root);
                println!();
            }
        }
        GroupBy::Tag => {
            let groups = group_by_first_tag(&rules);
            for (label, group) in groups {
                print_group_header(&label);
                print_rule_block(&group, root);
                println!();
            }
        }
    }
}

fn print_group_header(label: &str) {
    println!("{label}");
    println!("{}", "-".repeat(label.chars().count()));
}

fn print_rule_block(rules: &[&RuleConfig], root: &std::path::Path) {
    if rules.is_empty() {
        return;
    }
    let id_width = rules.iter().map(|r| r.id.as_str().len()).max().unwrap_or(0);
    let sev_width = rules
        .iter()
        .map(|r| severity_label(r.severity).len() + 2)
        .max()
        .unwrap_or(0);

    for (i, rule) in rules.iter().enumerate() {
        if i > 0 {
            println!();
        }
        let sev = format!("[{}]", severity_label(rule.severity));
        let lang = rule.language.map(|l| l.name()).unwrap_or("");
        println!(
            "{:id_w$}  {:sev_w$}  {}",
            rule.id.as_str(),
            sev,
            lang,
            id_w = id_width,
            sev_w = sev_width
        );
        if let Some(desc) = &rule.description {
            for line in wrap_description(desc) {
                println!("{INDENT}{line}");
            }
        }
        if !rule.tags.is_empty() {
            println!("{INDENT}tags: {}", rule.tags.join(", "));
        }
        if let Some(url) = &rule.docs_url {
            println!("{INDENT}docs: {url}");
        }
        let rel = rule
            .source_path
            .strip_prefix(root)
            .unwrap_or(rule.source_path.as_path());
        println!("{INDENT}source: {}", rel.display());
    }
}

fn group_by_language<'a>(rules: &[&'a RuleConfig]) -> Vec<(String, Vec<&'a RuleConfig>)> {
    use std::collections::BTreeMap;
    let mut named: BTreeMap<String, Vec<&'a RuleConfig>> = BTreeMap::new();
    let mut anon: Vec<&'a RuleConfig> = Vec::new();
    for r in rules {
        match r.language {
            Some(lang) => named.entry(lang.name().to_string()).or_default().push(r),
            None => anon.push(*r),
        }
    }
    let mut out: Vec<(String, Vec<&'a RuleConfig>)> = named.into_iter().collect();
    if !anon.is_empty() {
        out.push(("(any)".to_string(), anon));
    }
    out
}

fn group_by_first_tag<'a>(rules: &[&'a RuleConfig]) -> Vec<(String, Vec<&'a RuleConfig>)> {
    use std::collections::BTreeMap;
    let mut named: BTreeMap<String, Vec<&'a RuleConfig>> = BTreeMap::new();
    let mut untagged: Vec<&'a RuleConfig> = Vec::new();
    for r in rules {
        match r.tags.first() {
            Some(t) => named.entry(t.clone()).or_default().push(r),
            None => untagged.push(*r),
        }
    }
    let mut out: Vec<(String, Vec<&'a RuleConfig>)> = named.into_iter().collect();
    if !untagged.is_empty() {
        out.push(("(untagged)".to_string(), untagged));
    }
    out
}

fn wrap_description(text: &str) -> Vec<String> {
    // Hard newlines preserved as paragraph breaks; soft-wrap on whitespace
    // at WRAP_WIDTH within each line.
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

fn print_json(config: &Config) -> Result<(), CliError> {
    let array: Vec<_> = config.rules.iter().map(rule_to_json).collect();
    let json = serde_json::to_string_pretty(&array)
        .map_err(|err| CliError::internal(format!("json: {err}")))?;
    println!("{json}");
    Ok(())
}

fn rule_to_json(rule: &RuleConfig) -> serde_json::Value {
    let kind = match &rule.kind {
        RuleKind::Query(_) => "query",
        RuleKind::Match(_) => "match",
    };
    json!({
        "id": rule.id.as_str(),
        "severity": severity_label(rule.severity),
        "language": rule.language.map(|l| l.name()),
        "kind": kind,
        "description": rule.description,
        "source_path": rule.source_path.display().to_string(),
        "tags": rule.tags,
        "docs_url": rule.docs_url,
        "include": rule.include,
        "exclude": rule.exclude,
    })
}

fn severity_label(severity: Severity) -> &'static str {
    match severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
        Severity::Info => "info",
    }
}
