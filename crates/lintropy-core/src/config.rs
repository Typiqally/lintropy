//! Config loader: turn `lintropy.yaml` plus `.lintropy/**/*.{rule,rules}.yaml`
//! into a validated [`Config`] value.
//!
//! The flow is:
//!
//! 1. [`discovery::discover_from`] locates the anchoring `lintropy.yaml`
//!    and enumerates every rule file under `.lintropy/`.
//! 2. YAML is deserialised into crate-private raw types that mirror the
//!    on-disk shape (§4.2–§4.5 of the merged spec).
//! 3. Every rule is validated at load time: query compilation, capture-name
//!    references in `message`/`fix`, duplicate ids, custom predicate names
//!    (via the WP2 seam in [`crate::predicates`]).
//!
//! Public entry points mirror the WP1 contract:
//! [`Config::load_from_root`], [`Config::load_from_path`], and
//! [`Config::json_schema`].

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

// ─────────────────────────────── runtime types ───────────────────────────────

/// A fully loaded + validated lintropy config.
#[derive(Debug)]
pub struct Config {
    /// `version:` from `lintropy.yaml`. Currently always `1`.
    pub version: u32,
    /// Project-wide settings (merged with defaults).
    pub settings: Settings,
    /// Every rule merged across inline + `.lintropy/` files, deduplicated by id.
    pub rules: Vec<RuleConfig>,
    /// Non-fatal issues surfaced during load (e.g. a query rule without `@match`).
    pub warnings: Vec<ConfigWarning>,
    /// Root directory (parent of the anchoring `lintropy.yaml`).
    pub root_dir: PathBuf,
    /// Path to the anchoring `lintropy.yaml`.
    pub root_config: PathBuf,
}

/// Project-wide settings from the root `lintropy.yaml`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Settings {
    /// Exit non-zero when a diagnostic at this severity (or higher) fires.
    pub fail_on: Severity,
    /// Default severity for rules that omit `severity:`.
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

/// One rule after merging, validation, and query compilation.
#[derive(Debug)]
pub struct RuleConfig {
    /// User-visible identifier.
    pub id: RuleId,
    /// Resolved severity (falls back to `settings.default_severity`).
    pub severity: Severity,
    /// Human-facing message template, pre-interpolation.
    pub message: String,
    /// Optional inclusive gitignore-style globs.
    pub include: Vec<String>,
    /// Optional exclusive gitignore-style globs.
    pub exclude: Vec<String>,
    /// Free-form tags for filtering / grouping.
    pub tags: Vec<String>,
    /// External documentation URL surfaced in diagnostics.
    pub docs_url: Option<String>,
    /// Language handle; required for query rules, optional otherwise.
    pub language: Option<Language>,
    /// Discriminated rule body.
    pub kind: RuleKind,
    /// Replacement template for autofix; query rules only.
    pub fix: Option<String>,
    /// YAML file that defined this rule.
    pub source_path: PathBuf,
}

/// Rule kind discriminated on key presence (§4.5 of the spec).
#[derive(Debug)]
pub enum RuleKind {
    /// Tree-sitter query rule — Phase 1.
    Query(QueryRule),
    /// Regex match rule — Phase 2 (Phase 1 rejects these at load).
    Match(MatchRule),
}

/// Compiled query body attached to a query rule.
#[derive(Debug)]
pub struct QueryRule {
    /// Original S-expression source, for `lintropy explain`.
    pub source: String,
    /// Compiled query. Shared via `Arc` so downstream crates can hold a
    /// handle without recompiling.
    pub compiled: Arc<TsQuery>,
    /// Parsed custom predicates (see [`crate::predicates`]).
    pub predicates: Vec<CustomPredicate>,
}

/// Raw regex body for a Phase 2 match rule. Unused in Phase 1 — kept so
/// Phase 2 can land without a breaking API change.
#[derive(Debug, Clone)]
pub struct MatchRule {
    /// Regex that, when matched, produces one diagnostic per match.
    pub forbid: Option<String>,
    /// Regex that must appear in the file; absence produces one diagnostic.
    pub require: Option<String>,
    /// Whether to enable multiline + dotall flags.
    pub multiline: bool,
}

/// Non-fatal issue discovered at config load.
#[derive(Debug, Clone)]
pub struct ConfigWarning {
    /// Rule the warning is attached to, if any.
    pub rule_id: Option<RuleId>,
    /// File that the warning originated from.
    pub source_path: PathBuf,
    /// Human-facing warning text.
    pub message: String,
}

