//! What the table publishes (FR-78, FR-106): its rows, columns, headings
//! and cells, decided once for every output that draws it.
//!
//! The HTML page's table and the full PDF's (FR-112) both render this, so
//! the two outputs of one edition cannot disagree about what was published.

use crate::model::{Catalogue, ColumnKind, Item, Link};
use crate::options::Table;
use crate::order::Plotted;
use crate::trim;

/// The default table's heading, which names its region too (FR-106).
pub const RADAR_HEADING: &str = "Items on the radar";
/// `--table all`'s heading, as it was before FR-106.
pub const ALL_HEADING: &str = "Every item";
/// D-101's heading for the sector column, whatever the header calls it.
pub const SECTOR_HEADING: &str = "Sector";
/// The default table's one row when nothing is plotted (FR-106).
pub const NOTHING_PLOTTED: &str = "Nothing is on the radar.";
/// The number column's heading, as the eye reads it.
pub const NUMBER_HEADING: &str = "#";
/// The number column's heading as a screen reader says it, which may
/// read a bare `#` as "pound" or skip it.
pub const NUMBER_SPOKEN: &str = "Number";

/// How a column's cells are written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CellKind {
    /// The item's name: the row's header.
    Name,
    /// A short label, trimmed (D-76).
    Label,
    /// Prose, verbatim, its line breaks kept (FR-97).
    Prose,
    /// FR-96's link: an address, or refused text.
    Link,
}

/// How a column of `kind` is written: sector, ring, qualifier, status
/// and the dates are labels; description, domain, rationale and any
/// column radargen does not read are prose.
#[must_use]
pub const fn kind(column: ColumnKind) -> CellKind {
    match column {
        ColumnKind::Name => CellKind::Name,
        ColumnKind::Link => CellKind::Link,
        ColumnKind::Sector
        | ColumnKind::Ring
        | ColumnKind::Qualifier
        | ColumnKind::Status
        | ColumnKind::LastAffirmed
        | ColumnKind::ExitDate => CellKind::Label,
        ColumnKind::Description
        | ColumnKind::Domain
        | ColumnKind::Rationale
        | ColumnKind::Other => CellKind::Prose,
    }
}

/// One cell, as published.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Cell<'a> {
    /// The row header: the name, flagged when the item is not on the
    /// radar (FR-41).
    Name {
        /// The item's name.
        name: &'a str,
        /// Whether FR-18 gives it no number.
        unplotted: bool,
    },
    /// A label: the legend's sector or ring label (FR-9, FR-17), or the
    /// cell trimmed (D-76).
    Label(&'a str),
    /// Prose, verbatim (FR-97, D-76).
    Prose(&'a str),
    /// What the model decided of the cell's link (FR-96).
    Link(Option<&'a Link>),
}

impl<'a> Cell<'a> {
    /// The cell's paragraphs: prose split at its line breaks, blank lines
    /// kept as empty paragraphs; any other cell is one. An empty cell is
    /// one empty paragraph.
    #[must_use]
    pub fn paragraphs(&self) -> Vec<&'a str> {
        match *self {
            Self::Prose(text) => text.split('\n').collect(),
            Self::Name { name, .. } => vec![name],
            Self::Label(text) => vec![text],
            Self::Link(Some(Link::Address(text) | Link::Text(text))) => vec![text.as_str()],
            Self::Link(None) => vec![""],
        }
    }
}

/// One published row, keyed on its CSV line as its marker and legend row
/// are (architecture §5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublishedRow<'a> {
    /// The CSV line the record begins on (NFR-10).
    pub line: usize,
    /// FR-18's number; `None` for an unplotted row.
    pub number: Option<usize>,
    /// The sector label it sits under, for a page heading or a rule.
    pub sector: &'a str,
    /// One cell per published column, in [`Published::columns`]' order.
    pub cells: Vec<Cell<'a>>,
}

/// The table as published.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Published<'a> {
    /// The heading that names it (FR-106).
    pub heading: &'static str,
    /// Each column's kind and heading, unescaped, the number column
    /// aside. The kinds are here as well as on each cell because a layout
    /// sizes the columns before it reads the rows.
    pub columns: Vec<(CellKind, &'a str)>,
    /// The rows, in FR-34's order, then (under `all`) the unplotted ones
    /// in CSV order (FR-78).
    pub rows: Vec<PublishedRow<'a>>,
    /// Whether the table says [`NOTHING_PLOTTED`]: under `radar`, with no
    /// rows (FR-106).
    pub says_empty: bool,
}

