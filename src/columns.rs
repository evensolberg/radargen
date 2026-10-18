//! The columns radargen knows by name (FR-5, FR-27), a leaf both the model
//! and the diagnostics use.

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
