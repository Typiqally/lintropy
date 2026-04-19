# Lintropy — Claude Code plugin

Registers `lintropy lsp` as a Language Server for Claude Code so lintropy
diagnostics appear live while the agent reads and edits files.

## Install

### Marketplace (recommended)

Inside Claude Code:

```text
/plugin marketplace add Typiqally/lintropy
/plugin install lintropy-lsp@lintropy
```

For a local checkout:

```text
/plugin marketplace add /absolute/path/to/lintropy
/plugin install lintropy-lsp@lintropy
```

### CLI

```console
lintropy install claude-code
```

The CLI generates this plugin directory freshly (version synced to the
installed `lintropy`, extension map scoped to compiled-in languages,
`command` resolved to the absolute binary path), bundles the lintropy
skill at `skills/lintropy/SKILL.md`, and prints the
`claude --plugin-dir <path>` invocation to load it. Use
`/reload-plugins` inside a running session after manifest edits.

## What's inside

- `.claude-plugin/plugin.json` — registers the `lintropy` LSP server and
  maps the file extensions lintropy knows to their LSP language ids.
- `skills/lintropy/SKILL.md` — lintropy authoring skill for the agent,
  bundled so Claude Code loads it whenever the plugin is active.

The `lintropy` binary must be on `PATH`, or override `command` in
`plugin.json` with an absolute path.
