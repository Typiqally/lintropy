# Lintropy for VS Code / Cursor

Live diagnostics, code actions (autofix), and autoreload on rule changes —
powered by the `lintropy` language server.

## Install

1. Install the `lintropy` binary: `cargo install lintropy`.
   (Or `brew install lintropy` if you're using the tap.)
2. Install this extension (.vsix) via `code --install-extension lintropy.vsix`
   on VS Code, or `cursor --install-extension lintropy.vsix` on Cursor.

The extension activates automatically when your workspace contains
`lintropy.yaml` or a `.lintropy/` directory, or when you open a Rust file.

## Settings

| Setting                  | Default    | Description                                             |
| ------------------------ | ---------- | ------------------------------------------------------- |
| `lintropy.enable`        | `true`     | Toggle the language server on/off.                      |
| `lintropy.path`          | `lintropy` | Path to the binary. PATH lookup by default.             |
| `lintropy.trace.server`  | `off`      | `messages` / `verbose` logs LSP traffic to the channel. |

## Commands

- `Lintropy: Restart Language Server`

## How it works

The extension spawns `lintropy lsp` as a subprocess and speaks LSP over
stdio. Diagnostics, quickfixes, and config reload (on `.lintropy/**/*.yaml`
changes) all flow through the standard protocol — no custom API.

## Developing

```
pnpm install      # or npm install
pnpm run compile
pnpm exec vsce package --no-yarn -o lintropy.vsix
```
