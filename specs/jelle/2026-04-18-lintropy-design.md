# lintropy — design spec

**Date:** 2026-04-18
**Status:** Draft (idea-stage)
**Target:** v0.1

## 1. Idea

`lintropy` is a linter where the rule set for a project is declared in a single
YAML/JSON file at the repo root. The file is designed, first and foremost, to
be generated and maintained by AI coding agents (Claude Code, Codex, Cursor,
etc.).

Most linters ship with a fixed-universe opinion (ESLint, Clippy,
golangci-lint) and treat project-specific conventions as an afterthought —
plugin authoring, JS glue, code generators. `lintropy` inverts that:
project-specific rules are the killer feature, and the config format is
optimised for a machine author, not a human one.

Typical conventions `lintropy` is built to express:

- "all `*Controller.ts` must live under `src/api/`"
- "`src/domain/**` cannot import from `src/infra/**`"
- "every file in `src/api/` must have a matching test in `tests/api/`"
- "every `TODO` in source needs a ticket reference and an owner"
- "every migration `*.up.sql` needs a sibling `*.down.sql`"
- "no `.unwrap()` in library code"
- "React components use `PascalCase.tsx`; hooks start with `use`"
- "every feature dir contains `components/`, `hooks/`, `index.ts`"
- "every deployable service has a `Dockerfile` and a `README.md`"

All of the above are declared in `lintropy.yaml` — not written in Rust, not
shipped as a plugin, not vendored from a preset.

## 2. Design principles

- **Agent-friendly** — low-ceremony schema, explicit field names, predictable
  naming, narrow rule types over terse general-purpose DSLs.
- **Repo-first** — rules work from paths and text patterns before requiring
  AST work; AST is an escape hatch, not the default.
- **Composable** — rules are simple primitives that combine via `paths`
  scoping and multiple stanzas.
- **Explainable** — every diagnostic tells the user what was expected, where,
  and why; every rule has a `docs_url`.
- **Incremental** — support adoption in existing repos through warnings,
  per-rule `ignore` comments, and (eventually) a baseline file.

## 3. Target users

- Teams with strong internal conventions that generic linters don't capture.
- AI-assisted codebases where agents need explicit constraints to stay inside
  project boundaries.
- Monorepos with directory ownership and architectural layer boundaries.
- Projects where generic style / language linting is already handled
  elsewhere (ESLint, Clippy, Ruff, golangci-lint) and what's missing is
  structure enforcement.

## 4. Goals (v0.1)

- Single YAML/JSON config at repo root declares all rules, plus optional
  top-level `include` / `exclude` / `variables` / `project` blocks.
- Config schema is LLM-friendly: one `rules` array, each rule stanza
  self-contained, strict discriminator on `id`, rich field descriptions.
- Rule catalog covers the 25 buckets in §6 out of the box.
- `custom` rule type provides an escape hatch via a bounded predicate
  language (path / AST / relation) — no Turing-complete DSL.
- Multi-language AST via tree-sitter; v0.1 ships grammars for Rust, Go,
  TypeScript.
- CLI with text / JSON / SARIF output.
- Structured diagnostic suggestions (machine-applyable) — no autofix engine.
- Two severities (`error`, `warn`) + inline `// lintropy-ignore: <rule-id>`.
- Full agent tooling: `schema`, `rules list|show`, `explain`, `init
  --describe`, `init --from-agent-output`, `config validate`, `config
  explain`.
- Parallel walk + `--changed` git-diff mode for monorepo / CI scale.
- Per-rule optional metadata: `tags` (for grouping / filtering) and
  `docs_url` (surfaced in diagnostics).

## 5. Non-goals (v0.1)

- LSP server, editor integration, watch mode.
- Autofix / diff / rollback engine.
- Nested configs, `extends:`, shareable presets.
- WASM / dylib plugin loader for custom rules.
- Incremental mtime cache.
- Languages beyond Rust / Go / TypeScript.
- Baseline file for gradual adoption on legacy repos (deferred; tracked as a
  known near-term follow-up — see §14).

## 6. Rule catalog

All catalog rules implement a common `Rule` trait; each rule is its own module
in `lintropy-rules` with its own typed params + JSON Schema. The 25 buckets
below are the intended **v0.1 set** — ids and params are stable once
released (see §13 Stability).

