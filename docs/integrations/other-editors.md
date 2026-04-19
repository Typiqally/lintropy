---
title: Other LSP editors
---

# Other LSP-capable editors

Any editor that can launch an LSP server over stdio can use:

```console
lintropy lsp
```

The server:

- resolves the nearest ancestor `lintropy.yaml` per file instead of using one workspace-wide config
- treats a nested `lintropy.yaml` as a fresh rule context for that subtree
- merges `.lintropy/` rule files into the resolved context for that root
- republishes diagnostics when watched config files change
- supports quickfix code actions for diagnostics with `fix`

Confirmed clients include Neovim (`nvim-lspconfig`), Helix, and Zed. Point the client at the `lintropy lsp` command with no arguments; semantic tokens + diagnostics work out of the box.
