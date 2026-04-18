# lintropy

**A tree-sitter linter whose rules live in your repo — one YAML file each, written for humans and AI agents alike.**

Generic linters (Clippy, ESLint, Ruff) ship fixed catalogs. `lintropy` inverts that: the interesting lints in any real codebase encode *your team's* conventions — architectural boundaries, migration deadlines, banned APIs, required ceremony. `lintropy` ships **no catalog**. Every rule is a small YAML file an LLM (or a human) can author in under a minute.

## Why

- **Agent-native.** Rules are data, not code. Ships a `SKILL.md` + JSON Schema so coding agents write correct rules first try.
- **Post-write hooks.** `lintropy hook` wires into Claude Code / Codex — the agent writes a file, diagnostics flow back as blocking feedback, the model self-corrects before handing control back.
- **One rule, one file.** `.lintropy/no-unwrap.rule.yaml` *is* the rule. CODEOWNERS works. `git rm` disables.
- **Two primitives, no ceremony.** `query` (tree-sitter) for structure; `match` (regex) for text. That's the whole surface.
- **Explainable.** Every diagnostic names the file that defined the rule.

## Example

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
    (#eq? @method "unwrap")
    (#not-has-ancestor? @method "macro_invocation")) @match
```

```
warning[no-unwrap]: avoid .unwrap() on `client`
  --> src/handlers/users.rs:42:18
   |
42 |     let user = client.unwrap().get(id).await?;
   |                ^^^^^^^^^^^^^^^ help: replace with `client.expect("TODO: handle error")`
   |
   = rule defined in: .lintropy/no-unwrap.rule.yaml
```

## Quick start

```bash
lintropy init --with-skill     # scaffold lintropy.yaml + .lintropy/ + SKILL.md + hook wiring
lintropy check                 # run
lintropy check --fix           # apply autofixes
lintropy explain no-unwrap     # show rule source
lintropy ts-parse src/main.rs  # S-expression dump for query authoring
```

## Status

v0.1 — draft spec. Phase 1 MVP: Rust grammar, query rules, autofix, Claude Code hook integration. Go / TypeScript / Codex land phase 2. See [`specs/merged/`](specs/merged/) for the full design.