### Structural / layout

1. **`path-pattern`** — files matching glob X must live under dir Y.
   *Example:* `src/utils/UserController.ts` violates `*Controller.ts → src/api/`.

2. **`filename-pattern`** — filename casing + prefix/suffix rules.
   *Example:* React components use `PascalCase.tsx`; hooks start with
   `use`; test files end in `.test.ts`; migrations start with
   `YYYYMMDDHHMMSS_`.
   *Params:* `case` (`pascal` | `camel` | `snake` | `kebab` |
   `screaming_snake`), `prefix`, `suffix`, `regex`.

3. **`identifier-naming`** — identifier casing rules (fn / type / variable /
   enum-variant), evaluated via AST.
   *Example:* enum variants in `SCREAMING_SNAKE`; exported functions in
   `camelCase`; type aliases in `PascalCase`.
   *Params:* per-kind case style, `allow` / `deny` regexes.

4. **`mirror-file`** — every source file needs a peer (test, story, spec).
   *Example:* `src/api/users.ts` missing `tests/api/users.test.ts`.

5. **`directory-shape`** — required / allowed children of matched dirs.
   *Example:* every feature dir contains `components/`, `hooks/`, `index.ts`;
   route dirs may only contain `page.tsx` / `layout.tsx` / `loading.tsx`.
   *Params:* `dir_pattern`, `required_children`, `allowed_children`,
   `forbidden_children`.

6. **`file-presence`** — required files must exist in matched dirs (or at
   repo root).
   *Example:* every package has `README.md`; every service has `Dockerfile`;
   every workspace crate has `LICENSE`.
   *Params:* `dir_pattern` (or `root: true`), `require` (list of filenames
   or globs).

### Content / pattern

7. **`forbidden-pattern`** — regex/text disallowed in scope.
   *Example:* no `console.log` in `src/`; no `SELECT *` in `*.sql`.

8. **`required-pattern`** — regex/text must appear in scope.
   *Example:* license header on every `*.rs` file; `"use client"` on every
   client-only `*.tsx`.

### Architecture / dependency graph

9. **`layer-boundary`** — layer X cannot import from layer Y.
   *Example:* `src/domain/order.ts` imports `src/infra/db.ts` → violation.

10. **`import-allowlist`** — only whitelisted external deps importable in
    scope; also handles the "forbid direct use of lodash / axios / etc. in
    domain code" case.

11. **`no-circular-deps`** — import graph acyclic within scope.

### Cross-file relational

12. **`codeowners-coverage`** — every path covered by a `CODEOWNERS` entry.

13. **`manifest-invariant`** — `package.json` / `Cargo.toml` /
    `pyproject.toml` must have/lack specific fields, deps pinned, no `*`
    version ranges, license set, direct dep denylist, required scripts
    defined.
    *Example:* frontend packages may not depend on server-only libraries;
    every workspace crate needs `license = "MIT"`.

14. **`generated-in-sync`** — generated file fresh w.r.t. source.
    *Example:* `openapi.yaml` must match route definitions; `schema.sql`
    must equal `cargo run --bin dump-schema` output.

15. **`env-example-in-sync`** — `.env.example` keys superset of
    `env.get(...)` / `process.env.X` reads in code.

### Docs / metadata

16. **`frontmatter-required`** — markdown/mdx files need specific frontmatter
    fields.
    *Example:* every `docs/**/*.md` needs `title`, `owner`, `updated`.

17. **`doc-coverage`** — public fn / type / module needs docstring.
    *Example:* `pub fn` in a Rust lib missing `///` comment.

18. **`readme-per-dir`** — convenience specialisation of `file-presence` for
    the common "every module dir has `README.md`" case; keeps agent configs
    compact.

### Budgets / complexity

19. **`size-budget`** — max LOC per file, lines per fn, params per fn,
    nesting depth.

### Hygiene

20. **`todo-lifecycle`** — every `TODO` / `FIXME` has ticket ref + owner +
    optional expiry.
    *Example:* `// TODO: clean up` → violation;
    `// TODO(JIRA-1234, @jmaas, 2026-06): swap parser` → pass.

21. **`import-hygiene`** — no relative imports deeper than N; no wildcard
    re-exports; import groups ordered.

22. **`test-discipline`** — no `.only` / `.skip` / commented-out tests /
    empty `describe` in test files.

