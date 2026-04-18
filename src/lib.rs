//! Lintropy: structural linter driven by tree-sitter queries and repo-local YAML rules.
//!
//! Single-crate library surface. The `lintropy` binary wires these modules
//! into the CLI in `src/main.rs`; integration tests under `tests/` reach in
//! via this library root.

pub mod core;
pub mod langs;
pub mod output;

// CLI-internal modules, public only so the `lintropy` binary (src/main.rs)
// can reach them through the library crate. Not part of the stable API.
#[doc(hidden)]
pub mod agent_settings;
#[doc(hidden)]
pub mod cli;
#[doc(hidden)]
pub mod commands;
#[doc(hidden)]
pub mod editor_assets;
#[doc(hidden)]
pub mod exit;
#[doc(hidden)]
pub mod skill;
#[doc(hidden)]
pub mod walk;