/// The table `publish` sets for `catalogue` in `plotted`'s order.
#[must_use]
pub fn published<'a>(catalogue: &'a Catalogue, plotted: &Plotted, publish: Table) -> Published<'a> {
    let columns = order(catalogue, publish);
    let (rows, heading) = match publish {
        Table::Radar => (plotted.items.clone(), RADAR_HEADING),
        Table::All => (plotted.table_rows(), ALL_HEADING),
    };
    let rows: Vec<PublishedRow<'a>> = rows
        .into_iter()
        .filter_map(|i| {
            let item = catalogue.items.get(i)?;
            let number = plotted.numbers.get(i).copied().flatten();
            Some(row(catalogue, &columns, item, number))
        })
        .collect();
    Published {
        heading,
        columns: columns
            .iter()
            .map(|&c| {
                let kind_of = catalogue.columns.get(c).map_or(ColumnKind::Other, |c| c.kind);
                (kind(kind_of), column_heading(catalogue, c))
            })
            .collect(),
        says_empty: publish == Table::Radar && rows.is_empty(),
        rows,
    }
}

/// FR-78's columns, as indices into [`Catalogue::columns`]: `name`,
/// `sector`, `ring`, then every other named column in header order,
/// `description` included (D-80). Under `radar`, a column radargen does not
/// read ([`ColumnKind::Other`]) is left out (FR-106).
fn order(catalogue: &Catalogue, publish: Table) -> Vec<usize> {
    let first: Vec<usize> = [ColumnKind::Name, ColumnKind::Sector, ColumnKind::Ring]
        .into_iter()
        .filter_map(|kind| catalogue.column(kind))
        .collect();
    let published = |c: &usize| {
        catalogue
            .columns
            .get(*c)
            .is_some_and(|column| publish == Table::All || column.kind != ColumnKind::Other)
    };
    let rest = (0..catalogue.columns.len()).filter(|c| !first.contains(c) && published(c));
    first.iter().copied().chain(rest).collect()
}

/// A column's heading: the header cell as written, trimmed (FR-27), except
/// the sector column's, which is always [`SECTOR_HEADING`] so a `quadrant`
/// header does not put the old word back on the page (D-101).
fn column_heading(catalogue: &Catalogue, column: usize) -> &str {
    catalogue.columns.get(column).map_or("", |c| {
        if c.kind == ColumnKind::Sector {
            SECTOR_HEADING
        } else {
            trim(&c.heading)
        }
    })
}

/// One item's row. `number` is FR-18's, `None` for an unplotted row.
fn row<'a>(
    catalogue: &'a Catalogue,
    columns: &[usize],
    item: &'a Item,
    number: Option<usize>,
) -> PublishedRow<'a> {
    PublishedRow {
        line: item.line,
        number,
        sector: catalogue
            .sectors
            .get(item.sector)
            .map_or("", |q| q.label.as_str()),
        cells: columns
            .iter()
            .map(|&c| cell(catalogue, c, item, number.is_none()))
            .collect(),
    }
}

