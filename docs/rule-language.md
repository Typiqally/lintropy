# Rule Language

This document describes the rule language that `lintropy` accepts today, based on the current implementation in `src/core/config.rs`, `src/core/engine.rs`, `src/core/predicates.rs`, and `src/langs.rs`.

## Short version

The "rule language" is really two layers:

1. A YAML rule format owned by Lintropy.
2. A `query:` string whose syntax is Tree-sitter query syntax.

Lintropy does not invent a new AST pattern language from scratch. For `query` rules, it passes the `query:` text directly into `tree_sitter::Query::new(...)`. That means:

- node kinds come from the selected Tree-sitter grammar
- field names come from the selected Tree-sitter grammar
- captures like `@recv` and `@match` are standard Tree-sitter captures
- normal Tree-sitter query operators and built-in predicates come from Tree-sitter

Lintropy adds rule metadata, file scoping, interpolation, autofix behavior, and a small set of custom predicates.

## What is supported today

Current engine behavior:

- Only `query` rules execute.
- Supported `language` values: `rust`, `go`, `python`, `typescript` (the last also matches `.tsx`).
- Files are linted structurally only when their extension maps to a supported Tree-sitter language — `.rs`, `.go`, `.py` / `.pyi`, `.ts` / `.tsx` / `.mts` / `.cts`.

The schema already contains `forbid`, `require`, and `multiline`, but the loader rejects those rule kinds with:

`match rules are Phase 2`

So they are part of the planned surface, not the live rule language.

## The YAML layer Lintropy owns

A rule is loaded from one of these places:

- inline under `rules:` in `lintropy.yaml`
- `.lintropy/**/*.rule.yaml`
- `.lintropy/**/*.rules.yaml`

Rule identity rules:

- In `*.rule.yaml`, `id` defaults to the file stem.
- In `*.rules.yaml`, each rule must set `id`.
- Inline rules in `lintropy.yaml` must set `id`.

The current per-rule fields are:

| Field | Meaning | Owned by |
| --- | --- | --- |
| `id` | Stable rule id | Lintropy |
| `severity` | `info`, `warning`, or `error` | Lintropy |
| `message` | Diagnostic text | Lintropy |
| `include` | gitignore-style allow globs | Lintropy |
| `exclude` | gitignore-style deny globs | Lintropy |
| `tags` | free-form labels | Lintropy |
| `docs_url` | optional remediation URL | Lintropy |
| `description` | optional prose description | Lintropy |
| `language` | grammar selector for `query` rules | Lintropy |
| `query` | Tree-sitter query source | Tree-sitter syntax, embedded by Lintropy |
| `fix` | replacement template for the matched span | Lintropy |
| `forbid` | planned regex rule field, not executable yet | Planned |
| `require` | planned regex rule field, not executable yet | Planned |
| `multiline` | planned regex mode flag, not executable yet | Planned |

Exactly one rule kind is allowed:

- `query`
- or `forbid` / `require`

Mixing `query` with `forbid` or `require` is rejected.

## What comes from Tree-sitter

Inside `query: |`, the language is standard Tree-sitter query syntax.

Lintropy relies on Tree-sitter for:

- S-expression pattern syntax
- node names such as `call_expression`, `field_expression`, `macro_invocation`
- field constraints such as `function: (...)`
- captures such as `@recv`, `@method`, `@match`
- multi-pattern queries
- built-in predicates supported by the linked Tree-sitter version

Example:

```yaml
language: rust
message: "avoid .unwrap() on `{{recv}}`"
query: |
  (call_expression
    function: (field_expression
      value: (_) @recv
      field: (field_identifier) @method)
    (#eq? @method "unwrap")) @match
```

In that example:

- `call_expression`, `field_expression`, and `field_identifier` come from `tree-sitter-rust`
- `function:` and `field:` are grammar-defined field names
- `@recv`, `@method`, and `@match` are normal Tree-sitter captures
- `#eq?` is a Tree-sitter predicate, not a Lintropy-specific one

Practical rule: if `tree_sitter::Query::new(&language.ts_language(), query_source)` cannot compile the query, the rule is invalid.

## What Lintropy adds on top

### 1. Language selection

Tree-sitter queries are grammar-specific. Lintropy requires `language:` on query rules and uses it to choose the grammar handle.

Today the only valid value is:

```yaml
language: rust
```

### 2. File scoping

