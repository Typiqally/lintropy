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

## Editor support

This repo checks in JSON Schemas for all lintropy YAML surfaces:

- `editors/schemas/lintropy.schema.json` for repo-root `lintropy.yaml`
- `editors/schemas/lintropy-rule.schema.json` for `.lintropy/**/*.rule.yaml`
- `editors/schemas/lintropy-rules.schema.json` for `.lintropy/**/*.rules.yaml`

Refresh them after schema changes with:

```console
./scripts/export-editor-schemas.sh
```

### One command, any editor

```console
lintropy install-editor vscode       # or: cursor
lintropy install-editor jetbrains --dir ~/.lintropy
```

For VS Code / Cursor this installs a single extension that bundles **both**
the LSP client (live diagnostics, quickfixes, config-reload) **and** the
`query:` grammar injection that highlights tree-sitter queries inside YAML.
For JetBrains this unpacks **both** the LSP4IJ template (live diagnostics)
and the TextMate bundle (query-DSL highlighting) — both still need a one-time
import in the IDE (see subsections below).

The rest of this section covers the per-editor details, JSON schema mappings
for editor-side YAML completion, and the underlying building-block commands.

### VS Code / Cursor

Workspace settings in `.vscode/settings.json` associate those schemas with the
matching files, and `.vscode/extensions.json` recommends `redhat.vscode-yaml`.
Cursor uses the same workspace settings, so completions, hover docs, and
validation work there as well.

#### Extension install

```console
lintropy install-editor vscode       # or: cursor
```

Under the hood this calls `lintropy install-lsp-extension vscode`, which
downloads the matching `.vsix` from the GitHub release and hands it to
`code --install-extension`. The extension contributes both live diagnostics
and the injected `source.lintropy-query` grammar for YAML `query: |` blocks,
so there is no separate "query syntax" extension to install.

Once installed, the extension resolves the `lintropy` binary in this order:
explicit `lintropy.path` setting → PATH lookup → automatic download from
the matching GitHub release into the extension's global storage (controlled
by `lintropy.binarySource`). So `install-editor vscode` followed by opening
a Rust file is sufficient even on a machine where `lintropy` is not on PATH.

Other settings: `lintropy.enable`, `lintropy.trace.server`, `lintropy.binarySource`
(see [`editors/vscode/lintropy/README.md`](editors/vscode/lintropy/README.md)).

Contributors with a checkout can build + install the local `.vsix` directly:

```console
cd editors/vscode/lintropy
npm install && npm run compile
npx vsce package -o lintropy.vsix
code --install-extension lintropy.vsix
```

### JetBrains IDEs

Shared mappings live in `.idea/jsonSchemas.xml`, pointing JetBrains IDEs at the
same checked-in schema files for:

- `lintropy.yaml`
- `.lintropy/**/*.rule.yaml`
- `.lintropy/**/*.rules.yaml`

If your IDE ignores shared `.idea` files, add the same three mappings manually
under `Languages & Frameworks | Schemas and DTDs | JSON Schema Mappings`.

#### Assets install

```console
lintropy install-editor jetbrains --dir ~/.lintropy
```

This unpacks two independent assets side-by-side into `--dir`:

- `Lintropy Query.tmbundle/` — TextMate grammar for the query DSL inside
  YAML `query: |` blocks.
- `lsp4ij-template/` — LSP4IJ custom server template for live diagnostics.

Both still need a one-time import in the IDE (JetBrains has no equivalent
of `code --install-extension`).

##### TextMate bundle import

`Settings → Editor → TextMate Bundles → +` and select the extracted
`Lintropy Query.tmbundle` directory. The bundle adds a TextMate injection
that highlights YAML `query: |` blocks using the same
`source.lintropy-query` grammar as the VS Code / Cursor extension.

##### LSP4IJ server import

JetBrains IDEs plug into `lintropy lsp` through the
[LSP4IJ](https://plugins.jetbrains.com/plugin/23257-lsp4ij) community plugin.
Works on all JetBrains IDEs including free Community editions.

After `install-editor jetbrains --dir ~/.lintropy`, in your IDE:
`View → Tool Windows → LSP Console → + → New Language Server → Template →
Import from directory...` and pick `~/.lintropy/lsp4ij-template`. All
fields (name, command, `*.rs → rust` mapping) are pre-filled.

Full walkthrough including manual-setup fallback and troubleshooting:
[`editors/jetbrains/README.md`](editors/jetbrains/README.md).

## Status

Pre-1.0. CLI surface stable enough to pin a tag; YAML schema and diagnostic
format may change before 1.0. Track progress and file issues on
[GitHub](https://github.com/Typiqally/lintropy/issues).

## License

[MIT](LICENSE)
