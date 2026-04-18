//! Config loader: turn `lintropy.yaml` plus `.lintropy/**/*.{rule,rules}.yaml`
//! into a validated [`Config`] value.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use lintropy_langs::Language;
use schemars::JsonSchema;
use serde::Deserialize;
use tree_sitter::Query as TsQuery;

use crate::discovery::{self, Discovered};
use crate::predicates::{self, CustomPredicate};
use crate::{LintropyError, Result, RuleId, Severity};

const DEFAULT_SEVERITY: Severity = Severity::Error;
const DEFAULT_FAIL_ON: Severity = Severity::Error;
const SUPPORTED_VERSION: u32 = 1;

#[derive(Debug)]
pub struct Config {
    pub version: u32,
    pub settings: Settings,
    pub rules: Vec<RuleConfig>,
    pub warnings: Vec<ConfigWarning>,
    pub root_dir: PathBuf,
    pub root_config: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Settings {
    pub fail_on: Severity,
    pub default_severity: Severity,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            fail_on: DEFAULT_FAIL_ON,
            default_severity: DEFAULT_SEVERITY,
        }
    }
}

#[derive(Debug)]
pub struct RuleConfig {
    pub id: RuleId,
    pub severity: Severity,
    pub message: String,
    pub include: Vec<String>,
    pub exclude: Vec<String>,
    pub tags: Vec<String>,
    pub docs_url: Option<String>,
    pub language: Option<Language>,
    pub kind: RuleKind,
    pub fix: Option<String>,
    pub source_path: PathBuf,
}

#[derive(Debug)]
pub enum RuleKind {
    Query(QueryRule),
    Match(MatchRule),
}

#[derive(Debug)]
pub struct QueryRule {
    pub source: String,
    pub compiled: Arc<TsQuery>,
    pub predicates_by_pattern: Vec<Vec<CustomPredicate>>,
}

impl QueryRule {
    pub fn new(source: impl Into<String>, query: TsQuery) -> Result<Self> {
        let predicates_by_pattern = predicates::parse_general_predicates_by_pattern(&query)?;
        Ok(Self {
            source: source.into(),
            compiled: Arc::new(query),
            predicates_by_pattern,
        })
    }
}

#[derive(Debug, Clone)]
pub struct MatchRule {
    pub forbid: Option<String>,
    pub require: Option<String>,
    pub multiline: bool,
}

#[derive(Debug, Clone)]
pub struct ConfigWarning {
    pub rule_id: Option<RuleId>,
    pub source_path: PathBuf,
    pub message: String,
}

