use std::path::{Path, PathBuf};

use insta::assert_json_snapshot;
use lintropy_core::{
    config::{Config, QueryRule, RuleConfig, RuleKind, Settings},
    engine,
    RuleId, Severity,
};
use lintropy_langs::Language;
use serde_json::Value;
use tree_sitter::Query;

fn fixture_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/engine")
        .join(name)
}

fn query_rule(language: Language, query_src: &str) -> QueryRule {
    let ts_language = language.ts_language();
    let query = Query::new(&ts_language, query_src).expect("query");
    QueryRule::new(query_src, query).unwrap()
}

fn rule(
    id: &str,
    severity: Severity,
    message: &str,
    fix: Option<&str>,
    query_src: &str,
) -> RuleConfig {
    RuleConfig {
        id: RuleId::new(id),
        severity,
        message: message.to_string(),
        include: Vec::new(),
        exclude: Vec::new(),
        tags: Vec::new(),
        docs_url: None,
        language: Some(Language::Rust),
        kind: RuleKind::Query(query_rule(Language::Rust, query_src)),
        fix: fix.map(str::to_string),
        source_path: PathBuf::from(format!(".lintropy/{id}.rule.yaml")),
    }
}

#[test]
fn snapshots_canonical_rules() {
    let config = Config {
        version: 1,
        settings: Settings::default(),
        rules: vec![
            rule(
                "no-unwrap",
                Severity::Warning,
                "avoid .unwrap() on `{{recv}}`",
                Some(r#"{{recv}}.expect("TODO: handle error")"#),
                r#"
                (call_expression
                  function: (field_expression
                    value: (_) @recv
                    field: (field_identifier) @method)
                  (#eq? @method "unwrap")) @match
                "#,
            ),
            rule(
                "no-println",
                Severity::Info,
                "avoid println! in committed code",
                None,
                r#"
                (macro_invocation
                  macro: (identifier) @name
                  (#eq? @name "println")) @match
                "#,
            ),
            rule(
                "no-todo",
                Severity::Warning,
                "remove TODO comment before merge",
                None,
                r#"
                ((line_comment) @match
                  (#match? @match "TODO"))
                "#,
            ),
        ],
        warnings: Vec::new(),
        root_dir: PathBuf::new(),
        root_config: PathBuf::from("lintropy.yaml"),
    };

    let files = vec![fixture_path("sample.rs")];
    let diagnostics = engine::run(&config, &files).unwrap();
    assert_json_snapshot!("engine_canonical_rules", normalize_paths(diagnostics));
}

#[test]
fn respects_include_and_exclude_globs() {
    let mut only_src = rule(
        "no-println",
        Severity::Info,
        "avoid println! in committed code",
        None,
        r#"
        (macro_invocation
          macro: (identifier) @name
          (#eq? @name "println")) @match
        "#,
    );
    only_src.include = vec!["**/fixtures/engine/*.rs".into()];
    only_src.exclude = vec!["**/skip.rs".into()];

    let config = Config {
        version: 1,
        settings: Settings::default(),
        rules: vec![only_src],
        warnings: Vec::new(),
        root_dir: PathBuf::new(),
        root_config: PathBuf::from("lintropy.yaml"),
    };

    let files = vec![fixture_path("sample.rs"), fixture_path("skip.rs")];
    let diagnostics = engine::run(&config, &files).unwrap();
    assert_eq!(diagnostics.len(), 1);
    assert!(diagnostics[0].file.ends_with("sample.rs"));
}

fn normalize_paths<T: serde::Serialize>(value: T) -> Value {
    let mut json = serde_json::to_value(value).unwrap();
    let Value::Array(items) = &mut json else {
        return json;
    };

    for item in items {
        if let Some(file) = item.get_mut("file") {
            *file = Value::String("[file]".into());
        }
    }

    json
}
