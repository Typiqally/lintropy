# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
This project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] — unreleased

First release of the Phase 1 MVP (tracked in `specs/merged/2026-04-18-lintropy-merged.md`, §13.1).

### Added

- **Config surface.** `lintropy.yaml` root config with `settings.fail_on` /
  `settings.default_severity`; rule discovery from `.lintropy/**/*.{rule,rules}.yaml`;
  JSON schema emission via `lintropy schema`.
- **Query and match rules.** Tree-sitter `query:` rules with
  `@match`/`@capture` conventions and `{{capture}}` message/fix interpolation;
  regex `match:` rules for plain-text conventions.
- **Custom predicates.** `#has-ancestor?`, `#not-has-ancestor?`,
  `#has-child?`, `#not-has-child?`, `#not-has-preceding-comment?`, and the
  file-scope `#in-file?` / `#not-in-file?` pair.
- **Engine.** Parallel per-file execution via `rayon`, predicate filtering,
  deterministic diagnostic ordering.
- **Reporters.** Rustc-style text reporter (with rule-source and `explain`
  hint) + canonical JSON envelope (§7.3).
- **Autofix.** In-place `--fix` with overlap detection; unified-diff
  `--fix-dry-run`; atomic writes.
- **Suppressions.** `// lintropy-disable-next-line[(ids)]`,
  file-level `// lintropy-disable(ids)`, and an
  unused-directive reporter that surfaces never-matched IDs.
- **CLI.** `check` (default), `explain`, `rules`, `init`, `init --with-skill`,
  `schema`, `config validate`, `ts-parse`, and `hook` subcommands per §9.
- **Agent integration.** `init --with-skill` installs the canonical
  `SKILL.md` into `.claude/skills/lintropy/` and `.cursor/skills/lintropy/`
  and merges a `PostToolUse` entry into `.claude/settings.json`
  idempotently. `lintropy hook` reads a Claude-style stdin payload, scopes
  the engine to one file, and writes diagnostics to stderr with exit code
  `0` / `2` per §15.
- **Example repo.** `examples/rust-demo/` with four canonical diagnostics
  across three files, used as the integration-test fixture.
- **Integration tests.** `tests/integration_{check,fix,hook,init}.rs` at
  the crate root exercise the full pipeline end-to-end.
- **CI.** fmt + clippy (`-D warnings`) + test matrix
  (`ubuntu-latest` + `macos-latest`) + non-blocking `cargo deny` on stable
  Rust 1.95.
- **Languages.** First-class Go, Python, and TypeScript support in
  addition to Rust. Enabled by default via Cargo features `lang-go`,
  `lang-python`, `lang-typescript`; build a Rust-only binary with
  `cargo install lintropy --no-default-features`. TypeScript covers
  `.ts`, `.tsx`, `.mts`, `.cts`, and `.d.ts`; the CLI picks the
  `typescript` vs `tsx` grammar per file. Rules declare
  `language: typescript` for both.
- **`ts-parse` auto-detect.** `lintropy ts-parse <file>` now derives the
  language from the file extension by default; `--lang <name>` remains
  as an explicit override. Error messages list every compiled-in
  language so the user sees exactly what is available.

### Changed

- **Internal API.** `Language::ts_language` now takes a `&Path` argument
  so TypeScript can dispatch between the `typescript` and `tsx`
  grammars. Other languages ignore it. Not a published SDK surface.

### Notes

- The default binary grows by roughly 5–7 MB because it bundles the
  three additional tree-sitter grammars. `--no-default-features`
  produces a Rust-only build of the same size as before.

[0.1.0]: https://github.com/anthropics/lintropy/releases/tag/v0.1.0
