# Lintropy for VS Code / Cursor

Live diagnostics, code actions (autofix), autoreload on rule changes, and
semantic highlighting for tree-sitter `query: |` blocks in `.lintropy/*.yaml`
files — all delivered over LSP by a single extension.

## Install

One command (via the `lintropy` CLI):

```console
lintropy install-editor vscode        # or: cursor
```

The extension activates automatically when your workspace contains
`lintropy.yaml` or a `.lintropy/` directory, or when you open a Rust file.
If the `lintropy` binary is not on PATH, the extension auto-downloads the
matching release binary into its global storage on first activation (see
`lintropy.binarySource`).

## Settings

| Setting                  | Default    | Description                                                                 |
| ------------------------ | ---------- | --------------------------------------------------------------------------- |
| `lintropy.enable`        | `true`     | Toggle the language server on/off.                                          |
| `lintropy.path`          | `lintropy` | Path to the binary. PATH lookup + auto-download by default.                 |
| `lintropy.binarySource`  | `auto`     | `auto` (PATH + download fallback) vs `path` (require PATH / explicit path). |
| `lintropy.trace.server`  | `off`      | `messages` / `verbose` logs LSP traffic to the channel.                     |

## Commands

- `Lintropy: Restart Language Server`

## How it works

The extension spawns `lintropy lsp` as a subprocess and speaks LSP over
stdio. Diagnostics, quickfixes, config reload (on `.lintropy/**/*.yaml`
changes), and `query: |` syntax colouring all flow through the standard
protocol — no custom API. Query colouring is delivered via
`textDocument/semanticTokens`, so the server is the single source of
truth for query-DSL highlighting across every LSP-aware editor.

## Developing

```
pnpm install      # or npm install
pnpm run compile
pnpm exec vsce package --no-yarn -o lintropy.vsix
```
