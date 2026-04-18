/**
 * VS Code / Cursor client for the lintropy language server.
 *
 * Resolves a `lintropy` binary (explicit setting → PATH → auto-download
 * from GitHub Releases) and spawns `<binary> lsp` as a subprocess over
 * stdio. All diagnostic logic lives in the Rust server; this file is
 * glue plus the one-time binary bootstrap.
 */
import * as child_process from "node:child_process";
import * as fs from "node:fs";
import * as fsp from "node:fs/promises";
import * as https from "node:https";
import * as path from "node:path";
import {
  commands,
  ExtensionContext,
  ProgressLocation,
  Uri,
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

const REPO = "Typiqally/lintropy";

export async function activate(context: ExtensionContext): Promise<void> {
  const config = workspace.getConfiguration("lintropy");
  if (!config.get<boolean>("enable", true)) {
    return;
  }

  const binary = await resolveBinary(context, config);
  if (!binary) {
    return;
  }

  client = buildClient(binary);
  await client.start();

  context.subscriptions.push(
    commands.registerCommand("lintropy.restart", async () => {
      if (!client) return;
      await client.stop();
      const resolved = await resolveBinary(
        context,
        workspace.getConfiguration("lintropy"),
      );
      if (!resolved) return;
      client = buildClient(resolved);
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

function buildClient(binary: string): LanguageClient {
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
  };

  return new LanguageClient("lintropy", "Lintropy", serverOptions, clientOptions);
}

/**
 * Resolve the lintropy binary path.
 *
 * Precedence:
 *   1. `lintropy.path` if set to a non-default value → trust the user.
 *   2. `lintropy` on PATH → happy path for `cargo install` / `brew install`.
 *   3. `lintropy.binarySource === "auto"` → download + cache in globalStorage.
 *   4. error: tell the user how to fix it.
 */
async function resolveBinary(
  context: ExtensionContext,
  config: WorkspaceConfiguration,
): Promise<string | undefined> {
  const explicit = config.get<string>("path", "lintropy");
  const binarySource = config.get<"auto" | "path">("binarySource", "auto");

  if (explicit && explicit !== "lintropy") {
    if (await isRunnable(explicit)) return explicit;
    window.showErrorMessage(
      `Lintropy: configured lintropy.path (${explicit}) is not runnable.`,
    );
    return undefined;
  }

  if (await isOnPath("lintropy")) {
    return "lintropy";
  }

  if (binarySource === "path") {
    window.showErrorMessage(
      "Lintropy: `lintropy` not found on PATH. Install it with " +
        "`cargo install lintropy` / `brew install lintropy`, or set " +
        "`lintropy.binarySource` to `auto` to download it automatically.",
    );
    return undefined;
  }

  return downloadAndCache(context);
}

async function isRunnable(binary: string): Promise<boolean> {
  return new Promise((resolve) => {
    const proc = child_process.spawn(binary, ["--version"], {
      stdio: "ignore",
    });
    proc.on("error", () => resolve(false));
    proc.on("exit", (code) => resolve(code === 0));
  });
}

async function isOnPath(name: string): Promise<boolean> {
  return new Promise((resolve) => {
    const bin = process.platform === "win32" ? "where" : "which";
    const proc = child_process.spawn(bin, [name], { stdio: "ignore" });
    proc.on("error", () => resolve(false));
    proc.on("exit", (code) => resolve(code === 0));
  });
}

/**
 * Download the platform-matching `lintropy-<version>-<target>.tar.gz`
 * from the GitHub release that matches this extension's version, cache
 * it under the extension's globalStorageUri, and return the absolute
 * path to the unpacked binary.
 */
async function downloadAndCache(
  context: ExtensionContext,
): Promise<string | undefined> {
  const version = context.extension.packageJSON.version as string;
  const target = rustTarget();
  if (!target) {
    window.showErrorMessage(
      `Lintropy: no prebuilt binary for platform ${process.platform} / ${process.arch}. ` +
        "Install from source (`cargo install lintropy`) and set `lintropy.path`.",
    );
    return undefined;
  }

  const cacheRoot = Uri.joinPath(context.globalStorageUri, "bin", version);
  const binaryPath = path.join(cacheRoot.fsPath, "lintropy");
  if (fs.existsSync(binaryPath)) {
    return binaryPath;
  }

  await fsp.mkdir(cacheRoot.fsPath, { recursive: true });

  const archiveName = `lintropy-${version}-${target}.tar.gz`;
  const url = `https://github.com/${REPO}/releases/download/v${version}/${archiveName}`;
  const archivePath = path.join(cacheRoot.fsPath, archiveName);

  const ok = await window.withProgress(
    {
      location: ProgressLocation.Notification,
      title: `Lintropy: downloading v${version} (${target})`,
      cancellable: false,
    },
    async () => {
      try {
        await downloadFile(url, archivePath);
        await extractTar(archivePath, cacheRoot.fsPath);
        await fsp.unlink(archivePath).catch(() => {});
        return true;
      } catch (err) {
        window.showErrorMessage(
          `Lintropy: failed to download from ${url}: ${err instanceof Error ? err.message : err}`,
        );
        return false;
      }
    },
  );
  if (!ok) return undefined;

  // Release tarballs unpack to `lintropy-<version>-<target>/lintropy`.
  const unpackedDir = path.join(cacheRoot.fsPath, `lintropy-${version}-${target}`);
  const unpackedBin = path.join(unpackedDir, "lintropy");
  if (fs.existsSync(unpackedBin)) {
    await fsp.chmod(unpackedBin, 0o755);
    await fsp.rename(unpackedBin, binaryPath);
    await fsp.rm(unpackedDir, { recursive: true, force: true });
  } else if (fs.existsSync(binaryPath)) {
    // Tarball laid the binary out flat — nothing to do.
  } else {
    window.showErrorMessage(
      `Lintropy: extracted archive did not contain a lintropy binary at ${unpackedBin}.`,
    );
    return undefined;
  }
  await fsp.chmod(binaryPath, 0o755);
  return binaryPath;
}

function rustTarget(): string | undefined {
  if (process.platform === "darwin" && process.arch === "arm64")
    return "aarch64-apple-darwin";
  if (process.platform === "darwin" && process.arch === "x64")
    return "x86_64-apple-darwin";
  if (process.platform === "linux" && process.arch === "x64")
    return "x86_64-unknown-linux-gnu";
  return undefined;
}

async function downloadFile(url: string, destPath: string): Promise<void> {
  await new Promise<void>((resolve, reject) => {
    const attempt = (target: string, redirects: number) => {
      if (redirects > 5) {
        reject(new Error("too many redirects"));
        return;
      }
      https
        .get(target, (res) => {
          const status = res.statusCode ?? 0;
          if (status >= 300 && status < 400 && res.headers.location) {
            res.resume();
            attempt(res.headers.location, redirects + 1);
            return;
          }
          if (status !== 200) {
            res.resume();
            reject(new Error(`HTTP ${status} from ${target}`));
            return;
          }
          const file = fs.createWriteStream(destPath);
          res.pipe(file);
          file.on("finish", () => file.close(() => resolve()));
          file.on("error", reject);
        })
        .on("error", reject);
    };
    attempt(url, 0);
  });
}

async function extractTar(archive: string, dest: string): Promise<void> {
  await new Promise<void>((resolve, reject) => {
    const proc = child_process.spawn("tar", ["-xzf", archive, "-C", dest]);
    proc.on("error", reject);
    proc.on("exit", (code) =>
      code === 0 ? resolve() : reject(new Error(`tar exited with ${code}`)),
    );
  });
}