23. **`feature-flag-hygiene`** — every flag referenced in code exists in a
    registry file; flags past expiry flagged.

24. **`migration-discipline`** — migration filenames timestamped + monotonic;
    no edits to past migrations.

25. **`i18n-hygiene`** — no bare UI strings in component files; every
    `t("key")` call references a key present in every locale file.

### Escape hatch

- **`custom`** — user-defined rule via predicate (see §7). Params:
  `predicate` block + `message` + optional `suggestion`.

## 7. Predicate escape hatch

The `custom` rule supports three bounded predicate kinds. One per rule stanza
(no mixing in v0.1).

### 7.1 `path` — glob + regex

Match or forbid regex content within a file glob. Covers multiline / scoped
patterns the fixed `forbidden-pattern` / `required-pattern` rules don't
cover.

```yaml
- id: custom
  name: "no-todo-without-ticket"
  severity: warn
  paths: ["src/**/*.{ts,rs,go}"]
  predicate:
    path:
      forbid: '(?m)^\s*//\s*TODO(?!\([A-Z]+-\d+)'
  message: "TODO must reference a ticket: // TODO(ABC-123)"
```

### 7.2 `ast` — tree-sitter query

S-expression query against the parsed tree. Tree-sitter query syntax is
stable, documented, and LLMs generate it reliably (ast-grep, Grit, and GitHub
semantic all use close variants).

```yaml
- id: custom
  name: "no-unwrap-in-lib"
  severity: error
  paths: ["crates/*/src/**/*.rs"]
  exclude_paths: ["crates/*/src/bin/**", "**/*_test.rs"]
  predicate:
    ast:
      lang: rust
      query: |
        (call_expression
          function: (field_expression
            field: (field_identifier) @m (#eq? @m "unwrap"))) @bad
      report: "@bad"
  message: ".unwrap() forbidden in library code; return Result"
  suggestion:
    replace_capture: "@bad"
    with: "expect(\"TODO: handle\")"
```

- `lang` constrains the parser to one of `rust | go | typescript` (v0.1).
- `#eq?`, `#match?`, `#not-eq?` predicates supported.
- Multiple captures OK; `report` picks which becomes the diagnostic span.

### 7.3 `relation` — cross-file existence / count

Assert that for every file matching `from`, a file matching `to` exists (or
is absent, or satisfies a count condition). Generalises `mirror-file` to
arbitrary pairings.

```yaml
- id: custom
  name: "every-migration-has-rollback"
  severity: error
  paths: ["db/migrations/*.up.sql"]
  predicate:
    relation:
      for_each: "db/migrations/*.up.sql"
      require_exists: "db/migrations/{stem/\\.up$/}.down.sql"
  message: "migration missing rollback"
```

- `for_each` — glob of anchor files.
- `require_exists` / `require_absent` — template with `{stem/<regex>/}`
  capture-group substitution from the anchor filename.
- `require_count` — `{ glob, op: "==|>=|<=", n }` for N-to-M relations.

### 7.4 Shared predicate semantics

- Every `custom` rule has a `name`; it becomes `custom:<name>` in
  diagnostics.
- `message` required; `suggestion` optional.
- Predicate blocks validate against JSON Schema at config load.
- `custom.ast` queries compile against the named grammar at config load
  (fail fast on malformed queries).

## 8. Diagnostics and suggestions

Canonical diagnostic, identical across text / JSON / SARIF:

```jsonc
{
  "rule_id": "layer-boundary",
  "severity": "error",
  "message": "domain imports from infra: forbidden",
  "file": "src/domain/order.ts",
  "span": { "start": { "line": 3, "col": 1 }, "end": { "line": 3, "col": 42 } },
  "excerpt": "import { db } from \"../infra/db\";",
  "suggestion": {
    "kind": "replace_span",
    "detail": { "text": "// remove; use port instead" }
  },
  "docs_url": "lintropy://rules/layer-boundary"
}
```

### Suggestion kinds (v0.1)

| kind           | fields                                   | applied by                               |
|----------------|------------------------------------------|------------------------------------------|
| `replace_span` | `text`                                   | agent / editor swaps diagnostic span     |
| `insert_at`    | `line`, `col`, `text`                    | agent inserts text (e.g. license header) |
| `move_file`    | `new_path`                               | agent relocates file                     |
| `create_file`  | `path`, `contents` \| `template`         | agent scaffolds peer file                |

