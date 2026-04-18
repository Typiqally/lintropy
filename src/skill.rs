//! Embedded canonical `SKILL.md` + version tag.
//!
//! Consumed by `init --with-skill` to materialise the skill into agent
//! skill directories (`.claude/skills/lintropy/`, `.cursor/skills/lintropy/`).
//!
//! `SKILL_VERSION` must match the `# version: <semver>` header on the
//! first line of `SKILL.md` — `init --with-skill` uses it to decide
//! whether to upgrade an existing file in place.

pub const EMBEDDED_SKILL: &str = include_str!("../skill/SKILL.md");
pub const SKILL_VERSION: &str = "0.2.0";
