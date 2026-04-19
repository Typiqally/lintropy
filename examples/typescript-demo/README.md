# typescript-demo

TypeScript fixture for the lintropy CLI, covering both `.ts` and `.tsx`
(same rules compile against the `typescript` and `tsx` grammars). Run:

```console
cargo run -p lintropy -- check examples/typescript-demo --config examples/typescript-demo/lintropy.yaml
```

Rule layout:

- `.lintropy/no-console-log.rule.yaml` — scoped to `src/**/*.{ts,tsx}`; flags `console.log` calls.
- `.lintropy/no-any-type.rule.yaml` — scoped to `src/**/*.{ts,tsx}`; flags `any` type annotations.
- `.lintropy/no-todo-comment.rule.yaml` — unscoped; flags `TODO` comments that lack a `TODO(PROJ-123)` ticket reference.

Expected diagnostics:

| Rule | Location |
| --- | --- |
| `no-todo-comment` | `samples/comments/todo.ts:1` |
| `no-any-type` | `src/Card.tsx:5` |
| `no-console-log` | `src/Card.tsx:9` |
| `no-todo-comment` | `src/app.ts:1` |
| `no-console-log` | `src/app.ts:3` |
| `no-any-type` | `src/app.ts:6` |

Expected count: 6 warnings across 3 files.

Intentional allowed cases (must NOT fire):

- `samples/comments/todo.ts:6` — `TODO(PROJ-42)` has a ticket reference.
- `src/Card.tsx` is parsed with the `tsx` grammar; the JSX in `Card` must not produce parse errors or spurious diagnostics.
