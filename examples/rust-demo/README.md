# rust-demo

This example repo is the fixture for:

```console
cargo run -p lintropy-cli -- check examples/rust-demo
```

Expected diagnostics before `--fix`:

| Rule | Location |
| --- | --- |
| `no-unwrap` | `src/main.rs:5` |
| `no-println` | `src/main.rs:10` |
| `user-use-builder` | `src/user.rs:42` |
| `no-todo` | `tests/smoke.rs:3` |

Expected count: 4 diagnostics across 3 files.

Autofix expectation:

- `cargo run -p lintropy-cli -- check examples/rust-demo --fix` rewrites the
  `no-unwrap` match in `src/main.rs` from `.unwrap()` to
  `.expect("TODO: handle error")`.
- The `vec![macro_example.unwrap()]` line in `src/main.rs` is intentional and
  must not fire because `no-unwrap` excludes calls inside `macro_invocation`.
- After the autofix, the example crate should still build with
  `cargo build --manifest-path examples/rust-demo/Cargo.toml`.