impl RuleConfig {
    pub fn query_rule(&self) -> Option<&QueryRule> {
        match &self.kind {
            RuleKind::Query(rule) => Some(rule),
            RuleKind::Match(_) => None,
        }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct RawRoot {
    /// Schema version for the root `lintropy.yaml` file.
    version: u32,
    /// Global defaults and exit-code policy.
    #[serde(default)]
    settings: Option<RawSettings>,
    /// Inline rules authored directly in `lintropy.yaml`.
    #[serde(default)]
    rules: Option<Vec<RawRule>>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct RawSettings {
    /// Highest severity that should cause a non-zero exit code.
    #[serde(default)]
    fail_on: Option<Severity>,
    /// Severity applied when a rule omits `severity`.
    #[serde(default)]
    default_severity: Option<Severity>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct RawRule {
    /// Stable identifier for the rule. Optional in `*.rule.yaml`, required elsewhere.
    #[serde(default)]
    id: Option<String>,
    /// Diagnostic severity emitted when the rule matches.
    #[serde(default)]
    severity: Option<Severity>,
    /// User-facing diagnostic text, with `{{capture}}` interpolation for query rules.
    message: String,
    /// Inclusive gitignore-style globs restricting where the rule applies.
    #[serde(default)]
    include: Vec<String>,
    /// Exclusive gitignore-style globs removing files from the rule scope.
    #[serde(default)]
    exclude: Vec<String>,
    /// Free-form labels for grouping and filtering.
    #[serde(default)]
    tags: Vec<String>,
    /// Optional URL with docs or remediation guidance for the rule.
    #[serde(default)]
    docs_url: Option<String>,
    /// Tree-sitter language used to compile a `query` rule.
    #[serde(default)]
    language: Option<String>,
    /// Tree-sitter query source for structural matching.
    #[serde(default)]
    query: Option<String>,
    /// Regex that raises a diagnostic for each match. Phase 2 runtime support.
    #[serde(default)]
    forbid: Option<String>,
    /// Regex that raises a diagnostic when absent from a file. Phase 2 runtime support.
    #[serde(default)]
    require: Option<String>,
    /// Enables multiline / dotall regex behavior for match rules.
    #[serde(default)]
    #[allow(dead_code)]
    multiline: Option<bool>,
    /// Replacement text applied to the `@match` span for query-rule autofix.
    #[serde(default)]
    fix: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct RawRulesFile {
    /// Rule list contained in a grouped `.rules.yaml` file.
    rules: Vec<RawRule>,
}

impl Config {
    pub fn load_from_root(start: &Path) -> Result<Config> {
        let discovered = discovery::discover_from(start)?;
        Self::from_discovered(discovered)
    }

    pub fn load_from_path(config_path: &Path) -> Result<Config> {
        let root_config = config_path
            .canonicalize()
            .map_err(|e| LintropyError::ConfigLoad(format!("{}: {e}", config_path.display())))?;
        if !root_config.is_file() {
            return Err(LintropyError::ConfigLoad(format!(
                "{} is not a file",
                root_config.display()
            )));
        }
        let root_dir = root_config
            .parent()
            .ok_or_else(|| LintropyError::ConfigLoad("config path has no parent".into()))?
            .to_path_buf();
        let rule_files = discovery::enumerate_rule_files(&root_dir)?;
        Self::from_discovered(Discovered {
            root_config,
            root_dir,
            rule_files,
        })
    }

    pub fn json_schema() -> serde_json::Value {
        Self::root_json_schema()
    }

    pub fn root_json_schema() -> serde_json::Value {
        let schema = schemars::schema_for!(RawRoot);
        serde_json::to_value(&schema).expect("schemars root schema is JSON-serializable")
    }

    pub fn rule_json_schema() -> serde_json::Value {
        let schema = schemars::schema_for!(RawRule);
        serde_json::to_value(&schema).expect("schemars rule schema is JSON-serializable")
    }

    pub fn rules_file_json_schema() -> serde_json::Value {
        let schema = schemars::schema_for!(RawRulesFile);
        serde_json::to_value(&schema).expect("schemars rules-file schema is JSON-serializable")
    }

    fn from_discovered(disc: Discovered) -> Result<Config> {
        let raw_root = parse_root_yaml(&disc.root_config)?;
        if raw_root.version != SUPPORTED_VERSION {
            return Err(LintropyError::ConfigLoad(format!(
                "{}: unsupported version {} (only version {} is supported)",
                disc.root_config.display(),
                raw_root.version,
                SUPPORTED_VERSION,
            )));
        }

        let settings = resolve_settings(raw_root.settings.as_ref());
        let raw_rules = collect_raw_rules(&disc, raw_root.rules)?;

        let mut rules = Vec::with_capacity(raw_rules.len());
        let mut warnings = Vec::new();
        let mut seen: HashMap<String, PathBuf> = HashMap::new();

        for (raw, source_path, stem_default) in raw_rules {
            let built = build_rule(
                raw,
                source_path.clone(),
                stem_default,
                &settings,
                &mut warnings,
            )?;
            if let Some(prev) =
                seen.insert(built.id.as_str().to_string(), built.source_path.clone())
            {
                return Err(LintropyError::DuplicateRuleId {
                    rule_id: built.id.as_str().to_string(),
                    first: prev,
                    second: built.source_path,
                });
            }
            rules.push(built);
        }

        Ok(Config {
            version: raw_root.version,
            settings,
            rules,
            warnings,
            root_dir: disc.root_dir,
            root_config: disc.root_config,
        })
    }
}

type RawRuleEntry = (RawRule, PathBuf, Option<String>);

fn parse_root_yaml(path: &Path) -> Result<RawRoot> {
    let text = read_file(path)?;
    serde_yaml::from_str(&text).map_err(|e| yaml_err(path, e))
}

fn collect_raw_rules(disc: &Discovered, inline: Option<Vec<RawRule>>) -> Result<Vec<RawRuleEntry>> {
    let mut out: Vec<RawRuleEntry> = Vec::new();

    if let Some(inline) = inline {
        for r in inline {
            out.push((r, disc.root_config.clone(), None));
        }
    }

    for path in &disc.rule_files {
        let text = read_file(path)?;
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if name.ends_with(".rules.yaml") {
            let file: RawRulesFile = serde_yaml::from_str(&text).map_err(|e| yaml_err(path, e))?;
            for r in file.rules {
                out.push((r, path.clone(), None));
            }
        } else {
            let file: RawRule = serde_yaml::from_str(&text).map_err(|e| yaml_err(path, e))?;
            out.push((file, path.clone(), Some(file_stem_for_rule_yaml(path))));
        }
    }

    Ok(out)
}

fn build_rule(
    raw: RawRule,
    source_path: PathBuf,
    stem_default: Option<String>,
    settings: &Settings,
    warnings: &mut Vec<ConfigWarning>,
) -> Result<RuleConfig> {
    let id = resolve_id(&raw, &source_path, stem_default)?;
    let rule_id = RuleId::new(id.clone());

    let has_query = raw.query.is_some();
    let has_forbid = raw.forbid.is_some();
    let has_require = raw.require.is_some();
    let has_match = has_forbid || has_require;

    if has_query && has_match {
        return Err(LintropyError::ConfigLoad(format!(
            "{}: rule `{}` sets both `query` and `forbid`/`require` (exactly one kind per rule)",
            source_path.display(),
            id
        )));
    }
    if !has_query && !has_match {
        return Err(LintropyError::ConfigLoad(format!(
            "{}: rule `{}` needs one of `query`, `forbid`, or `require`",
            source_path.display(),
            id
        )));
    }

    let language = match raw.language.as_deref() {
        Some(name) => Some(Language::from_name(name).ok_or_else(|| {
            LintropyError::ConfigLoad(format!(
                "{}: rule `{}` uses unknown language `{}`",
                source_path.display(),
                id,
                name
            ))
        })?),
        None => None,
    };

    let kind = if has_query {
        build_query_kind(&raw, language, &id, &rule_id, &source_path, warnings)?
    } else {
        return Err(LintropyError::Unsupported(format!(
            "{}: rule `{}` uses regex `forbid`/`require`; match rules are Phase 2",
            source_path.display(),
            id
        )));
    };

    let severity = raw.severity.unwrap_or(settings.default_severity);

    Ok(RuleConfig {
        id: rule_id,
        severity,
        message: raw.message,
        include: raw.include,
        exclude: raw.exclude,
        tags: raw.tags,
        docs_url: raw.docs_url,
        language,
        kind,
        fix: raw.fix,
        source_path,
    })
}

fn build_query_kind(
    raw: &RawRule,
    language: Option<Language>,
    id: &str,
    rule_id: &RuleId,
    source_path: &Path,
    warnings: &mut Vec<ConfigWarning>,
) -> Result<RuleKind> {
    let language = language.ok_or_else(|| {
        LintropyError::ConfigLoad(format!(
            "{}: query rule `{}` is missing required `language`",
            source_path.display(),
            id
        ))
    })?;
    let query_source = raw.query.clone().expect("caller guarantees query rule");
    let compiled = TsQuery::new(&language.ts_language(), &query_source).map_err(|e| {
        LintropyError::QueryCompile {
            rule_id: id.to_string(),
            source_path: source_path.to_path_buf(),
            message: format!("{e}"),
        }
    })?;

    let predicates_by_pattern = predicates::parse_general_predicates_by_pattern(&compiled)
        .map_err(|e| contextualize_predicate_err(e, id, source_path))?;

    let capture_names: Vec<&str> = compiled.capture_names().to_vec();
    let captures: HashSet<&str> = capture_names.iter().copied().collect();

    validate_template(&raw.message, &captures, id, source_path)?;
    if let Some(fix) = raw.fix.as_deref() {
        validate_template(fix, &captures, id, source_path)?;
    }

    if !captures.contains("match") {
        warnings.push(ConfigWarning {
            rule_id: Some(rule_id.clone()),
            source_path: source_path.to_path_buf(),
            message: format!(
                "rule `{id}` has no `@match` capture; diagnostic span will use the query root"
            ),
        });
    }

    Ok(RuleKind::Query(QueryRule {
        source: query_source,
        compiled: Arc::new(compiled),
        predicates_by_pattern,
    }))
}

fn validate_template(
    template: &str,
    captures: &HashSet<&str>,
    rule_id: &str,
    source_path: &Path,
) -> Result<()> {
    for token in extract_capture_tokens(template) {
        if !captures.contains(token.as_str()) {
            return Err(LintropyError::UnknownCapture {
                rule_id: rule_id.to_string(),
                source_path: source_path.to_path_buf(),
                capture: token,
            });
        }
    }
    Ok(())
}

fn extract_capture_tokens(s: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let bytes = s.as_bytes();
    let mut i = 0usize;
    while i + 1 < bytes.len() {
        if bytes[i] == b'{' && bytes[i + 1] == b'{' {
            let rest_start = i + 2;
            if let Some(end) = s[rest_start..].find("}}") {
                let token = s[rest_start..rest_start + end].trim().to_string();
                if !token.is_empty() {
                    tokens.push(token);
                }
                i = rest_start + end + 2;
                continue;
            }
        }
        i += 1;
    }
    tokens
}

fn resolve_id(raw: &RawRule, source_path: &Path, stem_default: Option<String>) -> Result<String> {
    match (raw.id.as_ref(), stem_default) {
        (Some(id), _) => Ok(id.clone()),
        (None, Some(stem)) => Ok(stem),
        (None, None) => Err(LintropyError::ConfigLoad(format!(
            "{}: rule missing required `id` field",
            source_path.display()
        ))),
    }
}

fn resolve_settings(raw: Option<&RawSettings>) -> Settings {
    let Some(raw) = raw else {
        return Settings::default();
    };
    Settings {
        fail_on: raw.fail_on.unwrap_or(DEFAULT_FAIL_ON),
        default_severity: raw.default_severity.unwrap_or(DEFAULT_SEVERITY),
    }
}

fn contextualize_predicate_err(err: LintropyError, id: &str, source_path: &Path) -> LintropyError {
    match err {
        LintropyError::UnknownPredicate { predicate, .. } => LintropyError::UnknownPredicate {
            rule_id: id.to_string(),
            source_path: source_path.to_path_buf(),
            predicate,
        },
        other => other,
    }
}

fn file_stem_for_rule_yaml(path: &Path) -> String {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    name.strip_suffix(".rule.yaml")
        .map(str::to_owned)
        .unwrap_or_else(|| name.to_owned())
}

fn read_file(path: &Path) -> Result<String> {
    std::fs::read_to_string(path).map_err(LintropyError::Io)
}

fn yaml_err(path: &Path, err: serde_yaml::Error) -> LintropyError {
    LintropyError::Yaml(format!("{}: {err}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_capture_tokens_finds_named_captures() {
        let tokens = extract_capture_tokens("avoid .unwrap() on `{{recv}}` (method `{{method}}`)");
        assert_eq!(tokens, vec!["recv".to_string(), "method".to_string()]);
    }

    #[test]
    fn extract_capture_tokens_trims_whitespace() {
        let tokens = extract_capture_tokens("hello {{ name }} world");
        assert_eq!(tokens, vec!["name".to_string()]);
    }

    #[test]
    fn extract_capture_tokens_ignores_single_braces() {
        let tokens = extract_capture_tokens("{not} a {capture} at {all}");
        assert!(tokens.is_empty());
    }

    #[test]
    fn file_stem_strips_rule_yaml_suffix() {
        assert_eq!(
            file_stem_for_rule_yaml(Path::new(".lintropy/no-unwrap.rule.yaml")),
            "no-unwrap"
        );
    }

    #[test]
    fn default_settings_match_spec() {
        let settings = Settings::default();
        assert_eq!(settings.fail_on, Severity::Error);
        assert_eq!(settings.default_severity, Severity::Error);
    }

    #[test]
    fn json_schema_covers_root_shape() {
        let schema = Config::json_schema();
        let object = schema.as_object().expect("schema is an object");
        assert!(object.contains_key("$schema"));
        assert!(object.contains_key("properties"));
    }

    #[test]
    fn rule_json_schema_covers_rule_shape() {
        let schema = Config::rule_json_schema();
        let object = schema.as_object().expect("schema is an object");
        assert!(object.contains_key("$schema"));
        assert!(object.contains_key("properties"));
    }

    #[test]
    fn rules_file_json_schema_covers_rules_shape() {
        let schema = Config::rules_file_json_schema();
        let object = schema.as_object().expect("schema is an object");
        assert!(object.contains_key("$schema"));
        assert!(object.contains_key("properties"));
    }
}
