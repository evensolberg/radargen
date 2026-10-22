//! Records to meaning (architecture §2): which column is which, what each
//! row claims, and what is wrong with it (FR-5–FR-29, FR-93).
//!
//! Ordering and numbering are not here: FR-10 and FR-34 are `order.rs`'s
//! (`order::sector_order`, `order::resolve`), so a `Catalogue` keeps
//! sectors and items in CSV order.

// qual:allow(coupling, sdp) reason: "the pipeline runs csv → model → order → layout (architecture §2); depending on the previous stage is the design"

pub use crate::columns::ColumnKind;
use crate::csv::parse::{Record, Table};
use crate::date::is_iso_date;
use crate::diagnostics::{
    DateColumn, OffRadar, RowError, RowKind, Warning, Warnings, column_label,
};
pub use crate::link::Link;
use crate::rings::{RingMatch, RingSet};
use crate::settings::Origin;
use crate::{normalise, single_line, tidy, trim};
use std::borrow::Cow;
use std::sync::Arc;

/// FR-5's required columns, in the order its error lists them.
const REQUIRED: [ColumnKind; 4] = [
    ColumnKind::Name,
    ColumnKind::Sector,
    ColumnKind::Ring,
    ColumnKind::Description,
];

/// FR-6's columns: required, and never empty. `description` may be empty
/// (FR-7).
const NON_EMPTY: [ColumnKind; 3] = [ColumnKind::Name, ColumnKind::Sector, ColumnKind::Ring];

/// One named column, in header order (FR-27, FR-78).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Column {
    /// The header cell as written (FR-76 edits no displayed text). An
    /// `Other` column (FR-27) is headed with this; a renderer may give a
    /// known kind its own heading.
    pub heading: String,
    /// What the column means, matched from the header under FR-76.
    pub kind: ColumnKind,
    /// Its 0-based header index, unnamed columns counted: where a warning
    /// about one of its cells points (NFR-10, NFR-11).
    pub position: usize,
}

/// FR-20's recognised values. Blank and unrecognised values claim nothing,
/// so they have no variant: the marker is solid and the legend row shows
/// no qualifier (FR-74).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Qualifier {
    /// Written `default`: a solid marker, as for any other value (FR-20).
    Default,
    /// Written `conditional`: a hollow marker (FR-20).
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

    /// FR-20: `Conditional` is drawn hollow, every other value solid.
    #[must_use]
    pub const fn is_hollow(self) -> bool {
        matches!(self, Self::Conditional)
    }
}

/// FR-26's values. An unrecognised one is `NoChange`, with a warning.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// New this edition (FR-26).
    New,
    /// Moved toward the centre since the last edition (FR-26, FR-40).
    MovedIn,
    /// Moved toward the edge since the last edition (FR-26, FR-40).
    MovedOut,
    /// Unchanged; also the fallback for an unrecognised value (FR-26).
    NoChange,
}

impl Status {
    /// Every value, so the vocabulary is written once, in [`Status::label`].
    pub(crate) const ALL: [Self; 4] = [Self::New, Self::MovedIn, Self::MovedOut, Self::NoChange];

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

    /// `self` if a legend row or accessible name shows it: every value but
    /// `NoChange`, which is FR-40's plain marker (FR-50, FR-74).
    #[must_use]
    pub fn shown(self) -> Option<Self> {
        (self != Self::NoChange).then_some(self)
    }
}

/// One sector (FR-8, FR-9).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sector {
    /// FR-76's normalised value, which groups the rows.
    pub key: String,
    /// FR-9: the ordinally first variant, [`tidy`].
    pub label: String,
    /// The physical line of the first row that wrote `label`, where a
    /// warning about the drawn label points (FR-104, NFR-10).
    pub line: usize,
}

/// One CSV data row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    /// The physical line the record begins on (NFR-10). Lines ascend with
    /// CSV order, so this is also FR-34's last tiebreak.
    pub line: usize,
    /// [`tidy`]: as written after FR-90's removal, trimmed, each run of
    /// whitespace inside one space, which is how every view shows it (D-76).
    pub name: String,
    /// FR-76's key, computed once so no comparator folds (architecture §6).
    pub name_key: String,
    /// Index into [`Catalogue::sectors`], which is in first-appearance
    /// order — **not** FR-10's, and never a palette index (architecture
    /// §5). `order::resolve` gives the FR-10 rank.
    pub sector: usize,
    /// Index into [`Catalogue::rings`]; `None` when the ring is not
    /// declared (FR-17), which leaves the row unplotted.
    pub ring: Option<usize>,
    /// FR-20; `None` when blank or unrecognised.
    pub qualifier: Option<Qualifier>,
    /// FR-26: `None` when blank or absent; an unrecognised value is
    /// `Some(Status::NoChange)`, with a warning.
    pub status: Option<Status>,
    /// FR-96: `None` when blank or absent; a value that is not an `http` or
    /// `https` address is [`Link::Text`], with a warning.
    pub link: Option<Link>,
    /// Every named column's cell, aligned with [`Catalogue::columns`] (not
    /// with the header, which may hold unnamed columns), as written after
    /// FR-90's removal (FR-78).
    pub cells: Vec<String>,
}

impl Item {
    /// FR-20: drawn hollow.
    #[must_use]
    pub fn hollow(&self) -> bool {
        self.qualifier.is_some_and(Qualifier::is_hollow)
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
    /// In order of first appearance; `order::sector_order` gives FR-10's
    /// order.
    pub sectors: Vec<Sector>,
    /// The declared rings, edge-to-centre (FR-12). [`Item::ring`] indexes
    /// this copy; nothing below the model reads the `RingSet`.
    pub rings: Vec<String>,
    /// Where the rings were declared, so a warning about one names it
    /// (FR-17, FR-95, D-73).
    pub ring_origin: Origin,
    /// Every row as an item, in CSV order.
    pub items: Vec<Item>,
}

impl Catalogue {
    /// The index in [`Catalogue::columns`] of the first column of `kind`.
    #[must_use]
    pub fn column(&self, kind: ColumnKind) -> Option<usize> {
        self.columns.iter().position(|c| c.kind == kind)
    }

