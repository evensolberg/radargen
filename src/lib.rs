//! radargen — CSV to technology radar.
//!
//! See `docs/product/radargen-prd.md`. Requirement IDs in comments refer to it.

pub mod cli;
pub mod rings;
pub mod text;

/// FR-76: **one normalisation rule**, applied to every value read from the CSV
/// or from `--rings`. Both operands of a comparison are trimmed of
/// leading/trailing whitespace and folded case-insensitively before comparison.
/// Folding is Unicode's default lowercase mapping — `str::to_lowercase`, not
/// full case folding and not locale-aware (D-53).
///
/// This never edits displayed text — callers compare on the result and render
/// the original.
#[must_use]
pub fn normalise(value: &str) -> String {
    value.trim().to_lowercase()
}
