<div align="center">

# 🌿 Lintropy

**A tree-sitter linter whose rules live in your repo — one YAML file each, written for humans and AI agents alike.**

[![status](https://img.shields.io/badge/status-draft_v0.1-orange)](specs/merged/)
[![license](https://img.shields.io/badge/license-MIT-blue)](LICENSE)
[![rust](https://img.shields.io/badge/rust-1.75%2B-dea584)](https://www.rust-lang.org/)
[![agent-native](https://img.shields.io/badge/agent-native-8A2BE2)](#why)

<img src="docs/demo.gif" alt="Lintropy demo" width="720"/>

</div>

---

## The pitch

Generic linters (Clippy, ESLint, Ruff) ship fixed catalogs. **Lintropy** inverts that: the interesting lints in any real codebase encode *your team's* conventions — architectural boundaries, migration deadlines, banned APIs, required ceremony.

**Lintropy ships no catalog.** Every rule is a small YAML file an LLM (or a human) can author in under a minute.

## Why it's different

| | Generic linters | Lintropy |
|---|---|---|
| Rule source | Shipped by tool author | Lives in *your* repo |
| Authoring | Plugin / TS / Rust code | Single YAML file |
| Agent support | Afterthought | First-class (SKILL.md + JSON Schema + hook) |
| Scope | Language universals | *Your* team's conventions |
| Deploy a rule | Publish a plugin | `git add .lintropy/my-rule.rule.yaml` |

## ✨ Features

- 🤖 **Agent-native** — ships `SKILL.md` + JSON Schema so coding agents write correct rules first try.
- 🪝 **Post-write hooks** — `lintropy hook` wires into Claude Code / Codex. Agent writes a file, diagnostics return as blocking feedback, model self-corrects before handing back.
- 📄 **One rule, one file** — `.lintropy/no-unwrap.rule.yaml` *is* the rule. CODEOWNERS works. `git rm` disables.
- 🎯 **Two primitives** — `query` (tree-sitter) for structure; `match` (regex) for text. That's the whole surface.
- 🔧 **Autofix** — `{{capture}}`-interpolated replacements, `--fix` or `--fix-dry-run`.
- 🗣 **Explainable** — every diagnostic names the file that defined the rule.

## 📝 A rule, end-to-end

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

```console
$ lintropy check
warning[no-unwrap]: avoid .unwrap() on `client`
  --> src/handlers/users.rs:42:18
   |
42 |     let user = client.unwrap().get(id).await?;
   |                ^^^^^^^^^^^^^^^ help: replace with `client.expect("TODO: handle error")`
   |
   = rule defined in: .lintropy/no-unwrap.rule.yaml

Summary: 1 warning across 1 file. 1 autofix available — re-run with --fix.
```

## 🚀 Quick start

```bash
# install (placeholder)
cargo install lintropy

# scaffold config + agent SKILL.md + hook wiring
lintropy init --with-skill

# run
lintropy check
lintropy check --fix            # apply autofixes
lintropy explain no-unwrap      # show rule source
lintropy ts-parse src/main.rs   # S-expression dump for query authoring
```

## 🪝 Agent integration

Wire `lintropy hook` into Claude Code's `PostToolUse` — every `Write` / `Edit` triggers a scoped lint, feedback flows back as blocking stderr:

```json
{
  "hooks": {
    "PostToolUse": [
      {
        "matcher": "Write|Edit|NotebookEdit",
        "hooks": [{ "type": "command", "command": "lintropy hook --agent claude-code" }]
      }
    ]
  }
}
```

`init --with-skill` merges this for you. Codex support lands phase 2.

## 🗺 Roadmap

| Phase | Scope |
|---|---|
| **1 — MVP** | Rust grammar, query rules, autofix, text+JSON output, Claude Code hook |
| **2** | `match` regex rules, Go + TypeScript, SARIF, Codex hook, `--changed` |
| **3** | Structured fix kinds (insert / move / create), regex autofix, `config explain` |
| **v2+** | LSP, baselines, incremental cache, `extends:`, WASM plugins |

## 📚 Docs

- Design spec — [`specs/merged/2026-04-18-lintropy-merged.md`](specs/merged/2026-04-18-lintropy-merged.md)
- Example repo — [`examples/rust-demo/`](examples/rust-demo/) *(coming with MVP)*
- SKILL.md — generated into `.claude/skills/lintropy/` by `init --with-skill`

## License

MIT
