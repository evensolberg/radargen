//! The declared ring set (§6.1.4).
//!
//! Unlike `quadrant`, `ring` carries a lifecycle order that cannot be inferred
//! from the names, so it is declared rather than derived (D-4). The list is
//! **edge-to-centre** (FR-12, D-60): the first entry is the outermost band.

use crate::normalise;

/// FR-13: the in-house lifecycle, edge-to-centre, so it reads as written:
/// Assess -> Trial -> Adopt -> Retire.
pub const DEFAULT_RINGS: [&str; 4] = ["Assess", "Trial", "Adopt", "Retire"];

/// FR-63 violations. Each is a hard error naming the offending entry; none
/// can be resolved by guessing what the author meant.
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum RingsError {
    /// An empty entry would draw a nameless band. `index` is 1-based.
    #[error("entry {index} is empty")]
    EmptyEntry { index: usize },
    /// A duplicate would make FR-16's match ambiguous and FR-32's band order
    /// undefined. `entry` is the offending text as written, trimmed.
    /// `index` is the duplicate's own 1-based position, so a long list need
    /// not be searched for it.
    #[error("entry {index} '{entry}' repeats entry {first_index}")]
    Duplicate {
        index: usize,
        entry: String,
        first_index: usize,
    },
    /// The list yielded no rings at all.
    #[error("no rings declared")]
    Empty,
}

/// Parse `--rings`, or the FR-13 default when the flag is absent.
///
/// Entries are trimmed of leading/trailing whitespace only (FR-15), so a
/// multi-word ring name keeps its internal spaces. The declared casing is
/// preserved: it is what appears as a band label and legend subheading, and
/// FR-76 only ever trims a label, never re-cases one.
///
/// # Errors
///
/// Every FR-63 violation, in entry order (NFR-16 tier one: one run shows the
/// author everything to fix): each empty entry and each duplicate after
/// normalisation, the latter naming where the ring was first declared. A
/// list with no rings at all is the single [`RingsError::Empty`] rather
/// than one empty entry per comma.
pub fn parse_rings(declared: Option<&str>) -> Result<Vec<String>, Vec<RingsError>> {
    let Some(list) = declared else {
        return Ok(DEFAULT_RINGS.iter().map(|r| (*r).to_string()).collect());
    };

    let mut rings: Vec<String> = Vec::new();
    // Each ring first declared, normalised, with its 1-based entry, so a
    // duplicate names where the ring was first declared.
    let mut seen: Vec<(String, usize)> = Vec::new();
    let mut errors: Vec<RingsError> = Vec::new();
    for (i, raw) in list.split(',').enumerate() {
        let entry = raw.trim();
        if entry.is_empty() {
            errors.push(RingsError::EmptyEntry { index: i + 1 });
            continue;
        }
        let key = normalise(entry);
        if let Some((_, first)) = seen.iter().find(|(k, _)| *k == key) {
            errors.push(RingsError::Duplicate {
                index: i + 1,
                entry: entry.to_string(),
                first_index: *first,
            });
            continue;
        }
        seen.push((key, i + 1));
        rings.push(entry.to_string());
    }
    if rings.is_empty() {
        return Err(vec![RingsError::Empty]);
    }
    if errors.is_empty() {
        Ok(rings)
    } else {
        Err(errors)
    }
}
