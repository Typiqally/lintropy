# lintropy: Jelle vs Rens — spec comparison

**Date:** 2026-04-18
**Sources:**
- Jelle — `specs/jelle/2026-04-18-lintropy-design.md`
- Rens — `specs/rens/lintropy_dsl_mvp_df2a61d0.plan.md`

Both specs describe the same product name (`lintropy`) and share the same
core thesis: most interesting lints in a real codebase are
**project-specific** conventions that no upstream linter will ever ship, so
the linter's primary surface should be a config file that authors (humans
*or* agents) extend in-repo. Everything below is what they disagree on, or
what one spec covers that the other does not.

## 1. One-line framing

| | Jelle | Rens |
|---|---|---|
| Pitch | A linter whose **rule set** is declared in a single config file optimised for AI authors. | A **tree-sitter-DSL** linter where each rule is a query + message + (optional) fix. |
| Center of gravity | A curated **catalog** of 25 high-level rule types, plus a bounded `custom` escape hatch. | A single rule primitive (`query` + `message` + `fix`) that the user composes into whatever they need. |

Jelle is **catalog-first** with an escape hatch. Rens is **escape-hatch-first**
with recipes.

## 2. Config format and file layout

| | Jelle | Rens |
|---|---|---|
| Format | YAML or JSON | TOML |
| File(s) | One `lintropy.yaml` at repo root | `lintropy.toml` at repo root **plus** auto-discovered `.lintropy/**/*.rule.toml` (single-rule, `id` defaults to file stem) and `.lintropy/**/*.rules.toml` (multi-rule). |
| Subdirs / ownership | Single file (no nesting in v0.1; `extends:` deferred). | First-class: per-rule files designed for CODEOWNERS, easy `git rm` to disable, organisational subdirs (`.lintropy/architecture/…`). |
| Globals | `version`, `project`, `settings`, `include`, `exclude`, `variables`. | `[lintropy]` block (`fail_on`); per-rule `include` / `exclude`. |

Rens optimises for **scaling to hundreds of rules** in a real repo. Jelle
optimises for **the single artifact an agent reads, validates, and rewrites
in one shot**.

## 3. The rule model

### Jelle — typed catalog

- 25 named rule types (`path-pattern`, `filename-pattern`,
  `identifier-naming`, `mirror-file`, `directory-shape`, `file-presence`,
  `forbidden-pattern`, `required-pattern`, `layer-boundary`,
  `import-allowlist`, `no-circular-deps`, `codeowners-coverage`,
  `manifest-invariant`, `generated-in-sync`, `env-example-in-sync`,
  `frontmatter-required`, `doc-coverage`, `readme-per-dir`, `size-budget`,
  `todo-lifecycle`, `import-hygiene`, `test-discipline`,
  `feature-flag-hygiene`, `migration-discipline`, `i18n-hygiene`).
- Each rule has its own typed `params` block, JSON-Schema-described, with
  stable ids/fields once released.
- Escape hatch: a **`custom` rule** with three predicate kinds
  (`path` regex, `ast` tree-sitter query, `relation` cross-file
  existence/count). One predicate kind per stanza.
- Cross-file and structural rules (mirror files, directory shape,
  CODEOWNERS, generated-in-sync, env-example-in-sync, migration ordering)
  are **first-class** catalog entries.

### Rens — single primitive

- Every rule is the same shape: `language`, `query` (tree-sitter
  S-expression), `message`, `severity`, optional `include` / `exclude` /
  `fix`.
- The "catalog" is not encoded in the engine; it lives as **recipes** in
  the SKILL.md and example configs (banned APIs, layered imports,
  migrations, taxonomies, test discipline, dated deprecations, builder
  enforcement, etc.).
- Cross-file structural concerns (mirror files, directory shape, manifest
  invariants, CODEOWNERS coverage, env-example sync) are **not addressed**
  by the MVP — the primitive is single-file, single-language AST queries.
- Custom predicates (`#has-ancestor?`, `#not-has-ancestor?`,
  `#has-parent?`, `#has-sibling?` and negations) are pluggable via one
  enum variant + one `apply` arm.

The trade-off is sharp:

- **Jelle** can express "every package has a README" or "no circular
  imports" out of the box, in two lines of YAML, but adding a brand-new
  rule kind requires Rust work or falling back to `custom`.
- **Rens** can express anything tree-sitter can match in two lines of
  TOML, but cannot express path/structure/relational rules at all in v1
  (those would require non-AST predicate kinds analogous to Jelle's
  `relation`).

## 4. Languages

