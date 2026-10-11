//! Records to meaning (architecture §2): which column is which, what each
//! row claims, and what is wrong with it (FR-5–FR-29, FR-93).
//!
//! Ordering and numbering are not here: FR-10 and FR-34 are `order.rs`'s
//! (PR 07), so a `Catalogue` keeps quadrants and items in CSV order.

use crate::csv::parse::{Record, Table};
use crate::diagnostics::{Fatal, RowKind, Warning, Warnings};
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
    /// The kind a header name under FR-76 names.
    fn of(key: &str) -> Self {
        match key {
            "name" => Self::Name,
            "quadrant" => Self::Quadrant,
            "ring" => Self::Ring,
            "description" => Self::Description,
            "qualifier" => Self::Qualifier,
            "domain" => Self::Domain,
            "rationale" => Self::Rationale,
            "last_affirmed" => Self::LastAffirmed,
            "exit_date" => Self::ExitDate,
            "status" => Self::Status,
            _ => Self::Other,
        }
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
    /// Every named column's cell, aligned with [`Catalogue::columns`] (not
    /// with the header, which may hold unnamed columns), as written after
    /// FR-90's removal (FR-78).
    pub cells: Vec<String>,
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
}

impl Header {
    fn of(names: &[String]) -> Self {
        let mut slots = Vec::with_capacity(names.len());
        let mut columns = Vec::new();
        for name in names {
            let key = normalise(name);
            if key.is_empty() {
                slots.push(None);
                continue;
            }
            slots.push(Some(columns.len()));
            columns.push(Column {
                heading: name.clone(),
                kind: ColumnKind::of(&key),
            });
        }
        Self { slots, columns }
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
        // Built from the header, not zipped with the record, so a short
        // hand-built record still gives one cell per named column.
        let cells = self
            .header
            .slots
            .iter()
            .enumerate()
            .filter(|(_, slot)| slot.is_some())
            .map(|(p, _)| at(p).to_string())
            .collect();
        self.items.push(Item {
            line: record.line,
            name: name.to_string(),
            name_key,
            quadrant,
            ring,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::csv::parse::parse;
    use crate::rings::parse_rings;

    const HEAD: &str = "name,quadrant,ring,description\n";

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
}