`lintropy` itself never mutates files in v0.1. Agents apply suggestions;
SARIF output exposes `replace_span` / `insert_at` as native SARIF fixes
where possible.

### Text output (default)

```
error[layer-boundary]: domain imports from infra: forbidden
  --> src/domain/order.ts:3:1
   |
 3 | import { db } from "../infra/db";
   | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
   = suggestion: remove this import; use port instead
   = see: lintropy explain layer-boundary

2 errors, 1 warning. (checked 412 files in 0.8s)
```

### Output modes

- `--format text` (default) — rustc-style diagnostics on TTY.
- `--format json` — `{ version, diagnostics: [...], summary }` — primary
  surface for agents.
- `--format sarif` — SARIF 2.1.0 for GitHub code-scanning / CI integration.

### Suppression

Two in-source comment forms, scanned once per file:

```
// lintropy-ignore: <rule-id>[, <rule-id>...]       // suppresses next non-comment line
// lintropy-ignore-file: <rule-id>[, <rule-id>...]  // whole file; must appear in first 20 lines
```

- Must be on its own line (no trailing-code form).
- Unknown rule id in an ignore directive → a `warn` diagnostic from the
  always-on `suppress-unused` meta-rule. Meta-rules are engine-internal and
  do not appear in the catalog or config; they cannot be disabled in v0.1.
- `lintropy-ignore: *` deliberately not supported — agents would
  blanket-muzzle.

### Exit codes

| code | meaning |
|------|---------|
| 0    | no diagnostics at or above the `settings.fail_on` threshold |
| 1    | one or more diagnostics at or above `settings.fail_on` |
| 2    | config load / schema validation / parse failure |
| 3    | internal error (rule crash, tree-sitter panic caught) |

`settings.fail_on` defaults to `error`; setting it to `warn` makes warnings
also exit non-zero. `warn` diagnostics are always printed regardless.

## 9. Agent-authoring tooling

The product bet is that agents write `lintropy.yaml` in one shot. The CLI
exposes the surface they need to do that reliably.

### `lintropy schema`

Emits the whole-config JSON Schema to stdout. Derived from Rust types via
`schemars` — never hand-written.

- Top-level `Config` schema.
- `rules[]` is a `oneOf` discriminated on `id`: one branch per catalog rule,
  one for `custom`. Each branch inlines its `params` schema (no `$ref`) so
  LLMs see the full shape in one view.
- `description` on every field (single sentence) — LLM grounding.
- Published at `https://lintropy.dev/schema/v1.json` for editor autoload.

### `lintropy rules list`

Prints `id` + one-line summary for every catalog rule, plus `custom`.
`--format json` emits `[{ id, summary, tags }]` for agent consumption.

### `lintropy rules show <id>` / `lintropy explain <id>`

Full page per rule: summary, params schema, 1–2 worked examples, "what it
catches", related rule ids. Works for `custom:<name>` too — dumps the
resolved predicate plus the first five real-repo matches.

### `lintropy init --describe`

Writes a starter `lintropy.yaml` containing **every catalog rule, commented
out**, each with a short "what it catches" line and a params block. Agent
reads once, uncomments what it needs, edits params. `--minimal` variant
ships three live rules and a link to `rules list`.

### `lintropy init --from-agent-output <file>`

Second agent workflow: the agent emits a bare rules draft as
`rules-draft.yaml` (or JSON), then the CLI folds it into a well-formed
`lintropy.yaml` — adds the top-level scaffolding (`version`, `settings`,
`project`), fills in defaults, schema-validates, and writes the final file.
Supports re-running to re-merge: existing rule stanzas matched by `id` +
`name` are preserved; new stanzas appended; removed stanzas dropped only
with `--prune`.

### `lintropy config validate [path]`

Schema-validates config without running checks. Reports schema violations,
unknown rule ids, duplicate `custom` names, unreachable `paths` (match zero
files → `warn`), and `ast` query compile failures.

### `lintropy config explain`

Dry-run: prints each rule with its resolved scope and sample matched files.
Lets the agent sanity-check scope before running `check`.

### Intended agent workflow

