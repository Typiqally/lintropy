---
title: Post-write hook
---

# Post-write hook

For agent harnesses that cannot drive an LSP client directly, lintropy runs after each write and blocks the agent on findings.

## `lintropy init --with-skill`

When `.claude/` or `.cursor/` exists, this command installs the bundled `SKILL.md` into the appropriate skill directory.

For Claude Code, it also updates `.claude/settings.json` to add a `PostToolUse` command hook:

```text
lintropy hook --agent claude-code
```

## `lintropy hook`

This command is designed for machine-to-machine use, not direct human use.

Behavior:

- reads JSON payloads from stdin
- extracts a written file path
- skips work if the file is gitignored
- lints only that file
- emits compact text or JSON diagnostics to stderr
- returns a blocking exit code when diagnostics meet the configured hook threshold

Current agent support:

- Claude Code hook payloads are the implemented target
- Codex is present as a CLI option, but auto-detection is still effectively Claude-first