/// One cell of `item` under `column`. Sector and ring show the labels the
/// legend uses (FR-9, FR-17); a link shows what the model decided (FR-96);
/// prose is verbatim (FR-97, D-76: its whitespace may be deliberate); the
/// other labels are trimmed (D-76).
fn cell<'a>(catalogue: &'a Catalogue, column: usize, item: &'a Item, unplotted: bool) -> Cell<'a> {
    let kind = catalogue
        .columns
        .get(column)
        .map_or(ColumnKind::Other, |c| c.kind);
    let value = item.cell(column).unwrap_or_default();
    match kind {
        ColumnKind::Name => Cell::Name {
            name: &item.name,
            unplotted,
        },
        ColumnKind::Sector => Cell::Label(
            catalogue
                .sectors
                .get(item.sector)
                .map_or("", |q| q.label.as_str()),
        ),
        ColumnKind::Ring => Cell::Label(catalogue.ring_label(item)),
        ColumnKind::Link => Cell::Link(item.link.as_ref()),
        ColumnKind::Description
        | ColumnKind::Domain
        | ColumnKind::Rationale
        | ColumnKind::Other => Cell::Prose(value),
        ColumnKind::Qualifier
        | ColumnKind::Status
        | ColumnKind::LastAffirmed
        | ColumnKind::ExitDate => Cell::Label(trim(value)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::test_support::{PRD_EXAMPLE, plotted};
    use crate::model::Link;

    const HOSTILE: &str = include_str!("../tests/fixtures/table-hostile.csv");

    fn headings<'a>(table: &'a Published<'_>) -> Vec<&'a str> {
        table.columns.iter().map(|(_, heading)| *heading).collect()
    }

    fn lines(table: &Published<'_>) -> Vec<usize> {
        table.rows.iter().map(|row| row.line).collect()
    }

    #[test]
    fn radar_publishes_plotted_rows_and_read_columns() {
        let (catalogue, order) = plotted(HOSTILE, None);
        let table = published(&catalogue, &order, Table::Radar);
        assert_eq!(table.heading, RADAR_HEADING);
        assert_eq!(
            headings(&table),
            ["name", "Sector", "ring", "description", "link", "qualifier", "last_affirmed"]
        );
        let expected: Vec<usize> = order
            .items
            .iter()
            .filter_map(|&i| catalogue.items.get(i).map(|item| item.line))
            .collect();
        assert_eq!(lines(&table), expected);
        assert!(table.rows.iter().all(|row| row.number.is_some()));
        assert!(!table.says_empty);
    }

    #[test]
    fn all_adds_unplotted_rows_and_every_named_column() {
        let (catalogue, order) = plotted(HOSTILE, None);
        let table = published(&catalogue, &order, Table::All);
        assert_eq!(table.heading, ALL_HEADING);
        assert_eq!(
            headings(&table),
            [
                "name",
                "Sector",
                "ring",
                "description",
                "link",
                "qualifier",
                "last_affirmed",
                "a<b & \"c\"",
                "notes"
            ]
        );
        let last = table.rows.last().expect("rows");
        assert_eq!(last.number, None);
        assert!(matches!(
            last.cells.first(),
            Some(Cell::Name {
                name: "Gone",
                unplotted: true
            })
        ));
        assert_eq!(last.sector, "Platforms");
    }

    #[test]
    fn a_quadrant_header_reads_sector() {
        let (catalogue, order) = plotted("name,quadrant,ring,description\nA,Tools,Adopt,d\n", None);
        let table = published(&catalogue, &order, Table::Radar);
        assert_eq!(headings(&table).get(1), Some(&SECTOR_HEADING));
    }

    #[test]
    fn prose_is_verbatim_and_labels_are_trimmed() {
        let (catalogue, order) = plotted(HOSTILE, None);
        let table = published(&catalogue, &order, Table::All);
        let padded = table
            .rows
            .iter()
            .find(|row| row.line == 2)
            .expect("line 2");
        assert_eq!(
            padded.cells.get(3),
            Some(&Cell::Prose(
                "First paragraph.\nSecond line.\n\nThird paragraph."
            ))
        );
        assert_eq!(padded.cells.get(6), Some(&Cell::Label("2026-01-02")));
        assert_eq!(padded.cells.get(8), Some(&Cell::Prose("  n1  ")));
        assert!(matches!(
            padded.cells.get(4),
            Some(Cell::Link(Some(Link::Address(_))))
        ));
        let refused = table.rows.iter().find(|row| row.line == 6).expect("line 6");
        assert!(matches!(
            refused.cells.get(4),
            Some(Cell::Link(Some(Link::Text(_))))
        ));
    }

    #[test]
    fn paragraphs_keep_blank_lines() {
        assert_eq!(Cell::Prose("a\n\nb").paragraphs(), ["a", "", "b"]);
        assert_eq!(Cell::Prose("").paragraphs(), [""]);
        assert_eq!(Cell::Label("a\nb").paragraphs(), ["a\nb"]);
        assert_eq!(Cell::Link(None).paragraphs(), [""]);
    }

    #[test]
    fn nothing_plotted_says_so_only_under_radar() {
        let csv = "name,sector,ring,description\nA,Tools,Elsewhere,d\n";
        let (catalogue, order) = plotted(csv, None);
        let radar = published(&catalogue, &order, Table::Radar);
        assert_eq!(radar.rows.len(), 0);
        assert!(radar.says_empty);
        let (catalogue, order) = plotted("name,sector,ring,description\n", None);
        assert!(published(&catalogue, &order, Table::Radar).says_empty);
        let all = published(&catalogue, &order, Table::All);
        assert_eq!(all.rows.len(), 0);
        assert!(!all.says_empty);
    }

    #[test]
    fn only_unplotted_rows_under_all() {
        let csv = "name,sector,ring,description\nA,Tools,Elsewhere,d\nB,Tools,Gone,d\n";
        let (catalogue, order) = plotted(csv, None);
        let all = published(&catalogue, &order, Table::All);
        assert!(!all.says_empty);
        assert_eq!(all.rows.len(), 2);
        assert!(all.rows.iter().all(|row| matches!(
            row.cells.first(),
            Some(Cell::Name {
                unplotted: true,
                ..
            })
        )));
    }

    #[test]
    fn the_headings_are_fr78s() {
        assert_eq!(NUMBER_HEADING, "#");
        assert_eq!(NUMBER_SPOKEN, "Number");
        assert_eq!(ALL_HEADING, "Every item");
        assert_eq!(RADAR_HEADING, "Items on the radar");
        assert_eq!(NOTHING_PLOTTED, "Nothing is on the radar.");
        let (catalogue, order) = plotted(PRD_EXAMPLE, None);
        let kinds: Vec<CellKind> = published(&catalogue, &order, Table::Radar)
            .columns
            .iter()
            .map(|(kind, _)| *kind)
            .collect();
        assert_eq!(
            kinds.get(..4),
            Some(&[CellKind::Name, CellKind::Label, CellKind::Label, CellKind::Prose][..])
        );
    }
}
