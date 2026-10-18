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

    /// The kind a header name under FR-76 names.
    pub(crate) fn of(key: &str) -> Self {
        Self::KNOWN
            .into_iter()
            .find(|k| k.as_str() == key)
            .unwrap_or(Self::Other)
    }

    /// FR-108: the known column a header `key` under FR-76 is probably a
    /// misspelling of, among those not in `taken` — the columns the header
    /// already has, which it cannot have meant.
    ///
    /// A near miss is within the candidate's tolerance by Damerau–Levenshtein
    /// distance, so a transposition is one edit. Of several, the fewest edits
    /// wins, then the first in [`ColumnKind::KNOWN`]'s order (D-98). A known
    /// name is read, so it is no near miss.
    pub(crate) fn near_miss(key: &str, taken: &[Self]) -> Option<Self> {
        if Self::of(key) != Self::Other {
            return None;
        }
        Self::KNOWN
            .into_iter()
            .filter(|k| !taken.contains(k))
            .map(|k| (strsim::damerau_levenshtein(key, k.as_str()), k))
            .filter(|&(edits, k)| edits <= k.tolerance())
            // `min_by_key` keeps the first of equal minima: KNOWN's order.
            .min_by_key(|&(edits, _)| edits)
            .map(|(_, k)| k)
    }

    /// FR-108's guard: a short name is one edit from many ordinary words
    /// (`note` is two from `name`), so it allows one edit, a longer one two.
    const fn tolerance(self) -> usize {
        /// The longest name that allows only one edit: `name`, `ring`, `link`.
        const SHORT: usize = 4;
        if self.as_str().len() <= SHORT { 1 } else { 2 }
    }

    /// The canonical header name; empty for [`ColumnKind::Other`].
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Name => "name",
            Self::Quadrant => "quadrant",
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
        assert_eq!(near("descriptn"), Some(ColumnKind::Description));
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

    /// A known name is read, so it is not a near miss of anything; nor is
    /// a column far from every known name (D-95).
    #[test]
    fn known_and_unrelated_names_are_not_near_misses() {
        assert_eq!(near("rationale"), None);
        assert_eq!(near("ring"), None);
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
