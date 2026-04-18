//! Editor assets embedded into the `lintropy` binary so that users
//! with only the shipped binary can install them without a repo
//! checkout. Each asset is a directory embedded via `include_dir!`
//! and unpacked on demand by the corresponding `install-*` subcommand.

use include_dir::{include_dir, Dir};

pub static TMBUNDLE_DIR: Dir<'_> =
    include_dir!("$CARGO_MANIFEST_DIR/editors/textmate/Lintropy Query.tmbundle");
pub const TMBUNDLE_DIR_NAME: &str = "Lintropy Query.tmbundle";

pub static LSP4IJ_TEMPLATE_DIR: Dir<'_> =
    include_dir!("$CARGO_MANIFEST_DIR/editors/jetbrains/lsp4ij-template");
pub const LSP4IJ_TEMPLATE_DIR_NAME: &str = "lsp4ij-template";
