# version: 0.3.0

## 1. What lintropy is

`lintropy` is a YAML-configured, agent-first linter. Rules live in
`lintropy.yaml` (root) and `.lintropy/<id>.rule.yaml` (one rule per file).
Two rule kinds: **query rules** (tree-sitter S-expressions, AST-precise)
and **match rules** (regex, phase 2). Query rules support autofix via a
`fix:` field that interpolates `{{captures}}` into the `@match` span.

Diagnostics are rustc-styled text by default, JSON on `--format json`.
Exit codes: `0` clean, `1` diagnostics ≥ `fail_on`, `2` config/parse
failure, `3` internal crash.

**Trigger phrases** (if the user says any of these, reach for this skill):

- "write a lintropy rule"
- "add a lintropy rule / lint"
- "lintropy diagnostic"
- mention of `.rule.yaml` or `.rules.yaml`
- "ban <API> with lintropy"
- "enforce <convention> in CI"

## 2. Commands

```bash
lintropy check                                 # default; lint the repo
lintropy check --fix                           # apply autofixes in place
lintropy check --fix-dry-run                   # print unified diff, exit 0
lintropy check --format json -o report.json    # machine-readable
lintropy explain <rule-id>                     # message, query, fix, source
lintropy rules [--format json]                 # list loaded rules
lintropy ts-parse <file> [--lang <name>]       # dump S-expression (auto-detects by extension)
lintropy config validate                       # schema + queries, no run
lintropy init [--with-skill]                   # scaffold config; install SKILL
lintropy schema                                # JSON schema for the config
```

Exit codes (§7.6):

| code | meaning                                              |
|------|------------------------------------------------------|
| 0    | no diagnostics at/above `fail_on`                    |
| 1    | diagnostics present at/above `fail_on`               |
| 2    | config load / schema / parse failure                 |
| 3    | internal error                                       |

`fail_on` defaults to `error`. Set in `lintropy.yaml` under `settings`.

## 3. Rule anatomy

File layout:

```
<repo>/
  lintropy.yaml                      # root; settings + optional inline rules
  .lintropy/
    no-unwrap.rule.yaml              # one rule; id = file stem
    architecture/domain-no-infra.rule.yaml
    2026q2.rules.yaml                # multi-rule grouping
```

Annotated single-rule file (`*.rule.yaml`):

```yaml
# .lintropy/no-unwrap.rule.yaml
# id: no-unwrap                        # optional; defaults to file stem
severity: warning                      # error | warning | info
description: |                         # ALWAYS include — one sentence minimum
  Flags `.unwrap()` on Result/Option outside macro bodies. Unwraps panic
  in production; prefer `?`, `.expect("<context>")`, or explicit match.
message: "avoid .unwrap() on `{{recv}}`"
language: rust                         # required when `query:` present
include: ["**/*.rs"]                   # optional; gitignore-style globs
exclude: ["**/tests/**"]               # optional
tags: ["reliability"]                  # optional
docs_url: "https://example.com/rules/no-unwrap"   # optional
fix: '{{recv}}.expect("TODO: handle error")'      # optional (query rules only)
query: |                               # one of: query | forbid | require
  (call_expression
    function: (field_expression
      value: (_) @recv
      field: (field_identifier) @method)
    (#eq? @method "unwrap")) @match
```

Field reference (§4.8):

| key           | required                    | purpose                                          |
|---------------|-----------------------------|--------------------------------------------------|
| `id`          | inline + `*.rules.yaml`     | stable label; defaults to stem for `*.rule.yaml` |
| `severity`    | no                          | `error` \| `warning` \| `info`                   |
| `description` | **required when generating** (optional to loader) | prose rationale; why the rule exists, what bad pattern it catches |
| `message`     | yes                         | short diagnostic text; `{{capture}}` interpolated |
| `include`     | no                          | gitignore-style inclusive globs                  |
| `exclude`     | no                          | gitignore-style exclusive globs                  |
| `tags`        | no                          | free-form                                        |
| `docs_url`    | no                          | surfaced in diagnostics                          |
| `language`    | yes when `query:` present   | `rust` \| `go` \| `python` \| `typescript`       |
| `query`       | one-of `query/forbid/require` | tree-sitter S-expression                       |
| `forbid`      | one-of                      | regex; match = violation (phase 2)               |
| `require`     | one-of                      | regex; absence = violation (phase 2)             |
| `multiline`   | no (match rules only)       | regex `m`+`s` flags                              |
| `fix`         | no (query rules only)       | replacement for `@match`; `{{capture}}` interp   |

