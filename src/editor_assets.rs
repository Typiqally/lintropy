//! Editor assets embedded into the `lintropy` binary so that non-contributors
//! who only have the shipped binary can still install the query-highlight
//! extension (VS Code / Cursor) or TextMate bundle (JetBrains) without
//! cloning the repo.
//!
//! The `.vsix` is packed at build time by `build.rs` into `OUT_DIR`.
//! The TextMate bundle is embedded as a directory via `include_dir!`.

use include_dir::{include_dir, Dir};

pub const VSIX_BYTES: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/lintropy-query-syntax.vsix"));
pub const VSIX_FILE_NAME: &str = "lintropy-query-syntax.vsix";

pub static TMBUNDLE_DIR: Dir<'_> =
    include_dir!("$CARGO_MANIFEST_DIR/editors/textmate/Lintropy Query.tmbundle");
pub const TMBUNDLE_DIR_NAME: &str = "Lintropy Query.tmbundle";