    /// The header position of the first column of `kind`, where a warning
    /// about one of its cells points (NFR-10, NFR-11).
    #[must_use]
    pub fn position(&self, kind: ColumnKind) -> Option<usize> {
        self.columns
            .iter()
            .find(|c| c.kind == kind)
            .map(|c| c.position)
    }

    /// The ring `item` is shown under: its declared ring's label when it is
    /// plotted, otherwise its cell, stored [`tidy`], empty when the cell is
    /// (FR-17).
    ///
    /// # Panics
    ///
    /// If `item.ring` is out of range for this catalogue, as for an item
    /// from another one.
    #[must_use]
    // qual:allow(complexity, error_handling) reason: "the documented # Panics: an item from another catalogue is a caller bug"
    pub fn ring_label<'a>(&'a self, item: &'a Item) -> &'a str {
        item.ring.map_or_else(
            || {
                self.column(ColumnKind::Ring)
                    .and_then(|c| item.cell(c))
                    .unwrap_or("")
            },
            |r| {
                self.rings
                    .get(r)
                    .expect("item.ring is within this catalogue's rings")
                    .as_str()
            },
        )
    }

    /// FR-50, FR-75: an accessible name for a marker, a legend row and —
    /// until `rg-6ny` settles how table rows are exposed — a table row.
    /// Only a table row can be unplotted (FR-18).
    ///
    /// It starts with what the legend row shows — number, name, qualifier,
    /// status (FR-74) — so the visible text is in the name (WCAG 2.5.3).
    /// `No Change` is FR-40's plain marker and no row shows it, so it is
    /// left out too (FR-50).
    /// The sector and ring follow even where the legend's grouping shows
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
    /// If `item.sector` or `item.ring` is out of range for this
    /// catalogue, as for an item from another one. In debug builds, also if
    /// `number` is `Some` for an unplotted item or `None` for a plotted
    /// one (FR-18).
    #[must_use]
    // qual:allow(complexity, error_handling) reason: "the documented # Panics: an item from another catalogue is a caller bug"
    pub fn accessible_name(&self, item: &Item, number: Option<usize>) -> String {
        debug_assert_eq!(number.is_some(), item.ring.is_some(), "FR-18");
        let ring = self.ring_label(item);
        let row = [
            Some(item.name.as_str()),
            item.qualifier.map(Qualifier::label),
            item.status.and_then(Status::shown).map(Status::label),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(", ");
        let sector = &self
            .sectors
            .get(item.sector)
            .expect("item.sector is within this catalogue's sectors")
            .label;
        let position = if ring.is_empty() {
            format!("{sector} sector")
        } else {
            format!("{sector} sector, {ring} ring")
        };
        number.map_or_else(
            || format!("{row}; {position}; {NOT_ON_THE_RADAR}"),
            |n| format!("{n}. {row}; {position}"),
        )
    }
}

/// FR-5–FR-29 and FR-93: the catalogue, or every row-level error.
///
/// NFR-16 tier two: every missing required column (FR-5) and every empty
/// `name`, `sector` or `ring` (FR-6) is returned together, in line order
/// then column order. Rows are checked before any is built, so an `Err`
/// comes with no row warning from this stage; FR-108's header warnings come
/// first, since a misspelt required column is why FR-5 fails. `csv::parse`'s
/// FR-4 errors and FR-90 warnings are separate: `pipeline.rs` merges both
/// stages' errors in NFR-11's order and prints them after the warnings.
///
/// `off_radar` is where the run puts a row FR-17 leaves off the radar,
/// which its warning says (FR-106).
///
/// # Errors
///
/// [`RowError::MissingColumn`] and [`RowError::EmptyRequired`].
pub fn build_catalogue(
    table: &Table,
    rings: &RingSet,
    off_radar: OffRadar,
    w: &mut Warnings,
) -> Result<Catalogue, Vec<RowError>> {
    let header = Header::of(&table.header);
    warn_near_misses(&header, table.header_line, w);
    none_or_all(
        [
            header.missing(table.header_line),
            empty_required(&header, &table.records),
        ]
        .into_iter()
        .flatten()
        .collect(),
    )?;
    Ok(Builder::new(header, rings, off_radar).build(&table.records, w))
}

/// FR-108: one warning per header cell whose name nearly matches a known
/// column the header lacks. A repeat is FR-4's error already, so only its
/// first occurrence warns.
fn warn_near_misses(header: &Header, line: usize, w: &mut Warnings) {
    let taken: Vec<ColumnKind> = header
        .columns
        .iter()
        .map(|c| c.kind)
        .filter(|&k| k != ColumnKind::Other)
        .collect();
    let mut seen: Vec<String> = Vec::new();
    for column in header
        .columns
        .iter()
        .filter(|c| c.kind == ColumnKind::Other)
    {
        let key = normalise(&column.heading);
        if seen.contains(&key) {
            continue;
        }
        if let Some(known) = ColumnKind::near_miss(&key, &taken) {
            w.push(Warning::Row {
                line,
                column: column.position,
                kind: RowKind::NearMiss {
                    heading: column.heading.clone(),
                    known,
                },
            });
        }
        seen.push(key);
    }
}

/// `Ok` when there are no errors, else every one of them.
fn none_or_all(errors: Vec<RowError>) -> Result<(), Vec<RowError>> {
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
    /// The named columns; each one's [`Column::position`] is where its
    /// `Item::cells` entry is read from.
    columns: Vec<Column>,
    /// FR-93's unnamed columns.
    unnamed: Vec<usize>,
}

impl Header {
    fn of(names: &[String]) -> Self {
        let mut slots = Vec::with_capacity(names.len());
        let mut columns = Vec::new();
        let mut unnamed = Vec::new();
        for (p, name) in names.iter().enumerate() {
            let key = normalise(name);
            if key.is_empty() {
                slots.push(None);
                unnamed.push(p);
                continue;
            }
            slots.push(Some(columns.len()));
            columns.push(Column {
                heading: name.clone(),
                kind: ColumnKind::of(&key),
                position: p,
            });
        }
        Self {
            slots,
            columns,
            unnamed,
        }
    }