`include` and `exclude` are not Tree-sitter features. Lintropy applies them before running a rule against a file.

- They are compiled with `globset`.
- They operate on repo-relative paths when possible.
- If `include` is empty, the rule applies everywhere.
- If `exclude` matches, the rule is skipped for that file.

### 3. `{{capture}}` interpolation

Lintropy lets `message` and `fix` reference query captures using `{{name}}`.

Example:

```yaml
message: "avoid .unwrap() on `{{recv}}`"
fix: '{{recv}}.expect("TODO: handle error")'
```

Behavior:

- capture names must exist in the compiled query
- unknown capture names are rejected at load time
- the replacement text is the exact UTF-8 source text of the captured node

This interpolation syntax is Lintropy-specific. It is not part of Tree-sitter.

### 4. `@match` span convention

`@match` is just a normal capture name to Tree-sitter, but Lintropy gives it special meaning:

- the diagnostic span is taken from `@match`
- the autofix replacement range is taken from `@match`

If `@match` is missing, Lintropy falls back to the first capture in the query match and emits a config warning because the diagnostic span may be too broad.

So `@match` is a Lintropy convention layered on top of a standard Tree-sitter capture.

### 5. Autofix via `fix`

`fix` is a Lintropy feature, not a Tree-sitter feature.

When present on a query rule:

- Lintropy interpolates `{{captures}}` into the `fix` template
- the resulting text replaces the `@match` source range

Lintropy does one pass only. It does not repeatedly re-run fixes to a fixed point.

### 6. Custom predicates

Lintropy adds custom predicates beyond what Tree-sitter provides. These are parsed from `query.general_predicates(...)` and evaluated after Tree-sitter's normal `QueryCursor` matching.

Supported custom predicates today:

- `#has-ancestor?`
- `#not-has-ancestor?`
- `#has-parent?`
- `#not-has-parent?`
- `#has-sibling?`
- `#not-has-sibling?`
- `#has-preceding-comment?`
- `#not-has-preceding-comment?`

Examples:

```scheme
(#has-parent? @method "field_expression")
(#not-has-ancestor? @recv "function_item")
(#not-has-preceding-comment? @match "SAFETY:")
```

Argument shape is fixed by Lintropy:

- the first argument must be a capture
- remaining arguments are strings
- for the comment predicates, the pattern string is compiled as a Rust `regex::Regex`

Semantics:

- `ancestor` walks any parent chain upward
- `parent` checks only the direct parent
- `sibling` checks other children of the same parent
- `preceding-comment` scans upward through immediately preceding non-empty comment lines or block comments

Unknown predicate names are rejected during config loading.

## Boundary between Tree-sitter and Lintropy

Use this rule of thumb:

- If it talks about AST shape, node kinds, fields, captures, or built-in query predicates, it comes from Tree-sitter and the grammar crate.
- If it talks about repository discovery, ids, severity, globs, messages, interpolation, fixes, diagnostics, or extra predicates, it comes from Lintropy.

More concretely:

| Concern | Source |
| --- | --- |
| `call_expression` exists | `tree-sitter-rust` |
| `function:` is a valid field on that node | `tree-sitter-rust` |
| `(#eq? ...)` works | Tree-sitter |
| `@match` controls highlight/fix span | Lintropy |
| `{{recv}}` inside `message` works | Lintropy |
| `include` / `exclude` filtering works | Lintropy |
| `#has-ancestor?` works | Lintropy |
| `language: rust` maps to a grammar | Lintropy |

## Minimal valid rule today

```yaml
severity: warning
message: "avoid dbg!"
language: rust
query: |
  (macro_invocation
    macro: (identifier) @name
    (#eq? @name "dbg")) @match
```

## Non-goals of the current rule language

These are not live today:

- regex-backed `forbid` rules
- regex-backed `require` rules
- `multiline: true` runtime behavior
- languages beyond `rust`, `go`, `python`, `typescript`

## Authoring guidance

When you are writing a rule, treat the query body as "pure Tree-sitter" and everything around it as "Lintropy policy and UX".

For Rust rules specifically:

1. Inspect the real AST first with `lintropy ts-parse <file>`.
2. Write a normal Tree-sitter query that matches the right node.
3. Add `@match` to the node you want highlighted and replaced.
4. Add Lintropy metadata like `message`, `severity`, `include`, and optional `fix`.

