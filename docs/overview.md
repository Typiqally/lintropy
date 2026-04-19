# Overview

Lintropy is a repo-local linter for rules that are specific to one codebase.

Instead of shipping a fixed catalog, it loads rules from your repository:

- `lintropy.yaml` for root settings and optional inline rules
- `.lintropy/**/*.rule.yaml` for one-rule-per-file rules
- `.lintropy/**/*.rules.yaml` for grouped rule files

From a user's perspective, the product has five main jobs:

1. Load rules from the repo and validate them.
2. Walk files and run structural checks.
3. Print diagnostics that point back to the rule that fired.
4. Offer autofixes when a rule defines one.
5. Integrate with editors and agent workflows.

## What Lintropy is good at

Lintropy is strongest when the rule is local to your repo or team:

- architectural boundaries
- banned APIs or macros
- migration rules
- required safety or ceremony comments
- conventions around where code may live

It is not trying to replace a language linter like Clippy or ESLint. The usual setup is:

- keep your normal language linter
- add Lintropy for repo-specific policy

## What works today

Current user-visible behavior:

- structural `query` rules execute
- `language: rust` is supported
- autofix is available for query rules with `fix:`
- CLI reporting supports text and JSON
- LSP live diagnostics are available
- editor install helpers exist for VS Code, Cursor, and JetBrains

Not live yet:

- regex-backed `forbid` / `require` rules
- non-Rust Tree-sitter languages

Those fields already appear in some schemas and docs, but the current loader rejects match rules as Phase 2.

## Typical repo layout

```text
.
├── lintropy.yaml
├── .lintropy/
│   ├── no-unwrap.rule.yaml
│   ├── no-dbg.rule.yaml
│   └── architecture/
│       └── api-only-in-src-api.rule.yaml
└── src/
```

## Main commands

The most important commands for users are:

- `lintropy init`
- `lintropy check`
- `lintropy config validate`
- `lintropy rules`
- `lintropy explain <rule-id>`
- `lintropy ts-parse <file>`
- `lintropy lsp`

## Read next

- [Getting Started](./getting-started.md)
- [Configuration](./configuration.md)
- [CLI Guide](./cli.md)
- [Integrations](./integrations/index.md)
- [Rule Language](./rule-language.md)
