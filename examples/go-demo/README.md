# go-demo

Go fixture for the lintropy CLI. Run:

```console
cargo run -p lintropy -- check examples/go-demo --config examples/go-demo/lintropy.yaml
```

Rule layout:

- `.lintropy/no-fmt-println.rule.yaml` — scoped to `src/**/*.go`; flags `fmt.Println` calls.
- `.lintropy/no-todo-comment.rule.yaml` — unscoped; flags `TODO` comments that lack a `TODO(PROJ-123)` ticket reference.

Expected diagnostics:

| Rule | Location |
| --- | --- |
| `no-todo-comment` | `samples/comments/todo.go:3` |
| `no-fmt-println` | `src/main.go:6` |
| `no-todo-comment` | `src/main.go:11` |
| `no-fmt-println` | `src/main.go:12` |

Expected count: 4 warnings across 2 files.

Intentional allowed cases (must NOT fire):

- `samples/comments/todo.go:9` — `TODO(PROJ-42)` has a ticket reference.
