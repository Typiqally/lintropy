/**
 * VS Code / Cursor client for the lintropy language server.
 *
 * Spawns `lintropy lsp` as a subprocess (path configurable via
 * `lintropy.path`) and wires its LSP traffic into VS Code's diagnostics,
 * code actions, and file watchers. Treat this file as thin glue — all
 * diagnostic logic lives in the Rust server.
 */
import * as path from "node:path";
import {
  commands,
  ExtensionContext,
  window,
  workspace,
  WorkspaceConfiguration,
} from "vscode";
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
  TransportKind,
} from "vscode-languageclient/node";

let client: LanguageClient | undefined;

export async function activate(context: ExtensionContext): Promise<void> {
  const config = workspace.getConfiguration("lintropy");
  if (!config.get<boolean>("enable", true)) {
    return;
  }

  client = buildClient(config);
  await client.start();

  context.subscriptions.push(
    commands.registerCommand("lintropy.restart", async () => {
      if (!client) return;
      await client.stop();
      client = buildClient(workspace.getConfiguration("lintropy"));
      await client.start();
      window.showInformationMessage("Lintropy: language server restarted.");
    }),
  );
}

export async function deactivate(): Promise<void> {
  if (client) {
    await client.stop();
    client = undefined;
  }
}

function buildClient(config: WorkspaceConfiguration): LanguageClient {
  const binary = config.get<string>("path", "lintropy");
  const serverOptions: ServerOptions = {
    run: { command: binary, args: ["lsp"], transport: TransportKind.stdio },
    debug: { command: binary, args: ["lsp"], transport: TransportKind.stdio },
  };

  // Lint Rust files and re-lint all open buffers whenever any repo-local
  // rule YAML changes — the server treats `didChangeWatchedFiles` as a
  // full config-reload trigger.
  const clientOptions: LanguageClientOptions = {
    documentSelector: [{ scheme: "file", language: "rust" }],
    synchronize: {
      fileEvents: [
        workspace.createFileSystemWatcher("**/lintropy.yaml"),
        workspace.createFileSystemWatcher("**/.lintropy/**/*.yaml"),
        workspace.createFileSystemWatcher("**/.lintropy/**/*.yml"),
      ],
      configurationSection: "lintropy",
    },
    outputChannelName: "Lintropy",
    // Honored automatically because we expose `lintropy.trace.server` in
    // `contributes.configuration`.
  };

  void path; // keep import for future workspace-root helpers

  return new LanguageClient(
    "lintropy",
    "Lintropy",
    serverOptions,
    clientOptions,
  );
}