    /// The header position of the first column of `kind`. A repeat is
    /// already FR-4's error, so the first is the one that counts.
    fn position(&self, kind: ColumnKind) -> Option<usize> {
        self.slots.iter().position(|s| {
            s.is_some_and(|c| {
                self.columns
                    .get(c)
                    .expect("a slot holds an index into columns")
                    .kind
                    == kind
            })
        })
    }

    /// FR-5: every required column the header lacks.
    fn missing(&self, line: usize) -> Vec<RowError> {
        REQUIRED
            .into_iter()
            .filter(|&kind| self.position(kind).is_none())
            .map(|kind| RowError::MissingColumn { line, column: kind })
            .collect()
    }
}

/// FR-6: every empty `name`, `sector` or `ring` the header has, in line
/// order, then header column order (NFR-11).
fn empty_required(header: &Header, records: &[Record]) -> Vec<RowError> {
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
                .map(|&(_, kind)| RowError::EmptyRequired {
                    line: record.line,
                    column: kind,
                })
        })
        .collect()
}

/// Where each column the rows are read from sits in the header.
struct Positions {
    name: usize,
    sector: usize,
    ring: usize,
    description: usize,
    qualifier: Option<usize>,
    status: Option<usize>,
    last_affirmed: Option<usize>,
    exit_date: Option<usize>,
    link: Option<usize>,
}

impl Positions {
    /// The columns that are labels, not prose: a line break in one becomes
    /// a space (FR-97).
    fn labels(&self) -> impl Iterator<Item = usize> {
        [
            Some(self.name),
            Some(self.sector),
            Some(self.ring),
            self.qualifier,
            self.status,
            self.last_affirmed,
            self.exit_date,
            self.link,
        ]
        .into_iter()
        .flatten()
    }

    fn of(header: &Header) -> Self {
        // FR-5 has passed before a `Positions` is built, so the default is
        // never taken.
        let required = |kind| header.position(kind).unwrap_or_default();
        Self {
            name: required(ColumnKind::Name),
            sector: required(ColumnKind::Sector),
            ring: required(ColumnKind::Ring),
            description: required(ColumnKind::Description),
            qualifier: header.position(ColumnKind::Qualifier),
            status: header.position(ColumnKind::Status),
            last_affirmed: header.position(ColumnKind::LastAffirmed),
            exit_date: header.position(ColumnKind::ExitDate),
            link: header.position(ColumnKind::Link),
        }
    }
}

/// FR-17 and FR-41's words for a row the radar cannot plot: the table's
/// flag and the end of its accessible name, so a speech-input user can say
/// what a sighted reader sees (WCAG 2.5.3, D-66).
pub const NOT_ON_THE_RADAR: &str = "not on the radar";

/// Builds the catalogue a row at a time.
struct Builder<'r> {
    header: Header,
    at: Positions,
    /// The ring set, which matches each row's ring (FR-16, FR-107).
    rings: &'r RingSet,
    /// Where the run puts an FR-17 row, for its warning (FR-106).
    off_radar: OffRadar,
    /// The declared rings, shared by every FR-17 warning of the run, each
    /// holding a cheap `Arc` clone.
    declared: Arc<[String]>,
    sectors: Vec<Sector>,
    /// Each name's key, the line it first appears on and its spelling there,
    /// for FR-28.
    names: Vec<(String, usize, String)>,
    items: Vec<Item>,
}

impl<'r> Builder<'r> {
    fn new(header: Header, rings: &'r RingSet, off_radar: OffRadar) -> Self {
        Self {
            at: Positions::of(&header),
            header,
            rings,
            off_radar,
            declared: Arc::from(rings.rings()),
            sectors: Vec::new(),
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
            sectors: self.sectors,
            rings: self.rings.rings().to_vec(),
            ring_origin: self.rings.origin().clone(),
            items: self.items,
        }
    }

    fn add(&mut self, record: &Record, w: &mut Warnings) {
        let record = &single_lined(record, &self.at);
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
        let sector = self.sector(at(self.at.sector), record.line);
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
        let link = self.at.link.and_then(|p| read_link(at(p), p, &mut warn));
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
            .columns
            .iter()
            // The ring cell is shown for an unplotted row (FR-17), so it is
            // stored tidied as the declared rings are (D-89).
            .map(|column| {
                let p = column.position;
                if p == self.at.ring {
                    tidy(at(p))
                } else {
                    at(p).to_string()
                }
            })
            .collect();
        self.items.push(Item {
            line: record.line,
            name: tidy(name),
            name_key,
            sector,
            ring,
            qualifier,
            status,
            link,
            cells,
        });
    }

    /// FR-28: every later occurrence of a name warns, naming the first.
    fn name(&mut self, name: &str, line: usize, warn: &mut impl FnMut(usize, RowKind)) -> String {
        let key = normalise(name);
        match self.names.iter().find(|(k, _, _)| *k == key) {
            Some((_, first_line, first_value)) => warn(
                self.at.name,
                RowKind::DuplicateName {
                    value: name.to_string(),
                    first_line: *first_line,
                    first_value: first_value.clone(),
                },
            ),
            None => self.names.push((key.clone(), line, name.to_string())),
        }
        key
    }

    /// FR-8, FR-9: the row's sector, added on first sight. The label is
    /// the ordinally first variant, tidied, whatever order rows come in, and
    /// `line` is kept with it while it is the first to write that label.
    fn sector(&mut self, value: &str, line: usize) -> usize {
        let key = normalise(value);
        let label = tidy(value);
        if let Some((i, q)) = self
            .sectors
            .iter_mut()
            .enumerate()
            .find(|(_, q)| q.key == key)
        {
            // D-53: "ordinally" is code-point order, which is `str`'s `<`.
            // `order::compare_names` is the same rule; this site
            // cannot call it, since `order.rs` depends on this module.
            if label < q.label {
                q.label = label;
                q.line = line;
            }
            return i;
        }
        self.sectors.push(Sector { key, label, line });
        self.sectors.len() - 1
    }

