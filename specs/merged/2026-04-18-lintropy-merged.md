# lintropy — merged design spec

**Date:** 2026-04-18
**Status:** Draft (merged from `specs/jelle/` + `specs/rens/`)
**Target:** v0.1 (phased)

## 1. Idea

`lintropy` is a tree-sitter-backed linter whose rules are **authored in the
repo, one file each**, and designed to be written, read, and maintained by AI
coding agents as much as by humans.

Generic linters (Clippy, ESLint, Ruff) ship fixed catalogs of
language-universal rules. `lintropy` inverts that: the interesting lints in
any real codebase are the ones that encode *that team's* conventions —
architectural boundaries, migration deadlines, banned APIs, required
ceremony, domain taxonomies. `lintropy` ships **no catalog**. Every rule is a
small YAML file expressing a single pattern, written in a DSL that an LLM
(or a human) can author in under a minute.

Two rule kinds cover the space:

- **`query`** — tree-sitter query with capture-interpolated message and
  optional autofix. The primary rule kind.
- **`match`** — regex over file contents for cases where parsing is
  overkill or the target isn't parseable (markdown, SQL files, arbitrary
  text).

Both share the same stanza shape, reporter output, and suppression
mechanism.

## 2. Design principles

- **Agent-friendly** — low-ceremony YAML schema, one rule per file by
  default, short fields, every field described in the emitted JSON Schema,
  SKILL.md shipped in-tree for LLM onboarding.
- **One rule, one file** — `.lintropy/no-unwrap.rule.yaml` = the
  `no-unwrap` rule. CODEOWNERS works; `git rm` disables.
- **Tree-sitter first** — `query` is the primary kind; `match` covers what
  AST can't see.
- **No catalog** — no fixed rule universe. Examples and recipes ship in
  `SKILL.md` and `examples/`.
- **Explainable** — every diagnostic tells you where, why, and which file
  defined the rule.
- **Fail fast at config load** — malformed queries, unknown predicates,
  unknown captures, duplicate rule ids all error at load with source paths.

## 3. Target users

- Teams with strong internal conventions generic linters don't capture.
- AI-assisted codebases where agents need explicit, machine-authored
  constraints to stay inside project boundaries.
- Monorepos with directory ownership and architectural layer boundaries.
- Projects where generic style / language linting is already handled
  elsewhere and what's missing is project-specific enforcement.

## 4. Config shape

### 4.1 File layout

```
<repo root>/
  lintropy.yaml                              # anchors project root; globals + optional inline rules
  .lintropy/
    no-unwrap.rule.yaml                      # single rule; id defaults to "no-unwrap"
    no-dbg.rule.yaml
    architecture/
      domain-no-infra.rule.yaml              # subdirs purely organizational
    2026q2.rules.yaml                        # multi-rule grouping
```

**Discovery:**

1. Walk up from cwd looking for `lintropy.yaml` → that file anchors the
   project root.
2. From the project root, glob `.lintropy/**/*.rule.yaml` +
   `.lintropy/**/*.rules.yaml` (via the `ignore` crate; respects
   `.gitignore`).
3. Merge root inline rules + per-file + multi-file into one list.
4. Duplicate `id` → hard error naming both source files.
5. Every rule tagged with `source_path` for diagnostics.
6. `--config <PATH>` overrides root discovery.

### 4.2 Root `lintropy.yaml`

```yaml
version: 1

settings:
  fail_on: error           # exit non-zero for diagnostics at/above this severity
  default_severity: error  # default when a rule omits `severity`

# Inline rules optional — fine for small projects.
rules:
  - id: no-dbg
    severity: error
    message: "stray dbg!"
    language: rust
    query: |
      (macro_invocation
        macro: (identifier) @n
        (#eq? @n "dbg")) @match
```

Top-level keys: `version` (required), `settings` (optional), `rules`
(optional). No global `include`/`exclude`, no `variables`, no `project`
block — scope is per-rule; agents inline values.

### 4.3 `.lintropy/<anything>.rule.yaml` — single rule

