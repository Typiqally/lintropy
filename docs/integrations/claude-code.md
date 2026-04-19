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
lintropy install claude-code
```

Generates the plugin manifest fresh (version synced to the installed `lintropy`, extension map scoped to the compiled-in languages, `command` resolved to the absolute binary path), bundles the lintropy skill at `<plugin-dir>/skills/lintropy/SKILL.md`, and prints the `claude --plugin-dir <path>` invocation you should run to load the plugin. Use `/reload-plugins` inside a running session to pick up manifest edits without restarting.

Why not shell out to `claude plugin install` automatically? Current `claude` CLIs only accept `<name>@<marketplace>` for `install`, and registering a throwaway marketplace per dev iteration is worse ergonomics than `--plugin-dir`. For a persistent install, use the marketplace flow above.

### Flags

- `--dir <PATH>` — write the plugin directory somewhere other than the cwd.
- `--force` — overwrite an existing plugin directory.

Prefer the CLI path when you want the generated `command` to be an absolute path to the local `lintropy` binary, or when your Claude Code subprocess environment has a different `PATH` than your shell.

## What the plugin does

It maps YAML, Rust, Go, Python, and TypeScript file extensions to the LSP language ids `lintropy lsp` expects. The extension map is feature-gated, so a binary built without a language feature won't register `lintropy` for that file type. No environment variable is needed — Claude Code's LSP tool activates automatically once a plugin registers a server.

The plugin directory ships a bundled skill at `skills/lintropy/SKILL.md`. Claude Code auto-loads it when the plugin activates and drops it when the plugin is removed, so the skill lifecycle is tied to the plugin.
