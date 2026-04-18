# lintropy

`lintropy` is a Rust CLI scaffold for opinionated linting across a codebase.

The intended model is:

1. Load a small TOML config file that is easy for humans, Codex, or Claude Code to edit.
2. Discover files in the target repo.
3. Run a mix of external static analysis tools plus custom opinionated rules.
4. Fail the run according to policy.

The current scaffold includes:

- A `clap`-based CLI with `lint` and `init-config` commands.
- A TOML-backed config schema in `src/config.rs`.
- A starter `lintropy.toml` example designed to be LLM-editable.
- A placeholder lint engine in `src/engine.rs` that loads config and prints the planned pipeline.

## Quick start

Create a starter config:

```bash
cargo run -- init-config
```

Run the scaffolded linter:

```bash
cargo run -- lint .
```

## Config design goals

- Flat, explicit TOML structure.
- Avoid dense nesting unless a rule needs local settings.
- Keep rule IDs, severities, file globs, and messages as plain text.
- Make it safe for automated tools to insert, remove, or update entries.

## Next steps

- Add a real glob-based file selection layer.
- Execute configured external tools and normalize their diagnostics.
- Implement first-party opinionated rules over parsed source content.
- Add JSON output for editor and agent integration.
