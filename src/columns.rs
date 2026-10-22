//! The columns radargen knows by name (FR-5, FR-27) and the near misses of
//! them (FR-108), a leaf both the model and the diagnostics use.

/// What a named column means. A header name is matched under FR-76, so
/// `Name ` is `name`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ColumnKind {
    /// The item's name (FR-5); required and never empty (FR-6).
    Name,
    /// The quadrant the item belongs to (FR-5, FR-8).
    Quadrant,
    /// The ring the item sits in (FR-5, FR-16).
    Ring,
    /// Free text about the item (FR-5); may be empty (FR-7).
    Description,
    /// Whether the item is a default or conditional choice (FR-20).
    Qualifier,
    /// Free text for the detail panel, shown verbatim (FR-22).
    Domain,
    /// Free text for the detail panel, shown verbatim (FR-23).
    Rationale,
    /// An ISO-8601 date for the detail panel (FR-24).
    LastAffirmed,
    /// A date for the detail panel, whatever the ring (FR-25).
    ExitDate,
    /// Author-supplied movement since the last edition (FR-26, D-9).
    Status,
    /// A web address for the item (FR-96).
    Link,
    /// FR-27: a column the tool does not know. Only `--table all`'s table
    /// shows it (FR-78, FR-106).
    Other,
}

/// D-98's guess, shared by FR-108's columns and FR-91's keys: the first
/// of `candidates` that `key`, already normalised by the caller, is
/// probably a misspelling of.
///
/// A near miss is within the candidate's tolerance by Damerau–Levenshtein
/// distance, so a transposition is one edit. The tolerance is one edit,
/// unless the name is longer than four characters and `key` longer than
/// the name, which allows two: a short name is two edits from many
/// ordinary words (`note`, `name`), and so is a key no longer than the
/// name, by changing letters rather than adding them (`state`, `status`);
/// a doubled or stray letter makes a key longer. Of several, the fewest
/// edits wins, then the first in `candidates`' order. A candidate more
/// than two characters longer or shorter is skipped before any distance is
/// computed, since it cannot be within two edits, so a huge key costs
/// nothing (efficiency review 1).
pub(crate) fn nearest<T: Copy>(
    key: &str,
    candidates: impl IntoIterator<Item = T>,
    name: impl Fn(T) -> &'static str,
) -> Option<T> {
    /// The longest name that allows only one edit: `name`, `ring`, `link`.
    const SHORT: usize = 4;
    /// The most edits any candidate allows.
    const MOST: usize = 2;
    let length = key.chars().count();
    candidates
        .into_iter()
        // Names are ASCII, so bytes are characters.
        .filter(|&c| name(c).len().abs_diff(length) <= MOST)
        .map(|c| (strsim::damerau_levenshtein(key, name(c)), c))
        .filter(|&(edits, c)| {
            let size = name(c).len();
            edits
                <= if size > SHORT && length > size {
                    MOST
                } else {
                    1
                }
        })
        // `min_by_key` keeps the first of equal minima: the given order.
        .min_by_key(|&(edits, _)| edits)
        .map(|(_, c)| c)
}

/// D-101: the sector column's former name, which BYOR's CSVs use. It is
/// read as `sector`, matched under FR-76 as every header name is.
pub(crate) const SECTOR_ALIAS: &str = "quadrant";

impl ColumnKind {
    /// Every known kind, so the header names are written once, in
    /// [`ColumnKind::as_str`].
    pub(crate) const KNOWN: [Self; 11] = [
        Self::Name,
        Self::Quadrant,
        Self::Ring,
        Self::Description,
        Self::Qualifier,
        Self::Domain,
        Self::Rationale,
        Self::LastAffirmed,
        Self::ExitDate,
        Self::Status,
        Self::Link,
    ];

    /// The kind a header name under FR-76 names; the sector column's
    /// former name, [`SECTOR_ALIAS`], names it too (D-101).
    pub(crate) fn of(key: &str) -> Self {
        if key == SECTOR_ALIAS {
            return Self::Quadrant;
        }
        Self::KNOWN
            .into_iter()
            .find(|k| k.as_str() == key)
            .unwrap_or(Self::Other)
    }

    /// The name FR-4 compares a header `key` under FR-76 by: the canonical
    /// name for [`SECTOR_ALIAS`], so `quadrant` repeats `sector` (D-101);
    /// `key` itself otherwise.
    pub(crate) fn canonical(key: &str) -> &str {
        if key == SECTOR_ALIAS {
            Self::Quadrant.as_str()
        } else {
            key
        }
    }

    /// FR-108: the known column a header `key` under FR-76 is probably a
    /// misspelling of, among those not in `taken` — the columns the header
    /// already has, which it cannot have meant.
    ///
    /// `key` is a header [`ColumnKind::of`] does not know; the caller sees
    /// to that. A near miss is within the candidate's tolerance by
    /// Damerau–Levenshtein distance, so a transposition is one edit. Of
    /// several, the fewest edits wins, then the first in
    /// [`ColumnKind::KNOWN`]'s order (D-98). [`SECTOR_ALIAS`] is a
    /// candidate last, standing for the sector column, so a misspelt
    /// `quadrant` suggests the name to write (D-101).
    pub(crate) fn near_miss(key: &str, taken: &[Self]) -> Option<Self> {
        let names = Self::KNOWN
            .into_iter()
            .map(|k| (k.as_str(), k))
            .chain([(SECTOR_ALIAS, Self::Quadrant)]);
        nearest(
            key,
            names.filter(|(_, k)| !taken.contains(k)),
            |(name, _)| name,
        )
        .map(|(_, k)| k)
    }

