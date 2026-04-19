---
title: VS Code & Cursor
---

# VS Code and Cursor

```console
lintropy lsp install vscode
lintropy lsp install cursor
```

This builds the bundled extension source into a `.vsix` and hands it to `code --install-extension` / `cursor --install-extension`. The extension carries:

- the LSP client (diagnostics, quickfixes, config reload)
- semantic-token highlighting for the `query: |` DSL inside rule files

No separate "query syntax" extension.

## Flags

- `--profile <NAME>` — install into a named editor profile.
- `--package-only -o <PATH>` — build the `.vsix` but do not run the editor CLI. Useful in CI.

## Binary resolution

The installed extension resolves `lintropy` in this order: explicit `lintropy.path` setting → `PATH` lookup → extension-managed download from the matching GitHub release.

## Config resolution

Config is resolved path-locally, not workspace-wide:

- each source file uses the nearest ancestor `lintropy.yaml`
- a newly added nested `lintropy.yaml` creates a fresh rule context for that subtree and does not inherit the parent workspace rules
- changes under that root's `.lintropy/` directory merge into the same context
- saving or otherwise notifying the editor about `lintropy.yaml` / `.lintropy/**/*.yaml` changes triggers a config reload and republishes diagnostics for open files
