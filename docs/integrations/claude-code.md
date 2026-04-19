---
title: Claude Code
---

# Claude Code

Lintropy ships a Claude Code plugin that registers `lintropy lsp` as a Language Server. Two install paths — pick whichever matches your setup.

## Marketplace (recommended)

Inside Claude Code:

```text
/plugin marketplace add Typiqally/lintropy
/plugin install lintropy-lsp@lintropy
```

The marketplace manifest lives at the root of the GitHub repo, so this works from any clean Claude Code install — no `lintropy` CLI needed to bootstrap.

For a local checkout, point the marketplace at the absolute path instead:

```text
/plugin marketplace add /absolute/path/to/lintropy
/plugin install lintropy-lsp@lintropy
```

This reads the same `editors/claude-code/.claude-plugin/plugin.json` straight from disk, so edits to the manifest take effect after `/plugin marketplace update lintropy`.

## CLI

```console
lintropy lsp install claude-code
```

Generates the plugin manifest fresh (version synced to the installed `lintropy`, extension map scoped to the compiled-in languages, `command` resolved to the absolute binary path) and then shells out to `claude plugin install <dir> --scope <scope>` when the `claude` CLI is on `PATH`.

### Flags

- `--scope project` (default) — team-shared, recorded in `.claude/settings.json`.
- `--scope user` — personal-only install.
- `--no-install` — write the plugin directory but do not shell out; prints the `claude plugin install` command for you to run.
- `--dir <PATH>` — write the plugin directory somewhere other than the cwd.
- `--force` — overwrite an existing plugin directory.

Prefer the CLI path when you want the generated `command` to be an absolute path to the local `lintropy` binary, or when your Claude Code subprocess environment has a different `PATH` than your shell.

## What the plugin does

It maps YAML, Rust, Go, Python, and TypeScript file extensions to the LSP language ids `lintropy lsp` expects. The extension map is feature-gated, so a binary built without a language feature won't register `lintropy` for that file type. No environment variable is needed — Claude Code's LSP tool activates automatically once a plugin registers a server.
