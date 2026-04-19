---
title: Other agents
---

# Other agents

| Agent | LSP support | Notes |
| --- | --- | --- |
| Continue | Partial | Wrap `lintropy lsp` behind an MCP bridge and connect it through Continue's MCP config. |
| Cursor (agent mode) | IDE only | The Cursor IDE already runs the LSP extension (see [VS Code and Cursor](vscode.md)); the in-IDE agent sees those diagnostics without extra setup. |
| Aider | No | No LSP client in the CLI. Use the [post-write hook](post-write-hook.md) instead. |
| Codex CLI | No | No maintained LSP client. Use the [post-write hook](post-write-hook.md) instead. |

For any agent not listed here: if it launches an LSP server over stdio, the command is always `lintropy lsp` with no arguments. If it only supports hook-style integration, use the post-write hook.