// ──────────────────────────── raw YAML deserialisation ───────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct RawRoot {
    /// Config format version (currently always `1`).
    version: u32,
    /// Optional project-wide settings.
    #[serde(default)]
    settings: Option<RawSettings>,
    /// Optional inline rules — see §4.2.
    #[serde(default)]
    rules: Option<Vec<RawRule>>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct RawSettings {
    /// Exit non-zero when a diagnostic at this severity (or higher) fires.
    #[serde(default)]
    fail_on: Option<Severity>,
    /// Default severity for rules that omit `severity:`.
    #[serde(default)]
    default_severity: Option<Severity>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct RawRule {
    /// User label; defaults to the file stem in `*.rule.yaml`.
    #[serde(default)]
    id: Option<String>,
    /// Optional severity override; falls back to `settings.default_severity`.
    #[serde(default)]
    severity: Option<Severity>,
    /// Human-facing message template; `{{capture}}` interpolation.
    message: String,
    /// Inclusive gitignore-style globs.
    #[serde(default)]
    include: Vec<String>,
    /// Exclusive gitignore-style globs.
    #[serde(default)]
    exclude: Vec<String>,
    /// Free-form tags for filtering.
    #[serde(default)]
    tags: Vec<String>,
    /// Documentation URL surfaced in diagnostics.
    #[serde(default)]
    docs_url: Option<String>,
    /// Language name; required when `query` is set.
    #[serde(default)]
    language: Option<String>,
    /// Tree-sitter S-expression query (query-rule discriminator).
    #[serde(default)]
    query: Option<String>,
    /// Regex: match = violation (match-rule discriminator, Phase 2).
    #[serde(default)]
    forbid: Option<String>,
    /// Regex: absence = violation (match-rule discriminator, Phase 2).
    #[serde(default)]
    require: Option<String>,
    /// Enable regex multiline / dotall flags.
    #[serde(default)]
    #[allow(dead_code)]
    // match rules are Phase 2 (§13.2); keep field so deser stays compatible.
    multiline: Option<bool>,
    /// Replacement template for autofix; query rules only.
    #[serde(default)]
    fix: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct RawRulesFile {
    /// One or more rule stanzas keyed by required `id`.
    rules: Vec<RawRule>,
}

// ──────────────────────────────── public API ────────────────────────────────

impl Config {
    /// Walk up from `start`, load the project's config, and validate every rule.
    pub fn load_from_root(start: &Path) -> Result<Config> {
        let discovered = discovery::discover_from(start)?;
        Self::from_discovered(discovered)
    }

    /// Load a single `lintropy.yaml` by explicit path (`--config` override).
    ///
    /// Rule files under the config's adjacent `.lintropy/` directory are
    /// still discovered.
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

    /// Emit the JSON Schema for a root `lintropy.yaml` file.
    ///
    /// This is a thin wrapper over [`schemars::schema_for!`] on the raw YAML
    /// shape. The schema covers the root object, settings, and every
    /// per-rule field. Rule-stanza discrimination on key presence
    /// (`query` vs `forbid`/`require`) is described inline in field docs
    /// rather than encoded as a JSON Schema `oneOf` — see the hand-off
    /// notes for the Phase 2 upgrade plan.
    pub fn json_schema() -> serde_json::Value {
        let schema = schemars::schema_for!(RawRoot);
        serde_json::to_value(&schema).expect("schemars root schema is JSON-serializable")
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

// ──────────────────────────────── internals ─────────────────────────────────

/// Raw rule pulled from disk plus its provenance: (rule, source_path, stem_default).
///
/// `stem_default` is `Some` only for `.rule.yaml` files (single-rule form); its
/// value becomes the rule's id when the stanza omits `id:` explicitly.
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
    let query_source = raw
        .query
        .clone()
        .expect("caller guarantees `raw.query` is Some");
    let ts_lang = language.ts_language();

    let compiled =
        TsQuery::new(&ts_lang, &query_source).map_err(|e| LintropyError::QueryCompile {
            rule_id: id.to_string(),
            source_path: source_path.to_path_buf(),
            message: format!("{e}"),
        })?;

    let predicates_vec = predicates::parse_general_predicates(&compiled)
        .map_err(|e| contextualise_predicate_err(e, id, source_path))?;

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
        predicates: predicates_vec,
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
                let raw = &s[rest_start..rest_start + end];
                let token = raw.trim().to_string();
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

fn contextualise_predicate_err(e: LintropyError, id: &str, source_path: &Path) -> LintropyError {
    match e {
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

fn yaml_err(path: &Path, e: serde_yaml::Error) -> LintropyError {
    LintropyError::Yaml(format!("{}: {e}", path.display()))
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
        let s = Settings::default();
        assert_eq!(s.fail_on, Severity::Error);
        assert_eq!(s.default_severity, Severity::Error);
    }

    #[test]
    fn json_schema_covers_root_shape() {
        let schema = Config::json_schema();
        let as_obj = schema.as_object().expect("schema is an object");
        assert!(as_obj.contains_key("$schema"));
        assert!(as_obj.contains_key("properties"));
    }
}