```
1. lintropy init --describe > lintropy.yaml         # or: --from-agent-output draft.yaml
2. lintropy schema > /tmp/lintropy.schema.json      # grounding
3. lintropy rules list --format json                # pick rule set
4. (agent edits lintropy.yaml)
5. lintropy config validate                         # fix schema errors
6. lintropy config explain                          # verify scope
7. lintropy check --format json                     # iterate on params
```

Every step deterministic, short, machine-parseable.

### Example agent prompt

Shipped as part of `lintropy rules show agent-prompt` (or equivalent docs
anchor), for reuse in agent harnesses:

```text
Inspect this repository and generate a lintropy.yaml config that captures
recurring structural conventions. Focus on:

  - file placement (which directories hold which file shapes)
  - filename conventions (casing, prefix, suffix, timestamps)
  - import boundaries (layers, allowed external deps)
  - required companion files (tests, schemas, rollbacks)
  - forbidden direct dependency or API usage

Use stable, human-readable rule IDs and concrete `message` fields. Prefer
catalog rules; reach for `custom` only when no catalog rule fits. Run
`lintropy config validate` and `lintropy config explain` before finalising.
```

## 10. Execution model (brief)

- `lintropy check [path]` walks the tree (respecting `.gitignore`), groups
  files by language, parses on demand, runs every rule whose `paths` glob
  matches, emits diagnostics, applies suppression, prints output, exits
  with the appropriate code.
- `lintropy check --changed` uses `git diff` against the default branch (or
  `--since <ref>`) to limit the walk to touched files — PR / CI mode.
- File walk + per-file rule dispatch parallelised via `rayon`.
- Top-level `include` / `exclude` + per-rule `paths` compose: a file must
  pass the global filter before per-rule glob matching runs.

## 11. Config shape (reference)

```yaml
version: 1

project:
  name: acme-web
  root: .                     # optional; default "."

settings:
  fail_on: error              # exit non-zero only on error-severity diagnostics
  default_severity: error

include:                       # global file filter; optional
  - "src/**/*"
  - "app/**/*"
  - "tests/**/*"
  - "db/migrations/**/*"

exclude:
  - "node_modules/**/*"
  - "dist/**/*"
  - "target/**/*"
  - "**/*.generated.ts"

variables:                     # reusable substitutions; referenced as ${var}
  api_dir: "src/api"
  shared_dir: "src/shared"
  feature_dirs:
    - "src/features"

rules:
  - id: path-pattern
    severity: error
    paths: ["src/**/*.ts"]
    tags: [structural, architecture]
    docs_url: "https://acme.dev/eng/conventions#api-dir"
    params:
      must_match:
        - { glob: "**/*Controller.ts", under: "${api_dir}/" }

  - id: layer-boundary
    severity: error
    paths: ["src/**"]
    tags: [architecture]
    params:
      layers:
        domain: ["src/domain/**"]
        infra:  ["src/infra/**"]
        api:    ["${api_dir}/**"]
      allow:
        - { from: api,    to: [domain, infra] }
        - { from: infra,  to: [domain] }
        - { from: domain, to: [] }

  - id: custom
    name: "no-unwrap-in-lib"
    severity: error
    paths: ["crates/*/src/**/*.rs"]
    exclude_paths: ["crates/*/src/bin/**"]
    tags: [hygiene, rust]
    predicate:
      ast:
        lang: rust
        query: |
          (call_expression
            function: (field_expression
              field: (field_identifier) @m (#eq? @m "unwrap"))) @bad
    message: ".unwrap() forbidden in library code"
```

### Top-level keys

| key         | type     | required | purpose                                                  |
|-------------|----------|----------|----------------------------------------------------------|
| `version`   | integer  | yes      | schema version; currently `1`                            |
| `project`   | object   | no       | metadata (`name`, `root`)                                |
| `settings`  | object   | no       | `fail_on`, `default_severity`                            |
| `include`   | string[] | no       | global file filter (gitignore-style globs)               |
| `exclude`   | string[] | no       | global negative filter                                   |
| `variables` | object   | no       | named substitutions usable as `${var}` in globs          |
| `rules`     | array    | yes      | rule stanzas                                             |

### Per-rule stanza keys