Top-level keys *are* the rule fields; no `rules:` wrapper. `id` defaults to
the file stem if omitted.

```yaml
# .lintropy/no-unwrap.rule.yaml
severity: warning
message: "avoid .unwrap() on `{{recv}}`; use .expect(\"...\") or ?"
fix: '{{recv}}.expect("TODO: handle error")'
language: rust
query: |
  (call_expression
    function: (field_expression
      value: (_) @recv
      field: (field_identifier) @method)
    arguments: (arguments)
    (#eq? @method "unwrap")
    (#not-has-ancestor? @method "macro_invocation")) @match
```

### 4.4 `.lintropy/<anything>.rules.yaml` — multi-rule groupings

```yaml
# .lintropy/2026q2.rules.yaml
rules:
  - id: use-tracing-not-log
    severity: warning
    message: "use tracing::{{level}}! instead of log::{{level}}!"
    fix: "tracing::{{level}}!{{args}}"
    language: rust
    query: |
      (macro_invocation
        macro: (scoped_identifier
          path: (identifier) @ns
          name: (identifier) @level)
        (token_tree) @args
        (#eq? @ns "log")
        (#match? @level "^(trace|debug|info|warn|error)$")) @match

  - id: no-console-log
    severity: error
    message: "no console.log in src/"
    include: ["src/**/*.ts"]
    exclude: ["**/*.test.ts"]
    forbid: 'console\.log'
```

### 4.5 Per-rule stanza

| key        | type     | required                   | purpose                                          |
|------------|----------|----------------------------|--------------------------------------------------|
| `id`       | string   | inline + `*.rules.yaml`    | user label; defaults to file stem in `*.rule.yaml` |
| `severity` | enum     | no (→ `default_severity`)  | `error` \| `warning` \| `info`                   |
| `message`  | string   | yes                        | user-facing text; `{{capture}}` interpolation    |
| `include`  | string[] | no                         | gitignore-style glob scope (inclusive)           |
| `exclude`  | string[] | no                         | gitignore-style glob scope (exclusive)           |
| `tags`     | string[] | no                         | free-form grouping / filter                      |
| `docs_url` | string   | no                         | surfaced in diagnostics                          |
| `language` | enum     | yes when `query:` present  | `rust` (MVP); `go` / `typescript` (phase 2)      |
| `query`    | string   | one of `query`/`forbid`/`require` | tree-sitter S-expression query            |
| `forbid`   | string   | one of `query`/`forbid`/`require` | regex; match = violation                  |
| `require`  | string   | one of `query`/`forbid`/`require` | regex; absence in file = violation        |
| `multiline`| bool     | no (match rules only)      | regex multiline/dotall; default `false`          |
| `fix`      | string   | no (`query` rules only)    | replacement for `@match` span; `{{capture}}` interp |

Discriminator is key presence:

- `query:` set → **query rule** (AST). `language:` required.
- `forbid:` and/or `require:` set → **match rule** (regex). `multiline:`
  optional.
- Exactly one mode per rule: `query` may not coexist with `forbid`/`require`.
  `forbid` + `require` together is allowed (two checks per file).

No nested block, no explicit `kind:` field.

## 5. Rule kinds

### 5.1 Query rules (tree-sitter)

```yaml
id: no-unwrap
severity: warning
message: "avoid .unwrap() on `{{recv}}`"
fix: '{{recv}}.expect("TODO: handle error")'
language: rust
query: |
  (call_expression
    function: (field_expression
      value: (_) @recv
      field: (field_identifier) @method)
    (#eq? @method "unwrap")) @match
```

Semantics:

- Each query match against a file in the rule's `language` produces one
  diagnostic.
- Diagnostic span = the `@match` capture if present, else the whole match
  root. Rules without `@match` warn at config load (vague span).
- Built-in tree-sitter predicates supported free via `QueryCursor`:
  `#eq?`, `#not-eq?`, `#match?`, `#not-match?`, `#any-of?`.
- Custom general predicates (§6) applied as post-filter per match.
- `{{capture}}` in `message` and `fix` substitutes the captured node's
  source text. Unknown capture name → **hard error at config load** (fail
  fast).
