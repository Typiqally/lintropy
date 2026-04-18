# Multi-language support: Go, Python, TypeScript

- Date: 2026-04-18
- Status: Approved design, ready for implementation
- Scope: Additive. No breaking changes to existing rule YAML, CLI user-facing
  contracts, or config semantics. Internal `Language::ts_language` API gains
  a `&Path` argument (internal-only, not an SDK surface).

## 1. Motivation

Lintropy's MVP registers only `rust` in `src/langs.rs`. Users writing Go,
Python, or TypeScript code get no coverage. The engine, discovery, query
compiler, fix pipeline, and diagnostic reporter are already
language-agnostic: every language-specific branch is confined to
`src/langs.rs`'s `Language` enum and its helpers. Adding a language means
one enum variant, one Cargo feature, one grammar binding, a few extension
rows, and tests — no new subsystems.

This spec registers **Go**, **Python**, and **TypeScript** as first-class
languages. One combined spec because the mechanical work is symmetric
per language; three separate specs would triple the review overhead
without improving quality. Per-lang tasks in the implementation plan
can still be skipped individually if a grammar blocker surfaces.

## 2. Non-goals

- Per-language demo projects under `examples/`. `rust-demo` stays as the
  single dogfood project; Go/Python/TypeScript demos are deferred.
- Bundled seed rules shipped inside the CLI binary. No rules are embedded
  today and none are added here.
- CLI commands beyond `ts-parse`. `check`, `explain`, `rules`, `init`,
  `hook`, `config validate`, `schema` do not change.
- Language auto-detection in `lintropy check`. Discovery walks files by
  extension already; rules declare `language:` already; nothing to add.
- Rule dialect extensions, per-language inheritance, cross-lang rule
  expansion.
- Additional language families (Java/Kotlin/C++/Ruby). Vue/Svelte/Astro
  (needs embedded-language handling — separate future spec).

## 3. Language registry

### 3.1 `Language` enum (`src/langs.rs`)

Add three variants, each gated by a Cargo feature:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Language {
    Rust,
    #[cfg(feature = "lang-go")]
    Go,
    #[cfg(feature = "lang-python")]
    Python,
    #[cfg(feature = "lang-typescript")]
    TypeScript,
}
```

`Rust` is not feature-gated — it is the baseline language and always
available, so that `cargo install lintropy --no-default-features` produces
a working Rust-only build identical to today.

### 3.2 `from_name`

Canonical YAML values:

| `language:` YAML value | Variant |
|------------------------|---------|
| `rust` | `Language::Rust` |
| `go` | `Language::Go` |
| `python` | `Language::Python` |
| `typescript` | `Language::TypeScript` |

Unknown name → `None` (existing behavior). When a feature flag is off at
build time, the corresponding `match` arm is `#[cfg]`-gated, so
`from_name("go")` on a no-features build returns `None` — the same
response as an unknown language.

There is no `tsx` alias. Rule authors write `language: typescript` for
both `.ts` and `.tsx` files; grammar selection is per-file (§3.5).

### 3.3 `from_extension`

| Extension | Variant |
|-----------|---------|
| `rs` | `Rust` |
| `go` | `Go` |
| `py`, `pyi` | `Python` |
| `ts`, `tsx`, `mts`, `cts` | `TypeScript` |
| `d.ts` (compound suffix) | `TypeScript` |

Implementation: extension resolution checks compound suffix (`d.ts`)
before single extension so `.d.ts` routes to TypeScript without
being misclassified as `.ts` + dropped `d` prefix.

Helper:

```rust
fn language_from_path(path: &Path) -> Option<Language> {
    let name = path.file_name()?.to_str()?;
    if name.ends_with(".d.ts") {
        return Language::from_extension("d.ts");
    }
    let ext = path.extension()?.to_str()?;
    Language::from_extension(ext)
}
```

`from_extension("d.ts")` is the only compound-suffix entry. All other
matches use the single trailing extension.

### 3.4 `extensions()`

Returns the same list per variant. Consumers: discovery in
`lintropy-core`, auto-detect path in `ts-parse`.

