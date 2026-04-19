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

## `lintropy hook`

Runs a single-file check from an agent/editor post-write hook payload on stdin.

```console
some-tool | lintropy hook
some-tool | lintropy hook --agent claude-code
some-tool | lintropy hook --format json
some-tool | lintropy hook --fail-on warning
```

Current behavior:

- extracts a file path from known JSON payload shapes
- loads config from the current repo
- skips gitignored files
- runs a single-file lint
- exits `2` only when matching diagnostics meet the hook `--fail-on` threshold

If the hook payload is missing or malformed, the command quietly exits `0`.

## `lintropy lsp`

Starts the Language Server Protocol backend over stdio.

```console
lintropy lsp
```

Normally you do not run this directly in a terminal. Your editor starts it for you.

## Editor install commands

Query highlighting for VS Code or Cursor:

```console
lintropy install-query-extension vscode
lintropy install-query-extension cursor
lintropy install-query-extension --package-only -o ./lintropy-query.vsix
```

TextMate bundle for JetBrains query highlighting:

```console
lintropy install-textmate-bundle
lintropy install-textmate-bundle ./tmp
lintropy install-textmate-bundle --force
```

LSP extension for VS Code or Cursor:

```console
lintropy install-lsp-extension vscode
lintropy install-lsp-extension cursor --profile Default
lintropy install-lsp-extension --package-only -o ./lintropy.vsix
```

This builds the checked-out extension source with `pnpm`, packages a local
`.vsix`, and then either installs it into the editor or writes it to disk.

JetBrains LSP4IJ template:

```console
lintropy install-lsp-template jetbrains
lintropy install-lsp-template jetbrains ./tmp
```

## Suggested daily workflow

Most users only need these commands:

1. `lintropy check .`
2. `lintropy check . --fix-dry-run`
3. `lintropy check . --fix`
4. `lintropy rules`
5. `lintropy explain <id>`
6. `lintropy ts-parse <file>`