The CLI owns the extension-to-language mapping (`.rs` → rust, `.go` → go, `.py`/`.pyi` → python,
`.ts`/`.tsx`/`.mts`/`.cts`/`.d.ts` → typescript). Rules declare `language:` and let the CLI route
files. Extension mappings are not configurable.

**`description` vs `message`** — keep them distinct:

- `description` — prose explaining *why the rule exists* and *what bad
  pattern it targets*. One to three sentences. Plain text. NOT templated
  (no `{{capture}}` substitution). Never shown in per-violation output;
  surfaced by `lintropy rules` and `lintropy explain`.
- `message` — one-line diagnostic string shown at every violation site.
  Templated. Keep it short and imperative ("avoid .unwrap() on `{{recv}}`").

Always include `description` when you generate a rule, even for trivial
ones. A rule without a description is invisible to the catalogue and to
any agent reading `lintropy rules` for discovery.

Discriminator is **key presence**:

- `query:` present → query rule. `language:` mandatory.
- `forbid:` and/or `require:` present → match rule.
- Exactly one mode per rule. `forbid` + `require` together is allowed
  (two checks per file). `query` cannot coexist with `forbid`/`require`.

Multi-rule file (`*.rules.yaml`) — `id` becomes required:

```yaml
# .lintropy/2026q2.rules.yaml
rules:
  - id: use-tracing-not-log
    severity: warning
    description: |
      Flags `log::{trace,debug,info,warn,error}!` macros so callers
      migrate to `tracing::*`. Autofix rewrites the macro path; args
      are preserved.
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
    description: Bans stray `console.log` from shipped code under `src/`.
    message: "no console.log in src/"
    include: ["src/**/*.ts"]
    exclude: ["**/*.test.ts"]
    forbid: 'console\.log'
```

Inline rules in `lintropy.yaml` work the same; `id` required there too.

## 4. Writing the tree-sitter query

### 4.1 Always `ts-parse` first

Never guess node kinds. Run:

```bash
lintropy ts-parse src/some.rs
```

It prints the S-expression. Language is auto-detected from the file extension.
Pass `--lang <name>` only when the extension is unusual or ambiguous.
Copy the kinds you care about, then build the query bottom-up.

### 4.2 `tree-sitter-rust` node-kind cheat sheet

| construct                  | node kind(s)                                                     |
|----------------------------|------------------------------------------------------------------|
| file root                  | `source_file`                                                    |
| function / method          | `function_item` (fields: `name`, `parameters`, `body`)           |
| impl / trait / mod         | `impl_item`, `trait_item`, `mod_item`                            |
| struct / enum              | `struct_item`, `enum_item`                                       |
| use                        | `use_declaration` → `scoped_identifier` / `use_list`             |
| const / static / type      | `const_item`, `static_item`, `type_item`                         |
| call                       | `call_expression` (fields: `function`, `arguments`)              |
| method call                | `call_expression` → `function: (field_expression …)`             |
| field access               | `field_expression` (fields: `value`, `field`)                    |
| macro invocation           | `macro_invocation` (fields: `macro`, token_tree child)           |
| attribute                  | `attribute_item` → `attribute` → `identifier`/`scoped_identifier`|
| identifiers                | `identifier`, `field_identifier`, `type_identifier`, `scoped_identifier` |
| comments                   | `line_comment`, `block_comment`, `doc_comment`                   |
| literals                   | `string_literal`, `integer_literal`, `boolean_literal`           |
| control flow               | `match_expression`, `if_expression`, `try_expression`, `return_expression` |
| blocks                     | `block`, `expression_statement`, `let_declaration`               |
| unsafe                     | `unsafe_block`                                                   |
| arguments / tokens         | `arguments`, `parameters`, `token_tree`                          |

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
| `defer_statement`             | `defer foo()`                                         |
| `go_statement`                | `go foo()`                                             |

