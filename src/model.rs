//! Records to meaning (architecture §2): which column is which, what each
//! row claims, and what is wrong with it (FR-5–FR-29, FR-93).
//!
//! Ordering and numbering are not here: FR-10 and FR-34 are `order.rs`'s
//! (PR 07), so a `Catalogue` keeps quadrants and items in CSV order.

use crate::csv::parse::{Record, Table};
use crate::diagnostics::{DateColumn, Fatal, RowKind, Warning, Warnings};
use crate::rings::{RingSet, RingSource};
use crate::{normalise, trim};

/// FR-5's required columns, in the order its error lists them.
const REQUIRED: [ColumnKind; 4] = [
    ColumnKind::Name,
    ColumnKind::Quadrant,
    ColumnKind::Ring,
    ColumnKind::Description,
];

/// FR-6's columns: required, and never empty. `description` may be empty
/// (FR-7).
const NON_EMPTY: [ColumnKind; 3] = [ColumnKind::Name, ColumnKind::Quadrant, ColumnKind::Ring];

/// What a named column means. A header name is matched under FR-76, so
/// `Name ` is `name`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColumnKind {
    Name,
    Quadrant,
    Ring,
    Description,
    Qualifier,
    Domain,
    Rationale,
    LastAffirmed,
    ExitDate,
    Status,
    /// FR-27: a column the tool does not know. Kept for the table (FR-78).
    Other,
}

impl ColumnKind {
    /// Every known kind, so the header names are written once, in
    /// [`ColumnKind::as_str`].
    const KNOWN: [Self; 10] = [
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
    ];

    /// The kind a header name under FR-76 names.
    fn of(key: &str) -> Self {
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
            Self::Other => "",
        }
    }
}

/// One named column, in header order (FR-27, FR-78).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Column {
    /// The header cell as written (FR-76 edits no displayed text). An
    /// `Other` column (FR-27) is headed with this; a renderer may give a
    /// known kind its own heading.
    pub heading: String,
    pub kind: ColumnKind,
}

/// FR-20's recognised values. Blank and unrecognised values claim nothing,
/// so they have no variant: the marker is solid and the legend row shows
/// no qualifier (FR-74).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Qualifier {
    Default,
    Conditional,
}

impl Qualifier {
    /// What a legend row shows (FR-74), so an accessible name can contain
    /// it (WCAG 2.5.3).
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Default => "Default",
            Self::Conditional => "Conditional",
        }
    }
}

/// FR-26's values. An unrecognised one is `NoChange`, with a warning.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    New,
    MovedIn,
    MovedOut,
    NoChange,
}

impl Status {
    /// Every value, so the vocabulary is written once, in [`Status::label`].
    const ALL: [Self; 4] = [Self::New, Self::MovedIn, Self::MovedOut, Self::NoChange];

    /// The canonical spelling: what a legend row and the detail panel show
    /// (FR-26, FR-74), and what a cell matches under FR-76.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::New => "New",
            Self::MovedIn => "Moved In",
            Self::MovedOut => "Moved Out",
            Self::NoChange => "No Change",
        }
    }
}

/// One quadrant (FR-8, FR-9).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Quadrant {
    /// FR-76's normalised value, which groups the rows.
    pub key: String,
    /// FR-9: the ordinally first variant, trimmed.
    pub label: String,
}

/// One CSV data row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    /// The physical line the record begins on (NFR-10). Lines ascend with
    /// CSV order, so this is also FR-34's last tiebreak.
    pub line: usize,
    /// As written, after FR-90's removal.
    pub name: String,
    /// FR-76's key, computed once so no comparator folds (architecture §6).
    pub name_key: String,
    /// Index into [`Catalogue::quadrants`], which is in first-appearance
    /// order — **not** FR-10's, and never a palette index (architecture
    /// §5). `order.rs` (PR 07) gives the FR-10 rank.
    pub quadrant: usize,
    /// Index into [`Catalogue::rings`]; `None` when the ring is not
    /// declared (FR-17), which leaves the row unplotted.
    pub ring: Option<usize>,
    /// FR-20; `None` when blank or unrecognised.
    pub qualifier: Option<Qualifier>,
    /// FR-26: `None` when blank or absent; an unrecognised value is
    /// `Some(Status::NoChange)`, with a warning.
    pub status: Option<Status>,
    /// Every named column's cell, aligned with [`Catalogue::columns`] (not
    /// with the header, which may hold unnamed columns), as written after
    /// FR-90's removal (FR-78).
    pub cells: Vec<String>,
}

impl Item {
    /// FR-20: drawn hollow.
    #[must_use]
    pub fn hollow(&self) -> bool {
        self.qualifier == Some(Qualifier::Conditional)
    }

    /// The cell under `column`, an index into [`Catalogue::columns`], or
    /// `None` when it is empty under FR-76's trim, which FR-29 omits rather
    /// than showing an empty label.
    #[must_use]
    pub fn cell(&self, column: usize) -> Option<&str> {
        self.cells
            .get(column)
            .map(String::as_str)
            .filter(|c| !trim(c).is_empty())
    }
}

/// The rows and what they mean, in CSV order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Catalogue {
    /// The named columns in header order. An unnamed column is not here:
    /// FR-93 drops it.
    pub columns: Vec<Column>,
    /// In order of first appearance; FR-10 orders them (PR 07).
    pub quadrants: Vec<Quadrant>,
    /// The declared rings, edge-to-centre (FR-12). [`Item::ring`] indexes
    /// this copy; nothing below the model reads the `RingSet`.
    pub rings: Vec<String>,
    pub items: Vec<Item>,
}

impl Catalogue {
    /// The index in [`Catalogue::columns`] of the first column of `kind`.
    #[must_use]
    pub fn column(&self, kind: ColumnKind) -> Option<usize> {
        self.columns.iter().position(|c| c.kind == kind)
    }

