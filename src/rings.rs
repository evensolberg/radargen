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

/// The declared rings, edge-to-centre (FR-12), where they came from, and
/// the rings FR-107 keeps off the radar.
///
/// **Invariant:** no unplotted ring matches a declared one under FR-76, so
/// a ring is either drawn or kept off, never both. The fields are private,
/// so only [`parse_rings`], which names none unplotted, and
/// [`RingSet::with_unplotted`], which refuses any overlap, can build one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RingSet {
    /// As declared, after FR-15's trim: the band labels and legend
    /// subheadings.
    rings: Vec<String>,
    /// Whether the list came from `--rings` or the FR-13 default (D-61).
    source: RingSource,
    /// FR-107: `--unplotted-rings`, after FR-15's tidy.
    unplotted: Vec<String>,
    /// `rings` under FR-76, in the same order, for FR-16's match.
    ring_keys: Vec<String>,
    /// `unplotted` under FR-76.
    unplotted_keys: Vec<String>,
}

/// What a row's ring value is to a [`RingSet`] (FR-16, FR-17, FR-107).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RingMatch {
    /// A declared ring, by its edge-to-centre index: the row is plotted.
    Declared(usize),
    /// A ring `--unplotted-rings` names: off the radar, with no warning.
    Unplotted,
    /// Neither: off the radar, with FR-17's warning.
    Unknown,
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
    /// [`RingsError::Empty`] for `--unplotted-rings`, in the flag's own
    /// verb: it names rings rather than declaring them (FR-107).
    #[error("no rings named")]
    NoneNamed,
    /// FR-107: an `--unplotted-rings` entry the ring set declares, so it
    /// would be both drawn and left off. `by` is where the ring set came
    /// from, which says what to edit (D-61). Never raised for `--rings`.
    #[error("entry {index} '{entry}' {}", overlap(*.by))]
    Overlap {
        /// The entry's own 1-based position in `--unplotted-rings`.
        index: usize,
        /// The offending text as written, tidied.
        entry: String,
        /// `--rings`, or the default set.
        by: RingSource,
    },
}

/// What FR-107's overlap says after the entry: the default set has no list
/// to remove the ring from, so it says how to declare one without it.
const fn overlap(by: RingSource) -> &'static str {
    match by {
        RingSource::Flag => "is also in --rings, which draws it; remove it from one list",
        RingSource::Default => {
            "is a default ring, so it is drawn; to leave it off, pass --rings with the rings to draw, without it"
        }
    }
}

/// Parse `--rings`, or the FR-13 default when the flag is absent.
///
/// Entries get FR-76's trim at their ends (FR-15) and a run of whitespace
/// inside is one space ([`tidy`]), so a multi-word ring name keeps its
/// words. The declared casing is
/// preserved: it is what appears as a band label and legend subheading, and
/// FR-76 only ever tidies a label, never re-cases one.
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
        return Ok(RingSet::declared(
            DEFAULT_RINGS.iter().map(|r| (*r).to_string()).collect(),
            RingSource::Default,
        ));
    };
    Ok(RingSet::declared(
        parse_list(list, |_, _, _| None)?,
        RingSource::Flag,
    ))
}

/// FR-76's keys for `names`, in order: what every ring comparison reads.
fn keys(names: &[String]) -> Vec<String> {
    names.iter().map(|r| normalise(r)).collect()
}

impl RingSet {
    /// `rings` from `source`, with none unplotted.
    fn declared(rings: Vec<String>, source: RingSource) -> Self {
        Self {
            ring_keys: keys(&rings),
            rings,
            source,
            unplotted: Vec::new(),
            unplotted_keys: Vec::new(),
        }
    }

    /// The declared rings, edge-to-centre, as declared after FR-15's tidy.
    #[must_use]
    pub fn rings(&self) -> &[String] {
        &self.rings
    }

    /// Whether the rings came from `--rings` or the FR-13 default (D-61).
    #[must_use]
    pub const fn source(&self) -> RingSource {
        self.source
    }

    /// FR-107's unplotted rings, as named after FR-15's tidy.
    #[must_use]
    pub fn unplotted(&self) -> &[String] {
        &self.unplotted
    }

    /// Whether `--unplotted-rings` named any ring. An empty list is refused,
    /// so this is whether the flag was given.
    #[must_use]
    pub const fn names_unplotted(&self) -> bool {
        !self.unplotted.is_empty()
    }

    /// What `value`, a row's ring cell, is to this set under FR-76.
    #[must_use]
    pub fn classify(&self, value: &str) -> RingMatch {
        let key = normalise(value);
        match self.ring_keys.iter().position(|k| *k == key) {
            Some(i) => RingMatch::Declared(i),
            None if self.unplotted_keys.contains(&key) => RingMatch::Unplotted,
            None => RingMatch::Unknown,
        }
    }

    /// FR-107: this set with `list`, `--unplotted-rings`, as the rings left
    /// off the radar without a warning. The list is parsed as `--rings` is,
    /// FR-63's checks included.
    ///
    /// # Errors
    ///
    /// Every FR-63 violation, and every entry this set declares, the
    /// default set included, as [`RingsError::Overlap`], in entry order
    /// (NFR-16 tier one).
    pub fn with_unplotted(self, list: &str) -> Result<Self, Vec<RingsError>> {
        let unplotted = parse_list(list, |index, entry, key| {
            self.ring_keys
                .iter()
                .any(|d| d == key)
                .then(|| RingsError::Overlap {
                    index,
                    entry: entry.to_string(),
                    by: self.source,
                })
        })
        // `parse_list` returns `Empty` only alone.
        .map_err(|e| {
            if e == [RingsError::Empty] {
                vec![RingsError::NoneNamed]
            } else {
                e
            }
        })?;
        Ok(Self {
            unplotted_keys: keys(&unplotted),
            unplotted,
            ..self
        })
    }
}

/// A comma list under FR-15 and FR-63, with `check` asked about each entry
/// that is neither empty nor a duplicate: its 1-based position, its tidied
/// text and its FR-76 key.
fn parse_list(
    list: &str,
    check: impl Fn(usize, &str, &str) -> Option<RingsError>,
) -> Result<Vec<String>, Vec<RingsError>> {
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
                entry,
                first_index: *first,
            });
            continue;
        }
        errors.extend(check(i + 1, &entry, &key));
        seen.push((key, i + 1));
        rings.push(entry);
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
