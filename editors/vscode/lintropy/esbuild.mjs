import esbuild from "esbuild";

const watch = process.argv.includes("--watch");

const ctx = await esbuild.context({
  entryPoints: ["src/extension.ts"],
  bundle: true,
  external: ["vscode"],
  format: "cjs",
  outfile: "out/extension.js",
  platform: "node",
  sourcemap: true,
  target: "node20",
});

if (watch) {
  await ctx.watch();
  console.log("watching");
} else {
  await ctx.rebuild();
  await ctx.dispose();
}
