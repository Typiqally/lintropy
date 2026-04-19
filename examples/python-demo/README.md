# python-demo

Python fixture for the lintropy CLI. Run:

```console
cargo run -p lintropy -- check examples/python-demo --config examples/python-demo/lintropy.yaml
```

Rule layout:

- `.lintropy/no-print.rule.yaml` — scoped to `src/**/*.py`; flags bare `print()` calls.
- `.lintropy/no-todo-comment.rule.yaml` — unscoped; flags `TODO` comments that lack a `TODO(PROJ-123)` ticket reference.

Expected diagnostics:

| Rule | Location |
| --- | --- |
| `no-todo-comment` | `samples/comments/todo.py:1` |
| `no-todo-comment` | `src/app.py:5` |
| `no-print` | `src/app.py:6` |
| `no-print` | `src/app.py:11` |

Expected count: 4 warnings across 2 files.

Intentional allowed cases (must NOT fire):

- `samples/comments/todo.py:6` — `TODO(PROJ-42)` has a ticket reference.