    /// FR-50, FR-75: an accessible name for a marker, a legend row and —
    /// until `rg-6ny` settles how table rows are exposed — a table row.
    /// Only a table row can be unplotted (FR-18).
    ///
    /// It starts with what the legend row shows — number, name, qualifier,
    /// status (FR-74) — so the visible text is in the name (WCAG 2.5.3).
    /// The quadrant and ring follow even where the legend's grouping shows
    /// them, since a screen reader reaching one row by Tab sees no grouping,
    /// and are named as such, since their names are the author's own and a
    /// name may hold a comma.
    /// An unplotted row has no number (FR-18) and says it is not on the
    /// radar. The result is plain text; the emitter escapes it (NFR-6).
    ///
    /// `number` is FR-34's, for a plotted item, and `None` for an unplotted
    /// one.
    ///
    /// # Panics
    ///
    /// If `item.quadrant` or `item.ring` is out of range for this
    /// catalogue, as for an item from another one. In debug builds, also if
    /// `number` is `Some` for an unplotted item or `None` for a plotted
    /// one (FR-18).
    #[must_use]
    pub fn accessible_name(&self, item: &Item, number: Option<usize>) -> String {
        debug_assert_eq!(number.is_some(), item.ring.is_some(), "FR-18");
        let ring = item.ring.map_or_else(
            || {
                self.column(ColumnKind::Ring)
                    .and_then(|c| item.cell(c))
                    .map_or("", trim)
            },
            |r| self.rings[r].as_str(),
        );
        let row = [
            Some(trim(&item.name)),
            item.qualifier.map(Qualifier::label),
            item.status.map(Status::label),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(", ");
        let quadrant = &self.quadrants[item.quadrant].label;
        let position = if ring.is_empty() {
            format!("{quadrant} quadrant")
        } else {
            format!("{quadrant} quadrant, {ring} ring")
        };
        number.map_or_else(
            || format!("{row}; {position}; not on the radar"),
            |n| format!("{n}. {row}; {position}"),
        )
    }
}

/// FR-5–FR-29 and FR-93: the catalogue, or every row-level error.
///
/// NFR-16 tier two: every missing required column (FR-5) and every empty
/// `name`, `quadrant` or `ring` (FR-6) is returned together, in line order
/// then column order. Rows are checked before any is built, so an `Err`
/// comes with no warning from this stage. `csv::parse`'s FR-4 errors and
/// FR-90 warnings are separate: `main.rs` merges both stages' errors in
/// NFR-11's order and prints them after the warnings (PR 14).
///
/// # Errors
///
/// [`Fatal::MissingColumn`] and [`Fatal::EmptyRequired`].
pub fn build_catalogue(
    table: &Table,
    rings: &RingSet,
    w: &mut Warnings,
) -> Result<Catalogue, Vec<Fatal>> {
    let header = Header::of(&table.header);
    none_or_all(
        [
            header.missing(table.header_line),
            empty_required(&header, &table.records),
        ]
        .into_iter()
        .flatten()
        .collect(),
    )?;
    Ok(Builder::new(header, rings).build(&table.records, w))
}

/// `Ok` when there are no errors, else every one of them.
fn none_or_all(errors: Vec<Fatal>) -> Result<(), Vec<Fatal>> {
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// The cell at header position `p`. `parse` keeps only records as long as
/// the header, so a missing cell reads as empty rather than panicking on a
/// hand-built `Table`.
fn field(record: &Record, p: usize) -> &str {
    record.fields.get(p).map_or("", String::as_str)
}

/// The header, read once: which position holds which column.
struct Header {
    /// Per header position, the index of its named column in `columns`, or
    /// `None` for an unnamed one (FR-93).
    slots: Vec<Option<usize>>,
    columns: Vec<Column>,
    /// The header positions of the named columns, in order: where each
    /// `Item::cells` entry is read from.
    cell_positions: Vec<usize>,
    /// FR-93's unnamed columns.
    unnamed: Vec<usize>,
}

impl Header {
    fn of(names: &[String]) -> Self {
        let mut slots = Vec::with_capacity(names.len());
        let mut columns = Vec::new();
        let mut cell_positions = Vec::new();
        let mut unnamed = Vec::new();
        for (p, name) in names.iter().enumerate() {
            let key = normalise(name);
            if key.is_empty() {
                slots.push(None);
                unnamed.push(p);
                continue;
            }
            slots.push(Some(columns.len()));
            cell_positions.push(p);
            columns.push(Column {
                heading: name.clone(),
                kind: ColumnKind::of(&key),
            });
        }
        Self {
            slots,
            columns,
            cell_positions,
            unnamed,
        }
    }

    /// The header position of the first column of `kind`. A repeat is
    /// already FR-4's error, so the first is the one that counts.
    fn position(&self, kind: ColumnKind) -> Option<usize> {
        self.slots
            .iter()
            .position(|s| s.is_some_and(|c| self.columns[c].kind == kind))
    }

    /// FR-5: every required column the header lacks.
    fn missing(&self, line: usize) -> Vec<Fatal> {
        REQUIRED
            .into_iter()
            .filter(|&kind| self.position(kind).is_none())
            .map(|kind| Fatal::MissingColumn {
                line,
                column: kind.as_str(),
            })
            .collect()
    }
}

/// FR-6: every empty `name`, `quadrant` or `ring` the header has, in line
/// order, then header column order (NFR-11).
fn empty_required(header: &Header, records: &[Record]) -> Vec<Fatal> {
    let mut present: Vec<(usize, ColumnKind)> = NON_EMPTY
        .into_iter()
        .filter_map(|kind| header.position(kind).map(|p| (p, kind)))
        .collect();
    present.sort_by_key(|&(p, _)| p);
    records
        .iter()
        .flat_map(|record| {
            present
                .iter()
                .filter(|&&(p, _)| trim(field(record, p)).is_empty())
                .map(|&(_, kind)| Fatal::EmptyRequired {
                    line: record.line,
                    column: kind.as_str(),
                })
        })
        .collect()
}

/// Where each column the rows are read from sits in the header.
struct Positions {
    name: usize,
    quadrant: usize,
    ring: usize,
    description: usize,
    qualifier: Option<usize>,
    status: Option<usize>,
    last_affirmed: Option<usize>,
    exit_date: Option<usize>,
}

impl Positions {
    fn of(header: &Header) -> Self {
        // FR-5 has passed before a `Positions` is built, so the default is
        // never taken.
        let required = |kind| header.position(kind).unwrap_or_default();
        Self {
            name: required(ColumnKind::Name),
            quadrant: required(ColumnKind::Quadrant),
            ring: required(ColumnKind::Ring),
            description: required(ColumnKind::Description),
            qualifier: header.position(ColumnKind::Qualifier),
            status: header.position(ColumnKind::Status),
            last_affirmed: header.position(ColumnKind::LastAffirmed),
            exit_date: header.position(ColumnKind::ExitDate),
        }
    }
}

/// Builds the catalogue a row at a time.
struct Builder {
    header: Header,
    at: Positions,
    /// The declared rings under FR-76, for FR-16's match.
    ring_keys: Vec<String>,
    source: RingSource,
    rings: Vec<String>,
    quadrants: Vec<Quadrant>,
    /// Each name's key and the line it first appears on, for FR-28.
    names: Vec<(String, usize)>,
    items: Vec<Item>,
}

impl Builder {
    fn new(header: Header, rings: &RingSet) -> Self {
        Self {
            at: Positions::of(&header),
            header,
            ring_keys: rings.rings.iter().map(|r| normalise(r)).collect(),
            source: rings.source,
            rings: rings.rings.clone(),
            quadrants: Vec::new(),
            names: Vec::new(),
            items: Vec::new(),
        }
    }

    fn build(mut self, records: &[Record], w: &mut Warnings) -> Catalogue {
        for record in records {
            self.add(record, w);
        }
        Catalogue {
            columns: self.header.columns,
            quadrants: self.quadrants,
            rings: self.rings,
            items: self.items,
        }
    }

    fn add(&mut self, record: &Record, w: &mut Warnings) {
        let at = |p: usize| field(record, p);
        let mut warn = |column: usize, kind: RowKind| {
            w.push(Warning::Row {
                line: record.line,
                column,
                kind,
            });
        };
        let name = at(self.at.name);
        let name_key = self.name(name, record.line, &mut warn);
        let quadrant = self.quadrant(at(self.at.quadrant));
        let ring = self.ring(at(self.at.ring), &mut warn);
        if trim(at(self.at.description)).is_empty() {
            warn(self.at.description, RowKind::EmptyDescription);
        }
        let qualifier = self
            .at
            .qualifier
            .and_then(|p| read_qualifier(at(p), p, &mut warn));
        let status = self
            .at
            .status
            .and_then(|p| read_status(at(p), p, &mut warn));
        for (column, position) in [
            (DateColumn::LastAffirmed, self.at.last_affirmed),
            (DateColumn::ExitDate, self.at.exit_date),
        ] {
            if let Some(p) = position {
                check_date(at(p), p, column, &mut warn);
            }
        }
        for &p in &self.header.unnamed {
            check_unnamed(at(p), p, &mut warn);
        }
        // Built from the header, not zipped with the record, so a short
        // hand-built record still gives one cell per named column.
        let cells = self
            .header
            .cell_positions
            .iter()
            .map(|&p| at(p).to_string())
            .collect();
        self.items.push(Item {
            line: record.line,
            name: name.to_string(),
            name_key,
            quadrant,
            ring,
            qualifier,
            status,
            cells,
        });
    }

    /// FR-28: every later occurrence of a name warns, naming the first.
    fn name(&mut self, name: &str, line: usize, warn: &mut impl FnMut(usize, RowKind)) -> String {
        let key = normalise(name);
        match self.names.iter().find(|(k, _)| *k == key) {
            Some(&(_, first_line)) => warn(
                self.at.name,
                RowKind::DuplicateName {
                    value: name.to_string(),
                    first_line,
                },
            ),
            None => self.names.push((key.clone(), line)),
        }
        key
    }

    /// FR-8, FR-9: the row's quadrant, added on first sight. The label is
    /// the ordinally first variant, trimmed, whatever order rows come in.
    fn quadrant(&mut self, value: &str) -> usize {
        let key = normalise(value);
        let label = trim(value);
        if let Some(i) = self.quadrants.iter().position(|q| q.key == key) {
            // D-53: "ordinally" is code-point order, which is `str`'s `<`.
            // `order::compare_names` (PR 07) is the same rule; this site
            // cannot call it, since `order.rs` depends on this module.
            if label < self.quadrants[i].label.as_str() {
                self.quadrants[i].label = label.to_string();
            }
            return i;
        }
        self.quadrants.push(Quadrant {
            key,
            label: label.to_string(),
        });
        self.quadrants.len() - 1
    }

    /// FR-16, FR-17: the declared ring the value matches, or a warning
    /// naming where the ring set came from (D-61).
    fn ring(&self, value: &str, warn: &mut impl FnMut(usize, RowKind)) -> Option<usize> {
        let key = normalise(value);
        let found = self.ring_keys.iter().position(|k| *k == key);
        if found.is_none() {
            warn(
                self.at.ring,
                RowKind::UndeclaredRing {
                    value: value.to_string(),
                    source: self.source,
                },
            );
        }
        found
    }
}

/// FR-20: `conditional` and `default` are recognised; blank claims nothing;
/// anything else claims nothing too, with a warning (D-6, D-35).
fn read_qualifier(
    value: &str,
    column: usize,
    warn: &mut impl FnMut(usize, RowKind),
) -> Option<Qualifier> {
    match normalise(value).as_str() {
        "conditional" => Some(Qualifier::Conditional),
        "default" => Some(Qualifier::Default),
        "" => None,
        _ => {
            warn(
                column,
                RowKind::UnrecognisedQualifier {
                    value: value.to_string(),
                },
            );
            None
        }
    }
}

/// FR-26: blank is absent; an unrecognised value is `NoChange`, with a
/// warning (D-35).
fn read_status(
    value: &str,
    column: usize,
    warn: &mut impl FnMut(usize, RowKind),
) -> Option<Status> {
    let key = normalise(value);
    if key.is_empty() {
        return None;
    }
    let known = Status::ALL
        .into_iter()
        .find(|s| normalise(s.label()) == key);
    Some(known.unwrap_or_else(|| {
        warn(
            column,
            RowKind::UnrecognisedStatus {
                value: value.to_string(),
            },
        );
        Status::NoChange
    }))
}

/// FR-24, FR-25: a non-blank value that is not `YYYY-MM-DD` warns. Either
/// way it renders as written (FR-24), so nothing is returned.
fn check_date(value: &str, at: usize, column: DateColumn, warn: &mut impl FnMut(usize, RowKind)) {
    let t = trim(value);
    if !t.is_empty() && !is_iso_date(t) {
        warn(
            at,
            RowKind::UnparsableDate {
                column,
                value: value.to_string(),
            },
        );
    }
}

/// FR-93: a value under a column with no name is not shown, so say so.
fn check_unnamed(value: &str, at: usize, warn: &mut impl FnMut(usize, RowKind)) {
    if !trim(value).is_empty() {
        warn(
            at,
            RowKind::UnnamedValue {
                column: format!("column {}", at + 1),
            },
        );
    }
}

/// FR-24's shape: a digit wherever the pattern has a letter.
const ISO_DATE: &str = "YYYY-MM-DD";
/// Days per month in a common year.
const MONTH_DAYS: [usize; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
const FEBRUARY: usize = 2;
/// The Gregorian leap rule: every fourth year, except centuries, except
/// every fourth century.
const LEAP_YEAR: usize = 4;
const CENTURY: usize = 100;
const LEAP_CENTURY: usize = 400;

/// `YYYY-MM-DD`, a real calendar date. Exactly that shape: no week dates,
/// no times, no unpadded fields (FR-24). The shape is checked first, so
/// what is split is ASCII digits and hyphens only.
fn is_iso_date(s: &str) -> bool {
    let shaped = s.len() == ISO_DATE.len()
        && s.bytes().zip(ISO_DATE.bytes()).all(|(c, p)| {
            if p == b'-' {
                c == b'-'
            } else {
                c.is_ascii_digit()
            }
        });
    if !shaped {
        return false;
    }
    let mut fields = s.split('-').filter_map(|f| f.parse::<usize>().ok());
    let (Some(year), Some(month), Some(day)) = (fields.next(), fields.next(), fields.next()) else {
        return false;
    };
    let leap = year.is_multiple_of(LEAP_YEAR)
        && (!year.is_multiple_of(CENTURY) || year.is_multiple_of(LEAP_CENTURY));
    month
        .checked_sub(1)
        .and_then(|m| MONTH_DAYS.get(m))
        .map(|&d| d + usize::from(leap && month == FEBRUARY))
        .is_some_and(|d| (1..=d).contains(&day))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::csv::parse::parse;
    use crate::rings::parse_rings;

    const HEAD: &str = "name,quadrant,ring,description\n";
    const FULL: &str = "name,quadrant,ring,description,qualifier,last_affirmed,exit_date,status\n";

    /// `csv` through `parse` and `build_catalogue` under the default rings,
    /// with the rendered warnings. Parse must succeed with no error.
    fn build(csv: &str) -> (Result<Catalogue, Vec<Fatal>>, Vec<String>) {
        build_with(csv, None)
    }

    fn build_with(csv: &str, rings: Option<&str>) -> (Result<Catalogue, Vec<Fatal>>, Vec<String>) {
        let mut w = Warnings::default();
        let (table, errs) = parse(csv, &mut w).expect("parses");
        assert!(errs.is_empty(), "{errs:?}");
        let rings = parse_rings(rings).expect("valid rings");
        let result = build_catalogue(&table, &rings, &mut w);
        (result, w.render())
    }

    fn catalogue(csv: &str) -> Catalogue {
        build(csv).0.expect("builds")
    }

    fn errors(csv: &str) -> Vec<String> {
        build(csv)
            .0
            .expect_err("fails")
            .iter()
            .map(Fatal::render)
            .collect()
    }

    fn warnings(csv: &str) -> Vec<String> {
        let (result, warnings) = build(csv);
        result.expect("builds");
        warnings
    }

    /// Each item's `f`, in CSV order, with the rendered warnings.
    fn per_item<T>(csv: &str, f: impl Fn(&Item) -> T) -> (Vec<T>, Vec<String>) {
        let (result, warnings) = build(csv);
        (
            result.expect("builds").items.iter().map(f).collect(),
            warnings,
        )
    }

    fn missing(column: &'static str) -> String {
        Fatal::MissingColumn { line: 1, column }.render()
    }

    fn empty(line: usize, column: &'static str) -> String {
        Fatal::EmptyRequired { line, column }.render()
    }

    // --- The header: FR-5, FR-27, FR-78, FR-93 ---

    /// FR-78's columns in header order; FR-93 drops the unnamed one.
    #[test]
    fn columns_keep_header_order_and_drop_unnamed_ones() {
        let cat = catalogue("name,quadrant,,ring,description, Notes ,\nK,T,,Adopt,d,n,\n");
        let headings: Vec<_> = cat.columns.iter().map(|c| c.heading.as_str()).collect();
        assert_eq!(
            headings,
            ["name", "quadrant", "ring", "description", " Notes "]
        );
        assert_eq!(cat.columns[4].kind, ColumnKind::Other);
        assert_eq!(cat.items[0].cells, ["K", "T", "Adopt", "d", "n"]);
    }

    /// FR-76 (D-61): case, spaces and invisible format characters do not
    /// hide a required column — `rg-lme`'s case.
    #[test]
    fn required_columns_match_under_fr76() {
        let cat = catalogue("\u{feff}Name\u{200b}, QUADRANT ,Ring,description\nK,T,Adopt,d\n");
        let kinds: Vec<_> = cat.columns.iter().map(|c| c.kind).collect();
        assert_eq!(
            kinds,
            [
                ColumnKind::Name,
                ColumnKind::Quadrant,
                ColumnKind::Ring,
                ColumnKind::Description
            ]
        );
        // The heading is displayed text, so it stays as written (FR-76).
        let headings: Vec<_> = cat.columns.iter().map(|c| c.heading.as_str()).collect();
        assert_eq!(
            headings,
            ["\u{feff}Name\u{200b}", " QUADRANT ", "Ring", "description"]
        );
    }

    /// FR-5, NFR-16: every missing column at once, in FR-5's order.
    #[test]
    fn every_missing_required_column_is_reported() {
        assert_eq!(
            errors("ring,name\nAdopt,K\n"),
            [missing("quadrant"), missing("description")]
        );
    }

    /// FR-5's line is the header's, which blank lines above it move (FR-3).
    #[test]
    fn a_missing_column_names_the_header_line() {
        assert_eq!(
            errors("\n,,\nname,quadrant,ring\n"),
            [Fatal::MissingColumn {
                line: 3,
                column: "description"
            }
            .render()]
        );
    }

    /// FR-27: an unrecognised column is no error and no warning.
    #[test]
    fn an_unrecognised_column_is_kept_silently() {
        let csv = "name,quadrant,ring,description,ticket\nK,T,Adopt,d,OPS-1\n";
        let quiet = warnings(csv);
        assert!(quiet.is_empty(), "{quiet:?}");
        assert_eq!(catalogue(csv).items[0].cells[4], "OPS-1");
    }

    // --- FR-6 ---

    /// FR-6 under FR-76: spaces or a lone zero-width space are empty.
    /// Every one is reported, in line order then column order (NFR-16).
    #[test]
    fn every_empty_required_value_is_reported() {
        let csv = format!("{HEAD} ,T,Adopt,d\nK,\u{200b},  ,d\nK,T,Adopt,d\n");
        assert_eq!(
            errors(&csv),
            [empty(2, "name"), empty(3, "quadrant"), empty(3, "ring")]
        );
    }

    /// NFR-11: within a line, errors follow the header's column order, not
    /// FR-6's list.
    #[test]
    fn empty_required_values_follow_header_column_order() {
        assert_eq!(
            errors("ring,name,quadrant,description\n ,,T,d\n"),
            [empty(2, "ring"), empty(2, "name")]
        );
    }

    /// SC-10: FR-90's removal runs first, so a `name` of only U+0001 is
    /// the empty-`name` hard error.
    #[test]
    fn a_name_of_only_undisplayable_characters_is_empty() {
        assert_eq!(
            errors(&format!("{HEAD}\u{1},T,Adopt,d\n")),
            [empty(2, "name")]
        );
    }

    /// NFR-16: FR-5 and FR-6 arrive in one run. The present required
    /// columns are still checked.
    #[test]
    fn missing_columns_and_empty_values_arrive_together() {
        assert_eq!(
            errors("name,quadrant,ring\n,T,Adopt\n"),
            [missing("description"), empty(2, "name")]
        );
    }

    /// `rg-rnc`: a cleared row (`,,,`) between data rows is skipped, but
    /// its line still counts (FR-3), so later lines stay the editor's.
    #[test]
    fn a_cleared_row_does_not_shift_line_numbers() {
        let csv = format!("{HEAD}K,T,Adopt,d\n,,,\n,T,Adopt,d\n");
        assert_eq!(errors(&csv), [empty(4, "name")]);
    }

    /// FR-3's blank test counts only spaces and tabs (D-59), so a row
    /// holding only a zero-width space is a row, and FR-6 reports it.
    #[test]
    fn a_row_of_only_format_characters_is_not_blank() {
        assert_eq!(
            errors(&format!("{HEAD}\u{200b},,,\n")),
            [empty(2, "name"), empty(2, "quadrant"), empty(2, "ring")]
        );
    }

    /// `Table` and `Record` are public, so a hand-built short record reads
    /// as empty cells, which FR-6 reports, rather than a panic.
    #[test]
    fn a_short_record_reads_as_empty_not_a_panic() {
        let table = Table {
            header: ["name", "quadrant", "ring", "description"]
                .map(String::from)
                .to_vec(),
            header_line: 1,
            records: vec![Record {
                line: 2,
                fields: vec!["K".into()],
            }],
        };
        let rings = parse_rings(None).expect("default");
        let errs = build_catalogue(&table, &rings, &mut Warnings::default()).expect_err("fails");
        assert_eq!(
            errs.iter().map(Fatal::render).collect::<Vec<_>>(),
            [empty(2, "quadrant"), empty(2, "ring")]
        );
    }

    /// No warning from the model when it fails; `parse`'s FR-90 warnings
    /// are already pushed and stay.
    #[test]
    fn errors_push_no_model_warnings() {
        let csv = format!("{HEAD},T,Nope,\nK\u{1},T,Adopt,d\n");
        let (result, warnings) = build(&csv);
        assert!(result.is_err());
        assert_eq!(
            warnings,
            ["warning: line 3, name: removed U+0001, which the output cannot display"]
        );
    }

    /// A header-only file is FR-42's empty state, not an error, and warns
    /// nothing.
    #[test]
    fn a_header_only_file_builds_an_empty_catalogue() {
        let (result, warnings) = build(HEAD);
        let cat = result.expect("builds");
        assert!(cat.items.is_empty() && cat.quadrants.is_empty());
        assert_eq!((cat.columns.len(), cat.rings.len()), (4, 4));
        assert!(warnings.is_empty(), "{warnings:?}");
    }

    // --- Quadrants, rings and names: FR-8, FR-9, FR-11, FR-16, FR-17, FR-28 ---

    /// FR-9: one quadrant per normalised value, labelled by the ordinally
    /// first variant, trimmed — the same label whatever the row order.
    #[test]
    fn quadrants_group_and_take_the_ordinally_first_label() {
        for rows in [
            "a,tool ,Adopt,d\nb,Tool,Adopt,d\nc,TOOL,Adopt,d\n",
            "c,TOOL,Adopt,d\nb,Tool,Adopt,d\na,tool ,Adopt,d\n",
        ] {
            let cat = catalogue(&format!("{HEAD}{rows}"));
            assert_eq!(
                cat.quadrants,
                [Quadrant {
                    key: "tool".into(),
                    label: "TOOL".into()
                }]
            );
            assert!(cat.items.iter().all(|i| i.quadrant == 0));
        }
    }

    /// FR-9 compares the variants **trimmed**: untrimmed, ` tool` would
    /// sort first (a space is below `T`). A boundary format character goes
    /// too (D-61).
    #[test]
    fn quadrant_labels_compare_trimmed_variants() {
        let cat = catalogue(&format!(
            "{HEAD}a, tool,Adopt,d\nb,Tool,Adopt,d\nc,\u{200b}Tool,Adopt,d\n"
        ));
        assert_eq!(cat.quadrants[0].label, "Tool");
    }

    /// FR-8: quadrants in order of first appearance; FR-10 orders them later.
    #[test]
    fn quadrants_keep_first_appearance_order() {
        let cat = catalogue(&format!(
            "{HEAD}a,Tool,Adopt,d\nb,Platform,Adopt,d\nc,tool,Adopt,d\n"
        ));
        let labels: Vec<_> = cat.quadrants.iter().map(|q| q.label.as_str()).collect();
        assert_eq!(labels, ["Tool", "Platform"]);
        let indices: Vec<_> = cat.items.iter().map(|i| i.quadrant).collect();
        assert_eq!(indices, [0, 1, 0]);
    }

    /// FR-11: near-duplicates are neither merged nor flagged.
    #[test]
    fn near_duplicates_stay_apart_silently() {
        let csv = format!("{HEAD}Kafka,Tool,Adopt,d\nKafkas,Tools,Adopt,d\n");
        let (result, warnings) = build(&csv);
        assert_eq!(result.expect("builds").quadrants.len(), 2);
        assert!(warnings.is_empty(), "{warnings:?}");
    }

    /// FR-16: matched under FR-76, both sides normalised.
    #[test]
    fn rings_match_under_fr76() {
        let cat = catalogue(&format!("{HEAD}a,T, adopt\u{200b},d\nb,T,RETIRE,d\n"));
        let rings: Vec<_> = cat.items.iter().map(|i| i.ring).collect();
        assert_eq!(rings, [Some(2), Some(3)]);
        assert_eq!(cat.rings, ["Assess", "Trial", "Adopt", "Retire"]);
    }

    /// FR-17 (D-61): unplotted, kept, and the warning names the source.
    #[test]
    fn an_undeclared_ring_is_unplotted_and_names_the_ring_source() {
        let csv = format!("{HEAD}a,T,In Use,d\n");
        let (result, warnings) = build(&csv);
        assert_eq!(result.expect("builds").items[0].ring, None);
        assert_eq!(
            warnings,
            [
                "warning: line 2, ring: 'In Use' is not declared by the default ring set; the row is in the table view, not on the radar"
            ]
        );
        let (_, warnings) = build_with(&csv, Some("Adopt,Hold"));
        assert_eq!(
            warnings,
            [
                "warning: line 2, ring: 'In Use' is not declared by --rings; the row is in the table view, not on the radar"
            ]
        );
    }

    /// FR-28: every row is kept, and each later occurrence names the
    /// **first** line, not the previous one.
    #[test]
    fn a_repeated_name_names_its_first_line() {
        let csv = format!("{HEAD}Kafka,T,Adopt,d\nkafka ,T,Trial,d\nKAFKA,T,Assess,d\n");
        let mut w = Warnings::default();
        let (table, _) = parse(&csv, &mut w).expect("parses");
        let rings = parse_rings(None).expect("default");
        let cat = build_catalogue(&table, &rings, &mut w).expect("builds");
        assert_eq!(cat.items.len(), 3);
        let firsts: Vec<_> = w
            .as_slice()
            .iter()
            .filter_map(|warning| match warning {
                Warning::Row {
                    kind: RowKind::DuplicateName { first_line, .. },
                    ..
                } => Some(*first_line),
                _ => None,
            })
            .collect();
        assert_eq!(firsts, [2, 2]);
    }

    /// SC-10: FR-90's removal runs first, so `Kafka` + U+0001 repeats
    /// `Kafka`; so does `Kafka` behind a zero-width space (FR-76, D-61).
    #[test]
    fn invisible_characters_do_not_hide_a_repeated_name() {
        assert_eq!(
            warnings(&format!("{HEAD}Kafka,T,Adopt,d\nKafka\u{1},T,Adopt,d\n")),
            [
                "warning: line 3, name: removed U+0001, which the output cannot display",
                "warning: line 3, name: 'Kafka' repeats line 2; both are drawn",
            ]
        );
        assert_eq!(
            warnings(&format!("{HEAD}Kafka,T,Adopt,d\n\u{200b}Kafka,T,Adopt,d\n")),
            ["warning: line 3, name: 'Kafka' repeats line 2; both are drawn"]
        );
    }

    /// The name is kept as written; its key is FR-76's, computed once.
    #[test]
    fn names_keep_their_text_and_carry_a_key() {
        let item = &catalogue(&format!("{HEAD} Kafka ,T,Adopt,d\n")).items[0];
        assert_eq!(
            (item.name.as_str(), item.name_key.as_str()),
            (" Kafka ", "kafka")
        );
    }

    /// FR-7: an empty description renders, with a warning. Spaces or a
    /// lone zero-width space are empty.
    #[test]
    fn an_empty_description_warns() {
        assert_eq!(
            warnings(&format!("{HEAD}K,T,Adopt, \nL,T,Adopt,\u{200b}\n")),
            ["warning: 2 rows have an empty description — lines 2, 3; each detail panel omits it"]
        );
    }

    // --- Optional columns: FR-20, FR-21, FR-24–FR-26, FR-29, FR-93 ---

    /// FR-20: `conditional` is hollow and `default` is recognised; blank —
    /// spaces and format characters included (FR-76) — claims nothing,
    /// silently; anything else claims nothing, with a warning.
    #[test]
    fn qualifiers_are_read_under_fr76() {
        let csv = format!(
            "{FULL}a,T,Adopt,d, Conditional,,,\nb,T,Adopt,d,default,,,\nc,T,Adopt,d,,,,\nd,T,Adopt,d,  ,,,\ne,T,Adopt,d,\u{200b},,,\nf,T,Adopt,d,\u{200b}conditional,,,\ng,T,Adopt,d,maybe,,,\n"
        );
        let (qualifiers, warnings) = per_item(&csv, |i| (i.qualifier, i.hollow()));
        assert_eq!(
            qualifiers,
            [
                (Some(Qualifier::Conditional), true),
                (Some(Qualifier::Default), false),
                (None, false),
                (None, false),
                (None, false),
                (Some(Qualifier::Conditional), true),
                (None, false),
            ]
        );
        assert_eq!(
            warnings,
            ["warning: line 8, qualifier: unrecognised qualifier value 'maybe'; drawn solid"]
        );
    }

    /// FR-21: the qualifier ignores the ring, under any ring set.
    #[test]
    fn the_qualifier_ignores_the_ring() {
        let csv = format!("{FULL}a,T,Use,d,conditional,,,\nb,T,Gone,d,conditional,,,\n");
        let (result, _) = build_with(&csv, Some("Use,Approved"));
        let hollow: Vec<_> = result
            .expect("builds")
            .items
            .iter()
            .map(Item::hollow)
            .collect();
        assert_eq!(hollow, [true, true]);
    }

    /// FR-26: four values under FR-76; blank (spaces and format characters
    /// included) is absent; anything else is `No Change` with a warning.
    #[test]
    fn statuses_are_read_under_fr76() {
        let csv = format!(
            "{FULL}a,T,Adopt,d,,,,new\nb,T,Adopt,d,,,,Moved In\nc,T,Adopt,d,,,,MOVED OUT\nd,T,Adopt,d,,,,\u{feff}no change\ne,T,Adopt,d,,,,\nf,T,Adopt,d,,,,  \ng,T,Adopt,d,,,,\u{200b}\nh,T,Adopt,d,,,,Gone\n"
        );
        let (statuses, warnings) = per_item(&csv, |i| i.status);
        assert_eq!(
            statuses,
            [
                Some(Status::New),
                Some(Status::MovedIn),
                Some(Status::MovedOut),
                Some(Status::NoChange),
                None,
                None,
                None,
                Some(Status::NoChange)
            ]
        );
        assert_eq!(
            warnings,
            ["warning: line 9, status: unrecognised status value 'Gone'; drawn as No Change"]
        );
    }

    /// FR-24, FR-25: an unparsable date warns in either date column, naming
    /// it; a blank one does not, and nor does one inside spaces or format
    /// characters. Either way the cell is kept as written.
    #[test]
    fn unparsable_dates_warn_and_keep_their_text() {
        let csv = format!(
            "{FULL}a,T,Adopt,d,, 2026-06-01 ,June 2027,\nb,T,Adopt,d,,\u{200b}2026-06-01\u{feff},,\nc,T,Adopt,d,,,,\nd,T,Adopt,d,,2026-6-1,,\ne,T,Adopt,d,,\u{200b},,\n"
        );
        let (result, warnings) = build(&csv);
        let items = result.expect("builds").items;
        assert_eq!(items[0].cell(5), Some(" 2026-06-01 "));
        assert_eq!(items[1].cell(5), Some("\u{200b}2026-06-01\u{feff}"));
        assert_eq!(
            warnings,
            [
                "warning: line 2, exit_date: 'June 2027' is not a YYYY-MM-DD date; shown as written",
                "warning: line 5, last_affirmed: '2026-6-1' is not a YYYY-MM-DD date; shown as written",
            ]
        );
    }

    /// FR-24's shape exactly, and a real calendar date. A multi-byte
    /// character at a field boundary is rejected, not sliced.
    #[test]
    fn iso_dates_are_exactly_the_shape_and_real() {
        for ok in [
            "2026-06-01",
            "2026-04-30",
            "2024-02-29",
            "2000-02-29",
            "1999-12-31",
        ] {
            assert!(is_iso_date(ok), "{ok}");
        }
        for bad in [
            "2026-6-1",
            "2026-04-31",
            "2026-02-30",
            "2023-02-29",
            "1900-02-29",
            "2026-13-01",
            "2026-00-10",
            "2026-01-00",
            "2026/06/01",
            "20260601",
            "2026-06-01T00:00",
            "２０２６-06-01",
            "202é06-01",
            "",
        ] {
            assert!(!is_iso_date(bad), "{bad}");
        }
    }

    /// FR-29: an absent optional column claims nothing and warns nothing.
    #[test]
    fn absent_optional_columns_claim_nothing() {
        let csv = format!("{HEAD}K,T,Adopt,d\n");
        let (result, warnings) = build(&csv);
        let item = &result.expect("builds").items[0];
        assert_eq!((item.qualifier, item.status), (None, None));
        assert!(warnings.is_empty(), "{warnings:?}");
    }

    /// FR-29: an empty cell is omitted, not shown as an empty label.
    #[test]
    fn an_empty_cell_is_omitted() {
        let item = &catalogue(&format!("{FULL}K,T,Adopt,d,,  ,,\n")).items[0];
        assert_eq!(
            (item.cell(4), item.cell(5), item.cell(99)),
            (None, None, None)
        );
        assert_eq!(item.cell(0), Some("K"));
    }

    /// `Item::cell` indexes the named columns, not the header: an unnamed
    /// column before `exit_date` does not shift it.
    #[test]
    fn cells_index_named_columns_not_header_positions() {
        let cat = catalogue(
            "name,,quadrant,ring,description,exit_date\nOld,,Tool,Rejected,d,2027-01-01\n",
        );
        assert_eq!(cat.items[0].cell(4), Some("2027-01-01"));
    }

    /// FR-93: Excel's trailing padding passes silently, and so does a cell
    /// of only a format character (FR-76); a value under no name warns,
    /// naming the column's position — wherever the column is, and whether
    /// the header cell is empty or only a format character.
    #[test]
    fn unnamed_columns_warn_only_when_they_hold_a_value() {
        let padding = warnings("name,quadrant,ring,description,,\nK,T,Adopt,d,,\n");
        assert!(padding.is_empty(), "{padding:?}");
        let invisible = warnings("name,quadrant,ring,description,\nK,T,Adopt,d,\u{200b}\n");
        assert!(invisible.is_empty(), "{invisible:?}");
        assert_eq!(
            warnings("name,quadrant,ring,description,,\nK,T,Adopt,d,, \nL,T,Adopt,d,,x\n"),
            [
                "warning: line 3, column 6: this column has no name in the header row, so its value is not shown; add a header name, or delete the value"
            ]
        );
        assert_eq!(
            warnings("name,\u{200b},quadrant,ring,description\nK,x,T,Adopt,d\n"),
            [
                "warning: line 2, column 2: this column has no name in the header row, so its value is not shown; add a header name, or delete the value"
            ]
        );
    }

    /// `rg-rnc`: FR-90 still warns for a cell in a column FR-93 drops, and
    /// FR-93 warns as well, since the cell still holds a value.
    #[test]
    fn a_dropped_column_still_gets_fr90s_warning() {
        assert_eq!(
            warnings("name,quadrant,ring,description,\nK,T,Adopt,d,a\u{1}\n"),
            [
                "warning: line 2, column 5: removed U+0001, which the output cannot display",
                "warning: line 2, column 5: this column has no name in the header row, so its value is not shown; add a header name, or delete the value",
            ]
        );
    }

    /// NFR-11 across the two stages: parse's and the model's warnings on
    /// one line sort by header column, whichever stage pushed first.
    #[test]
    fn warnings_from_both_stages_sort_by_header_column() {
        let csv = "name,,quadrant,ring,description,\nK,x,T,Adpot,d\u{1},\nL,,T,Adopt,,\n";
        assert_eq!(
            warnings(csv),
            [
                "warning: line 2, column 2: this column has no name in the header row, so its value is not shown; add a header name, or delete the value",
                "warning: line 2, ring: 'Adpot' is not declared by the default ring set; the row is in the table view, not on the radar",
                "warning: line 2, description: removed U+0001, which the output cannot display",
                "warning: line 3, description: empty; the detail panel omits it",
            ]
        );
    }

    /// Two unnamed columns put the ring at header position 4 but named
    /// index 2, so a model warning keyed by the named index would sort
    /// before FR-90's at header position 3 (NFR-11).
    #[test]
    fn model_warnings_use_header_indices_not_named_ones() {
        assert_eq!(
            warnings("name,quadrant,,,ring,description\nK,T,,\u{1},Adpot,d\n"),
            [
                "warning: line 2, column 4: removed U+0001, which the output cannot display",
                "warning: line 2, ring: 'Adpot' is not declared by the default ring set; the row is in the table view, not on the radar",
            ]
        );
    }

    // --- FR-50, FR-75 ---

    /// The legend row's text first — number, name, qualifier, status —
    /// then the quadrant and ring, named as such; an unplotted row says so
    /// instead of a number (FR-18).
    #[test]
    fn accessible_names_carry_the_row_text_and_its_position() {
        let cat = catalogue(&format!(
            "{FULL} Kafka ,tool,adopt,d,conditional,,,moved in\nOld,Tool,Rejected,d,,,,\n"
        ));
        assert_eq!(
            cat.accessible_name(&cat.items[0], Some(3)),
            "3. Kafka, Conditional, Moved In; Tool quadrant, Adopt ring"
        );
        assert_eq!(
            cat.accessible_name(&cat.items[1], None),
            "Old; Tool quadrant, Rejected ring; not on the radar"
        );
    }

    /// The unplotted ring is read from the named columns, so an unnamed
    /// column before it does not shift it onto another cell.
    #[test]
    fn an_unplotted_ring_is_found_past_an_unnamed_column() {
        let cat = catalogue("name,,quadrant,ring,description\nOld,,Tool,Rejected,d\n");
        assert_eq!(
            cat.accessible_name(&cat.items[0], None),
            "Old; Tool quadrant, Rejected ring; not on the radar"
        );
        assert_eq!(cat.column(ColumnKind::Ring), Some(2));
    }

    /// A catalogue built by hand without a `ring` column names no ring,
    /// rather than panicking or leaving an empty one in the name.
    #[test]
    fn an_accessible_name_without_a_ring_column_omits_the_ring() {
        let cat = Catalogue {
            columns: vec![Column {
                heading: "name".into(),
                kind: ColumnKind::Name,
            }],
            quadrants: vec![Quadrant {
                key: "tool".into(),
                label: "Tool".into(),
            }],
            rings: Vec::new(),
            items: vec![Item {
                line: 2,
                name: "Old".into(),
                name_key: "old".into(),
                quadrant: 0,
                ring: None,
                qualifier: None,
                status: None,
                cells: vec!["Old".into()],
            }],
        };
        assert_eq!(
            cat.accessible_name(&cat.items[0], None),
            "Old; Tool quadrant; not on the radar"
        );
    }

    /// FR-18: only a plotted item has a number, and a caller passing one
    /// for an unplotted item is a bug caught in debug builds.
    #[test]
    #[cfg(debug_assertions)]
    #[should_panic(expected = "FR-18")]
    fn a_number_for_an_unplotted_item_is_a_bug() {
        let cat = catalogue(&format!("{HEAD}Old,Tool,Rejected,d\n"));
        let _ = cat.accessible_name(&cat.items[0], Some(1));
    }
}