    /// The canonical header name; empty for [`ColumnKind::Other`].
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Name => "name",
            Self::Quadrant => "sector",
            Self::Ring => "ring",
            Self::Description => "description",
            Self::Qualifier => "qualifier",
            Self::Domain => "domain",
            Self::Rationale => "rationale",
            Self::LastAffirmed => "last_affirmed",
            Self::ExitDate => "exit_date",
            Self::Status => "status",
            Self::Link => "link",
            Self::Other => "",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ColumnKind;

    fn near(key: &str) -> Option<ColumnKind> {
        ColumnKind::near_miss(key, &[])
    }

    /// FR-108: one or two edits from a long name is a near miss.
    #[test]
    fn one_or_two_edits_from_a_long_name_is_a_near_miss() {
        assert_eq!(near("rationalle"), Some(ColumnKind::Rationale));
        assert_eq!(near("descripton"), Some(ColumnKind::Description));
        assert_eq!(near("last affirmed"), Some(ColumnKind::LastAffirmed));
        assert_eq!(near("rationaleee"), Some(ColumnKind::Rationale));
        assert_eq!(near("rationaleeee"), None, "three edits");
    }

    /// FR-108: a transposition is one edit, so `nmae` is within a short
    /// name's single edit, where plain Levenshtein would count two.
    #[test]
    fn a_transposition_is_one_edit() {
        assert_eq!(near("nmae"), Some(ColumnKind::Name));
        assert_eq!(near("qaudrant"), Some(ColumnKind::Quadrant));
        assert_eq!(near("sectro"), Some(ColumnKind::Quadrant));
    }

    /// D-101: `quadrant` is the sector column, by its former name.
    #[test]
    fn quadrant_names_the_sector_column() {
        assert_eq!(ColumnKind::of("sector"), ColumnKind::Quadrant);
        assert_eq!(ColumnKind::of("quadrant"), ColumnKind::Quadrant);
        assert_eq!(ColumnKind::canonical("quadrant"), "sector");
        assert_eq!(ColumnKind::canonical("sector"), "sector");
        assert_eq!(ColumnKind::canonical("ring"), "ring");
    }

    /// FR-108, D-101: a misspelt `quadrant` suggests `sector`, the name to
    /// write; with the column already present, nothing.
    #[test]
    fn a_misspelt_alias_suggests_sector() {
        assert_eq!(near("quadrnat"), Some(ColumnKind::Quadrant));
        assert_eq!(near("quadrants"), Some(ColumnKind::Quadrant));
        assert_eq!(
            ColumnKind::near_miss("quadrnat", &[ColumnKind::Quadrant]),
            None
        );
    }

    /// FR-108's guard: a name of four characters or fewer allows one edit,
    /// so `note` is not `name`.
    #[test]
    fn a_short_name_allows_one_edit() {
        assert_eq!(near("nme"), Some(ColumnKind::Name));
        assert_eq!(near("lnik"), Some(ColumnKind::Link));
        assert_eq!(near("note"), None);
        assert_eq!(near("nm"), None);
        assert_eq!(near("rate"), None);
    }

    /// FR-108's length guard: a header shorter than the name, or as long,
    /// allows one edit, so a register's `state` is not `status`; only a
    /// longer header, with letters to spare, allows two.
    #[test]
    fn a_header_no_longer_than_the_name_allows_one_edit() {
        for key in ["state", "main", "remain", "descriptn"] {
            assert_eq!(near(key), None, "{key}");
        }
        // One dropped letter is within any tolerance, so `stats` is still a
        // hint for `status`, accepted (D-98).
        assert_eq!(near("stats"), Some(ColumnKind::Status));
        for (key, known) in [
            ("descripton", ColumnKind::Description),
            ("rationalle", ColumnKind::Rationale),
            ("lastaffirmed", ColumnKind::LastAffirmed),
            ("exit date", ColumnKind::ExitDate),
            ("rings", ColumnKind::Ring),
            ("statuses", ColumnKind::Status),
        ] {
            assert_eq!(near(key), Some(known), "{key}");
        }
    }

    /// A column far from every known name is no near miss (D-95). An exact
    /// name never gets here: the caller passes only unknown headers.
    #[test]
    fn unrelated_names_are_not_near_misses() {
        assert_eq!(near("owner"), None);
        assert_eq!(near("notes"), None);
    }

    /// Two candidates: the fewer edits wins, then the first in
    /// [`ColumnKind::KNOWN`]'s order; a column the header already has is
    /// no candidate.
    #[test]
    fn ties_go_to_the_first_known_name_the_header_lacks() {
        assert_eq!(near("rink"), Some(ColumnKind::Ring), "ring before link");
        assert_eq!(
            ColumnKind::near_miss("rink", &[ColumnKind::Ring]),
            Some(ColumnKind::Link)
        );
        assert_eq!(
            ColumnKind::near_miss("rationalle", &[ColumnKind::Rationale]),
            None
        );
    }
}