| key             | type       | required                | purpose                                     |
|-----------------|------------|-------------------------|---------------------------------------------|
| `id`            | string     | yes                     | catalog id or literal `"custom"`            |
| `name`          | string     | when `id: custom`       | identifier used in diagnostics              |
| `severity`      | enum       | no (defaults)           | `error` \| `warn`                           |
| `paths`         | string[]   | yes                     | gitignore-style globs (scoping)             |
| `exclude_paths` | string[]   | no                      | negative globs                              |
| `params`        | object     | yes (catalog)           | shape depends on `id`                       |
| `predicate`     | object     | yes (`custom`)          | one of `path` \| `ast` \| `relation`        |
| `message`       | string     | yes (`custom`)          | user-facing explanation                     |
| `suggestion`    | object     | no                      | structured suggestion                       |
| `tags`          | string[]   | no                      | free-form grouping / filter                 |
| `docs_url`      | string     | no                      | override default `docs_url` in diagnostics  |

## 12. Execution surfaces

| surface                       | v0.1 |
|-------------------------------|------|
| `lintropy check`              | yes  |
| `lintropy check --changed`    | yes  |
| `lintropy check --format ...` | yes  |
| `lintropy rules list/show`    | yes  |
| `lintropy explain`            | yes  |
| `lintropy init --describe`    | yes  |
| `lintropy init --from-agent-output` | yes |
| `lintropy config validate`    | yes  |
| `lintropy config explain`     | yes  |
| `lintropy schema`             | yes  |
| LSP / watch / autofix         | no (see §5) |

## 13. Stability

- Rule ids and params, once released, are append-only (no renames, no field
  removals) before a `version: 2` bump.
- The top-level `version` field gates schema migrations.
- Tree-sitter grammar versions pinned per `lintropy` release.

## 14. Deferred / known near-term follow-ups

Tracked as non-goals for v0.1 but likely next:

- **Baseline file** — `lintropy baseline` snapshots current violations so
  new violations fail while existing ones warn; enables gradual rule
  adoption on legacy repos.
- **LSP server** — real-time diagnostics in editors.
- **Watch mode** — `lintropy watch`.
- **Incremental cache** — file-hash-keyed skip of unchanged files.
- **Nested configs + `extends:`** — shared presets across monorepo
  packages.
- **WASM plugin rules** — sandboxed user-authored rules distributed as
  crates.
- **Autofix engine** — earliest when `replace_span` suggestions prove
  stable enough to apply mechanically.
- **Additional languages** — Python, Java, Kotlin, Swift.

## 15. Recommended first milestone (internal phasing)

Before shipping the full 25-rule catalog + tree-sitter stack, prove the
differentiator with a smaller, non-AST subset:

1. Workspace scaffold (`lintropy-core`, `lintropy-rules`, `lintropy-cli`).
2. Config loader (YAML + JSON) + JSON Schema emission.
3. Three catalog rules: `path-pattern`, `filename-pattern`,
   `forbidden-pattern` — all path/regex, no AST.
4. `custom` predicate: `path` kind only (AST + relation later).
5. `lintropy check` + `lintropy rules list/show` + text + JSON output.
6. Example configs for a TypeScript app and a Rust workspace.

That slice validates the "agent writes config → lintropy enforces" loop
without paying tree-sitter cost. AST predicates, SARIF output, `--changed`
mode, the remaining 22 catalog rules, and the full tooling suite layer on
afterwards en route to v0.1.

## 16. Rust implementation notes

Practical starting stack (v0.1):

- `clap` — CLI parsing (already present).
- `serde` + `serde_yaml` + `serde_json` — config deserialisation.
- `schemars` — JSON Schema derivation from Rust types.
- `globset` + `ignore` — glob matching + gitignore-honouring walk.
- `regex` — content predicates.
- `walkdir` (via `ignore` crate) — filesystem traversal.
- `rayon` — parallel per-file rule dispatch.
- `tree-sitter` + per-lang grammar crates (`tree-sitter-rust`,
  `tree-sitter-go`, `tree-sitter-typescript`) — AST predicates.
- `git2` — `--changed` git-diff scoping.
- `miette` or hand-rolled formatter — text diagnostics.
- `serde_sarif` (or hand-rolled) — SARIF 2.1.0 output.
- `insta` — snapshot-based CLI golden tests.

Crate split (`crates/lintropy-{core,langs,rules,output,cli}`) keeps
tree-sitter weight isolated from rule logic and lets each rule be
independently testable.
