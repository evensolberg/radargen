//! The declared ring set (§6.1.4).
//!
//! Unlike `quadrant`, `ring` carries a lifecycle order that cannot be inferred
//! from the names, so it is declared rather than derived (D-4). The list is
//! **edge-to-centre** (FR-12, D-60): the first entry is the outermost band.

use std::fmt;

use crate::{normalise, tidy};

/// FR-13: the in-house lifecycle, edge-to-centre, so it reads as written:
/// Assess -> Trial -> Adopt -> Retire.
pub const DEFAULT_RINGS: [&str; 4] = ["Assess", "Trial", "Adopt", "Retire"];

/// Where the ring set came from. FR-17's warning names it, so an author
/// told a ring is undeclared knows what to edit (D-61).
///
/// `--config`'s `rings` key (FR-91) adds a variant when PR 19 adds the
/// file; that variant names the file as given on the command line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RingSource {
    /// `--rings` on the command line.
    Flag,
    /// FR-13's default, when nothing declared a ring set.
    Default,
}

/// How FR-17's warning names it: `not declared by {source}`.
// qual:allow(dry, boilerplate) reason: "deriving Display needs a crate (strum, derive_more) for one two-variant enum"
impl fmt::Display for RingSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Flag => "--rings",
            Self::Default => "the default ring set",
        })
    }
}

/// The declared rings, edge-to-centre (FR-12), and where they came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RingSet {
    /// As declared, after FR-15's trim: the band labels and legend
    /// subheadings.
    pub rings: Vec<String>,
    /// Whether the list came from `--rings` or the FR-13 default (D-61).
    pub source: RingSource,
}

/// FR-63 violations. Each is a hard error naming the offending entry; none
/// can be resolved by guessing what the author meant.
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum RingsError {
    /// An empty entry would draw a nameless band. `index` is 1-based.
    #[error("entry {index} is empty")]
    EmptyEntry {
        /// The empty entry's 1-based position.
        index: usize,
    },
    /// A duplicate would make FR-16's match ambiguous and FR-32's band order
    /// undefined. `entry` is the offending text as written, trimmed.
    /// `index` is the duplicate's own 1-based position, so a long list need
    /// not be searched for it.
    #[error("entry {index} '{entry}' repeats entry {first_index}")]
    Duplicate {
        /// The duplicate's own 1-based position.
        index: usize,
        /// The offending text as written, trimmed.
        entry: String,
        /// The 1-based position of the earlier entry it repeats.
        first_index: usize,
    },
    /// The list yielded no rings at all.
    #[error("no rings declared")]
    Empty,
}

/// Parse `--rings`, or the FR-13 default when the flag is absent.
///
/// Entries get FR-76's trim at their ends (FR-15) and a run of whitespace
/// inside is one space ([`tidy`]), so a multi-word ring name keeps its
/// words. The declared casing is
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
pub fn parse_rings(declared: Option<&str>) -> Result<RingSet, Vec<RingsError>> {
    let Some(list) = declared else {
        return Ok(RingSet {
            rings: DEFAULT_RINGS.iter().map(|r| (*r).to_string()).collect(),
            source: RingSource::Default,
        });
    };

    let mut rings: Vec<String> = Vec::new();
    // Each ring first declared, normalised, with its 1-based entry, so a
    // duplicate names where the ring was first declared.
    let mut seen: Vec<(String, usize)> = Vec::new();
    let mut errors: Vec<RingsError> = Vec::new();
    for (i, raw) in list.split(',').enumerate() {
        let entry = tidy(raw);
        if entry.is_empty() {
            errors.push(RingsError::EmptyEntry { index: i + 1 });
            continue;
        }
        let key = normalise(&entry);
        if let Some((_, first)) = seen.iter().find(|(k, _)| *k == key) {
            errors.push(RingsError::Duplicate {
                index: i + 1,
                entry: entry.clone(),
                first_index: *first,
            });
            continue;
        }
        seen.push((key, i + 1));
        rings.push(entry);
    }
    if rings.is_empty() {
        return Err(vec![RingsError::Empty]);
    }
    if errors.is_empty() {
        Ok(RingSet {
            rings,
            source: RingSource::Flag,
        })
    } else {
        Err(errors)
    }
}
