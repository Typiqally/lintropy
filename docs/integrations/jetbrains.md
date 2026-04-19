---
title: JetBrains IDEs
---

# JetBrains IDEs

```console
lintropy lsp install jetbrains --dir ~/.lintropy
```

This unpacks the [LSP4IJ](https://plugins.jetbrains.com/plugin/23257-lsp4ij) custom server template. One import step in the IDE:

`View → Tool Windows → LSP Console → + → New Language Server → Template → Import from directory…`

Pick the extracted directory (default name `lsp4ij-template`). All fields — name, command, `*.rs → rust`, `*.rule.yaml → yaml` mappings — are pre-filled.

## Flags

- `--dir <PATH>` — parent directory for the extracted template.
- `--force` — overwrite an existing template directory.

## Notes

Semantic tokens for the `query: |` DSL are not painted inside JetBrains IDEs — LSP4IJ discards them for composite PSI elements. Diagnostics and inline rule-file linting still work. See [`editors/jetbrains/README.md`](https://github.com/Typiqally/lintropy/blob/main/editors/jetbrains/README.md) for the manual-setup fallback.
