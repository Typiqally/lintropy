# Configuration

This page explains the user-facing configuration model: where Lintropy looks, what files it loads, and how rules are scoped.

For the query syntax itself, see [Rule Language](./rule-language.md).

## Root file

At the repo root, Lintropy expects:

```yaml
version: 1
settings:
  fail_on: error
  default_severity: warning
```

`version` is required.

`settings` is optional.

## Settings

### `fail_on`

Controls which severities cause `lintropy check` to exit non-zero.

Valid values:

- `info`
- `warning`
- `error`

Behavior:

- `fail_on: error` means only errors fail the command
- `fail_on: warning` means warnings and errors fail the command
- `fail_on: info` means any diagnostic fails the command

### `default_severity`

Used when a rule omits `severity`.

Valid values:

- `info`
- `warning`
- `error`

## Where rules come from

Lintropy collects rules from:

- inline `rules:` in `lintropy.yaml`
- `.lintropy/**/*.rule.yaml`
- `.lintropy/**/*.rules.yaml`

Rule files are part of the repo. They are not installed globally.

## Rule identity

How `id` is assigned:

- in `*.rule.yaml`, `id` defaults to the filename stem
- in `*.rules.yaml`, every rule must set `id`
- inline root rules must set `id`

Rule ids must be unique across the loaded config set.

## Per-rule fields

Current rule fields:

| Field | Required | Notes |
| --- | --- | --- |
| `id` | sometimes | required inline and in grouped files |
| `severity` | no | falls back to `default_severity` |
| `message` | yes | may interpolate `{{captures}}` |
| `include` | no | gitignore-style path globs |
| `exclude` | no | gitignore-style path globs |
| `tags` | no | free-form labels |
| `docs_url` | no | shown in machine-readable output and rule descriptions |
| `description` | no | human explanation of why the rule exists |
| `language` | yes for `query` rules | `rust`, `go`, `python`, `typescript` |
| `query` | yes for active rules | Tree-sitter query source |
| `fix` | no | query rules only |

Fields already modeled but not executable yet:

- `forbid`
- `require`
- `multiline`

## Include and exclude

`include` and `exclude` are path filters applied before a rule runs.

Example:

```yaml
include: ["src/**/*.rs"]
exclude: ["src/generated/**", "**/*_test.rs"]
```

Behavior:

- empty `include` means "all files"
- `exclude` removes files even if they matched `include`
- matching is done with gitignore-style glob semantics

## Inline rules vs rule files

Inline rules are useful for tiny setups:

```yaml
version: 1
rules:
  - id: no-dbg
    severity: warning
    message: "avoid dbg!"
    language: rust
    query: |
      (macro_invocation
        macro: (identifier) @name
        (#eq? @name "dbg")) @match
```

In practice, one-rule-per-file under `.lintropy/` is easier to review and maintain.

## Validation behavior

`lintropy config validate` does more than YAML parsing. It also checks:

- rule discovery
- duplicate ids
- supported languages
- Tree-sitter query compilation
- custom predicate names
- `{{capture}}` references in `message` and `fix`

Warnings can also be emitted for questionable but still-loadable rules, such as a query that omits `@match`.

## Suppression directives

Lintropy supports in-source suppression comments.

Line-scoped suppression:

```rust
// lintropy-ignore: no-unwrap
let value = client.unwrap();
```

File-scoped suppression within the first 20 lines:

```rust
// lintropy-ignore-file: no-unwrap
```

Current suppression rules:

- directives must be on their own line
- only `//`-style comments are recognized (Rust, Go, TypeScript); Python `#` comments are not yet supported
- trailing-code comments are ignored
- `*` wildcards are rejected

## Recommended layout

For most repos, this is the cleanest setup:

```text
lintropy.yaml
.lintropy/
  no-dbg.rule.yaml
  no-println.rule.yaml
  safety/
    unsafe-comment-required.rule.yaml
  architecture/
    domain-no-infra-imports.rule.yaml
```

That keeps the root config small and makes rules easy to review in pull requests.
