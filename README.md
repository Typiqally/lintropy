<div align="center">

# 🌿 Lintropy

**Repo-native linting for architecture, boundaries, and team-specific rules.**

[![ci](https://github.com/Typiqally/lintropy/actions/workflows/ci.yaml/badge.svg)](https://github.com/Typiqally/lintropy/actions/workflows/ci.yaml)
[![release](https://img.shields.io/github/v/release/Typiqally/lintropy?include_prereleases&sort=semver)](https://github.com/Typiqally/lintropy/releases)
[![rust](https://img.shields.io/badge/rust-1.95%2B-dea584)](https://www.rust-lang.org/)
[![license](https://img.shields.io/badge/license-MIT-blue)](LICENSE)
[![agent native](https://img.shields.io/badge/agent-native-8b5cf6)](#built-for-agent-workflows)
[![rules](https://img.shields.io/badge/rules-YAML-0ea5e9)](#how-it-works)
[![engine](https://img.shields.io/badge/engine-tree--sitter-16a34a)](#how-it-works)

</div>

---

Lintropy is a linter for rules your repo actually cares about. It started at
[The IDE Reimagined: JetBrains Codex Hackathon](https://cerebralvalley.ai/e/jetbrains-x-openai-hack),
a two-day San Francisco event focused on building AI-powered developer tools
alongside JetBrains and OpenAI engineers.

> [!WARNING]
> This project began as hackathon code and is not actively maintained right
> now. Expect rough edges, incomplete features, and bugs before using it in
> production workflows.

Most linters ship a fixed catalog. Lintropy does the opposite: rules live in
your repo, one YAML file at a time, describing **your** conventions:

- API code must live in `src/api/`
- feature modules cannot import each other directly
- `dbg!`, `println!`, or `.unwrap()` are banned outside tests
- migrations require rollback files
- only one module can touch `process.env`

Linting for architecture, boundaries, migration policies, and team ceremony —
not just style.

## Install

### Homebrew (macOS and Linux)

```console
brew tap Typiqally/lintropy
brew install lintropy
```

### From source

Stable Rust 1.95 or newer required.

```console
cargo install --path .
```

Not yet on crates.io.

## Supported languages

- **Rust**, **Go**, **Python**, **TypeScript** (incl. `.tsx`) — structural `query` rules via tree-sitter
- **Any text file** — regex `match` rules

More tree-sitter languages planned. Vote or contribute via issues.

## Demo

```console
$ lintropy check .
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

## Why Lintropy

Generic linters are great at universal rules. They are weak at codebase-local
rules that only make sense inside one company, one monorepo, or one product.

Lintropy fills that gap:

- rules stored in the repo, versioned alongside the code they govern
- rules easy to review
- rules simple enough for agents to generate
- diagnostics tell you which rule file fired
- tree-sitter handles structure, regex handles plain text

| | Generic linters | Lintropy |
|---|---|---|
| Rule source | Built into the tool | Lives in your repo |
| Authoring | Plugin code or complex config | Small YAML files |
| Scope | Language-wide conventions | Project-specific constraints |
| Best use | Style and correctness | Architecture and boundaries |
| Agent support | Incidental | First-class |

## How it works

Two rule types:

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

## Built for agent workflows

Designed so Codex, Claude Code, and similar agents can write valid rules
without much prompting overhead.

- one rule per file
- low-ceremony YAML
- deterministic repo discovery
- explainable diagnostics
- schema-friendly config
- hook-based workflows for post-edit feedback

If agents write code, they should write and respect the repo's guardrails too.

## Quickstart

The `examples/rust-demo/` crate doubles as the reference fixture. Clone this
repo and run:

```console
cd examples/rust-demo
lintropy check .
```

You should see four warnings (`no-unwrap`, `no-println`, `user-use-builder`,
`no-todo`) across three files, plus a hint that one autofix is available:

```console
lintropy check . --fix          # apply the no-unwrap autofix in place
lintropy check . --fix-dry-run  # print the unified diff instead
```

Analogous single-language demos ship for the other supported languages:

- [`examples/go-demo/`](examples/go-demo) — `no-fmt-println`, `no-todo-comment`
- [`examples/python-demo/`](examples/python-demo) — `no-print`, `no-todo-comment`
- [`examples/typescript-demo/`](examples/typescript-demo) — `no-console-log`,
  `no-any-type`, `no-todo-comment` (covers `.ts` and `.tsx`)

To scaffold lintropy inside your own repo:

```console
lintropy init                   # writes lintropy.yaml + .lintropy/no-unwrap.rule.yaml
lintropy init --with-skill      # also installs SKILL.md + wires the Claude Code hook
```

The canonical `SKILL.md` at
[`skill/SKILL.md`](skill/SKILL.md)
is what `init --with-skill` installs into agent skill directories.

## Editor and agent support

Lintropy ships one LSP server (`lintropy lsp`) and one install command that wires it into every supported target:

```console
lintropy lsp install vscode        # VS Code
lintropy lsp install cursor        # Cursor
lintropy lsp install jetbrains     # JetBrains IDEs (LSP4IJ template)
lintropy lsp install claude-code   # Claude Code plugin
```

Each target gives you live diagnostics, quickfixes, config reload, and semantic-token highlighting for the `query: |` DSL. No separate "query syntax" extension. Per-integration walkthroughs live under [`docs/integrations/`](docs/integrations/index.md).

### VS Code and Cursor

```console
lintropy lsp install vscode        # or: cursor
lintropy lsp install cursor --profile Default
```

Builds the bundled extension source into a `.vsix` and installs it via `code --install-extension` / `cursor --install-extension`. The extension resolves the `lintropy` binary in this order: explicit `lintropy.path` setting → `PATH` → extension-managed download from the matching GitHub release.

Package the `.vsix` without installing (useful for CI):

```console
lintropy lsp install vscode --package-only -o ./lintropy.vsix
```

Config resolution is per file rather than one workspace-wide root: each source file uses the nearest ancestor `lintropy.yaml`. A newly added nested `lintropy.yaml` creates a fresh rule context for that subtree, while `.lintropy/` changes merge into the rules for the already-resolved root and republish diagnostics for open files.

See [`editors/vscode/lintropy/README.md`](editors/vscode/lintropy/README.md) for per-setting reference.

### JetBrains IDEs

```console
lintropy lsp install jetbrains --dir ~/.lintropy
```

Unpacks the [LSP4IJ](https://plugins.jetbrains.com/plugin/23257-lsp4ij) custom server template to `~/.lintropy/lsp4ij-template`. One import step in the IDE:

`View → Tool Windows → LSP Console → + → New Language Server → Template → Import from directory…`

Pick the extracted directory. All fields are pre-filled. Full walkthrough including manual-setup fallback: [`editors/jetbrains/README.md`](editors/jetbrains/README.md).

### Claude Code

Two paths, pick whichever matches your setup.

**Marketplace (recommended).** Inside Claude Code:

```text
/plugin marketplace add Typiqally/lintropy
/plugin install lintropy-lsp@lintropy
```

The marketplace manifest lives at the repo root so this works from any clean Claude Code install — no `lintropy` CLI required. For a local checkout: `/plugin marketplace add /absolute/path/to/lintropy`.

**CLI.** When you want the plugin manifest to pin the absolute path of your local `lintropy` binary:

```console
lintropy lsp install claude-code                    # --scope project by default
lintropy lsp install claude-code --scope user       # personal-only
lintropy lsp install claude-code --no-install       # print the claude plugin install command instead
```

The CLI generates the plugin manifest fresh (version synced to `lintropy`, extension map scoped to compiled-in languages, `command` resolved to the absolute binary path), writes it to the cwd, and shells out to `claude plugin install` when the `claude` CLI is on `PATH`.

### JSON Schemas

This repo checks in JSON Schemas for every lintropy YAML surface:

- `editors/schemas/lintropy.schema.json` — repo-root `lintropy.yaml`
- `editors/schemas/lintropy-rule.schema.json` — `.lintropy/**/*.rule.yaml`
- `editors/schemas/lintropy-rules.schema.json` — `.lintropy/**/*.rules.yaml`

Workspace settings in `.vscode/settings.json` and `.idea/jsonSchemas.xml` wire them into VS Code / Cursor and JetBrains IDEs respectively. Refresh after schema changes with `./scripts/export-editor-schemas.sh`.

## Status

Pre-1.0. CLI surface stable enough to pin a tag; YAML schema and diagnostic
format may change before 1.0. Track progress and file issues on
[GitHub](https://github.com/Typiqally/lintropy/issues).

## License

[MIT](LICENSE)