| | Jelle | Rens |
|---|---|---|
| v0.1 / MVP | Rust, Go, TypeScript (tree-sitter). | Rust only. Adding a language = one `lang.rs` arm + one Cargo dep. |
| AST stack | tree-sitter, isolated in a `lintropy-langs` crate. | tree-sitter, in `src/lang.rs`. |
| Non-AST rules | Many catalog rules are pure path/text, no parser cost. | All rules are AST queries; non-AST cases (path-only) are not in scope. |

## 5. Suggestions and autofix

| | Jelle | Rens |
|---|---|---|
| Position | Diagnostic carries a structured **suggestion** (`replace_span`, `insert_at`, `move_file`, `create_file`); engine **never mutates files** in v0.1. Agents apply. | Engine produces `Fix { range, replacement }`; CLI **applies them in place** with `--fix`, or shows unified diff with `--fix-dry-run`. |
| Fix template | n/a — suggestions are concrete strings/paths emitted by the rule. | `fix = '…{{capture}}…'` template, `{{name}}` interpolation from query captures, same syntax as `message`. |
| Conflict handling | Out of scope (agent's problem). | Sort fixes per file by start byte descending, drop overlapping fixes (warn so user re-runs). |

Rens treats the linter as a **code-modification tool**; Jelle treats it as
a **diagnostic-emission tool** that hands structured edits to a downstream
applier (agent or SARIF consumer).

## 6. Output formats

| Format | Jelle | Rens |
|---|---|---|
| Text (rustc-style) | yes | yes |
| JSON | yes (`{ version, diagnostics: [...], summary }`) | yes (top-level array of diagnostic objects) |
| SARIF 2.1.0 | yes | yes |
| Write to file (`-o`) | not specified | yes, atomic via `tempfile` + rename |
| Color toggle | implicit | explicit `--no-color`, auto-disabled on file/pipe |

Both converge on the same three formats and on rustc-style text.
Diagnostic schemas differ in field names but cover the same information
(rule id, severity, file, span, message, suggestion/fix, source-of-rule
link).

## 7. Suppression / ignore

| | Jelle | Rens |
|---|---|---|
| In-source ignore | `// lintropy-ignore: <rule-id>` (next line) and `// lintropy-ignore-file: <rule-id>` (whole file, must appear in first 20 lines). Rejects `*` blanket suppress. Unused-suppression meta-rule. | Not specified. |
| Severity-based gate | `settings.fail_on` (defaults to `error`). | `[lintropy] fail_on = "warning"` (same idea). |
| Exit codes | Distinguishes config errors (2) and internal errors (3) from "diagnostics found" (1). | `check` exits 1 when diagnostics ≥ `fail_on`; other codes not enumerated. |

Jelle goes much further on **graceful adoption** primitives (suppression
syntax, unused-suppress detection, baseline file as a known follow-up).
Rens does not address suppression at all in the MVP.

## 8. Agent-authoring tooling

This is where the specs diverge most in **strategy**, even though both
target agent authoring.

### Jelle — agent tools as CLI subcommands

A purpose-built CLI surface designed so an agent can author a config
end-to-end without prose instructions:

- `lintropy schema` — emits the full config JSON Schema (derived via
  `schemars`), inlines all rule param schemas (no `$ref`), every field
  has a description sentence.
- `lintropy rules list` / `rules show <id>` / `explain <id>` — discoverable
  catalog with worked examples and resolved-predicate dumps.
- `lintropy init --describe` — writes a starter config with **every
  catalog rule, commented out** + per-rule explanation. Agent uncomments
  what fits.
- `lintropy init --from-agent-output <draft>` — agent emits a bare rules
  draft; CLI folds it into a complete, schema-validated config. Re-runnable
  to merge.
- `lintropy config validate` — schema-validate without running.
- `lintropy config explain` — dry-run; prints each rule's resolved scope
  and sample matched files.

The intended workflow is a **deterministic 7-step pipeline**
(`init --describe → schema → rules list → edit → validate → explain →
check --format json`).

### Rens — agent tools as a Skill + helpers

- `lintropy init --with-skill` writes `.cursor/skills/lintropy/SKILL.md`
  into the user's repo; the skill is also `include_str!`-embedded in the
  binary.
- The skill teaches the DSL in 9 sections: what lintropy is, how to run
  it, anatomy of a rule, **writing tree-sitter queries** (the longest
  section, with node-kind cheatsheet, predicate cheatsheet, capture
  conventions), 8 copy-pasteable recipes, how to interpret diagnostics,
  decision tree for fixing them, the `ts-parse` helper, anti-patterns.
- `lintropy ts-parse <file>` — print S-expression to iterate on queries.
- `lintropy explain <rule-id>` and `lintropy rules` — discoverability of
  loaded rules, including their `source_path`.

The bet is: agents in Cursor have skills; teach them the DSL once via
SKILL.md and let them author rules the same way they write any other
TOML file.

### Differences in posture

| Question | Jelle | Rens |
|---|---|---|
| What does the agent edit? | A high-level **catalog selection** with typed params. Most edits are 5-line stanzas referencing a known rule id. | Tree-sitter S-expressions. Most edits are 5–15 lines of query syntax. |
| How does the agent learn it? | By reading the JSON Schema + `rules list` + `rules show`, all derived from typed Rust. | By reading SKILL.md (markdown, hand-authored, shipped with the binary). |
| How does the agent verify scope? | `config explain` dry-run prints sample matched files. | Run `lintropy check` and read diagnostics; iterate via `ts-parse`. |
| How does the agent author "the project's rules" from scratch? | `init --describe` (full commented catalog) or `init --from-agent-output` (draft → CLI scaffolds). | `init --with-skill` then write `.lintropy/<name>.rule.toml` files by hand using recipes. |

## 9. Execution model

| | Jelle | Rens |
|---|---|---|
| File walk | `ignore` crate (gitignore-aware) + `rayon` parallel dispatch. | `ignore` crate (gitignore-aware). Parallelism not specified for MVP. |
| Globs | `globset`, top-level `include`/`exclude` + per-rule `paths`. | `globset`, per-rule `include`/`exclude` (default to language extensions). |
| Changed-only mode | `lintropy check --changed` via `git diff` (PR/CI mode). | Not specified. |
| Variables | `variables: { name: "..." }` referenced as `${var}` in globs. | Not specified. |

Jelle takes monorepo / CI scale seriously up front (`--changed`, parallel
walk, top-level filters, variables). Rens leaves these for after the MVP.

## 10. Crate / module layout

| | Jelle | Rens |
|---|---|---|
| Layout | Multi-crate workspace (`lintropy-{core, langs, rules, output, cli}`) so tree-sitter weight is isolated and each rule is independently testable. | Single binary, modules under `src/` (`config.rs`, `lang.rs`, `predicates.rs`, `engine.rs`, `fix.rs`, `report.rs`, `template.rs`). |
| Schema generation | `schemars` from Rust types, never hand-written; published at a public URL for editor autoload. | n/a — TOML schema is documented in the spec; no JSON-Schema artifact. |

Jelle's layout reflects the catalog model (each catalog rule is its own
module). Rens's reflects the single-primitive model (one engine, one
rule type, configuration drives everything).