```rust
pub fn extensions(self) -> &'static [&'static str] {
    match self {
        Language::Rust => &["rs"],
        #[cfg(feature = "lang-go")]
        Language::Go => &["go"],
        #[cfg(feature = "lang-python")]
        Language::Python => &["py", "pyi"],
        #[cfg(feature = "lang-typescript")]
        Language::TypeScript => &["ts", "tsx", "mts", "cts", "d.ts"],
    }
}
```

### 3.5 Grammar dispatch — API change

Current API:

```rust
pub fn ts_language(self) -> TsLanguage
```

New API:

```rust
pub fn ts_language(self, path: &Path) -> TsLanguage
```

`path` is ignored for every variant except `TypeScript`, where:

- file ending in `.tsx` → `tree_sitter_typescript::language_tsx()`
- any other TypeScript extension (`.ts`, `.mts`, `.cts`, `.d.ts`) →
  `tree_sitter_typescript::language_typescript()`

The two TypeScript grammars are **not interchangeable**: the `typescript`
grammar parses type assertions `<T>x`; the `tsx` grammar parses JSX
`<Foo/>`. Feeding a `.tsx` file to the `typescript` grammar or vice
versa produces broken trees.

All call sites (`src/core/engine.rs`, `src/commands/ts_parse.rs`, any
other) update to pass the source file path. `ts-parse` already has a
file-path argument (`TsParseArgs.file: PathBuf`), so the path is
directly available. For `--lang typescript` combined with a non-TS
extension (user override), the `typescript` (non-tsx) grammar is
selected because the path's extension is not `.tsx`. Users who want
the tsx grammar must use a `.tsx` file; this matches §4.1 which has
no stdin support.

### 3.5.1 Query compilation for TypeScript

Tree-sitter `Query::new(language, source)` binds to a specific grammar's
symbol IDs. The `typescript` and `tsx` grammars have different symbol
tables, so a single compiled query works for only one of them.
`QueryRule` today stores `compiled: Arc<TsQuery>` — one query per rule
(`src/core/config.rs:69-81`).

For TypeScript rules, `build_rule` compiles the query source **twice**
— once against each grammar — and stores a pair:

```rust
pub struct QueryRule {
    pub source: String,
    pub compiled: Arc<TsQuery>,           // primary grammar
    pub compiled_tsx: Option<Arc<TsQuery>>, // Some(_) only for TypeScript rules
}
```

For Rust / Go / Python rules `compiled_tsx` is always `None`. For
TypeScript rules:

- If both grammars compile successfully → both stored.
- If only one compiles (e.g. a query using `jsx_element` succeeds in
  tsx but fails in typescript) → store the one that compiled. The
  rule applies only to files parsed by that grammar. Rule load
  succeeds. Non-fatal.
