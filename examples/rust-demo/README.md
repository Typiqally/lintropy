# rust-demo

This example repo is the fixture for:

```console
cargo run -p lintropy -- check examples/rust-demo --config examples/rust-demo/lintropy.yaml
```

The current active fixture only loads the rules in `.lintropy/`. Translated
Rens-spec-inspired examples now also live in
`.lintropy/project-rules.rules.yaml` and the `samples/` tree. Those sample
`.rs` files are lint fixtures, not part of the Cargo crate build.

Added Rens-derived examples:

| Rule | Sample file | Purpose |
| --- | --- | --- |
| `domain-no-infra` | `samples/architecture/domain_violation.rs` | Architectural boundary: domain code importing infra |
| `use-tracing-not-log` | `samples/migrations/log_usage.rs` | Migration rule with autofix from `log::info!` to `tracing::info!` |
| `no-dbg` | `samples/banned/dbg_usage.rs` | Banned debugging macro |
| `todo-needs-ticket` | `samples/comments/todo_ticket.rs` | TODO/FIXME comment must include a ticket id |
| `safety-comment-required` | `samples/safety/missing_comment.rs` | `unsafe` blocks must have a preceding `// SAFETY:` comment |
| `safety-comment-required` non-match | `samples/safety/with_comment.rs` | Intentional allowed case with a matching `// SAFETY:` comment |
| `metric-naming` | `samples/taxonomy/metrics.rs` | Metric taxonomy enforcement |
| `no-stray-ignore` | `samples/tests/stray_ignore.rs` | `#[ignore]` outside `tests/flaky/` |
| `no-stray-ignore` non-match | `samples/tests/flaky/allowed_ignore.rs` | Intentional allowed case excluded by the rule |
| `test-name-prefix` | `samples/tests/tokio_naming.rs` | `#[tokio::test]` function names must start with `test_` |
| `old-config-removed-2026Q2` | `samples/deprecations/old_config.rs` | Dated deprecation rule with autofix |

Rule layout:

- `.lintropy/no-unwrap.rule.yaml`, `.lintropy/no-println.rule.yaml`, and `.lintropy/style.rules.yaml` keep the original focused demo rules for `src/` and `tests/`.
- `.lintropy/project-rules.rules.yaml` groups the added Rens-derived examples into one multi-rule file.
- `samples/` holds the extra Rust files that intentionally trigger those rules.

Expected diagnostics before `--fix`:

| Rule | Location |
| --- | --- |
| `domain-no-infra` | `samples/architecture/domain_violation.rs:1` |
| `no-dbg` | `samples/banned/dbg_usage.rs:2` |
| `todo-needs-ticket` | `samples/comments/todo_ticket.rs:2` |
| `safety-comment-required` | `samples/safety/missing_comment.rs:2` |
| `old-config-removed-2026Q2` | `samples/deprecations/old_config.rs:2` |
| `use-tracing-not-log` | `samples/migrations/log_usage.rs:2` |
| `metric-naming` | `samples/taxonomy/metrics.rs:2` |
| `no-stray-ignore` | `samples/tests/stray_ignore.rs:1` |
| `test-name-prefix` | `samples/tests/tokio_naming.rs:2` |
| `no-unwrap` | `src/main.rs:5` |
| `no-println` | `src/main.rs:10` |
| `user-use-builder` | `src/user.rs:42` |
| `no-todo` | `tests/smoke.rs:3` |

Expected count: 13 diagnostics across 12 files.

Autofix expectation:

- `cargo run -p lintropy -- check examples/rust-demo --config examples/rust-demo/lintropy.yaml --fix` rewrites the `no-unwrap` match in `src/main.rs` from `.unwrap()` to `.expect("TODO: handle error")`.
- The same run also rewrites `samples/migrations/log_usage.rs` from `log::info!` to `tracing::info!`, and rewrites `samples/deprecations/old_config.rs` from `OldConfig::load()` to `AppConfig::from_env()`.
- The `vec![macro_example.unwrap()]` line in `src/main.rs` is intentional and must not fire because `no-unwrap` excludes calls inside `macro_invocation`.
- `samples/safety/with_comment.rs` is intentional and must not fire because `safety-comment-required` recognizes the preceding `// SAFETY:` comment.
- `samples/tests/flaky/allowed_ignore.rs` is intentional and must not fire because `no-stray-ignore` excludes `samples/tests/flaky/**`.
- After the autofix, the example crate should still build with `cargo build --manifest-path examples/rust-demo/Cargo.toml`.

Notes:

- The `test-name-prefix` example needed one syntax adjustment from the original spec: in the current Rust grammar, the `attribute_item` and `function_item` are siblings under `source_file`, so the query matches them there instead of nesting the attribute under the function.