    /// FR-16, FR-17: the declared ring the value matches, or a warning
    /// naming where the ring set came from (D-61), unless FR-107 names the
    /// value as a ring kept off the radar.
    fn ring(&self, value: &str, warn: &mut impl FnMut(usize, RowKind)) -> Option<usize> {
        match self.rings.classify(value) {
            RingMatch::Declared(i) => Some(i),
            RingMatch::Unplotted => None,
            RingMatch::Unknown => {
                warn(
                    self.at.ring,
                    RowKind::UndeclaredRing {
                        value: value.to_string(),
                        origin: self.rings.origin().clone(),
                        rings: Arc::clone(&self.declared),
                        off_radar: self.off_radar,
                        suggest_unplotted: !self.rings.names_unplotted(),
                    },
                );
                None
            }
        }
    }
}

/// `record` with its label columns on one line (FR-97); prose keeps its
/// newlines, which parsing has already made plain. Borrowed unchanged in the
/// usual case of no break in any label.
fn single_lined<'r>(record: &'r Record, at: &Positions) -> Cow<'r, Record> {
    if at
        .labels()
        .all(|p| !field(record, p).contains(['\r', '\n']))
    {
        return Cow::Borrowed(record);
    }
    let mut record = record.clone();
    for p in at.labels() {
        if let Some(cell) = record.fields.get_mut(p)
            && cell.contains(['\r', '\n'])
        {
            *cell = single_line(cell);
        }
    }
    Cow::Owned(record)
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

