# Multi-language support Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Register Go, Python, and TypeScript as first-class languages in
lintropy via Cargo features (default-on), with per-file grammar dispatch
for TypeScript (typescript vs tsx).

**Architecture:** Extend `Language` enum with feature-gated variants. Add
three optional tree-sitter grammar deps. Change `Language::ts_language`
to take `&Path` for tsx dispatch. For TypeScript rules, compile the query
against **both** grammars at load time and store a pair on `QueryRule`.
At parse time, engine picks the grammar and compiled query based on
file extension. `ts-parse` already auto-detects language from extension;
error messages tighten to list compiled-in languages. SKILL.md grows
per-lang node-kind cheat sheets and worked rule examples.

**Tech Stack:** Rust stable (1.95.0), `tree-sitter = "0.22"`, three new
optional grammar crates (`tree-sitter-go`, `tree-sitter-python`,
`tree-sitter-typescript` — versions pinned at Task 1 time to newest
release compatible with `tree-sitter = "0.22"`), `clap`, `serde`,
`serde_yaml`, `schemars`, `assert_cmd`, `predicates`, `tempfile`,
`insta`, `serde_json`.

**Spec:** `specs/2026-04-18-multilang-support.md`

---

## File Structure

Files to touch:

- **Modify** `Cargo.toml`
  - Add `[features]` block: `default = ["lang-go","lang-python","lang-typescript"]`, three per-lang features each enabling the corresponding optional dep.
  - Add three optional deps under `[dependencies]`.

- **Modify** `src/langs.rs`
  - Add `Go`, `Python`, `TypeScript` variants to `Language`, each gated by `#[cfg(feature = "lang-<name>")]`.
  - Extend `from_name`, `from_extension`, `extensions` with all three.
  - Add `language_from_path` helper handling compound `.d.ts` suffix.
  - Change `ts_language` signature from `fn(self) -> TsLanguage` to `fn(self, &Path) -> TsLanguage`. `path` ignored by all variants except `TypeScript`; `TypeScript` dispatches to `language_tsx()` for `.tsx` paths, else `language_typescript()`.
  - Add feature-gated unit tests per spec §6.1.

- **Modify** `src/commands/ts_parse.rs`
  - Update `ts_language()` call site to pass `&args.file`.
  - Improve error messages in `resolve_language` to list compiled-in langs via a small `available_langs()` helper (per spec §4.1).

- **Modify** `src/core/engine.rs`
  - Update `ts_language()` call site in `run_file` to pass `path`.
  - Extend `RulesByLanguage` struct with fields for all enabled languages (or swap to `HashMap<Language, Vec<ScopedRule>>`).
  - In `run_file`, pick `rule.compiled_tsx` vs `rule.compiled` based on whether the file is `.tsx`.

- **Modify** `src/core/config.rs`
  - Add `compiled_tsx: Option<Arc<TsQuery>>` to `QueryRule`.
  - In `build_rule`, for TypeScript rules compile the query against **both** `typescript` and `tsx` grammars (using `Path::new("_.ts")` and `Path::new("_.tsx")` as path arguments to `ts_language`). Store results per §3.5.1.
  - Expand existing tests around `QueryRule` compilation to cover dual-compile path (TypeScript only).

- **Modify** `src/core/predicates.rs`
  - Update four test-helper call sites that invoke `Language::Rust.ts_language()` to pass a dummy `Path::new("t.rs")`.

- **Modify** `Cargo.lock` — regenerated automatically on first `cargo build`.

- **Create** `tests/fixtures/multilang/go/` — one source file + one `.lintropy/<id>.rule.yaml`.
- **Create** `tests/fixtures/multilang/python/` — same.
- **Create** `tests/fixtures/multilang/typescript/` — same, plus a `.tsx` file + a tsx-only rule.

- **Create** `tests/integration_multilang.rs`
  - One end-to-end `check` test per new language.
  - One TSX-vs-TS grammar-dispatch test.

- **Modify** `tests/cli_ts_parse.rs`
  - Extend with per-lang auto-detect + override + unknown-extension cases per spec §6.2.

- **Modify** `skill/SKILL.md`
  - §2 commands block: `[--lang rust]` → `[--lang <name>]`.
  - §3 field reference table: `language` row allowed values → `rust | go | python | typescript`.
  - §3 body: one-line note that extensions per language are CLI-fixed.
  - §4.1 ts-parse example: drop `--lang rust`.
  - Add §4.3 Go cheat sheet + worked rule example.
  - Add §4.4 Python cheat sheet + worked rule example.
  - Add §4.5 TypeScript cheat sheet + worked rule example.
  - Add §9 anti-pattern: `language: tsx` has no such variant.
  - Bump front-matter `# version: 0.2.0` → `0.3.0`.

- **Modify** `CHANGELOG.md`
  - Entry per spec §7.3.

---

## Task 1: Add Cargo features and optional grammar dependencies

**Files:**
- Modify: `Cargo.toml`

- [ ] **Step 1: Open `Cargo.toml` and add `[features]` block**

Current `[dependencies]` section has no features. Add a new section above `[dependencies]`:

```toml
[features]
default = ["lang-go", "lang-python", "lang-typescript"]
lang-go = ["dep:tree-sitter-go"]
lang-python = ["dep:tree-sitter-python"]
lang-typescript = ["dep:tree-sitter-typescript"]
```

- [ ] **Step 2: Add three optional deps to `[dependencies]`**

Run `cargo search tree-sitter-go --limit 3`, pick the newest version that compiles against `tree-sitter = "0.22"` (likely `"0.21"` at plan time; verify). Repeat for `tree-sitter-python` and `tree-sitter-typescript`.

Add under `[dependencies]`:

```toml
tree-sitter-go = { version = "<pinned>", optional = true }
tree-sitter-python = { version = "<pinned>", optional = true }
tree-sitter-typescript = { version = "<pinned>", optional = true }
```

Replace `<pinned>` with the version selected. If any version conflicts with `tree-sitter = "0.22"` (compile error referencing symbol-version mismatch), escalate — do **not** silently bump `tree-sitter` to a newer version. Resolution alternative: pin to whichever upstream grammar release last supported `tree-sitter 0.22`.

- [ ] **Step 3: Build with default features**

```bash
cargo build
```

Expected: build succeeds, all three grammar crates fetched and compiled. `Cargo.lock` updates.

- [ ] **Step 4: Build with no default features**

```bash
cargo build --no-default-features
```

Expected: build succeeds, `tree-sitter-go`/`python`/`typescript` NOT compiled (check `cargo build -vv --no-default-features 2>&1 | grep tree-sitter-` — should see only `tree-sitter` and `tree-sitter-rust`).

- [ ] **Step 5: Run existing test suite**

```bash
cargo test
```

