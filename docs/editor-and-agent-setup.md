# Editor And Agent Setup

Lintropy ships both local editor support and agent-oriented hook support.

## VS Code and Cursor

There are two separate integration pieces:

1. Query syntax highlighting for `query: |` blocks inside YAML rules.
2. LSP-based live diagnostics and quickfixes.

### Query syntax highlighting

Install the bundled extension:

```console
lintropy install-query-extension vscode
lintropy install-query-extension cursor
```

Optional profile:

```console
lintropy install-query-extension cursor --profile Default
```

Package the `.vsix` without installing:

```console
lintropy install-query-extension --package-only -o ./lintropy-query-syntax.vsix
```

### Live diagnostics and quickfixes

Install the LSP extension:

```console
lintropy install-lsp-extension vscode
lintropy install-lsp-extension cursor
```

From a source checkout, this builds the local extension, packages a `.vsix`,
and installs it into the target editor.

The extension starts `lintropy lsp`, publishes diagnostics as buffers change, and exposes quickfix actions when a diagnostic carries an autofix.

The VS Code / Cursor client resolves the `lintropy` binary in this order:

1. explicit `lintropy.path`
2. `PATH`
3. extension-managed download from the matching GitHub release

### JSON Schema support

Lintropy also ships JSON Schemas for:

- `lintropy.yaml`
- `.lintropy/*.rule.yaml`
- `.lintropy/*.rules.yaml`

These support completion and validation in YAML-aware editors.

## JetBrains IDEs

JetBrains support is split the same way:

1. query highlighting through a TextMate bundle
2. diagnostics through an LSP4IJ template

### Query highlighting

Extract the bundled TextMate bundle:

```console
lintropy install-textmate-bundle
```

Then import it in the IDE:

`Settings → Editor → TextMate Bundles → +`

### Live diagnostics via LSP4IJ

Extract the LSP template:

```console
lintropy install-lsp-template jetbrains
```

Then in the IDE:

`View → Tool Windows → LSP Console → + → New Language Server → Template → Import from directory`

Select the extracted template directory.

## Other LSP-capable editors

Any editor that can launch an LSP server over stdio can use:

```console
lintropy lsp
```

The server:

- loads config from the workspace root on initialize
- republishes diagnostics as files change
- supports quickfix code actions for diagnostics with `fix`

## Agent workflows

Lintropy also supports post-write hook workflows for coding agents.

### `lintropy init --with-skill`

When `.claude/` or `.cursor/` exists, this command installs the bundled `SKILL.md` into the appropriate skill directory.

For Claude Code, it also updates `.claude/settings.json` to add a `PostToolUse` command hook:

```text
lintropy hook --agent claude-code
```

### `lintropy hook`

This command is designed for machine-to-machine use, not direct human use.

Behavior:

- reads JSON payloads from stdin
- extracts a written file path
- skips work if the file is gitignored
- lints only that file
- emits compact text or JSON diagnostics to stderr
- returns a blocking exit code when diagnostics meet the configured hook threshold

Current agent support:

- Claude Code hook payloads are the implemented target
- Codex is present as a CLI option, but auto-detection is still effectively Claude-first

## Recommended setup

For most teams:

1. Run `lintropy init --with-skill`.
2. Install query highlighting in the main editor.
3. Install the LSP integration.
4. Keep `lintropy check .` in CI.
5. Use the hook only for fast feedback after edits, not as the only enforcement point.