/// FR-96: blank is absent; a value that is not an `http` or `https`
/// address is shown as text, with a warning (D-77).
fn read_link(value: &str, column: usize, warn: &mut impl FnMut(usize, RowKind)) -> Option<Link> {
    let link = Link::of(value);
    if let Some(Link::Text(_)) = link {
        warn(
            column,
            RowKind::RefusedLink {
                value: value.to_string(),
            },
        );
    }
    link
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
                column: column_label(at),
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::csv::parse::parse;
    use crate::rings::default_rings;

    const HEAD: &str = "name,sector,ring,description\n";
    const FULL: &str = "name,sector,ring,description,qualifier,last_affirmed,exit_date,status\n";

    /// `csv` through `parse` and `build_catalogue` under the default rings,
    /// with the rendered warnings. Parse must succeed with no error.
    fn build(csv: &str) -> (Result<Catalogue, Vec<RowError>>, Vec<String>) {
        build_with(csv, None)
    }

    fn build_with(
        csv: &str,
        rings: Option<&str>,
    ) -> (Result<Catalogue, Vec<RowError>>, Vec<String>) {
        let mut w = Warnings::default();
        let (table, errs) = parse(csv, &mut w).expect("parses");
        assert!(errs.is_empty(), "{errs:?}");
        let rings = crate::rings::flag_or_default(rings);
        let result = build_catalogue(&table, &rings, OffRadar::Listed, &mut w);
        (result, w.render())
    }

    fn catalogue(csv: &str) -> Catalogue {
        build(csv).0.expect("builds")
    }

    /// rg-c4l: a line break in a cell (Excel's Alt+Enter) is one space in a
    /// single-line column and a plain newline in prose, so labels never carry
    /// a carriage return and the panel can still show paragraphs.
    #[test]
    fn line_breaks_are_spaces_in_names_and_newlines_in_prose() {
        let csv = "name,sector,ring,description,qualifier,status,note\n\
                   \"Kafka\r\n\r\nStreams\",\"Tool\rkit\",Adopt,\"first\r\nsecond\rthird\",\"default\r\n\",\"moved\nin\",\"a\r\nb\"\n";
        let cat = catalogue(csv);
        let item = cat.items.first().expect("one item");
        assert_eq!(item.name, "Kafka Streams", "a run of breaks is one space");
        assert_eq!(cat.sectors.first().expect("a sector").label, "Tool kit");
        assert_eq!(
            item.qualifier,
            Some(Qualifier::Default),
            "a trailing break is trimmed away"
        );
        assert_eq!(
            item.status,
            Some(Status::MovedIn),
            "matched after the break became a space"
        );
        let description = cat.column(ColumnKind::Description).expect("description");
        assert_eq!(item.cell(description), Some("first\nsecond\nthird"));
        let note = cat
            .columns
            .iter()
            .position(|c| c.heading == "note")
            .expect("note");
        assert_eq!(item.cell(note), Some("a\nb"));
        for cell in &item.cells {
            assert!(!cell.contains('\r'), "{cell:?}");
        }
    }

    /// FR-97 before FR-28 and FR-9: a name or sector with a break matches
    /// its spaced twin.
    #[test]
    fn a_break_matches_its_spaced_twin() {
        let csv = format!(
            "{}\"Kafka\r\nStreams\",Tool,Adopt,d\nKafka Streams,\"Tool\nkit\",Adopt,d\nFlink,Tool kit,Adopt,d\n",
            "name,sector,ring,description\n"
        );
        let (built, warnings) = build(&csv);
        let cat = built.expect("builds");
        assert_eq!(cat.sectors.len(), 2, "Tool, and Tool kit grouped");
        assert!(
            warnings.iter().any(|w| w.contains("repeats line 2")),
            "a duplicate across the break: {warnings:?}"
        );
    }

    fn errors(csv: &str) -> Vec<String> {
        build(csv)
            .0
            .expect_err("fails")
            .iter()
            .map(RowError::render)
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

    fn missing(column: ColumnKind) -> String {
        RowError::MissingColumn { line: 1, column }.render()
    }

    fn empty(line: usize, column: ColumnKind) -> String {
        RowError::EmptyRequired { line, column }.render()
    }

    // --- The header: FR-5, FR-27, FR-78, FR-93 ---

    /// FR-108's warnings `csv`'s header raises, as (column, heading, known).
    fn near_misses(csv: &str) -> Vec<(usize, String, ColumnKind)> {
        let mut w = Warnings::default();
        let (table, _) = parse(csv, &mut w).expect("parses");
        let rings = default_rings();
        let _ = build_catalogue(&table, &rings, OffRadar::Listed, &mut w);
        w.as_slice()
            .iter()
            .filter_map(|warning| match warning {
                Warning::Row {
                    line: 1,
                    column,
                    kind: RowKind::NearMiss { heading, known },
                } => Some((*column, heading.clone(), *known)),
                _ => None,
            })
            .collect()
    }

    /// FR-108: each near-miss header warns at line 1, its column, matched
    /// under FR-76; known and unrelated columns, and FR-93's unnamed one,
    /// stay silent.
    #[test]
    fn near_miss_headers_warn_at_their_column() {
        let found = near_misses(
            "name,sector,ring,description,RATIONALLE,owner,,Last Affirmed,link\nK,T,Adopt,d,r,o,,2026-01-01,\n",
        );
        assert_eq!(
            found,
            [
                (4, "RATIONALLE".to_string(), ColumnKind::Rationale),
                (7, "Last Affirmed".to_string(), ColumnKind::LastAffirmed),
            ]
        );
    }

    /// FR-108 beside FR-5: a misspelt required column fails the run, and the
    /// warning says which header was meant.
    #[test]
    fn a_misspelt_required_column_warns_beside_its_error() {
        let csv = "name,sector,ring,descripton\nK,T,Adopt,d\n";
        assert_eq!(errors(csv), [missing(ColumnKind::Description)]);
        assert_eq!(
            near_misses(csv),
            [(3, "descripton".to_string(), ColumnKind::Description)]
        );
    }

    /// FR-108 beside FR-4: a repeated header is already an error, so only
    /// its first occurrence warns; a known column the header has is no
    /// candidate.
    #[test]
    fn a_repeated_or_taken_near_miss_warns_once_or_not_at_all() {
        assert_eq!(
            near_misses("name,sector,ring,description,rationalle,Rationalle\nK,T,Adopt,d,r,r\n"),
            [(4, "rationalle".to_string(), ColumnKind::Rationale)]
        );
        assert_eq!(
            near_misses("name,sector,ring,description,rationale,rationalle\nK,T,Adopt,d,r,r\n"),
            []
        );
    }

    /// FR-78's columns in header order; FR-93 drops the unnamed one.
    #[test]
    fn columns_keep_header_order_and_drop_unnamed_ones() {
        let cat = catalogue("name,sector,,ring,description, Notes ,\nK,T,,Adopt,d,n,\n");
        let headings: Vec<_> = cat.columns.iter().map(|c| c.heading.as_str()).collect();
        assert_eq!(
            headings,
            ["name", "sector", "ring", "description", " Notes "]
        );
        assert_eq!(
            cat.columns
                .get(4)
                .expect("the header names five columns")
                .kind,
            ColumnKind::Other
        );
        let [item] = cat.items.as_slice() else {
            panic!("expected one item: {:#?}", cat.items)
        };
        assert_eq!(item.cells, ["K", "T", "Adopt", "d", "n"]);
    }

    /// FR-76 (D-61): case, spaces and invisible format characters do not
    /// hide a required column — `rg-lme`'s case.
    #[test]
    fn required_columns_match_under_fr76() {
        let cat = catalogue("\u{feff}Name\u{200b}, SECTOR ,Ring,description\nK,T,Adopt,d\n");
        let kinds: Vec<_> = cat.columns.iter().map(|c| c.kind).collect();
        assert_eq!(
            kinds,
            [
                ColumnKind::Name,
                ColumnKind::Sector,
                ColumnKind::Ring,
                ColumnKind::Description
            ]
        );
        // The heading is displayed text, so it stays as written (FR-76).
        let headings: Vec<_> = cat.columns.iter().map(|c| c.heading.as_str()).collect();
        assert_eq!(
            headings,
            ["\u{feff}Name\u{200b}", " SECTOR ", "Ring", "description"]
        );
    }

    /// FR-5, NFR-16: every missing column at once, in FR-5's order.
    #[test]
    fn every_missing_required_column_is_reported() {
        assert_eq!(
            errors("ring,name\nAdopt,K\n"),
            [
                missing(ColumnKind::Sector),
                missing(ColumnKind::Description)
            ]
        );
    }

    /// FR-5's line is the header's, which blank lines above it move (FR-3).
    #[test]
    fn a_missing_column_names_the_header_line() {
        assert_eq!(
            errors("\n,,\nname,sector,ring\n"),
            [RowError::MissingColumn {
                line: 3,
                column: ColumnKind::Description
            }
            .render()]
        );
    }

    /// FR-27: an unrecognised column is no error and no warning.
    #[test]
    fn an_unrecognised_column_is_kept_silently() {
        let csv = "name,sector,ring,description,ticket\nK,T,Adopt,d,OPS-1\n";
        let quiet = warnings(csv);
        assert!(quiet.is_empty(), "{quiet:?}");
        let cat = catalogue(csv);
        let [item] = cat.items.as_slice() else {
            panic!("expected one item: {:#?}", cat.items)
        };
        assert_eq!(item.cell(4), Some("OPS-1"));
    }

    // --- FR-6 ---

    /// FR-6 under FR-76: spaces or a lone zero-width space are empty.
    /// Every one is reported, in line order then column order (NFR-16).
    #[test]
    fn every_empty_required_value_is_reported() {
        let csv = format!("{HEAD} ,T,Adopt,d\nK,\u{200b},  ,d\nK,T,Adopt,d\n");
        assert_eq!(
            errors(&csv),
            [
                empty(2, ColumnKind::Name),
                empty(3, ColumnKind::Sector),
                empty(3, ColumnKind::Ring)
            ]
        );
    }

    /// NFR-11: within a line, errors follow the header's column order, not
    /// FR-6's list.
    #[test]
    fn empty_required_values_follow_header_column_order() {
        assert_eq!(
            errors("ring,name,sector,description\n ,,T,d\n"),
            [empty(2, ColumnKind::Ring), empty(2, ColumnKind::Name)]
        );
    }

    /// SC-10: FR-90's removal runs first, so a `name` of only U+0001 is
    /// the empty-`name` hard error.
    #[test]
    fn a_name_of_only_undisplayable_characters_is_empty() {
        assert_eq!(
            errors(&format!("{HEAD}\u{1},T,Adopt,d\n")),
            [empty(2, ColumnKind::Name)]
        );
    }

    /// NFR-16: FR-5 and FR-6 arrive in one run. The present required
    /// columns are still checked.
    #[test]
    fn missing_columns_and_empty_values_arrive_together() {
        assert_eq!(
            errors("name,sector,ring\n,T,Adopt\n"),
            [missing(ColumnKind::Description), empty(2, ColumnKind::Name)]
        );
    }

    /// `rg-rnc`: a cleared row (`,,,`) between data rows is skipped, but
    /// its line still counts (FR-3), so later lines stay the editor's.
    #[test]
    fn a_cleared_row_does_not_shift_line_numbers() {
        let csv = format!("{HEAD}K,T,Adopt,d\n,,,\n,T,Adopt,d\n");
        assert_eq!(errors(&csv), [empty(4, ColumnKind::Name)]);
    }

    /// FR-3's blank test skips format characters FR-90 keeps, but not
    /// whitespace FR-90 removes (D-59): a row holding only a vertical tab
    /// is a row, and FR-6 reports it.
    #[test]
    fn a_row_of_only_removed_whitespace_is_not_blank() {
        assert_eq!(
            errors(&format!("{HEAD}\u{b},,,\n")),
            [
                empty(2, ColumnKind::Name),
                empty(2, ColumnKind::Sector),
                empty(2, ColumnKind::Ring)
            ]
        );
    }

    /// `Table` and `Record` are public, so a hand-built short record reads
    /// as empty cells, which FR-6 reports, rather than a panic.
    #[test]
    fn a_short_record_reads_as_empty_not_a_panic() {
        let table = Table {
            header: ["name", "sector", "ring", "description"]
                .map(String::from)
                .to_vec(),
            header_line: 1,
            records: vec![Record {
                line: 2,
                fields: vec!["K".into()],
            }],
        };
        let rings = default_rings();
        let errs = build_catalogue(&table, &rings, OffRadar::Listed, &mut Warnings::default())
            .expect_err("fails");
        assert_eq!(
            errs.iter().map(RowError::render).collect::<Vec<_>>(),
            [empty(2, ColumnKind::Sector), empty(2, ColumnKind::Ring)]
        );
    }

    /// No row warning from the model when it fails; `parse`'s FR-90 warnings
    /// are already pushed and stay. FR-108's header warnings are the one
    /// exception: they are pushed before the check, and this header has
    /// none.
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
        assert!(cat.items.is_empty() && cat.sectors.is_empty());
        assert_eq!((cat.columns.len(), cat.rings.len()), (4, 4));
        assert!(warnings.is_empty(), "{warnings:?}");
    }

    // --- Sectors, rings and names: FR-8, FR-9, FR-11, FR-16, FR-17, FR-28 ---

    /// FR-9: one sector per normalised value, labelled by the ordinally
    /// first variant, trimmed — the same label whatever the row order —
    /// and pointing at the first row that wrote it (FR-104).
    #[test]
    fn sectors_group_and_take_the_ordinally_first_label() {
        for (rows, line) in [
            (
                "a,tool ,Adopt,d\nb,Tool,Adopt,d\nc,TOOL,Adopt,d\nd,TOOL,Adopt,d\n",
                4,
            ),
            ("c,TOOL,Adopt,d\nb,Tool,Adopt,d\na,tool ,Adopt,d\n", 2),
        ] {
            let cat = catalogue(&format!("{HEAD}{rows}"));
            assert_eq!(
                cat.sectors,
                [Sector {
                    key: "tool".into(),
                    label: "TOOL".into(),
                    line,
                }]
            );
            assert!(cat.items.iter().all(|i| i.sector == 0));
        }
    }

    /// FR-9 compares the variants **trimmed**: untrimmed, ` tool` would
    /// sort first (a space is below `T`). A boundary format character goes
    /// too (D-61).
    #[test]
    fn sector_labels_compare_trimmed_variants() {
        let cat = catalogue(&format!(
            "{HEAD}a, tool,Adopt,d\nb,Tool,Adopt,d\nc,\u{200b}Tool,Adopt,d\n"
        ));
        assert_eq!(
            cat.sectors
                .first()
                .expect("the catalogue has a sector")
                .label,
            "Tool"
        );
    }

    /// rg-cuj: a doubled space inside a sector or a name is one space, so
    /// `Tool  Kit` and `Tool Kit` are one sector labelled as a wrapped
    /// heading reads it, and the name shows as its wrapped row reads.
    #[test]
    fn internal_whitespace_runs_collapse_in_labels_and_names() {
        let cat = catalogue(&format!(
            "{HEAD}Kafka  Streams,Tool  Kit,Adopt,d\nb,Tool Kit,Adopt,d\n"
        ));
        assert_eq!(
            cat.sectors,
            [Sector {
                key: "tool kit".into(),
                label: "Tool Kit".into(),
                line: 2,
            }]
        );
        assert_eq!(cat.items.first().expect("an item").name, "Kafka Streams");
        // FR-28 matches the tidied name, so the doubled space is a duplicate.
        let (_, warned) = build(&format!(
            "{HEAD}Kafka  Streams,T,Adopt,d\nKafka Streams,T,Adopt,d\n"
        ));
        assert!(
            warned.iter().any(|w| w.contains("repeats line 2's name")),
            "{warned:?}"
        );
        // A row's ring matches a declared ring however either is spaced.
        let (cat, warned) = build_with(&format!("{HEAD}a,T,early adopt,d\n"), Some("Early  Adopt"));
        let cat = cat.expect("builds");
        assert_eq!(cat.rings, ["Early Adopt"]);
        assert_eq!(
            cat.items.first().expect("an item").ring,
            Some(0),
            "{warned:?}"
        );
    }

    /// FR-8: sectors in order of first appearance; FR-10 orders them later.
    #[test]
    fn sectors_keep_first_appearance_order() {
        let cat = catalogue(&format!(
            "{HEAD}a,Tool,Adopt,d\nb,Platform,Adopt,d\nc,tool,Adopt,d\n"
        ));
        let labels: Vec<_> = cat.sectors.iter().map(|q| q.label.as_str()).collect();
        assert_eq!(labels, ["Tool", "Platform"]);
        let indices: Vec<_> = cat.items.iter().map(|i| i.sector).collect();
        assert_eq!(indices, [0, 1, 0]);
    }

    /// FR-11: near-duplicates are neither merged nor flagged.
    #[test]
    fn near_duplicates_stay_apart_silently() {
        let csv = format!("{HEAD}Kafka,Tool,Adopt,d\nKafkas,Tools,Adopt,d\n");
        let (result, warnings) = build(&csv);
        assert_eq!(result.expect("builds").sectors.len(), 2);
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
        assert_eq!(
            result
                .expect("builds")
                .items
                .first()
                .expect("items[0] exists")
                .ring,
            None
        );
        assert_eq!(
            warnings,
            [
                "warning: line 2, ring: 'In Use' is not one of the default rings (Assess, Trial, Adopt, Retire); the row is in the table view, not on the radar; to plot it, pass --rings with every ring to draw (it replaces the defaults); to leave it off without this warning, add it to --unplotted-rings"
            ]
        );
        let (_, warnings) = build_with(&csv, Some("Adopt,Hold"));
        assert_eq!(
            warnings,
            [
                "warning: line 2, ring: 'In Use' is not in --rings (Adopt, Hold); the row is in the table view, not on the radar; to plot it, add it to --rings; to leave it off without this warning, add it to --unplotted-rings"
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
        let rings = default_rings();
        let cat = build_catalogue(&table, &rings, OffRadar::Listed, &mut w).expect("builds");
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
                "warning: line 3, name: 'Kafka' repeats line 2's name 'Kafka' (names match ignoring case and extra spaces); both are drawn, so rename or delete one if they are the same item",
            ]
        );
        assert_eq!(
            warnings(&format!("{HEAD}Kafka,T,Adopt,d\n\u{200b}Kafka,T,Adopt,d\n")),
            [
                "warning: line 3, name: 'Kafka' repeats line 2's name 'Kafka' (names match ignoring case and extra spaces); both are drawn, so rename or delete one if they are the same item"
            ]
        );
    }

    /// The name is kept tidied, as it is shown; its key is FR-76's,
    /// computed once.
    #[test]
    fn names_keep_their_text_and_carry_a_key() {
        let cat = catalogue(&format!("{HEAD} Kafka ,T,Adopt,d\n"));
        let [item] = cat.items.as_slice() else {
            panic!("expected one item: {:#?}", cat.items)
        };
        assert_eq!(
            (item.name.as_str(), item.name_key.as_str()),
            ("Kafka", "kafka")
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
        let [first, second, ..] = items.as_slice() else {
            panic!("expected at least 2 items: {items:#?}")
        };
        assert_eq!(first.cell(5), Some(" 2026-06-01 "));
        assert_eq!(second.cell(5), Some("\u{200b}2026-06-01\u{feff}"));
        assert_eq!(
            warnings,
            [
                "warning: line 2, exit_date: 'June 2027' is not a YYYY-MM-DD date; shown as written",
                "warning: line 5, last_affirmed: '2026-6-1' is not a YYYY-MM-DD date; shown as written",
            ]
        );
    }

    /// FR-29: an absent optional column claims nothing and warns nothing.
    #[test]
    fn absent_optional_columns_claim_nothing() {
        let csv = format!("{HEAD}K,T,Adopt,d\n");
        let (result, warnings) = build(&csv);
        let cat = result.expect("builds");
        let [item] = cat.items.as_slice() else {
            panic!("expected one item: {:#?}", cat.items)
        };
        assert_eq!((item.qualifier, item.status), (None, None));
        assert!(warnings.is_empty(), "{warnings:?}");
    }

    /// FR-29: an empty cell is omitted, not shown as an empty label.
    #[test]
    fn an_empty_cell_is_omitted() {
        let cat = catalogue(&format!("{FULL}K,T,Adopt,d,,  ,,\n"));
        let [item] = cat.items.as_slice() else {
            panic!("expected one item: {:#?}", cat.items)
        };
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
        let cat =
            catalogue("name,,sector,ring,description,exit_date\nOld,,Tool,Rejected,d,2027-01-01\n");
        let [item] = cat.items.as_slice() else {
            panic!("expected one item: {:#?}", cat.items)
        };
        assert_eq!(item.cell(4), Some("2027-01-01"));
    }

    /// FR-93: Excel's trailing padding passes silently, and so does a cell
    /// of only a format character (FR-76); a value under no name warns,
    /// naming the column's position — wherever the column is, and whether
    /// the header cell is empty or only a format character.
    #[test]
    fn unnamed_columns_warn_only_when_they_hold_a_value() {
        let padding = warnings("name,sector,ring,description,,\nK,T,Adopt,d,,\n");
        assert!(padding.is_empty(), "{padding:?}");
        let invisible = warnings("name,sector,ring,description,\nK,T,Adopt,d,\u{200b}\n");
        assert!(invisible.is_empty(), "{invisible:?}");
        assert_eq!(
            warnings("name,sector,ring,description,,\nK,T,Adopt,d,, \nL,T,Adopt,d,,x\n"),
            [
                "warning: line 3, column 6 (F): this column has no name in the header row, so its value is not shown; add a header name, or delete the value"
            ]
        );
        assert_eq!(
            warnings("name,\u{200b},sector,ring,description\nK,x,T,Adopt,d\n"),
            [
                "warning: line 2, column 2 (B): this column has no name in the header row, so its value is not shown; add a header name, or delete the value"
            ]
        );
    }

    /// `rg-rnc`: FR-90 still warns for a cell in a column FR-93 drops, and
    /// FR-93 warns as well, since the cell still holds a value.
    #[test]
    fn a_dropped_column_still_gets_fr90s_warning() {
        assert_eq!(
            warnings("name,sector,ring,description,\nK,T,Adopt,d,a\u{1}\n"),
            [
                "warning: line 2, column 5 (E): removed U+0001, which the output cannot display",
                "warning: line 2, column 5 (E): this column has no name in the header row, so its value is not shown; add a header name, or delete the value",
            ]
        );
    }

    /// NFR-11 across the two stages: parse's and the model's warnings on
    /// one line sort by header column, whichever stage pushed first.
    #[test]
    fn warnings_from_both_stages_sort_by_header_column() {
        let csv = "name,,sector,ring,description,\nK,x,T,Adpot,d\u{1},\nL,,T,Adopt,,\n";
        assert_eq!(
            warnings(csv),
            [
                "warning: line 2, column 2 (B): this column has no name in the header row, so its value is not shown; add a header name, or delete the value",
                "warning: line 2, ring: 'Adpot' is not one of the default rings (Assess, Trial, Adopt, Retire); the row is in the table view, not on the radar; to plot it, pass --rings with every ring to draw (it replaces the defaults); to leave it off without this warning, add it to --unplotted-rings",
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
            warnings("name,sector,,,ring,description\nK,T,,\u{1},Adpot,d\n"),
            [
                "warning: line 2, column 4 (D): removed U+0001, which the output cannot display",
                "warning: line 2, ring: 'Adpot' is not one of the default rings (Assess, Trial, Adopt, Retire); the row is in the table view, not on the radar; to plot it, pass --rings with every ring to draw (it replaces the defaults); to leave it off without this warning, add it to --unplotted-rings",
            ]
        );
    }

    // --- FR-50, FR-75 ---

    /// The legend row's text first — number, name, qualifier, status —
    /// then the sector and ring, named as such; an unplotted row says so
    /// instead of a number (FR-18).
    #[test]
    fn accessible_names_carry_the_row_text_and_its_position() {
        let cat = catalogue(&format!(
            "{FULL} Kafka ,tool,adopt,d,conditional,,,moved in\nOld,Tool,Rejected,d,,,,\n"
        ));
        let [kafka, old] = cat.items.as_slice() else {
            panic!("expected two items: {:#?}", cat.items)
        };
        assert_eq!(
            cat.accessible_name(kafka, Some(3)),
            "3. Kafka, Conditional, Moved In; Tool sector, Adopt ring"
        );
        assert_eq!(
            cat.accessible_name(old, None),
            "Old; Tool sector, Rejected ring; not on the radar"
        );
    }

    /// `No Change` is FR-40's plain marker and no row shows it, so the name
    /// leaves it out (FR-50); `New` is shown and stays.
    #[test]
    fn accessible_name_omits_no_change_and_keeps_new() {
        let cat = catalogue(&format!(
            "{FULL}Kept,Tool,adopt,d,,,,new
Plain,Tool,adopt,d,,,,no change\n"
        ));
        let [kept, plain] = cat.items.as_slice() else {
            panic!("expected two items: {:#?}", cat.items)
        };
        assert_eq!(plain.status, Some(Status::NoChange));
        assert_eq!(
            cat.accessible_name(kept, Some(1)),
            "1. Kept, New; Tool sector, Adopt ring"
        );
        assert_eq!(
            cat.accessible_name(plain, Some(2)),
            "2. Plain; Tool sector, Adopt ring"
        );
    }

    /// FR-104 points at a cell by its header position, which an unnamed
    /// column puts ahead of its index among the named columns.
    #[test]
    fn a_column_keeps_its_header_position() {
        let cat = catalogue("name,,sector,ring,description\nOld,,Tool,Adopt,d\n");
        let positions: Vec<usize> = cat.columns.iter().map(|c| c.position).collect();
        assert_eq!(positions, [0, 2, 3, 4]);
        assert_eq!(cat.position(ColumnKind::Sector), Some(2));
        assert_eq!(cat.position(ColumnKind::Status), None);
    }

    /// The unplotted ring is read from the named columns, so an unnamed
    /// column before it does not shift it onto another cell.
    #[test]
    fn an_unplotted_ring_is_found_past_an_unnamed_column() {
        let cat = catalogue("name,,sector,ring,description\nOld,,Tool,Rejected,d\n");
        let [item] = cat.items.as_slice() else {
            panic!("expected one item: {:#?}", cat.items)
        };
        assert_eq!(
            cat.accessible_name(item, None),
            "Old; Tool sector, Rejected ring; not on the radar"
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
                position: 0,
            }],
            sectors: vec![Sector {
                key: "tool".into(),
                label: "Tool".into(),
                line: 2,
            }],
            rings: Vec::new(),
            ring_origin: Origin::rings(crate::settings::Source::Default),
            items: vec![Item {
                line: 2,
                name: "Old".into(),
                name_key: "old".into(),
                sector: 0,
                ring: None,
                qualifier: None,
                status: None,
                link: None,
                cells: vec!["Old".into()],
            }],
        };
        let [item] = cat.items.as_slice() else {
            panic!("expected one item: {:#?}", cat.items)
        };
        assert_eq!(
            cat.accessible_name(item, None),
            "Old; Tool sector; not on the radar"
        );
    }

    /// FR-18: only a plotted item has a number, and a caller passing one
    /// for an unplotted item is a bug caught in debug builds.
    #[test]
    #[cfg(debug_assertions)]
    #[should_panic(expected = "FR-18")]
    fn a_number_for_an_unplotted_item_is_a_bug() {
        let cat = catalogue(&format!("{HEAD}Old,Tool,Rejected,d\n"));
        let [item] = cat.items.as_slice() else {
            panic!("expected one item: {:#?}", cat.items)
        };
        let _ = cat.accessible_name(item, Some(1));
    }
}
