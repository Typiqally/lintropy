import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";

function readLintropyVersion(): string {
  try {
    const cargoPath = fileURLToPath(new URL("../Cargo.toml", import.meta.url));
    const cargo = readFileSync(cargoPath, "utf8");
    const match = cargo.match(/^version\s*=\s*"([^"]+)"/m);
    return match?.[1] ?? "dev";
  } catch {
    return "dev";
  }
}

export default defineConfig({
  base: "/",
  plugins: [react(), tailwindcss()],
  define: {
    __LINTROPY_VERSION__: JSON.stringify(readLintropyVersion()),
  },
});
