<div align="center">

# 🌿 Lintropy

**The linter for rules your repo actually cares about.**

[![status](https://img.shields.io/badge/status-draft-orange)](#demo)
[![rust](https://img.shields.io/badge/rust-1.83%2B-dea584)](https://www.rust-lang.org/)
[![license](https://img.shields.io/badge/license-MIT-blue)](LICENSE)
[![agent native](https://img.shields.io/badge/agent-native-8b5cf6)](#built-for-agent-workflows)
[![rules](https://img.shields.io/badge/rules-YAML-0ea5e9)](#how-it-works)
[![engine](https://img.shields.io/badge/engine-tree--sitter-16a34a)](#how-it-works)

</div>

---

Most linters ship a fixed catalog.

Lintropy does the opposite: the rules live in your repo, one YAML file at a
time, and they describe **your** conventions:

- API code must live in `src/api/`
- feature modules cannot import each other directly
- `dbg!`, `println!`, or `.unwrap()` are banned outside tests
- migrations require rollback files
- only one module can touch `process.env`

This is linting for architecture, boundaries, migration policies, and team
ceremony, not just style.

## Why Lintropy

Generic linters are great at universal rules. They are weak at codebase-local
rules that only make sense inside one company, one monorepo, or one product.

Lintropy is built for that gap:

- rules are stored in the repo
- rules are easy to review and version
- rules are simple enough for agents to generate
- diagnostics tell you which rule file fired
- tree-sitter handles structure, regex handles plain text

## How it works

Lintropy uses two rule types:

- `query`: tree-sitter rules for structural patterns
- `match`: regex rules for text patterns

Typical layout:

```text
.
├── lintropy.yaml
└── .lintropy/
    ├── no-unwrap.rule.yaml
    ├── no-dbg.rule.yaml
    └── architecture/
        └── api-only-in-src-api.rule.yaml
```

Example rule:

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
    (#eq? @method "unwrap")) @match
```

Minimal root config:

```yaml
version: 1

settings:
  fail_on: error
  default_severity: error
```

## Demo

```console
$ lintropy check
warning[no-unwrap]: avoid .unwrap() on `client`; use .expect("...") or ?
  --> src/handlers/users.rs:42:18
   |
42 |     let user = client.unwrap().get(id).await?;
   |                ^^^^^^^^^^^^^^^ help: replace with `client.expect("TODO: handle error")`
   |
   = rule defined in: .lintropy/no-unwrap.rule.yaml

error[api-only-in-src-api]: API handlers must live under src/api/
  --> src/features/users/create_user.rs:1:1
   |
1  | pub async fn create_user(...) { ... }
   | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
   |
   = rule defined in: .lintropy/architecture/api-only-in-src-api.rule.yaml

Summary: 1 error, 1 warning, 2 files affected.
```

## Built for agent workflows

Lintropy is intentionally designed so Codex, Claude Code, and similar agents
can write valid rules without a lot of prompting overhead.

- one rule per file
- low-ceremony YAML
- deterministic repo discovery
- explainable diagnostics
- schema-friendly config
- hook-based workflows for post-edit feedback

The idea is simple: if agents are writing code, they should also be able to
write and respect the repo’s guardrails.

## What makes it different

| | Generic linters | Lintropy |
|---|---|---|
| Rule source | Built into the tool | Lives in your repo |
| Authoring | Plugin code or complex config | Small YAML files |
| Scope | Language-wide conventions | Project-specific constraints |
| Best use | Style and correctness | Architecture and boundaries |
| Agent support | Incidental | First-class |

## What ships in the workflow

- repo-root `lintropy.yaml`
- `.lintropy/**/*.rule.yaml` discovery
- tree-sitter `query` rules
- regex `match` rules
- capture-based messages and autofix
- text and JSON diagnostics
- rule-source-aware reporting
- agent-oriented hooks and schema output

## Editor Support

This repo now checks in JSON Schemas for all lintropy YAML surfaces:

- `editors/schemas/lintropy.schema.json` for repo-root `lintropy.yaml`
- `editors/schemas/lintropy-rule.schema.json` for `.lintropy/**/*.rule.yaml`
- `editors/schemas/lintropy-rules.schema.json` for `.lintropy/**/*.rules.yaml`

Refresh them after schema changes with:

```console
./scripts/export-editor-schemas.sh
```

### VS Code / Cursor

Workspace settings in `.vscode/settings.json` associate those schemas with the
matching files, and `.vscode/extensions.json` recommends `redhat.vscode-yaml`.
Cursor uses the same workspace settings, so completions, hover docs, and
validation work there as well.

For `query: |` blocks, the repo also ships a local TextMate-powered extension
at `editors/vscode/lintropy-query-syntax/`. It injects `source.lintropy-query`
highlighting into YAML block scalars whose key is `query`.

Install it in VS Code or Cursor from the folder, or package it first:

```console
cd editors/vscode/lintropy-query-syntax
npx @vscode/vsce package
```

Then install the resulting `.vsix` in VS Code / Cursor. The YAML schemas and
the injected TextMate grammar work together: schema-backed validation for the
file shape, TextMate syntax highlighting for the embedded S-expression.

### JetBrains IDEs

Shared mappings live in `.idea/jsonSchemas.xml`, pointing JetBrains IDEs at the
same checked-in schema files for:

- `lintropy.yaml`
- `.lintropy/**/*.rule.yaml`
- `.lintropy/**/*.rules.yaml`

If your IDE ignores shared `.idea` files, add the same three mappings manually
under `Languages & Frameworks | Schemas and DTDs | JSON Schema Mappings`.

For embedded query highlighting, import the TextMate bundle from:

- `editors/textmate/Lintropy Query.tmbundle`

In JetBrains IDEs this is under `Editor | TextMate Bundles`. The bundle adds a
TextMate injection that highlights YAML `query: |` blocks using the same
`source.lintropy-query` grammar as the VS Code / Cursor extension.

## License

MIT