Expected: all pre-existing tests pass unchanged. Feature flag did not affect behaviour yet.

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml Cargo.lock
git commit -m "build: add opt-in features and grammar deps for go/python/typescript"
```

---

## Task 2: Change `Language::ts_language` signature to take `&Path`

**Files:**
- Modify: `src/langs.rs`
- Modify: `src/commands/ts_parse.rs`
- Modify: `src/core/engine.rs`
- Modify: `src/core/config.rs`
- Modify: `src/core/predicates.rs`

This task threads a `&Path` through `ts_language` without adding new variants. All existing variants (just `Rust` today) ignore the path.

- [ ] **Step 1: Update `Language::ts_language` signature in `src/langs.rs`**

Replace:

```rust
/// Native `tree-sitter` language handle for the grammar.
pub fn ts_language(self) -> TsLanguage {
    match self {
        Language::Rust => tree_sitter_rust::language(),
    }
}
```

With:

```rust
/// Native `tree-sitter` language handle for the grammar.
///
/// `path` picks between multi-grammar languages (TypeScript's
/// `typescript` vs `tsx`). Other variants ignore it.
pub fn ts_language(self, _path: &std::path::Path) -> TsLanguage {
    match self {
        Language::Rust => tree_sitter_rust::language(),
    }
}
```

- [ ] **Step 2: Update the existing Rust test helper in `src/langs.rs:73`**

Replace:

```rust
let lang = Language::Rust.ts_language();
```

With:

```rust
let lang = Language::Rust.ts_language(std::path::Path::new("t.rs"));
```

- [ ] **Step 3: Update call site in `src/commands/ts_parse.rs:18`**

Replace:

```rust
.set_language(&language.ts_language())
```

With:

```rust
.set_language(&language.ts_language(&args.file))
```

- [ ] **Step 4: Update call site in `src/core/engine.rs:105`**

In `run_file`, the `path: &Path` is already in scope. Replace:

```rust
.set_language(&language.ts_language())
```

With:

```rust
.set_language(&language.ts_language(path))
```

- [ ] **Step 5: Update call site in `src/core/config.rs:402`**

`build_rule` receives `source_path: &Path`. For the only variant today (`Rust`) path is ignored, so any path works. For clarity use a stable sentinel:

Replace:

```rust
let compiled = TsQuery::new(&language.ts_language(), &query_source).map_err(|e| {
```

With:

```rust
let compiled = TsQuery::new(&language.ts_language(source_path), &query_source).map_err(|e| {
```

(TypeScript will later require dual compilation; Task 6 refactors this site.)

- [ ] **Step 6: Update test-helper call sites in `src/core/predicates.rs`**

Lines 252, 259, 282, 315, 349 each call `Language::Rust.ts_language()`. Replace every instance with:

```rust
Language::Rust.ts_language(std::path::Path::new("t.rs"))
```

Example at line 252:

```rust
.set_language(&Language::Rust.ts_language(std::path::Path::new("t.rs")))
```

and at the `let language = ...;` bindings (lines 259, 282, 315, 349):

```rust
let language = Language::Rust.ts_language(std::path::Path::new("t.rs"));
```

- [ ] **Step 7: Build + test**

```bash
cargo build
cargo test
```

Expected: both succeed. Behaviour unchanged; only signatures changed.

- [ ] **Step 8: Commit**

```bash
git add src/langs.rs src/commands/ts_parse.rs src/core/engine.rs src/core/config.rs src/core/predicates.rs
git commit -m "refactor(langs): ts_language takes &Path for upcoming tsx dispatch"
```

---

## Task 3: Add `Go` variant

**Files:**
- Modify: `src/langs.rs`

- [ ] **Step 1: Write failing unit tests**

Open `src/langs.rs`. In the existing `#[cfg(test)] mod tests` block, append:

```rust
    #[cfg(feature = "lang-go")]
    #[test]
    fn from_name_resolves_go() {
        assert_eq!(Language::from_name("go"), Some(Language::Go));
        assert_eq!(Language::Go.name(), "go");
    }

    #[cfg(feature = "lang-go")]
    #[test]
    fn from_extension_resolves_go() {
        assert_eq!(Language::from_extension("go"), Some(Language::Go));
        assert!(Language::Go.extensions().contains(&"go"));
    }

    #[cfg(feature = "lang-go")]
    #[test]
    fn go_ts_language_parses_hello_world() {
        let lang = Language::Go.ts_language(std::path::Path::new("t.go"));
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&lang).unwrap();
        let tree = parser.parse("package main\nfunc main() {}", None).unwrap();
        assert_eq!(tree.root_node().kind(), "source_file");
    }
```

- [ ] **Step 2: Run tests — expect compile errors**

```bash
cargo test --features lang-go
```

Expected: `error: no variant named 'Go' found for enum 'Language'`.

- [ ] **Step 3: Add `Go` variant to the enum**

Replace the enum:

```rust
pub enum Language {
    Rust,
}
```

With:

```rust
pub enum Language {
    Rust,
    #[cfg(feature = "lang-go")]
    Go,
}
```

- [ ] **Step 4: Update `from_name`, `from_extension`, `name`, `extensions`, `ts_language`**

Replace each match block:

```rust
pub fn from_name(name: &str) -> Option<Language> {
    match name {
        "rust" => Some(Language::Rust),
        #[cfg(feature = "lang-go")]
        "go" => Some(Language::Go),
        _ => None,
    }
}

pub fn from_extension(ext: &str) -> Option<Language> {
    match ext {
        "rs" => Some(Language::Rust),
        #[cfg(feature = "lang-go")]
        "go" => Some(Language::Go),
        _ => None,
    }
}

pub fn name(self) -> &'static str {
    match self {
        Language::Rust => "rust",
        #[cfg(feature = "lang-go")]
        Language::Go => "go",
    }
}

pub fn extensions(self) -> &'static [&'static str] {
    match self {
        Language::Rust => &["rs"],
        #[cfg(feature = "lang-go")]
        Language::Go => &["go"],
    }
}

pub fn ts_language(self, _path: &std::path::Path) -> TsLanguage {
    match self {
        Language::Rust => tree_sitter_rust::language(),
        #[cfg(feature = "lang-go")]
        Language::Go => tree_sitter_go::language(),
    }
}
```

- [ ] **Step 5: Run tests — expect pass**

```bash
cargo test --features lang-go
```

Expected: all previous tests + three new Go tests pass.

```bash
cargo test --no-default-features
```

Expected: all previous tests pass; new Go tests skipped (feature not enabled).

- [ ] **Step 6: Commit**

```bash
git add src/langs.rs
git commit -m "feat(langs): register go language"
```

---

## Task 4: Add `Python` variant

**Files:**
- Modify: `src/langs.rs`

Mirror Task 3 for Python, including the `pyi` extension.

- [ ] **Step 1: Write failing unit tests**

Append to the test module:

```rust
    #[cfg(feature = "lang-python")]
    #[test]
    fn from_name_resolves_python() {
        assert_eq!(Language::from_name("python"), Some(Language::Python));
        assert_eq!(Language::Python.name(), "python");
    }

    #[cfg(feature = "lang-python")]
    #[test]
    fn from_extension_resolves_python_and_pyi() {
        assert_eq!(Language::from_extension("py"), Some(Language::Python));
        assert_eq!(Language::from_extension("pyi"), Some(Language::Python));
        let exts = Language::Python.extensions();
        assert!(exts.contains(&"py"));
        assert!(exts.contains(&"pyi"));
    }

    #[cfg(feature = "lang-python")]
    #[test]
    fn python_ts_language_parses_hello_world() {
        let lang = Language::Python.ts_language(std::path::Path::new("t.py"));
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&lang).unwrap();
        let tree = parser.parse("def hi():\n    pass\n", None).unwrap();
        assert_eq!(tree.root_node().kind(), "module");
    }
```

- [ ] **Step 2: Run tests — expect fail**

```bash
cargo test --features lang-python
```

Expected: compile error referencing `Language::Python`.

- [ ] **Step 3: Extend the enum and match arms**

Add variant:

```rust
pub enum Language {
    Rust,
    #[cfg(feature = "lang-go")]
    Go,
    #[cfg(feature = "lang-python")]
    Python,
}
```

Add arms to each helper:

- `from_name`: `#[cfg(feature = "lang-python")] "python" => Some(Language::Python),`
- `from_extension`: `#[cfg(feature = "lang-python")] "py" | "pyi" => Some(Language::Python),`
- `name`: `#[cfg(feature = "lang-python")] Language::Python => "python",`
- `extensions`: `#[cfg(feature = "lang-python")] Language::Python => &["py", "pyi"],`
- `ts_language`: `#[cfg(feature = "lang-python")] Language::Python => tree_sitter_python::language(),`

- [ ] **Step 4: Run tests — expect pass**

```bash
cargo test --features lang-python
cargo test --no-default-features
```

Expected: both succeed.

- [ ] **Step 5: Commit**

```bash
git add src/langs.rs
git commit -m "feat(langs): register python language (py, pyi)"
```

---

## Task 5: Add `TypeScript` variant with path-based tsx dispatch

**Files:**
- Modify: `src/langs.rs`

- [ ] **Step 1: Write failing unit tests**

Append to the test module:

```rust
    #[cfg(feature = "lang-typescript")]
    #[test]
    fn from_name_resolves_typescript() {
        assert_eq!(Language::from_name("typescript"), Some(Language::TypeScript));
        assert_eq!(Language::TypeScript.name(), "typescript");
        // No `tsx` alias — rule authors use `typescript` for both.
        assert_eq!(Language::from_name("tsx"), None);
    }

    #[cfg(feature = "lang-typescript")]
    #[test]
    fn from_extension_resolves_typescript_family() {
        for ext in ["ts", "tsx", "mts", "cts"] {
            assert_eq!(
                Language::from_extension(ext),
                Some(Language::TypeScript),
                "extension: {ext}"
            );
        }
    }

    #[cfg(feature = "lang-typescript")]
    #[test]
    fn language_from_path_handles_d_ts_compound_suffix() {
        use std::path::Path;
        assert_eq!(language_from_path(Path::new("src/types.d.ts")), Some(Language::TypeScript));
        assert_eq!(language_from_path(Path::new("src/api.ts")), Some(Language::TypeScript));
        assert_eq!(language_from_path(Path::new("src/app.tsx")), Some(Language::TypeScript));
        assert_eq!(language_from_path(Path::new("src/lib.rs")), Some(Language::Rust));
        assert_eq!(language_from_path(Path::new("src/no-ext")), None);
    }

    #[cfg(feature = "lang-typescript")]
    #[test]
    fn typescript_dispatch_picks_tsx_grammar_for_tsx_ext() {
        use std::path::Path;
        let lang = Language::TypeScript.ts_language(Path::new("f.tsx"));
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&lang).unwrap();
        let tree = parser.parse("const x = <Foo/>;", None).unwrap();
        assert!(
            !has_error(tree.root_node()),
            "tsx grammar should parse JSX: {}",
            tree.root_node().to_sexp()
        );
    }

    #[cfg(feature = "lang-typescript")]
    #[test]
    fn typescript_dispatch_picks_typescript_grammar_for_ts_ext() {
        use std::path::Path;
        let lang = Language::TypeScript.ts_language(Path::new("f.ts"));
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&lang).unwrap();
        let tree = parser.parse("const x = <number>42;", None).unwrap();
        assert!(
            !has_error(tree.root_node()),
            "typescript grammar should parse type assertions: {}",
            tree.root_node().to_sexp()
        );
    }

    #[cfg(feature = "lang-typescript")]
    #[test]
    fn typescript_dispatch_picks_typescript_for_d_ts() {
        use std::path::Path;
        let lang = Language::TypeScript.ts_language(Path::new("types.d.ts"));
        // Same grammar handle as plain `.ts`. Compare addresses via equality by
        // parsing a type-assertion that only the non-tsx grammar accepts.
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&lang).unwrap();
        let tree = parser.parse("type T = <U>(x: U) => U;", None).unwrap();
        assert!(!has_error(tree.root_node()));
    }

    #[cfg(feature = "lang-typescript")]
    fn has_error(node: tree_sitter::Node) -> bool {
        if node.is_error() {
            return true;
        }
        for i in 0..node.child_count() {
            if has_error(node.child(i).unwrap()) {
                return true;
            }
        }
        false
    }
```

- [ ] **Step 2: Run tests — expect compile error**

```bash
cargo test --features lang-typescript
```

Expected: `error: no variant named 'TypeScript'` and `language_from_path not found`.

- [ ] **Step 3: Add the variant, dispatcher, and helper**

Extend the enum:

```rust
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

Add match arms to each helper. `from_extension` gains `ts | tsx | mts | cts`:

```rust
pub fn from_name(name: &str) -> Option<Language> {
    match name {
        "rust" => Some(Language::Rust),
        #[cfg(feature = "lang-go")]
        "go" => Some(Language::Go),
        #[cfg(feature = "lang-python")]
        "python" => Some(Language::Python),
        #[cfg(feature = "lang-typescript")]
        "typescript" => Some(Language::TypeScript),
        _ => None,
    }
}

pub fn from_extension(ext: &str) -> Option<Language> {
    match ext {
        "rs" => Some(Language::Rust),
        #[cfg(feature = "lang-go")]
        "go" => Some(Language::Go),
        #[cfg(feature = "lang-python")]
        "py" | "pyi" => Some(Language::Python),
        #[cfg(feature = "lang-typescript")]
        "ts" | "tsx" | "mts" | "cts" | "d.ts" => Some(Language::TypeScript),
        _ => None,
    }
}

pub fn name(self) -> &'static str {
    match self {
        Language::Rust => "rust",
        #[cfg(feature = "lang-go")]
        Language::Go => "go",
        #[cfg(feature = "lang-python")]
        Language::Python => "python",
        #[cfg(feature = "lang-typescript")]
        Language::TypeScript => "typescript",
    }
}

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

pub fn ts_language(self, path: &std::path::Path) -> TsLanguage {
    match self {
        Language::Rust => tree_sitter_rust::language(),
        #[cfg(feature = "lang-go")]
        Language::Go => tree_sitter_go::language(),
        #[cfg(feature = "lang-python")]
        Language::Python => tree_sitter_python::language(),
        #[cfg(feature = "lang-typescript")]
        Language::TypeScript => {
            if is_tsx_path(path) {
                tree_sitter_typescript::language_tsx()
            } else {
                tree_sitter_typescript::language_typescript()
            }
        }
    }
}
```

Rename `_path` to `path` since it is now used by TypeScript.

Add module-level helpers:

```rust
/// Resolve a path to a [`Language`], handling the `.d.ts` compound suffix.
pub fn language_from_path(path: &std::path::Path) -> Option<Language> {
    let name = path.file_name()?.to_str()?;
    if name.ends_with(".d.ts") {
        return Language::from_extension("d.ts");
    }
    let ext = path.extension()?.to_str()?;
    Language::from_extension(ext)
}

#[cfg(feature = "lang-typescript")]
fn is_tsx_path(path: &std::path::Path) -> bool {
    path.extension().and_then(|e| e.to_str()) == Some("tsx")
}
```

- [ ] **Step 4: Run tests — expect pass**

```bash
cargo test --features lang-typescript
cargo test  # default features
cargo test --no-default-features
```

Expected: all suites pass. `cargo test --no-default-features` still only has the single `Rust` variant active, and the typescript tests are skipped.

- [ ] **Step 5: Commit**

```bash
git add src/langs.rs
git commit -m "feat(langs): register typescript with per-path tsx grammar dispatch"
```

---

## Task 6: Dual-compile TypeScript queries at rule load

**Files:**
- Modify: `src/core/config.rs`

This task lands §3.5.1 of the spec.

- [ ] **Step 1: Write failing unit tests**

Open `src/core/config.rs`. In the existing `#[cfg(test)] mod tests` block, append:

```rust
    #[cfg(feature = "lang-typescript")]
    #[test]
    fn typescript_rule_compiles_against_both_grammars() {
        let rules_dir = tempfile::tempdir().unwrap();
        let rule_path = rules_dir.path().join("no-any.rule.yaml");
        std::fs::write(
            &rule_path,
            r#"severity: warning
message: "avoid any"
language: typescript
query: |
  (predefined_type) @t (#eq? @t "any")
"#,
        )
        .unwrap();
        let cfg = Config::load_from_rules_dir(rules_dir.path()).unwrap();
        let rule = &cfg.rules[0];
        let query = rule.kind.query_rule().expect("is a query rule");
        assert!(
            query.compiled_tsx.is_some(),
            "typescript rule should have a tsx-grammar compilation"
        );
    }

    #[cfg(feature = "lang-typescript")]
    #[test]
    fn typescript_rule_with_jsx_only_compiles_against_tsx() {
        let rules_dir = tempfile::tempdir().unwrap();
        let rule_path = rules_dir.path().join("no-div.rule.yaml");
        std::fs::write(
            &rule_path,
            r#"severity: warning
message: "no raw div"
language: typescript
query: |
  (jsx_element) @m
"#,
        )
        .unwrap();
        let cfg = Config::load_from_rules_dir(rules_dir.path()).unwrap();
        let rule = &cfg.rules[0];
        let query = rule.kind.query_rule().expect("is a query rule");
        assert!(
            query.compiled_tsx.is_some(),
            "tsx grammar should accept jsx_element"
        );
        // `jsx_element` does not exist in the plain typescript grammar, so
        // `compiled` (primary) should have fallen back to whatever the
        // non-tsx grammar accepts... which means compilation failed. The
        // loader keeps the tsx-only compilation and sets primary to a
        // dummy empty query, or (cleaner) marks primary as unusable by
        // setting compiled to the tsx one as well. Implementation choice:
        // primary = first grammar that compiles; compiled_tsx = tsx
        // compilation when different. If only tsx compiles, primary ==
        // compiled_tsx (the same Arc).
        assert!(Arc::ptr_eq(
            &query.compiled,
            query.compiled_tsx.as_ref().unwrap()
        ));
    }

    #[cfg(feature = "lang-typescript")]
    #[test]
    fn typescript_rule_that_fails_both_grammars_errors() {
        let rules_dir = tempfile::tempdir().unwrap();
        let rule_path = rules_dir.path().join("broken.rule.yaml");
        std::fs::write(
            &rule_path,
            r#"severity: warning
message: "broken"
language: typescript
query: |
  (this_node_kind_does_not_exist) @m
"#,
        )
        .unwrap();
        let err = Config::load_from_rules_dir(rules_dir.path()).unwrap_err();
        assert!(format!("{err}").contains("broken"), "error surfaces rule id");
    }

    #[test]
    fn non_typescript_rules_have_no_tsx_compilation() {
        let rules_dir = tempfile::tempdir().unwrap();
        let rule_path = rules_dir.path().join("r.rule.yaml");
        std::fs::write(
            &rule_path,
            r#"severity: warning
message: "m"
language: rust
query: |
  (identifier) @m
"#,
        )
        .unwrap();
        let cfg = Config::load_from_rules_dir(rules_dir.path()).unwrap();
        let rule = &cfg.rules[0];
        let query = rule.kind.query_rule().expect("is a query rule");
        assert!(query.compiled_tsx.is_none());
    }
```

(If `Config::load_from_rules_dir` does not exist with that name, use whatever test loader the existing config tests use — check surrounding test cases. The helper that exists is the one to call.)

- [ ] **Step 2: Run tests — expect fail**

```bash
cargo test --features lang-typescript typescript_rule
```

Expected: compile error — field `compiled_tsx` missing on `QueryRule`.

- [ ] **Step 3: Add `compiled_tsx` to `QueryRule`**

Locate the `pub struct QueryRule` definition (around line 69). Replace:

```rust
pub struct QueryRule {
    pub source: String,
    pub compiled: Arc<TsQuery>,
    // ... any other existing fields ...
}
```

with:

```rust
pub struct QueryRule {
    pub source: String,
    pub compiled: Arc<TsQuery>,
    /// Only `Some` for TypeScript rules. Same `Arc` as `compiled` if the
    /// query compiles only against one of the two TypeScript grammars.
    pub compiled_tsx: Option<Arc<TsQuery>>,
    // ... existing fields ...
}
```

Update `QueryRule::new` if it is used by tests (search for callers). If it exists:

```rust
impl QueryRule {
    pub fn new(source: impl Into<String>, query: TsQuery) -> Result<Self> {
        Ok(Self {
            source: source.into(),
            compiled: Arc::new(query),
            compiled_tsx: None,
        })
    }
}
```

- [ ] **Step 4: Implement dual-compile for TypeScript rules in `build_rule`**

Locate the existing single-compile block (around line 402):

```rust
let compiled = TsQuery::new(&language.ts_language(source_path), &query_source).map_err(|e| {
    LintropyError::QueryCompile {
        rule_id: id.to_string(),
        source_path: source_path.to_path_buf(),
        message: format!("{e}"),
    }
})?;
```

Replace with a helper that dispatches on language:

```rust
let (compiled, compiled_tsx) = compile_query_for(language, id, source_path, &query_source)?;
```

Add the helper near the top of the file (after imports):

```rust
fn compile_query_for(
    language: crate::langs::Language,
    id: &str,
    source_path: &std::path::Path,
    query_source: &str,
) -> Result<(Arc<TsQuery>, Option<Arc<TsQuery>>)> {
    #[cfg(feature = "lang-typescript")]
    if language == crate::langs::Language::TypeScript {
        use std::path::Path;
        let ts_grammar = language.ts_language(Path::new("_.ts"));
        let tsx_grammar = language.ts_language(Path::new("_.tsx"));

        let ts_result = TsQuery::new(&ts_grammar, query_source);
        let tsx_result = TsQuery::new(&tsx_grammar, query_source);

        match (ts_result, tsx_result) {
            (Ok(ts), Ok(tsx)) => {
                let ts_arc = Arc::new(ts);
                let tsx_arc = Arc::new(tsx);
                return Ok((ts_arc, Some(tsx_arc)));
            }
            (Ok(ts), Err(_)) => {
                let ts_arc = Arc::new(ts);
                return Ok((ts_arc.clone(), Some(ts_arc)));
            }
            (Err(_), Ok(tsx)) => {
                let tsx_arc = Arc::new(tsx);
                return Ok((tsx_arc.clone(), Some(tsx_arc)));
            }
            (Err(ts_err), Err(tsx_err)) => {
                return Err(LintropyError::QueryCompile {
                    rule_id: id.to_string(),
                    source_path: source_path.to_path_buf(),
                    message: format!(
                        "typescript grammar: {ts_err}; tsx grammar: {tsx_err}"
                    ),
                });
            }
        }
    }

    let compiled = TsQuery::new(&language.ts_language(source_path), query_source).map_err(|e| {
        LintropyError::QueryCompile {
            rule_id: id.to_string(),
            source_path: source_path.to_path_buf(),
            message: format!("{e}"),
        }
    })?;
    Ok((Arc::new(compiled), None))
}
```

Update the `RuleKind::Query(QueryRule { ... })` construction to populate the new field. Replace:

```rust
Ok(RuleKind::Query(QueryRule {
    source: query_source,
    compiled: Arc::new(compiled),
    // ... other fields ...
}))
```

with:

```rust
Ok(RuleKind::Query(QueryRule {
    source: query_source,
    compiled,
    compiled_tsx,
    // ... other fields ...
}))
```

Note that `compiled` and `compiled_tsx` now come from the destructured helper result — the existing `Arc::new(compiled)` wrapping drops out because `compile_query_for` already returns `Arc<TsQuery>`.

Also update lines below (411-413) that call `.capture_names()` on `compiled`: `compiled` is now `Arc<TsQuery>` so calls become `compiled.capture_names()` (same API).

- [ ] **Step 5: Run tests — expect pass**

```bash
cargo test --features lang-typescript
cargo test  # default
cargo test --no-default-features
```

Expected: all pass.

- [ ] **Step 6: Commit**

```bash
git add src/core/config.rs
git commit -m "feat(core): dual-compile typescript queries against ts + tsx grammars"
```

---

## Task 7: Engine grammar dispatch and `RulesByLanguage` extension

**Files:**
- Modify: `src/core/engine.rs`

- [ ] **Step 1: Write failing integration test**

This test goes in `tests/integration_multilang.rs` — create the file:

```rust
use assert_cmd::Command;
use tempfile::TempDir;

fn write(dir: &std::path::Path, rel: &str, contents: &str) {
    let path = dir.join(rel);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(path, contents).unwrap();
}

#[cfg(feature = "lang-typescript")]
#[test]
fn tsx_jsx_rule_matches_only_in_tsx_files() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    write(root, "lintropy.yaml", "version: 1\n");
    write(
        root,
        ".lintropy/no-raw-div.rule.yaml",
        r#"severity: warning
message: "no raw <div>"
language: typescript
query: |
  (jsx_element
    (jsx_opening_element (identifier) @name)
    (#eq? @name "div")) @m
"#,
    );
    write(root, "src/app.tsx", "const x = <div/>;\n");
    write(root, "src/lib.ts", "const x: number = 1;\n");

    let mut cmd = Command::cargo_bin("lintropy").unwrap();
    cmd.current_dir(root).arg("check").arg("--format").arg("json");
    let output = cmd.output().unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("app.tsx"), "tsx match missing: {stdout}");
    assert!(!stdout.contains("lib.ts"), "false positive on lib.ts: {stdout}");
}
```

This test will fail until the engine routes by language AND picks `compiled_tsx` for `.tsx` files.

- [ ] **Step 2: Run test — expect fail**

```bash
cargo test --features lang-typescript --test integration_multilang
```

Expected: either a compile error, or the `.tsx` fixture is skipped (engine only handles Rust), resulting in empty output.

- [ ] **Step 3: Extend `RulesByLanguage` for all enabled languages**

Open `src/core/engine.rs`. Replace the struct definition (line 35-37) and its impl (line 45-65) with a `HashMap`-based design:

```rust
use std::collections::HashMap;

struct RulesByLanguage<'a> {
    by_lang: HashMap<crate::langs::Language, Vec<ScopedRule<'a>>>,
}

impl<'a> RulesByLanguage<'a> {
    fn new(config: &'a Config) -> Result<Self> {
        let mut by_lang: HashMap<_, Vec<_>> = HashMap::new();
        for rule in &config.rules {
            let Some(language) = rule.language else {
                continue;
            };
            if rule.query_rule().is_none() {
                continue;
            }
            let scoped = ScopedRule {
                rule,
                include: compile_globs(&rule.include)?,
                exclude: compile_globs(&rule.exclude)?,
            };
            by_lang.entry(language).or_default().push(scoped);
        }
        Ok(Self { by_lang })
    }

    fn get(&self, language: crate::langs::Language) -> &[ScopedRule<'a>] {
        self.by_lang.get(&language).map(|v| v.as_slice()).unwrap_or(&[])
    }
}
```

Update `run_file` (line 86 onwards) to use `.get(language)`:

```rust
let scoped_rules = rules_by_language.get(language);
if scoped_rules.is_empty() {
    return Ok(Vec::new());
}
```

- [ ] **Step 4: Pick `compiled_tsx` vs `compiled` at parse time**

In `run_file`, after parsing, find the place where `QueryCursor::matches(&rule.compiled, ...)` is called (search `QueryCursor` or `.compiled` in the file). Add a helper above `run_file`:

```rust
fn pick_compiled<'a>(rule: &'a crate::core::config::RuleConfig, path: &std::path::Path) -> &'a tree_sitter::Query {
    let Some(query_rule) = rule.query_rule() else {
        unreachable!("run_file only handles query rules");
    };
    #[cfg(feature = "lang-typescript")]
    if path.extension().and_then(|e| e.to_str()) == Some("tsx") {
        if let Some(tsx) = &query_rule.compiled_tsx {
            return tsx;
        }
    }
    &query_rule.compiled
}
```

Replace `rule.compiled` / `query_rule.compiled` usage in matcher calls with `pick_compiled(rule, path)`.

- [ ] **Step 5: Run tests — expect pass**

```bash
cargo test --features lang-typescript
cargo test
cargo test --no-default-features
```

Expected: all green, including `tsx_jsx_rule_matches_only_in_tsx_files`.

- [ ] **Step 6: Commit**

```bash
git add src/core/engine.rs tests/integration_multilang.rs
git commit -m "feat(engine): route rules by language + pick tsx query for .tsx files"
```

---

## Task 8: `ts-parse` error-message tightening

**Files:**
- Modify: `src/commands/ts_parse.rs`
- Modify: `tests/cli_ts_parse.rs`

- [ ] **Step 1: Write failing tests**

Open `tests/cli_ts_parse.rs`. Add tests:

```rust
#[test]
fn ts_parse_unknown_extension_lists_available_langs() {
    let tmp = tempfile::tempdir().unwrap();
    let file = tmp.path().join("foo.unknown");
    std::fs::write(&file, "hello").unwrap();
    let mut cmd = assert_cmd::Command::cargo_bin("lintropy").unwrap();
    cmd.arg("ts-parse").arg(&file);
    let assert = cmd.assert().failure();
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr).into_owned();
    assert!(
        stderr.contains("rust"),
        "error should list rust among available langs: {stderr}"
    );
}

#[test]
fn ts_parse_unknown_language_lists_available_langs() {
    let tmp = tempfile::tempdir().unwrap();
    let file = tmp.path().join("foo.txt");
    std::fs::write(&file, "hello").unwrap();
    let mut cmd = assert_cmd::Command::cargo_bin("lintropy").unwrap();
    cmd.arg("ts-parse").arg(&file).arg("--lang").arg("brainfuck");
    let assert = cmd.assert().failure();
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr).into_owned();
    assert!(stderr.contains("brainfuck"), "echo unknown name: {stderr}");
    assert!(stderr.contains("rust"), "list rust: {stderr}");
}
```

- [ ] **Step 2: Run tests — expect fail**

```bash
cargo test --test cli_ts_parse ts_parse_unknown
```

Expected: assertion failure — existing error messages don't list available languages.

- [ ] **Step 3: Add `available_langs()` helper and use it**

In `src/commands/ts_parse.rs`, replace `resolve_language`:

```rust
fn resolve_language(args: &TsParseArgs) -> Result<Language, CliError> {
    if let Some(name) = &args.lang {
        return Language::from_name(name).ok_or_else(|| {
            CliError::user(format!(
                "unknown language `{name}`. Available: {}",
                available_langs()
            ))
        });
    }
    let ext = args
        .file
        .extension()
        .and_then(|e| e.to_str())
        .ok_or_else(|| {
            CliError::user(format!(
                "could not detect language for {} (no extension); pass --lang <{}>",
                args.file.display(),
                available_langs()
            ))
        })?;
    Language::from_extension(ext).ok_or_else(|| {
        CliError::user(format!(
            "unknown file extension `.{ext}`; pass --lang <{}> to override",
            available_langs()
        ))
    })
}

fn available_langs() -> String {
    let mut langs = vec!["rust"];
    #[cfg(feature = "lang-go")]
    langs.push("go");
    #[cfg(feature = "lang-python")]
    langs.push("python");
    #[cfg(feature = "lang-typescript")]
    langs.push("typescript");
    langs.join("|")
}
```

- [ ] **Step 4: Run tests — expect pass**

```bash
cargo test --test cli_ts_parse
```

Expected: both new tests pass, previous tests still pass.

- [ ] **Step 5: Commit**

```bash
git add src/commands/ts_parse.rs tests/cli_ts_parse.rs
git commit -m "feat(cli): ts-parse lists compiled-in languages in error messages"
```

---

## Task 9: Per-language end-to-end integration tests and fixtures

**Files:**
- Create: `tests/fixtures/multilang/go/src/main.go`
- Create: `tests/fixtures/multilang/go/.lintropy/no-println.rule.yaml`
- Create: `tests/fixtures/multilang/go/lintropy.yaml`
- Create: `tests/fixtures/multilang/python/src/app.py`
- Create: `tests/fixtures/multilang/python/.lintropy/no-print.rule.yaml`
- Create: `tests/fixtures/multilang/python/lintropy.yaml`
- Create: `tests/fixtures/multilang/typescript/src/app.ts`
- Create: `tests/fixtures/multilang/typescript/.lintropy/no-console-log.rule.yaml`
- Create: `tests/fixtures/multilang/typescript/lintropy.yaml`
- Modify: `tests/integration_multilang.rs`
- Modify: `tests/cli_ts_parse.rs`

- [ ] **Step 1: Write the Go fixture**

Create `tests/fixtures/multilang/go/lintropy.yaml`:

```yaml
version: 1
```

Create `tests/fixtures/multilang/go/.lintropy/no-println.rule.yaml`:

```yaml
severity: warning
description: "Flags fmt.Println calls; use a structured logger instead."
message: "avoid fmt.Println"
language: go
query: |
  (call_expression
    function: (selector_expression
      operand: (identifier) @pkg
      field: (field_identifier) @fn)
    (#eq? @pkg "fmt")
    (#eq? @fn "Println")) @match
```

Create `tests/fixtures/multilang/go/src/main.go`:

```go
package main

import "fmt"

func main() {
    fmt.Println("hello")
}
```

- [ ] **Step 2: Write the Python fixture**

Create `tests/fixtures/multilang/python/lintropy.yaml`:

```yaml
version: 1
```

Create `tests/fixtures/multilang/python/.lintropy/no-print.rule.yaml`:

```yaml
severity: warning
description: "Flags bare print() calls; prefer logging."
message: "avoid print()"
language: python
query: |
  (call
    function: (identifier) @fn
    (#eq? @fn "print")) @match
```

Create `tests/fixtures/multilang/python/src/app.py`:

```python
def main():
    print("hello")
```

- [ ] **Step 3: Write the TypeScript fixture**

Create `tests/fixtures/multilang/typescript/lintropy.yaml`:

```yaml
version: 1
```

Create `tests/fixtures/multilang/typescript/.lintropy/no-console-log.rule.yaml`:

```yaml
severity: warning
description: "Flags console.log calls; use a structured logger."
message: "avoid console.log"
language: typescript
query: |
  (call_expression
    function: (member_expression
      object: (identifier) @obj
      property: (property_identifier) @prop)
    (#eq? @obj "console")
    (#eq? @prop "log")) @match
```

Create `tests/fixtures/multilang/typescript/src/app.ts`:

```typescript
function main() {
    console.log("hello");
}
```

- [ ] **Step 4: Add per-lang integration tests**

Append to `tests/integration_multilang.rs`:

```rust
fn fixture_root(lang: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/multilang")
        .join(lang)
}

#[cfg(feature = "lang-go")]
#[test]
fn go_fixture_flags_fmt_println() {
    let root = fixture_root("go");
    let mut cmd = assert_cmd::Command::cargo_bin("lintropy").unwrap();
    cmd.current_dir(&root).arg("check").arg("--format").arg("json");
    let output = cmd.output().unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("main.go"), "expected go diag: {stdout}");
    assert!(stdout.contains("no-println"), "rule id missing: {stdout}");
}

#[cfg(feature = "lang-python")]
#[test]
fn python_fixture_flags_print_call() {
    let root = fixture_root("python");
    let mut cmd = assert_cmd::Command::cargo_bin("lintropy").unwrap();
    cmd.current_dir(&root).arg("check").arg("--format").arg("json");
    let output = cmd.output().unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("app.py"), "expected python diag: {stdout}");
    assert!(stdout.contains("no-print"), "rule id missing: {stdout}");
}

#[cfg(feature = "lang-typescript")]
#[test]
fn typescript_fixture_flags_console_log() {
    let root = fixture_root("typescript");
    let mut cmd = assert_cmd::Command::cargo_bin("lintropy").unwrap();
    cmd.current_dir(&root).arg("check").arg("--format").arg("json");
    let output = cmd.output().unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("app.ts"), "expected ts diag: {stdout}");
    assert!(stdout.contains("no-console-log"), "rule id missing: {stdout}");
}
```

- [ ] **Step 5: Add ts-parse auto-detect tests per language**

Append to `tests/cli_ts_parse.rs`:

```rust
#[cfg(feature = "lang-go")]
#[test]
fn ts_parse_auto_detects_go_from_extension() {
    let tmp = tempfile::tempdir().unwrap();
    let file = tmp.path().join("t.go");
    std::fs::write(&file, "package main\n").unwrap();
    let mut cmd = assert_cmd::Command::cargo_bin("lintropy").unwrap();
    cmd.arg("ts-parse").arg(&file).assert().success()
        .stdout(predicates::str::contains("source_file"));
}

#[cfg(feature = "lang-python")]
#[test]
fn ts_parse_auto_detects_python_from_extension() {
    let tmp = tempfile::tempdir().unwrap();
    let file = tmp.path().join("t.py");
    std::fs::write(&file, "x = 1\n").unwrap();
    let mut cmd = assert_cmd::Command::cargo_bin("lintropy").unwrap();
    cmd.arg("ts-parse").arg(&file).assert().success()
        .stdout(predicates::str::contains("module"));
}

#[cfg(feature = "lang-typescript")]
#[test]
fn ts_parse_auto_detects_typescript_from_ts_extension() {
    let tmp = tempfile::tempdir().unwrap();
    let file = tmp.path().join("t.ts");
    std::fs::write(&file, "const x: number = 1;\n").unwrap();
    let mut cmd = assert_cmd::Command::cargo_bin("lintropy").unwrap();
    cmd.arg("ts-parse").arg(&file).assert().success()
        .stdout(predicates::str::contains("program"));
}

#[cfg(feature = "lang-typescript")]
#[test]
fn ts_parse_auto_detects_typescript_from_tsx_extension() {
    let tmp = tempfile::tempdir().unwrap();
    let file = tmp.path().join("t.tsx");
    std::fs::write(&file, "const x = <div/>;\n").unwrap();
    let mut cmd = assert_cmd::Command::cargo_bin("lintropy").unwrap();
    cmd.arg("ts-parse").arg(&file).assert().success()
        .stdout(predicates::str::contains("program"));
}
```

- [ ] **Step 6: Run tests**

```bash
cargo test
```

Expected: all new tests pass, no regressions.

- [ ] **Step 7: Commit**

```bash
git add tests/fixtures/multilang tests/integration_multilang.rs tests/cli_ts_parse.rs
git commit -m "test(multilang): per-lang fixtures + end-to-end check tests"
```

---

## Task 10: SKILL.md updates

**Files:**
- Modify: `skill/SKILL.md`

No code; doc-only. The implementer verifies every node-kind entry against `tree-sitter-<lang>`'s published `node-types.json` or by running `lintropy ts-parse` on a representative file.

- [ ] **Step 1: Bump version and update commands block**

Open `skill/SKILL.md`. Change line 1 from:

```
# version: 0.2.0
```

to:

```
# version: 0.3.0
```

In §2 commands block (around line 33), change:

```
lintropy ts-parse <file> [--lang rust]         # dump S-expression
```

to:

```
lintropy ts-parse <file> [--lang <name>]       # dump S-expression (auto-detects by extension)
```

- [ ] **Step 2: Update §3 field reference table**

Find the row for `language` (around line 99). Change its "allowed values" column from:

```
| `rust` (MVP)                                     |
```

to:

```
| `rust` \| `go` \| `python` \| `typescript`       |
```

Add a sentence to the §3 body explaining that file extensions for each language are fixed by the CLI (rule authors don't configure them). Place after the table, something like:

> The CLI owns the extension-to-language mapping (`.rs` → rust, `.go` → go, `.py`/`.pyi` → python, `.ts`/`.tsx`/`.mts`/`.cts`/`.d.ts` → typescript). Rules declare `language:` and let the CLI route files.

- [ ] **Step 3: Update §4.1 ts-parse example**

Find the example (around line 167):

```
lintropy ts-parse src/some.rs --lang rust
```

Change to:

```
lintropy ts-parse src/some.rs
```

Add a note that `--lang` is an optional override for unusual extensions.

- [ ] **Step 4: Add §4.3 Go cheat sheet + worked example**

After the existing §4.2 Rust cheat sheet (find `### 4.2` and locate its end), insert:

```
### 4.3 `tree-sitter-go` node-kind cheat sheet

| node kind                     | what it matches                                        |
|-------------------------------|--------------------------------------------------------|
| `source_file`                 | top-level file                                         |
| `function_declaration`        | `func Foo(...) { ... }`                                |
| `method_declaration`          | `func (r Receiver) Foo(...) { ... }`                   |
| `call_expression`             | `foo(a, b)`                                            |
| `selector_expression`         | `pkg.Ident`                                            |
| `identifier`                  | bare identifier                                        |
| `field_identifier`            | field/method name after `.`                            |
| `interpreted_string_literal`  | double-quoted string                                   |
| `defer_statement`             | `defer foo()`                                          |
| `go_statement`                | `go foo()`                                             |

Worked example — `.lintropy/no-fmt-println.rule.yaml`:

​```yaml
severity: warning
description: |
  Flags fmt.Println calls. Production code should emit structured logs
  through a configured logger, not stdlib fmt.
message: "avoid fmt.Println; use a structured logger"
language: go
query: |
  (call_expression
    function: (selector_expression
      operand: (identifier) @pkg
      field: (field_identifier) @fn)
    (#eq? @pkg "fmt")
    (#eq? @fn "Println")) @match
```
```

(The triple-backtick fence inside a triple-backtick block requires the implementer to use four-backtick outer fencing or escape appropriately. The above is illustrative; use whatever markdown convention the file already uses for nested code blocks.)

- [ ] **Step 5: Add §4.4 Python cheat sheet + worked example**

After §4.3, insert:

```
### 4.4 `tree-sitter-python` node-kind cheat sheet

| node kind                   | what it matches                                    |
|-----------------------------|----------------------------------------------------|
| `module`                    | top-level file                                     |
| `function_definition`       | `def foo(...):`                                    |
| `call`                      | `foo(a, b)`                                        |
| `attribute`                 | `obj.attr`                                         |
| `identifier`                | bare identifier                                    |
| `string`                    | string literal                                     |
| `import_statement`          | `import foo`                                       |
| `import_from_statement`     | `from foo import bar`                              |
| `class_definition`          | `class Foo:`                                       |
| `decorator`                 | `@foo`                                             |

Worked example — `.lintropy/no-print-in-prod.rule.yaml`:

​```yaml
severity: warning
description: |
  Flags bare print() calls. print() bypasses the logging module and
  makes log levels/destinations unconfigurable in production.
message: "avoid print(); use logging.getLogger(__name__)"
language: python
query: |
  (call
    function: (identifier) @fn
    (#eq? @fn "print")) @match
```
```

- [ ] **Step 6: Add §4.5 TypeScript cheat sheet + worked example**

After §4.4, insert:

```
### 4.5 `tree-sitter-typescript` node-kind cheat sheet

| node kind                     | what it matches                                        |
|-------------------------------|--------------------------------------------------------|
| `program`                     | top-level file                                         |
| `function_declaration`        | `function foo() { ... }`                               |
| `arrow_function`              | `(x) => x`                                             |
| `call_expression`             | `foo(a, b)`                                            |
| `member_expression`           | `obj.prop` / `obj["prop"]`                             |
| `identifier`                  | bare identifier                                        |
| `property_identifier`         | property name after `.`                                |
| `import_statement`            | `import ... from "mod"`                                |
| `type_alias_declaration`      | `type T = ...`                                         |
| `interface_declaration`       | `interface I { ... }`                                  |
| `jsx_element` (tsx only)      | `<Foo>...</Foo>` (only present when parsing `.tsx`)    |

Rule authors write `language: typescript` for both `.ts` and `.tsx`
files. The CLI picks the `typescript` vs `tsx` grammar per file based
on the extension. A rule using tsx-only node kinds (e.g. `jsx_element`)
will silently match nothing in `.ts` files — this is correct.

Worked example — `.lintropy/no-console-log.rule.yaml`:

​```yaml
severity: warning
description: |
  Flags console.log calls. Shipping code should emit through a
  structured logger so levels, sampling, and sinks are configurable.
message: "avoid console.log; use a structured logger"
language: typescript
query: |
  (call_expression
    function: (member_expression
      object: (identifier) @obj
      property: (property_identifier) @prop)
    (#eq? @obj "console")
    (#eq? @prop "log")) @match
```
```

- [ ] **Step 7: Add `language: tsx` anti-pattern to §9**

Find `## 9.` (anti-patterns section). Append an entry:

```
- **Don't pick `language: tsx`.** There is no `tsx` variant. Write
  `language: typescript` for both `.ts` and `.tsx` files. The CLI
  selects the `typescript` vs `tsx` grammar per file based on the
  extension.
```

- [ ] **Step 8: Verify**

```bash
cargo test --test cli_skill_contents 2>/dev/null || true
```

If a SKILL.md contents test exists (check `tests/` for any file asserting substrings of SKILL.md), run it. If it asserts on old wording (e.g. the `--lang rust` example), update the assertion.

Manual check: `rg "version:" skill/SKILL.md` → exactly one line, `# version: 0.3.0`.

- [ ] **Step 9: Commit**

```bash
git add skill/SKILL.md
git commit -m "docs(skill): multi-language sections for go/python/typescript"
```

---

## Task 11: CHANGELOG entry

**Files:**
- Modify: `CHANGELOG.md`

- [ ] **Step 1: Add entry**

Open `CHANGELOG.md`. Add under the `## Unreleased` heading (create the heading if absent):

```markdown
## Unreleased

### Added

- Language support for **Go**, **Python**, and **TypeScript** (including
  `.tsx`). Enabled by default via Cargo features `lang-go`,
  `lang-python`, `lang-typescript`. Build a Rust-only binary with
  `cargo install lintropy --no-default-features`.
- `lintropy ts-parse` auto-detects language from file extension;
  `--lang` becomes an explicit override listing every compiled-in
  language in its error messages.

### Changed

- Internal API: `Language::ts_language` now takes a `&Path` argument
  used to pick the `typescript` vs `tsx` grammar for TypeScript files.

### Notes

- Default binary grows by ~5–7 MB due to bundled tree-sitter grammars.
```

- [ ] **Step 2: Commit**

```bash
git add CHANGELOG.md
git commit -m "chore(changelog): note go/python/typescript language support"
```

---

## Task 12: Final sweep and polish

**Files:**
- Potentially: any file flagged by fmt / clippy / tests.

- [ ] **Step 1: Run full test matrix**

```bash
cargo test
cargo test --no-default-features
cargo test --features lang-go
cargo test --features lang-python
cargo test --features lang-typescript
```

Expected: all five invocations pass.

- [ ] **Step 2: Run fmt + clippy**

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo clippy --all-targets --no-default-features -- -D warnings
```

Expected: no diffs, no warnings. If fmt fails, run `cargo fmt --all` and add the diff.

- [ ] **Step 3: Smoke-test the CLI end-to-end**

```bash
# Rust auto-detect (existing behavior)
cargo run --quiet -- ts-parse examples/rust-demo/src/main.rs | head -5

# Go auto-detect on the new fixture
cargo run --quiet -- ts-parse tests/fixtures/multilang/go/src/main.go | head -5

# Python auto-detect
cargo run --quiet -- ts-parse tests/fixtures/multilang/python/src/app.py | head -5

# TypeScript auto-detect
cargo run --quiet -- ts-parse tests/fixtures/multilang/typescript/src/app.ts | head -5

# Unknown lang error message
cargo run --quiet -- ts-parse /tmp/foo.unknown 2>&1 | head -5
```

Expected: first four print S-expressions starting with the correct top-level node (`source_file`, `source_file`, `module`, `program`). Fifth prints an error listing available languages.

- [ ] **Step 4: (If anything needed touching) commit the fixes**

Only if fmt / clippy / smoke tests surfaced issues:

```bash
git add -p
git commit -m "fix: post-review cleanup"
```

---

## Self-Review Notes

- **Spec coverage:**
  - §3.1 `Language` enum additions → T3/T4/T5.
  - §3.2 `from_name` → T3/T4/T5.
  - §3.3 `from_extension` + `.d.ts` compound suffix → T5.
  - §3.4 `extensions()` → T3/T4/T5.
  - §3.5 `ts_language(&Path)` signature + tsx dispatch → T2 (signature) + T5 (dispatch).
  - §3.5.1 dual-compile for TypeScript → T6.
  - §3.6 Cargo features → T1.
  - §4.1 `ts-parse` error-message tightening → T8 (current auto-detect already exists).
  - §4.2 no changes → verified by existing integration tests staying green in T7/T9.
  - §5 SKILL.md updates → T10.
  - §6.1 unit tests → distributed across T3/T4/T5/T6.
  - §6.2 integration tests → T7 (tsx dispatch), T8 (ts-parse errors), T9 (per-lang check).
  - §6.3 fixtures → T9.
  - §7 backwards compat — verified by running full test matrix in T12.
  - §8 file-level summary — matches the File Structure section above.
- **Placeholder scan:** `<pinned>` in T1 step 2 is an explicit delegation with a documented selection rule (newest compatible with `tree-sitter = "0.22"`); the implementer is told to escalate rather than silently adjust. Not a TBD.
- **Type consistency:**
  - `Language::ts_language(self, &Path) -> TsLanguage` introduced in T2 and used in T5/T6/T7 with the same signature.
  - `QueryRule.compiled_tsx: Option<Arc<TsQuery>>` introduced in T6, consumed in T7 (`pick_compiled`).
  - `language_from_path` defined in T5, referenced only internally (not by other tasks), so no cross-task consistency risk.
- **Known duplication:** T8 `available_langs()` helper duplicates the compiled-in-language list logic with T5 (`Language::from_name`). Call sites = 2 (`ts-parse` error messages, one each). Hoisting noted as YAGNI. If SKILL.md content ever becomes programmatically generated, revisit.
- **Hazards flagged:**
  - T6 step 4 destructures `(compiled, compiled_tsx)` from a helper and stops using the `Arc::new(compiled)` wrapping in the `RuleKind::Query { ... }` constructor. Implementer must verify there are no other call sites in `config.rs` relying on `compiled` being a raw `TsQuery` (grep before changing).
  - T7 step 4 assumes `run_file` has easy access to the `RuleConfig` when calling the matcher. If the current code passes only `&QueryRule` or only `&TsQuery` into the matcher call chain, add a path argument down the chain or compute `pick_compiled` upstream. Inspect during implementation.
  - `tree-sitter-python` `call` node: the `function:` field is often an `attribute` or `identifier`. The T9 Python fixture intentionally uses the `identifier` form to avoid false-negatives in the test; a real production rule would need both shapes.
