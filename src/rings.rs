//! The declared ring set (§6.1.4).
//!
//! Unlike `quadrant`, `ring` carries a lifecycle order that cannot be inferred
//! from the names, so it is declared rather than derived (D-4). The list is
//! **centre-to-edge** (FR-12).

use crate::normalise;

/// FR-13: the in-house lifecycle, centre-to-edge — so the radar reads
/// outside-in as Assess -> Trial -> Adopt -> Retire.
pub const DEFAULT_RINGS: [&str; 4] = ["Retire", "Adopt", "Trial", "Assess"];

/// FR-63 violations. Each is a hard error naming the offending entry; neither
/// can be resolved by guessing what the author meant.
#[derive(Debug, PartialEq, Eq)]
pub enum RingsError {
    /// An empty entry would draw a nameless band. `index` is 1-based.
    EmptyEntry { index: usize },
    /// A duplicate would make FR-16's match ambiguous and FR-32's band order
    /// undefined. `entry` is the offending text as written, trimmed.
    Duplicate { entry: String, first_index: usize },
    /// The list yielded no rings at all.
    Empty,
}

/// Parse `--rings`, or the FR-13 default when the flag is absent.
///
/// Entries are trimmed of leading/trailing whitespace only (FR-15), so a
/// multi-word ring name keeps its internal spaces. The declared casing is
/// preserved: it is what appears as a band label and legend subheading, and
/// FR-76 only ever trims a label, never re-cases one.
pub fn parse_rings(declared: Option<&str>) -> Result<Vec<String>, RingsError> {
    let Some(list) = declared else {
        return Ok(DEFAULT_RINGS.iter().map(|r| (*r).to_string()).collect());
    };

    let mut rings: Vec<String> = Vec::new();
    let mut seen: Vec<String> = Vec::new();
    for (i, raw) in list.split(',').enumerate() {
        let entry = raw.trim();
        if entry.is_empty() {
            // A lone empty string is "no rings at all" rather than a stray
            // comma, and the message should say so.
            if list.split(',').count() == 1 {
                return Err(RingsError::Empty);
            }
            return Err(RingsError::EmptyEntry { index: i + 1 });
        }
        let key = normalise(entry);
        if let Some(first) = seen.iter().position(|s| *s == key) {
            return Err(RingsError::Duplicate {
                entry: entry.to_string(),
                first_index: first + 1,
            });
        }
        seen.push(key);
        rings.push(entry.to_string());
    }
    if rings.is_empty() {
        return Err(RingsError::Empty);
    }
    Ok(rings)
}
