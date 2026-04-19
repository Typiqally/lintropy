# CLI Guide

This page covers the user-facing CLI commands shipped today.

## `lintropy check`

Runs lintropy against one or more paths.

```console
lintropy check .
lintropy check src tests
```

Important flags:

- `--config PATH`: use an explicit config file
- `--format text|json`: choose reporter format
- `-o, --output PATH`: write output to a file
- `--fix`: apply autofixes in place
- `--fix-dry-run`: print a unified diff instead of applying fixes
- `--no-color`: disable color in text output
- `--quiet`: suppress report output while keeping the exit code behavior

Exit behavior:

- `0` when no diagnostic meets `settings.fail_on`
- `1` when at least one diagnostic meets `settings.fail_on`
- `2` for user/config errors
- `3` for internal errors

## `lintropy config validate`

Loads config and validates it without running the engine.

```console
lintropy config validate
lintropy config validate ./lintropy.yaml
```

Use this when:

- adding or editing rules
- debugging broken config
- checking whether a repo loads before running a full scan

## `lintropy rules`

Lists loaded rules.

```console
lintropy rules
lintropy rules --format json
lintropy rules --group-by language
lintropy rules --group-by tag
```

Text output is best for browsing. JSON output is best for tooling.

## `lintropy explain <rule-id>`

Prints one rule in expanded form.

```console
lintropy explain no-unwrap
```

This includes:

- severity
- language
- source file
- tags
- docs URL
- message
- description
- query or match body
- fix template

Use it when a rule fires and you want to see the exact source definition.

## `lintropy init`

Scaffolds a repo.

```console
lintropy init
lintropy init path/to/repo
lintropy init --with-skill
lintropy init --with-skill --skill-dir ./somewhere
```

It creates:

- `lintropy.yaml`
- `.lintropy/no-unwrap.rule.yaml`
- `.vscode/extensions.json` if missing

`--with-skill` also installs the bundled `SKILL.md` into detected `.claude/` or `.cursor/` directories.

The command refuses to overwrite existing scaffold files.

## `lintropy ts-parse <file>`

Prints the Tree-sitter S-expression for a source file.

```console
lintropy ts-parse src/main.rs
lintropy ts-parse some-file.rs --lang rust
```

Use this before writing a structural rule. It is the quickest way to confirm node names and nesting.

## `lintropy schema`

Prints the JSON Schema for one config surface.

```console
lintropy schema
lintropy schema --kind rule
lintropy schema --kind rules
lintropy schema --kind root -o ./lintropy.schema.json
```

Useful for:

- editor integration
- tooling
- AI grounding

## `lintropy lsp`

Starts the Language Server Protocol backend over stdio.

```console
lintropy lsp
```

Normally you do not run this directly in a terminal. Your editor starts it for you.

## `lintropy install <target>`

One top-level command covers every target:

```console
lintropy install vscode                          # VS Code
lintropy install cursor --profile Default        # Cursor, named profile
lintropy install vscode --package-only -o out.vsix  # just build the .vsix
lintropy install jetbrains --dir ~/.lintropy     # JetBrains (LSP4IJ)
lintropy install claude-code                     # Claude Code plugin + bundled skill
```

For VS Code / Cursor this builds the checked-out extension source with
`pnpm`, packages a local `.vsix`, and either installs it into the editor
or writes it to disk. For JetBrains it unpacks the LSP4IJ custom template
for a one-time IDE import. For Claude Code it generates the plugin
manifest fresh (version + feature-gated extension map + absolute binary
path), bundles the lintropy skill inside the plugin directory at
`skills/lintropy/SKILL.md`, and prints the `claude --plugin-dir <path>`
invocation needed to load the plugin. The skill loads and unloads with
the plugin.

See [`Integrations`](integrations/index.md) for per-target walkthroughs
including the Claude Code marketplace flow.

## Suggested daily workflow

Most users only need these commands:

1. `lintropy check .`
2. `lintropy check . --fix-dry-run`
3. `lintropy check . --fix`
4. `lintropy rules`
5. `lintropy explain <id>`
6. `lintropy ts-parse <file>`