- Queries compiled at config load; compile failures name the rule + source
  file + offending line/col.

### 5.2 Match rules (regex)

```yaml
id: no-console-log
severity: error
message: "no console.log in src/"
include: ["src/**/*.ts"]
exclude: ["**/*.test.ts"]
forbid: 'console\.log'
```

```yaml
id: license-header
severity: error
message: "missing SPDX license header"
include: ["**/*.rs"]
require: '^// SPDX-License-Identifier:'
```

Semantics:

- `forbid`: every regex match = one diagnostic. Span = match range. Regex
  capture groups usable as `{{1}}`, `{{2}}`... in `message` (`{{0}}` =
  whole match).
- `require`: file without a match = one diagnostic. Span = file-level
  (line 1, col 1).
- `multiline: true` enables regex `m` + `s` flags (so `^`/`$` match line
  boundaries and `.` matches newlines).
- No `fix:` field on match rules — regex-driven autofix lands in a later
  phase if the need proves itself.

## 6. Custom tree-sitter predicates

Applicable to query rules only.

Built-in (free from `QueryCursor`): `#eq?`, `#not-eq?`, `#match?`,
`#not-match?`, `#any-of?`.

Custom (host-applied post-filter, parsed at config load — unknown predicate
name = hard error):

| predicate                              | meaning                                          |
|----------------------------------------|--------------------------------------------------|
| `#has-ancestor? @cap "kind"...`        | capture has an ancestor of any named kind        |
| `#not-has-ancestor? @cap "kind"...`    | negation                                         |
| `#has-parent? @cap "kind"...`          | immediate parent matches                         |
| `#not-has-parent? @cap "kind"...`      | negation                                         |
| `#has-sibling? @cap "kind"...`         | some sibling matches                             |
| `#not-has-sibling? @cap "kind"...`     | negation                                         |
| `#has-preceding-comment? @cap "regex"` | nearest preceding comment matches regex          |
| `#not-has-preceding-comment? @cap "regex"` | negation                                     |

Implementation: `src/predicates.rs` defines
`enum CustomPredicate { ... }` + `fn apply(&self, &QueryMatch, &Node) -> bool`.
New predicate = one variant + one `apply` arm. No plugin system in v0.1.

Path-scoped predicates (`#filename-matches?`, `#in-file?`) intentionally
omitted — `include` / `exclude` globs cover them.

## 7. Diagnostics and output

### 7.1 Canonical diagnostic

```jsonc
{
  "rule_id": "no-unwrap",
  "severity": "warning",
  "message": "avoid .unwrap() on `client`",
  "file": "src/handlers/users.rs",
  "line": 42, "column": 18,
  "end_line": 42, "end_column": 33,
  "byte_start": 1284, "byte_end": 1299,
  "rule_source": ".lintropy/no-unwrap.rule.yaml",
  "docs_url": null,
  "fix": {
    "replacement": "client.expect(\"TODO: handle error\")",
    "byte_start": 1284, "byte_end": 1299
  }
}
```

### 7.2 Text format (default)

Rustc-style block, one diagnostic at a time. Colored on TTY via
`owo-colors`; auto-disabled for pipes, `--output`, `--no-color`.

```
warning[no-unwrap]: avoid .unwrap() on `client`
  --> src/handlers/users.rs:42:18
   |
42 |     let user = client.unwrap().get(id).await?;
   |                ^^^^^^^^^^^^^^^ help: replace with `client.expect("TODO: handle error")`
   |
   = rule defined in: .lintropy/no-unwrap.rule.yaml
   = see: lintropy explain no-unwrap

Summary: 1 warning across 1 file. 1 autofix available — re-run with --fix.
```

### 7.3 JSON format

```jsonc
{
  "version": 1,
  "diagnostics": [ /* canonical diagnostic objects */ ],
  "summary": {
    "errors": 0,
    "warnings": 1,
    "infos": 0,
    "files_checked": 412,
    "duration_ms": 820
  }
}
```