## 11. Stability story

| | Jelle | Rens |
|---|---|---|
| Stability commitment | Rule ids and params are **append-only** between major schema versions. Top-level `version` field gates migrations. Tree-sitter grammar versions pinned per release. | Not explicitly addressed — the surface is much smaller (one rule shape) so backwards compatibility burden lands on the `query` predicates and `lang` registry. |

## 12. Explicit non-goals / deferred work

Both specs enumerate non-goals and they overlap heavily: **no LSP, no
watch mode, no autofix engine** (Jelle), **no multi-language beyond
MVP, no LSP, no AST-aware fixes** (Rens). The notable cross-spec
differences:

- **Autofix:** Jelle defers it (suggestions only); Rens ships it.
- **Cross-file structural rules:** Jelle ships them as catalog entries
  in v0.1; Rens defers (no `relation`-style predicate yet).
- **Baseline file:** Jelle calls it out as the top near-term follow-up
  to enable adoption on legacy repos. Rens does not mention it.
- **Multi-language:** Jelle ships Rust/Go/TS in v0.1; Rens ships Rust
  only and treats more languages as trivial follow-ups.
- **Per-rule files / ownership:** Rens ships `.lintropy/**/*.rule.toml`
  in MVP; Jelle defers nested configs and `extends:`.

## 13. Where the specs would benefit from each other

If the two were to converge, each spec has clear gifts for the other:

- **Jelle → Rens:** the in-source suppression syntax and unused-suppress
  meta-rule, the `fail_on` exit-code matrix, the `--changed` mode for
  CI, the `relation` predicate for cross-file rules, the schema-driven
  `config validate` / `config explain` tooling, and the baseline-file
  follow-up.
- **Rens → Jelle:** the `.lintropy/**/*.rule.toml` layout (per-rule
  files, CODEOWNERS-friendly), the autofix model (`fix` template +
  `--fix-dry-run` diff + overlap handling), the SKILL.md mechanism for
  teaching the query language to agents in-repo, and the `ts-parse`
  helper subcommand for query iteration.

The two specs are largely **complementary**: Jelle defines the
high-level rule catalog and the agent-authoring CLI surface; Rens
defines the underlying query primitive, autofix mechanics, per-rule
file layout, and the in-repo SKILL.md handoff to agents. A combined
design would likely keep Jelle's catalog + JSON-Schema-driven tooling
on top of Rens's tree-sitter-query primitive (catalog rules become
"named, parameterised queries"), with Rens's per-file layout and
autofix engine as the storage and application layer.
