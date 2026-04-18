# Lintropy in JetBrains IDEs (IntelliJ, RustRover, GoLand, WebStorm, PyCharm, …)

There are two supported setups. Pick based on your IDE tier and whether
you want a click-to-install flow or are happy with a per-project config.

## Option A — LSP4IJ (free, works on Community editions)

[LSP4IJ](https://plugins.jetbrains.com/plugin/23257-lsp4ij) is a community
plugin that turns any external LSP server into a first-class JetBrains
diagnostic source. Ships on every JetBrains IDE including the free
Community editions.

### Fast setup — template import (recommended)

1. Install the `lintropy` binary (`cargo install lintropy` or the Homebrew tap).
2. In your IDE: **Settings → Plugins → Marketplace**, search for
   "LSP4IJ", install, restart.
3. Export the bundled template to a known directory:
   ```console
   lintropy install-lsp-template jetbrains --dir ~/.lintropy/lsp4ij
   ```
   (Contributors with a checkout of this repo can point LSP4IJ straight
   at [`editors/jetbrains/lsp4ij-template/`](./lsp4ij-template) instead
   and skip the `install-lsp-template` command.)
4. **View → Tool Windows → LSP Console → `+` → New Language Server →
   Template → Import from directory…** and pick the directory from
   step 3.
5. Confirm the fields are pre-filled (**Name**, **Command**, **Mappings**).
6. Click **OK**. LSP4IJ spawns the server and diagnostics appear on
   `.rs` files.

### Manual setup (fallback)

Use this if the template import fails or you want to customise fields.

1. **Settings → Languages & Frameworks → Language Servers → `+` → New Language Server**.
2. Fill in:
   - **Name**: `lintropy`
   - **Command**: `lintropy lsp`
     (use the absolute path if `lintropy` is not on the IDE's `PATH` —
     e.g. `/opt/homebrew/bin/lintropy lsp`).
3. Open the **Mappings** tab and add:
   - **File name patterns** → `*.rs` → language id `rust`.
4. Open the **Configuration** tab and paste:
   ```json
   {
     "settings": {
       "lintropy": { "enable": true }
     }
   }
   ```
5. Click **OK**.

### Config-reload behavior

LSP4IJ automatically forwards `didChangeWatchedFiles` to the server.
When you edit `lintropy.yaml` or any `.lintropy/**/*.yaml`, the server
reloads and re-lints all open buffers. No IDE restart required.

### Troubleshooting

- **No diagnostics appear** — open **Language Servers → lintropy → Trace**,
  set to `verbose`, then watch the **LSP Console** tool window. The server
  logs config-load failures and missing workspace roots there.
- **"Command not found"** — use an absolute path in step 4. The IDE's
  `PATH` may differ from your shell's (especially on macOS when launched
  from the Dock).

## Option B — Native IntelliJ Platform LSP API (paid IDEs only)

JetBrains shipped a first-class LSP API in 2024.2+. It's available on
**Ultimate-tier** IDEs only: IntelliJ IDEA Ultimate, RustRover, WebStorm,
GoLand, PyCharm Professional, etc. A native plugin built on this API
gives slightly tighter UI integration (inlay hints, quick-fix preview
popups) than LSP4IJ.

A native plugin isn't shipped yet. Track
[`editors/jetbrains/native/`](./native) for progress. Until it lands,
use **Option A** — functionally equivalent.