### 7.4 SARIF

**Deferred to phase 2.** SARIF 2.1.0 via `serde-sarif`; `rule_source` →
`properties.ruleSource`; `fix` → SARIF `fixes` entries.

### 7.5 Suppression

In-source comments, scanned once per file:

```
// lintropy-ignore: <rule-id>[, <rule-id>...]       # next non-comment line
// lintropy-ignore-file: <rule-id>[, <rule-id>...]  # whole file; first 20 lines
```

- Must be on its own line (no trailing-code form).
- `*` wildcard **not supported** — agents would blanket-muzzle.
- Unknown rule id in a directive → `warning` from the always-on
  `suppress-unused` meta-rule (engine-internal; not configurable in v0.1).

### 7.6 Exit codes

| code | meaning |
|------|---------|
| 0    | no diagnostics at or above `settings.fail_on` |
| 1    | one or more diagnostics at or above `fail_on` |
| 2    | config load / schema / parse failure          |
| 3    | internal error (rule crash, tree-sitter panic caught) |

`fail_on` defaults to `error`.

### 7.7 `--output`

Atomic write via `tempfile` + rename. Auto-disables color on text format.
Exit code unchanged by `--output`; purely destination.

## 8. Autofix

- Query rules only. Match rules don't autofix in v0.1.
- `fix:` field = `{{capture}}`-interpolated replacement for the `@match`
  span (or query root if no `@match`).
- Default run (no flag): `help: replace with <fix>` printed inline + `fix`
  object in JSON.
- `--fix`: collect per-file fixes, sort by start-byte descending, drop
  overlaps (warning emitted, user re-runs), splice bytes, atomic file
  write.
- `--fix-dry-run`: unified diff via `similar` crate, exit 0.
- Single-pass only for MVP — no fixpoint iteration. User re-runs if fixes
  cascade.
- Structured suggestion kinds beyond `replace_span` (`insert_at`,
  `move_file`, `create_file`) deferred to phase 3.

## 9. CLI surface

```
lintropy check [PATHS...]                  # default subcommand
    --config <PATH>                        # override root discovery
    --format <text|json>                   # default text
    --output, -o <PATH>                    # atomic file write
    --fix                                  # apply fixes in place
    --fix-dry-run                          # print unified diff
    --no-color
    --quiet

lintropy explain <rule-id>                 # print message, query, fix, source path
lintropy rules [--format json]             # list loaded rules
lintropy init                              # scaffold lintropy.yaml + .lintropy/
    --with-skill                           # also write SKILL.md into agent dir
lintropy schema                            # emit JSON Schema (LLM grounding)
lintropy config validate [path]            # schema + queries + predicates, no run
lintropy ts-parse <file> [--lang <name>]   # S-expression dump; query-iteration loop
```

**Deferred** (phase 2+): `check --changed [--since <ref>]`,
`init --describe`, `init --from-agent-output`, `config explain`,
`--format sarif`.

## 10. Agent tooling

### 10.1 `lintropy schema`

Emits the whole-config JSON Schema to stdout. Derived from Rust types via
`schemars` — never hand-written. Every field has `description`. Rule
stanza is a `oneOf` discriminated on key presence (`query` vs
`forbid`/`require`). Published at `https://lintropy.dev/schema/v1.json`
once the domain exists; `lintropy schema` is sufficient for MVP.

### 10.2 `SKILL.md`

Authored in-repo at `skill/SKILL.md`, embedded into the binary via
`include_str!`, written to the user's repo by `lintropy init --with-skill`
(primary target: `.claude/skills/lintropy/SKILL.md`; also
`.cursor/skills/lintropy/` if `.cursor/` detected; `--skill-dir <path>`
overrides).

Idempotent re-run upgrades with a `# version: <semver>` header.

**Sections** (intended to make an LLM competent in one read):

1. What lintropy is + trigger phrases (`"write a lintropy rule"`,
   `"lintropy diagnostic"`, `"add a lint"`, `.rule.yaml`).