Worked example — `.lintropy/no-fmt-println.rule.yaml`:

```yaml
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

```yaml
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
matches only in `.tsx` files — the same rule against `.ts` files
silently produces zero diagnostics, which is correct.

Worked example — `.lintropy/no-console-log.rule.yaml`:

```yaml
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

### 4.6 Built-in predicates (free from `QueryCursor`)

| predicate                              | semantics                              |
|----------------------------------------|----------------------------------------|
| `(#eq? @cap "text")`                   | capture text equals literal            |
| `(#not-eq? @cap "text")`               | negation                               |
| `(#match? @cap "regex")`               | capture text matches regex             |
| `(#not-match? @cap "regex")`           | negation                               |
| `(#any-of? @cap "a" "b" "c")`          | capture text ∈ set                     |

### 4.7 Custom predicates (host-applied, §6)

| predicate                                  | semantics                               |
|--------------------------------------------|-----------------------------------------|
| `(#has-ancestor? @cap "kind"...)`          | capture has ancestor of any named kind  |
| `(#not-has-ancestor? @cap "kind"...)`      | negation                                |
| `(#has-parent? @cap "kind"...)`            | immediate parent kind matches           |
| `(#not-has-parent? @cap "kind"...)`        | negation                                |
| `(#has-sibling? @cap "kind"...)`           | some sibling kind matches               |
| `(#not-has-sibling? @cap "kind"...)`       | negation                                |
| `(#has-preceding-comment? @cap "regex")`   | nearest preceding comment matches regex |
| `(#not-has-preceding-comment? @cap "regex")` | negation                              |

Unknown predicate name = hard error at config load. No plugin system.
Path-scoped predicates (`#filename-matches?`, `#in-file?`) are not
provided — use `include` / `exclude` globs instead.

### 4.8 `@match` convention

Every query should capture a `@match` node; diagnostic span = the
`@match` capture, else the match root. A rule without `@match` gets a
warning at load (vague span). Put `@match` on the node you want
highlighted **and** replaced by `fix:`.

### 4.9 `{{capture}}` interpolation

In `message:` and `fix:`, `{{name}}` substitutes the captured node's
source text. Unknown capture name = hard error at load (fail fast).

```yaml
message: "avoid .unwrap() on `{{recv}}`"
fix: '{{recv}}.expect("TODO: handle error")'
```

## 5. Writing a match (regex) rule

Match rules are **phase 2**; they are documented so the SKILL ships
complete, but the engine may not evaluate them yet.

```yaml
# .lintropy/no-console-log.rule.yaml
severity: error
description: Bans stray `console.log` from shipped code under `src/`.
message: "no console.log in src/"
include: ["src/**/*.ts"]
exclude: ["**/*.test.ts"]
forbid: 'console\.log'
```

```yaml
# .lintropy/license-header.rule.yaml
severity: error
description: |
  Requires every Rust source file to begin with an `// SPDX-License-Identifier:`
  header so licence provenance is grep-able and CI-verifiable.
message: "missing SPDX license header"
include: ["**/*.rs"]
require: '^// SPDX-License-Identifier:'
```

Semantics:

- `forbid`: every match = one diagnostic. Span = match range.
- `require`: file with no match = one diagnostic. Span = line 1, col 1.
- `multiline: true` enables regex `m` + `s` flags.
- Capture groups usable as `{{0}}` (whole match), `{{1}}`, `{{2}}`… in
  `message`.
- No `fix:` for match rules in v0.1.

## 6. Common recipes

Each recipe below is a complete, valid `.rule.yaml` — drop the file into
`.lintropy/` and `lintropy check`.

### 6.1 Banned API (single call)

```yaml
# .lintropy/no-dbg.rule.yaml
severity: error
description: Flags stray `dbg!()` macros left from debugging sessions.
message: "stray dbg!() — remove before merging"
language: rust
query: |
  (macro_invocation
    macro: (identifier) @n
    (#eq? @n "dbg")) @match
```

### 6.2 Banned API with captured receiver + autofix

```yaml
# .lintropy/no-unwrap.rule.yaml
severity: warning
description: |
  Flags `.unwrap()` on Result/Option. Unwraps panic in production;
  prefer `?`, `.expect("<context>")`, or explicit `match`.
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

### 6.3 Layered import boundary (domain must not use infra)

```yaml
# .lintropy/domain-no-infra.rule.yaml
severity: error
description: |
  Enforces the domain/infra boundary — modules under `src/domain/` must
  not import from `infra::`. Keeps the pure core decoupled from IO.
message: "domain/ must not import from infra/"
include: ["src/domain/**/*.rs"]
language: rust
query: |
  (use_declaration
    (scoped_identifier
      path: (identifier) @root)
    (#eq? @root "infra")) @match
```

### 6.4 Migration with autofix (`log::*` → `tracing::*`)

```yaml
# .lintropy/use-tracing-not-log.rule.yaml
severity: warning
description: |
  Migrates `log::{trace,debug,info,warn,error}!` to `tracing::*`.
  Autofix rewrites the macro path while preserving arguments.
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
```

### 6.5 Required ceremony — TODO must reference a ticket

```yaml
# .lintropy/todo-needs-ticket.rule.yaml
severity: warning
description: |
  Requires every `TODO` comment to reference a tracker ticket
  (`(PROJ-123)`) or a URL, so stray TODOs don't rot in the codebase.
message: "TODO without ticket ref — add (PROJ-123) or issue URL"
language: rust
query: |
  ((line_comment) @match
    (#match? @match "TODO")
    (#not-match? @match "\\([A-Z]+-[0-9]+\\)")
    (#not-match? @match "https?://"))
```

### 6.6 Required ceremony — SAFETY comment above `unsafe` block

```yaml
# .lintropy/unsafe-needs-safety.rule.yaml
severity: error
description: |
  Requires every `unsafe` block to be preceded by a `// SAFETY:`
  comment explaining the invariant that makes the unsafe code sound.
message: "unsafe block without preceding `// SAFETY:` comment"
language: rust
query: |
  ((unsafe_block) @match
    (#not-has-preceding-comment? @match "SAFETY:"))
```

### 6.7 Taxonomy regex — enforce error-type naming

```yaml
# .lintropy/error-enum-naming.rule.yaml
severity: warning
description: |
  Enforces the `FooError` naming convention — enums ending in `Err`
  (but not `Error`) are flagged so error types read consistently.
message: "error enums should end in `Error` (got `{{name}}`)"
language: rust
query: |
  (enum_item
    name: (type_identifier) @name
    (#match? @name "Err$")
    (#not-match? @name "Error$")) @match
```

### 6.8 Test discipline — forbid `#[ignore]`

```yaml
# .lintropy/no-ignored-tests.rule.yaml
severity: error
description: |
  Flags `#[ignore]` on tests. Ignored tests silently mask regressions;
  either delete them or fix what they were ignoring.
message: "`#[ignore]` on tests masks failures — delete or fix"
include: ["**/*.rs"]
language: rust
query: |
  (attribute_item
    (attribute
      (identifier) @name)
    (#eq? @name "ignore")) @match
```

### 6.9 Dated deprecation with autofix

```yaml
# .lintropy/legacy-client-deprecated.rule.yaml
severity: warning
description: |
  Marks `LegacyClient` as deprecated (since 2026-01-01). Autofix
  rewrites the type to `Client`. Remove the rule once all usages are
  migrated.
message: "`LegacyClient` is deprecated since 2026-01-01 — use `Client`"
fix: "Client"
language: rust
query: |
  ((type_identifier) @match
    (#eq? @match "LegacyClient"))
```

### 6.10 Builder enforcement — require `UserBuilder` over `User::new`

```yaml
# .lintropy/use-user-builder.rule.yaml
severity: warning
description: |
  Steers callers from `User::new(...)` to `UserBuilder`, which validates
  required fields at compile time and makes construction discoverable.
message: "use UserBuilder instead of User::new"
language: rust
query: |
  (call_expression
    function: (scoped_identifier
      path: (identifier) @ty
      name: (identifier) @method)
    (#eq? @ty "User")
    (#eq? @method "new")) @match
```

### 6.11 License-header requirement (match rule, phase 2)

```yaml
# .lintropy/license-header.rule.yaml
severity: error
description: |
  Requires every Rust source file to begin with an `// SPDX-License-Identifier:`
  header so licence provenance is grep-able and CI-verifiable.
message: "missing SPDX license header"
include: ["**/*.rs"]
require: '^// SPDX-License-Identifier:'
```

## 7. Interpreting diagnostics

Default text format (rustc-styled, one diagnostic at a time, color on
TTY):

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

How to read it:

- `warning[no-unwrap]` — severity + rule id.
- `--> file:line:col` — start of the diagnostic span.
- The source-context line plus `^^^^^^^` under the `@match` span.
- `help:` — the interpolated `fix:` replacement (when present).
- `= rule defined in:` — the YAML file the rule came from. **Edit that
  file to tune the rule.** Edit the source line under `^^^` to fix the
  code.
- `= see: lintropy explain <id>` — full context.

JSON format (`--format json`) wraps canonical diagnostics in an envelope
with a summary block; each diagnostic carries `byte_start`/`byte_end`,
`rule_source`, `docs_url`, and an optional `fix` object (see §7.1/§7.3
of the merged spec).

Suppression in source:

```
// lintropy-ignore: no-unwrap          # next non-comment line only
// lintropy-ignore-file: no-unwrap     # whole file; must appear in first 20 lines
```

No wildcard. Must be on its own line. Unknown ids trigger the always-on
`suppress-unused` meta-warning.

## 8. Fix decision tree

When a diagnostic fires, decide which of these four situations you're in:

1. **Real problem in the source.** Fix the code.
   - If `help:` shows a fix and you trust it: `lintropy check --fix`.
   - Review the diff — autofix is single-pass, no fixpoint iteration.

2. **Rule is too broad** (legitimate call site is flagged).
   - Prefer adding a predicate to the query (`#not-has-ancestor?`, etc.)
     or tightening `exclude` / `include`.
   - Do **not** sprinkle `// lintropy-ignore:` as the default escape —
     that's the anti-pattern in §9 below.

3. **Rule is wrong** (genuinely bad rule).
   - `lintropy explain <rule-id>` to find the source path.
   - Edit the rule file; re-run `lintropy config validate` before
     `lintropy check`.

4. **Query won't parse / predicate unknown.**
   - Config load prints `rule <id> in <file>:<line>:<col>: <reason>`.
   - Fix the YAML and re-run. Use `lintropy ts-parse <file>` to confirm
     node kinds.

## 9. Anti-patterns

- **Don't omit `@match`.** The span will fall back to the whole query
  root, which is usually a huge node. Always capture `@match` on the
  exact token you want highlighted and (for autofix) replaced.
- **Don't set `id` in `*.rule.yaml`** unless overriding the stem. The
  filename is the source of truth; two ids for the same rule drift.
- **Don't `exclude: ["**/*"]` and re-add paths.** Use `include`
  instead — exclusion-as-selection is confusing and order-dependent.
- **Don't write a match (regex) rule where a query rule would be
  precise.** Regex over code is brittle; AST is cheap.
- **Don't write a 30-line AST query for a text pattern `grep` would
  catch.** Some things (license headers, banned strings in comments)
  are genuinely textual.
- **Don't blanket-suppress with `// lintropy-ignore:` as a first
  resort.** Fix the code or tighten the rule first.
- **Don't rely on unverified node kinds.** Always `lintropy ts-parse`
  before writing a query — guessing kinds is the #1 source of
  compile-at-load failures.
- **Don't ship a rule without a `description`.** The catalogue exposed
  by `lintropy rules` is the primary discovery surface for humans and
  agents; a rule with no description is effectively invisible. One or
  two sentences minimum — what it catches and why it matters. Keep
  rationale in `description`; keep short diagnostic text in `message`.
- **Don't pick `language: tsx`.** There is no `tsx` variant. Write
  `language: typescript` for both `.ts` and `.tsx` files. The CLI
  selects the `typescript` vs `tsx` grammar per file based on the
  extension.
