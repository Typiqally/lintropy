//! Custom tree-sitter predicate parser **stub** for the WP1↔WP2 seam.
//!
//! WP2 owns the real implementation (§6 of the merged spec). Until it lands,
//! this module exposes the minimum surface WP1's config validator needs to
//! call: a [`CustomPredicate`] placeholder and [`parse_general_predicates`]
//! that accepts every predicate unconditionally.
//!
//! The public signatures here intentionally match WP2's contract so swapping
//! the implementation is a single-file change.

use tree_sitter::Query;

use crate::Result;

/// Placeholder variant. WP2 replaces this with the real `enum CustomPredicate`
/// carrying `HasAncestor`, `HasParent`, `HasSibling`, `HasPrecedingComment`,
/// and their negations.
#[derive(Debug, Clone)]
pub struct CustomPredicate {
    /// Predicate name without the leading `#` or trailing `?`.
    pub name: String,
}

/// Parse every general predicate in `query` into a [`CustomPredicate`] list.
///
/// **WP1 stub:** always returns an empty list. WP2 replaces this with the
/// real parser, which surfaces `LintropyError::UnknownPredicate` when a
/// query references a predicate lintropy does not implement. Until then,
/// unknown predicates slip through config validation and will only surface
/// at engine run time.
pub fn parse_general_predicates(_query: &Query) -> Result<Vec<CustomPredicate>> {
    Ok(Vec::new())
}