- If **neither** compiles → rule load fails with the original query
  compile error (both grammars' errors joined).

At parse time (`src/core/engine.rs` `run_file`), the engine picks the
query matching the file's grammar choice. Same path-based logic as
§3.5 grammar dispatch:

```rust
let query = if path_is_tsx(path) {
    rule.compiled_tsx.as_ref().unwrap_or(&rule.compiled)
} else {
    &rule.compiled
};
```

(`unwrap_or(&rule.compiled)` covers the case where tsx compilation
failed; the primary is still a typescript-grammar query, which will
not match any nodes in a tsx-parsed tree but also will not crash.)

Engine-internal bucketing: `RulesByLanguage` today has one `rust` field
(`src/core/engine.rs:35-37`). Extend to a `HashMap<Language,
Vec<ScopedRule>>` (or explicit fields per enabled language). Either is
acceptable; HashMap is cleaner for symmetry, explicit fields are
zero-overhead. Implementer picks; neither is a spec decision.

Predicate evaluation (`src/core/predicates.rs`) references
`language.ts_language()` in test helpers only; production code uses
the already-compiled `TsQuery`. Test helpers update to pass a dummy
`Path::new("t.rs")`.

### 3.6 Cargo features

`Cargo.toml`:

```toml
[features]
default = ["lang-go", "lang-python", "lang-typescript"]
lang-go = ["dep:tree-sitter-go"]
lang-python = ["dep:tree-sitter-python"]
lang-typescript = ["dep:tree-sitter-typescript"]

[dependencies]
tree-sitter-go = { version = "<pin-at-impl-time>", optional = true }
tree-sitter-python = { version = "<pin-at-impl-time>", optional = true }
tree-sitter-typescript = { version = "<pin-at-impl-time>", optional = true }
```

Version constraints: the implementer pins each grammar to the newest
release compatible with `tree-sitter = "0.22"`. If a grammar crate has
no 0.22-compatible release, the implementer escalates rather than
silently downgrading.

**Release builds** use default features — all four languages available.
**`cargo install lintropy --no-default-features`** yields a Rust-only
build with the same binary size as today. Intermediate subsets
(`--features lang-go`) also work.

Binary size impact on default build: roughly **+5-7 MB**. Documented in
the CHANGELOG entry (§7.3).

## 4. CLI changes

### 4.1 `ts-parse` auto-detect

Today: `lintropy ts-parse <file>` already supports optional `--lang`
override with extension-based auto-detect (`src/commands/ts_parse.rs`
`resolve_language`). The mechanism is sound; only the error messages
need updating to list compiled-in languages and the docs in SKILL.md
need correction.

Target resolution order (unchanged in shape, tightened in messages):

1. If `--lang <name>` is provided → use `Language::from_name(name)`; on
   failure exit code 2 with
   `unknown language: <name>. Available: rust[, go][, python][, typescript]`
   (list only compiled-in langs).
2. Else, resolve from file extension via the helper in §3.3.
3. Else → exit code 2 with
   `could not detect language for <path> (extension not recognized); pass --lang <rust|go|python|typescript>`.

`--lang` takes precedence over the extension for unusual extensions. No
stdin support today; `lintropy ts-parse` requires a file path. Stdin
is out of scope here.

Implementation in `src/commands/ts_parse.rs`:

```rust
fn resolve_language(path: Option<&Path>, override_name: Option<&str>)
    -> Result<Language, CliError>
{
    if let Some(name) = override_name {
        return Language::from_name(name)
            .ok_or_else(|| CliError::user(
                format!("unknown language: {name}. Available: {}", available())
            ));
    }
    let path = path.ok_or_else(|| CliError::user(
        "--lang is required when reading from stdin"
    ))?;
    language_from_path(path).ok_or_else(|| CliError::user(
        format!("could not detect language for {}; pass --lang <name>", path.display())
    ))
}
```

`available()` returns the comma-separated list of compiled-in language
names so error messages reflect the current build's capabilities.

### 4.2 No changes to other commands

`check`, `explain`, `rules`, `init`, `hook`, `config validate`, `schema`:
unchanged. Language handling is entirely inside the registry layer.

## 5. SKILL.md updates

Current file: `skill/SKILL.md`, 559 lines, version `0.2.0`.

### 5.1 Generic updates

- §2 commands block: `lintropy ts-parse <file> [--lang rust]` →
  `lintropy ts-parse <file> [--lang <name>]`. Add note that language is
  auto-detected from the extension and the flag is an override.
- §3 field reference table: `language` row's allowed values change from
  `rust (MVP)` to `rust | go | python | typescript`.
- §3 body: one-sentence note that extensions per language are fixed by
  the CLI; rule authors don't configure them.
- §4.1 ts-parse example: drop `--lang rust` (the example uses `.rs` so
  auto-detect resolves it).
- Front-matter: bump `# version:` from `0.2.0` to `0.3.0`.

### 5.2 Per-lang node-kind cheat sheets

Add three subsections after the existing §4.2 Rust cheat sheet. Each
mirrors §4.2's shape:

- §4.3 **`tree-sitter-go` node-kind cheat sheet** — ~10 most-common node
  kinds. Seed list: `source_file`, `function_declaration`,
  `method_declaration`, `call_expression`, `selector_expression`,
  `identifier`, `field_identifier`, `interpreted_string_literal`,
  `defer_statement`, `go_statement`.
- §4.4 **`tree-sitter-python` node-kind cheat sheet** — seed list:
  `module`, `function_definition`, `call`, `attribute`, `identifier`,
  `string`, `import_statement`, `import_from_statement`,
  `class_definition`, `decorator`.
- §4.5 **`tree-sitter-typescript` node-kind cheat sheet** — seed list:
  `program`, `function_declaration`, `arrow_function`,
  `call_expression`, `member_expression`, `identifier`,
  `property_identifier`, `import_statement`, `type_alias_declaration`,
  `interface_declaration`, `jsx_element` (note that `jsx_element` is
  tsx-grammar-only).

Each subsection also carries **one worked rule example** (single
`.rule.yaml` file contents, ≤20 YAML lines), with `description`,
`message`, `query`, and `fix` fields, demonstrating the cheat-sheet node
types in action. The implementer picks realistic example rules (e.g.
`no-fmt-println` for Go, `no-print-in-prod` for Python,
`no-console-log` for TypeScript). Descriptions are required (per the
existing SKILL.md directive).

Node-type lists are starting seeds: the implementer verifies each entry
against `tree-sitter-<lang>`'s `node-types.json` and against hands-on
output from `lintropy ts-parse` before finalising.

### 5.3 §6 recipes — unchanged

Rust recipes stay. No per-lang recipe bloat; the §4.x worked examples
cover per-lang rule authoring.

### 5.4 §9 anti-patterns

Add one entry:

> **Don't pick `language: tsx`.** There is no `tsx` variant. Rule
> authors write `language: typescript` for both `.ts` and `.tsx` files.
> The CLI picks the `tree-sitter-typescript` `typescript` vs `tsx`
> grammar per file based on the extension.

## 6. Testing

### 6.1 Unit tests — `src/langs.rs`

- `from_name_covers_all_enabled_langs` — round-trip for every canonical
  name when the corresponding feature is enabled.
- `from_extension_maps_all_known` — table test for every row in §3.3,
  including the compound `d.ts` entry.
- `from_extension_unknown_returns_none` — `md`, `toml`, empty string,
  unknown extensions.
- `language_from_path_handles_d_ts` — `foo.d.ts` resolves to TypeScript,
  `foo.ts` resolves to TypeScript, `foo.tsx` resolves to TypeScript.
- `ts_language_loads_<lang>` — one per enabled lang: feed a minimal
  valid source to the grammar, assert `root_node().kind()` matches the
  expected top-level kind (`source_file` for rust/go, `module` for
  python, `program` for typescript).
- `typescript_dispatch_picks_tsx_for_tsx_ext` — parse
  `const x = <Foo/>;` with `Path::new("f.tsx")`, assert no `ERROR` node
  at the root; parse `const x = <T>y;` with `Path::new("f.ts")`, assert
  no `ERROR` node at the root.
- `typescript_dispatch_picks_typescript_for_d_ts` — `Path::new("f.d.ts")`
  resolves to the `typescript` (non-tsx) grammar.
- `feature_gated_variants_absent_when_flag_off` — a
  `#[cfg(not(feature = "lang-go"))]` test verifying
  `Language::from_name("go") == None`. One such test per feature.

### 6.2 Integration tests

**`tests/cli_ts_parse.rs`** (existing file — extend):

- `ts_parse_auto_detects_rust_from_rs_extension` — `lintropy ts-parse
  some.rs` without `--lang`; assert stdout contains `source_file`.
- Equivalent tests for `.go` (expect `source_file`), `.py`
  (`module`), `.ts` (`program`), `.tsx` (`program` with JSX body).
- `ts_parse_rejects_unknown_extension_without_lang_flag` — `.txt` file,
  no `--lang`, assert exit code 2 and expected error substring.
- `ts_parse_lang_flag_overrides_extension` — `.go` file passed with
  `--lang rust`; parser emits `ERROR` nodes (rust grammar rejects Go
  syntax); test asserts `lintropy ts-parse` still exits 0 but the
  output contains `ERROR`.

**`tests/integration_multilang.rs`** (new file):

- One test per new language: a fixture with one source file containing a
  pattern, a `.lintropy/<id>.rule.yaml` with a representative query,
  run `lintropy check`, assert one diagnostic at the expected line.
- `tsx_jsx_rule_matches_in_tsx_and_not_in_ts` — a rule with a JSX query
  matches in a `.tsx` fixture, does not produce false positives against
  a `.ts` fixture carrying the same text in comments or strings.

### 6.3 Fixtures — `tests/fixtures/multilang/<lang>/`

Per lang: one small source file + `.lintropy/<id>.rule.yaml`. Kept
minimal — not a standalone example project.

### 6.4 Snapshot tests

Not introduced here. Existing snapshot strategy (`insta`) applies to
diagnostic output, which is language-agnostic.

## 7. Backwards compatibility

### 7.1 YAML + CLI user contracts

- Existing `language: rust` rules: unchanged behavior.
- Existing `lintropy ts-parse foo.rs --lang rust`: still works (explicit
  override path).
- Exit codes, diagnostic formats, `rules`/`explain`/`check` JSON output:
  unchanged.

### 7.2 Internal API change

- `Language::ts_language(self) -> TsLanguage` becomes
  `Language::ts_language(self, &Path) -> TsLanguage`. Consumed by
  `lintropy` internals only; there is no `lintropy-core` SDK crate
  published today. All in-tree call sites update in the same change.

### 7.3 Binary size

Default-feature build grows by ~5-7 MB. Documented in the CHANGELOG:

> Add Go, Python, TypeScript language support. Default binary grows by
> ~5-7 MB due to bundled tree-sitter grammars. For a Rust-only build use
> `cargo install lintropy --no-default-features`.

Homebrew and GitHub release binaries ship default-features and carry
the extra weight; this is an intentional trade-off for zero-config UX.

## 8. File-level summary of changes

| File | Change |
|------|--------|
| `Cargo.toml` | Add optional `tree-sitter-go`, `tree-sitter-python`, `tree-sitter-typescript` deps; `[features]` block with `default = ["lang-go","lang-python","lang-typescript"]`. |
| `src/langs.rs` | Add `Go`, `Python`, `TypeScript` variants under `#[cfg(feature = "lang-<name>")]`; extend `from_name`, `from_extension`, `extensions`; change `ts_language` signature to take `&Path`; add `language_from_path` helper; expand unit tests per §6.1. |
| `src/core/engine.rs` | Update `ts_language()` call sites to pass file path; extend `RulesByLanguage` for all enabled languages; pick tsx vs typescript compiled query at parse time (§3.5.1). |
| `src/core/config.rs` | Add `compiled_tsx: Option<Arc<TsQuery>>` to `QueryRule`; in `build_rule`, compile TypeScript queries against both grammars per §3.5.1; update `QueryRule::new` signature. |
| `src/core/predicates.rs` | Update test helpers to pass dummy `&Path` to `ts_language`. |
| `src/commands/ts_parse.rs` | Make `--lang` optional; add `resolve_language(path, override)` helper; pass path to `ts_language`; adjust error messages per §4.1. |
| `tests/cli_ts_parse.rs` | Extend with auto-detect + override cases for every lang per §6.2. |
| `tests/integration_multilang.rs` | New file. Per-lang end-to-end `check` test + TSX vs TS grammar-dispatch test. |
| `tests/fixtures/multilang/<lang>/` | Small fixture tree per lang (one source + one `.rule.yaml`). |
| `skill/SKILL.md` | Generic updates per §5.1; add §4.3/§4.4/§4.5 cheat sheets + worked rule examples per §5.2; add `language: tsx` anti-pattern per §5.4; bump version to `0.3.0`. |
| `CHANGELOG.md` | Entry per §7.3. |

## 9. Out-of-scope follow-ups

Recorded for future design work:

- `examples/go-demo/`, `examples/python-demo/`, `examples/typescript-demo/`
  standalone example projects mirroring `examples/rust-demo/`.
- Bundled seed rules shipped inside the CLI binary (currently none are).
- Vue / Svelte / Astro — need embedded-language handling, separate spec.
- Additional language families (Java/Kotlin/C++/Ruby).
- Per-lang auto-detection for `lintropy check` stdin mode (today stdin
  is not a `check` input).
- TSX grammar override for `ts-parse` when the file extension is not
  `.tsx` (e.g. `--grammar <typescript|tsx>` sub-flag). Current workaround:
  rename / copy to a `.tsx` file first.
- Stdin input for `ts-parse` (today requires a file path).
