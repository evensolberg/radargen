//! radargen — CSV to technology radar.
//!
//! See `docs/product/PRD.md`. Requirement IDs in comments refer to it.

pub mod cli;
pub mod rings;
pub mod text;

/// FR-76: **one normalisation rule**, applied to every value read from the CSV
/// or from `--rings`. Both operands of a comparison are trimmed of
/// leading/trailing whitespace and folded case-insensitively before comparison.
///
/// This never edits displayed text — callers compare on the result and render
/// the original.
pub fn normalise(value: &str) -> String {
    value.trim().to_lowercase()
}