2. Commands (`check`, `check --fix`, `check --format json -o report.json`,
   `explain`, `rules`, `ts-parse`).
3. Rule anatomy — annotated `.rule.yaml` covering every field, required
   vs optional, id-from-file-stem default, query-vs-match discriminator.
4. Writing the tree-sitter query:
   - Always run `lintropy ts-parse <file>` first to inspect node kinds.
   - Node-kind cheat sheet for `tree-sitter-rust`.
   - Built-in predicates (`#eq?`, `#not-eq?`, `#match?`, `#not-match?`,
     `#any-of?`).
   - Custom predicates (has-ancestor/parent/sibling + negations,
     has-preceding-comment + negation).
   - `@match` convention + `{{capture}}` interpolation.
5. Writing a match (regex) rule (`forbid` / `require` / `multiline`;
   capture groups as `{{0}}`, `{{1}}`…).
6. Common recipes — copy-paste starters for banned API, layered import
   boundary, migration with autofix, required ceremony (TODO ticket ref,
   SAFETY comment), taxonomy regex, test discipline (`no #[ignore]`),
   dated deprecation with autofix, builder enforcement, license-header
   requirement.
7. Interpreting diagnostics — rustc block, `--> file:line:col`, source-
   context line, `help:` line, `= rule defined in:` footer. To fix the
   source, edit the line under `^^^`; to tune the rule, edit the file
   under `rule defined in:`.
8. Fix decision tree:
   - Real problem → fix source (use `--fix` if available; review diff).
   - Rule too broad → add predicate / tighten `exclude`.
   - Rule wrong → `lintropy explain <rule-id>`, edit source file, re-run
     `lintropy config validate`.
   - Query failing to parse → load-time validator prints offending
     line/col inside the YAML; fix and re-run.
9. Anti-patterns — don't omit `@match` (vague span); don't set `id` in
   `.rule.yaml` files unless overriding the stem; don't
   `exclude: ["**/*"]` and re-add — use `include`; don't write a match
   rule where a query rule would be precise (or vice versa — don't write
   a 30-line AST query for a text pattern `grep` would catch).

### 10.3 `ts-parse <file>`

Ships MVP. Thin wrapper over `tree_sitter::Parser`; prints the
S-expression so agents iterate on queries. Language from extension,
`--lang <name>` override.

## 11. Execution model

- `lintropy check [PATHS...]` walks the tree (respecting `.gitignore` via
  `ignore` crate), groups files by language, parses on demand, runs every
  rule whose `include`/`exclude` globs match, emits diagnostics, applies
  suppression, prints output, exits with the appropriate code.
- File walk + per-file rule dispatch parallelised via `rayon`.
- Per-language parse cache per file: parse once, run all query rules for
  that language against the same tree.
- Match rules skip parsing entirely — read file bytes, run regex.

## 12. Stability

- Rule modes (`query`, `match`), stanza keys, predicate names, and
  diagnostic JSON schema are append-only before a `version: 2` bump.
- `version` field at the top of `lintropy.yaml` gates schema migrations.
- Tree-sitter grammar versions pinned per `lintropy` release.
- Custom predicate enum is append-only — new predicates add variants; no
  renames or removals pre-v2.

## 13. Phasing

### Phase 1 — MVP

- Config loader (YAML; root `lintropy.yaml` + `.lintropy/**/*.{rule,rules}.yaml`).
- Query rule kind only. Rust grammar (`tree-sitter-rust`).
- All 6 structural custom predicates + `has-preceding-comment?` pair.
- `{{capture}}` interpolation in `message` + `fix`.
- Autofix: `--fix` + `--fix-dry-run`.
- Reporters: text + JSON (SARIF deferred).
- Suppression (`// lintropy-ignore:` + `-ignore-file:`).
- CLI: `check`, `explain`, `rules`, `init [--with-skill]`, `schema`,
  `config validate`, `ts-parse`.
- `SKILL.md` embedded.
- Example repo `examples/rust-demo/` with `no-unwrap`, `no-println`,
  `no-todo`, `user-use-builder`.

### Phase 2

