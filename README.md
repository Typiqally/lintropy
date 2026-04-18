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

## Roadmap

The current direction is:

- Rust-first support
- repo-root `lintropy.yaml`
- `.lintropy/**/*.rule.yaml` discovery
- tree-sitter query rules
- regex match rules
- autofix for query-based replacements
- text and JSON diagnostics
- agent-oriented hooks and schema output

## License

MIT