- Match rule kind (`forbid` / `require` / `multiline`).
- Additional grammars: Go, TypeScript.
- `check --changed [--since <ref>]` (`git2` dep).
- SARIF output (`serde-sarif`).

### Phase 3

- `config explain` dry-run (resolved scope + sample matches).
- Structured suggestion kinds beyond `replace_span` (`insert_at`,
  `move_file`, `create_file`) — agent-applied, not engine-applied.
- Autofix for match rules (regex capture substitution).

### Deferred (v2+)

- LSP / watch mode.
- Baseline file for gradual adoption.
- Incremental cache (file-hash-keyed skip).
- Nested configs / `extends:` / shareable presets.
- WASM plugin rules.
- Multi-pass autofix fixpoint.
- Cross-file relational rules (`for_each` + `require_exists` — dropped
  from v0.1 to keep surface tight; revisit only if real demand emerges).
- Python / Java / Kotlin / Swift / etc.

## 14. Rust implementation notes

**Dependencies (MVP):**

- `clap` — CLI parsing.
- `serde` + `serde_yaml` — config deserialisation.
- `schemars` — JSON Schema derivation.
- `globset` + `ignore` — glob matching + gitignore-honouring walk.
- `regex` — `#match?` predicates (match rules land phase 2).
- `rayon` — parallel per-file rule dispatch.
- `tree-sitter` + `tree-sitter-rust` — AST predicates.
- `anyhow` + `thiserror` — error handling.
- `owo-colors` — TTY-aware severity coloring.
- `similar` — unified diff for `--fix-dry-run`.
- `tempfile` — atomic writes for `--output` + `--fix`.
- `serde_json` — JSON reporter.
- `miette` (or hand-rolled) — text diagnostics.
- `insta` — snapshot tests.

**Crate split:** `lintropy-core`, `lintropy-langs`, `lintropy-rules`,
`lintropy-output`, `lintropy-cli`. Keeps tree-sitter weight isolated from
rule logic.

**Module layout (inside `lintropy-core` or the single crate at MVP):**

- `src/main.rs` — clap definitions, dispatch.
- `src/config.rs` — `Config`, `RuleConfig`, YAML deserialisation. Root
  discovery + `.lintropy/` glob. Tags every rule with `source_path`.
  Validates queries, `fix` captures, custom predicates, duplicate ids at
  load.
- `src/lang.rs` — `Language` enum + `fn from_name(&str) -> Option<LanguageEntry>`;
  initial entries: `rust` (`["rs"]`).
- `src/predicates.rs` — `CustomPredicate` enum, parser over
  `Query::general_predicates()`, `fn apply(&self, &QueryMatch, &Node) -> bool`.
- `src/engine.rs` — walk, parse, run queries, post-filter via custom
  predicates, build `Diagnostic` with interpolated message + optional fix.
- `src/fix.rs` — collect per-file fixes, drop overlaps, splice, atomic
  write.
- `src/report.rs` — `Reporter` trait + `TextReporter`, `JsonReporter`.
  All reporters write through a `Box<dyn Write>` for `--output` routing.
- `src/template.rs` — `{{name}}` substitution (no full template engine).
- `src/suppress.rs` — `// lintropy-ignore:` + `-ignore-file:` parsing +
  meta-rule for unused suppressions.

## 15. Example repo

`examples/rust-demo/`:

```
examples/rust-demo/
  lintropy.yaml                          # global config only (settings)
  .lintropy/
    no-unwrap.rule.yaml                  # id = "no-unwrap" (file stem)
    no-println.rule.yaml
    style.rules.yaml                     # multi-rule: no-todo + user-use-builder
  src/
    main.rs                              # triggers no-unwrap + no-println
    user.rs                              # triggers user-use-builder
  tests/
    smoke.rs                             # triggers no-todo
```

Exercises `@match` span, autofix (`no-unwrap`), custom predicate
(`#not-has-ancestor?` on `macro_invocation`), and multi-rule files.
Doubles as integration-test fixture via `cargo run -- check examples/rust-demo`.
