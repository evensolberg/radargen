//! `errors::render_errors` (D-62, NFR-11, NFR-16, NFR-20): the merged row-level
//! errors of both stages, grouped and ordered as they print.

use radargen::columns::ColumnKind;
use radargen::csv::parse::parse;
use radargen::diagnostics::{OffRadar, RowError, Warnings};
use radargen::errors::render_errors;
use radargen::model::build_catalogue;
use radargen::rings::default_rings;

/// A mismatched row against a two-column header with no unnamed column.
const fn row(line: usize, found: usize) -> RowError {
    RowError::FieldCount {
        line,
        found,
        expected: 2,
        unnamed: 0,
        padded: false,
    }
}

/// A mismatched row with more values than the header, all of them padding.
const fn padded(line: usize, found: usize) -> RowError {
    RowError::FieldCount {
        line,
        found,
        expected: 2,
        unnamed: 0,
        padded: true,
    }
}

const fn empty(line: usize, column: ColumnKind) -> RowError {
    RowError::EmptyRequired { line, column }
}

const fn missing(line: usize, column: ColumnKind) -> RowError {
    RowError::MissingColumn { line, column }
}

fn repeats(line: usize, index: usize) -> RowError {
    RowError::DuplicateHeader {
        line,
        index,
        name: "a".into(),
        first_index: 0,
        first_name: "a".into(),
    }
}

/// The one line `errors` render to.
fn sole(errors: &[RowError], well_formed_rows: usize) -> String {
    let mut lines = render_errors(errors, well_formed_rows);
    assert_eq!(lines.len(), 1, "{lines:?}");
    lines.pop().unwrap_or_default()
}

const FEWER: &str =
    "a row with fewer values is usually missing one, or an unquoted line break split it";
const MORE: &str =
    "a row with more values usually has a comma that needs the value in double quotes";
const PADDED: &str = "rows with more values end in empty cells: delete them, or give the header matching empty columns";
const HEADER_FIRST: &str = "no row matches the header, so check the header first";

#[test]
fn one_row_reads_as_the_one_row_message() {
    assert_eq!(
        render_errors(&[row(4, 1)], 3),
        [format!(
            "error: line 4: the row has 1 value but the header has 2 columns; {FEWER}"
        )]
    );
}

#[test]
fn one_row_with_no_good_row_adds_the_header_first_cause() {
    assert_eq!(
        render_errors(&[row(4, 3)], 0),
        [format!(
            "error: line 4: the row has 3 values but the header has 2 columns; {HEADER_FIRST}; {MORE}"
        )]
    );
}

#[test]
fn one_row_names_the_unnamed_columns() {
    let one = RowError::FieldCount {
        line: 2,
        found: 1,
        expected: 5,
        unnamed: 1,
        padded: false,
    };
    assert_eq!(
        render_errors(&[one], 1),
        [format!(
            "error: line 2: the row has 1 value but the header has 5 columns, 1 of them unnamed; {FEWER}"
        )]
    );
}

/// NFR-20's example, verbatim up to its closing causes.
#[test]
fn nfr_20_field_count_example() {
    let unnamed = |line, found| RowError::FieldCount {
        line,
        found,
        expected: 5,
        unnamed: 1,
        padded: false,
    };
    let mut errors: Vec<RowError> = (2..=201).map(|line| unnamed(line, 4)).collect();
    errors.push(unnamed(202, 6));
    assert_eq!(
        render_errors(&errors, 0),
        [format!(
            "error: 201 rows have a different number of values from the header's 5 columns, 1 of them unnamed — 4 values (200 rows: lines 2–201), 6 values (1 row: line 202); {HEADER_FIRST}; {FEWER}; {MORE}"
        )]
    );
}

/// NFR-20's example for FR-6, verbatim.
#[test]
fn nfr_20_empty_required_example() {
    let errors = [3, 5, 8].map(|line| empty(line, ColumnKind::Ring));
    assert_eq!(
        render_errors(&errors, 9),
        [
            "error: 3 rows have no value in the ring column — lines 3, 5, 8; every row needs a ring, so fill them in or delete the rows"
        ]
    );
}

#[test]
fn line_lists_collapse_consecutive_lines() {
    let lines = [2, 3, 4, 9, 11, 12];
    let errors = lines.map(|line| empty(line, ColumnKind::Name));
    assert_eq!(
        render_errors(&errors, 9),
        [
            "error: 6 rows have no value in the name column — lines 2–4, 9, 11–12; every row needs a name, so fill them in or delete the rows"
        ]
    );
}

#[test]
fn a_group_of_two_lists_both_lines() {
    let errors = [row(2, 1), row(3, 1)];
    assert_eq!(
        render_errors(&errors, 1),
        [format!(
            "error: 2 rows have a different number of values from the header's 2 columns — 1 value (2 rows: lines 2–3); {FEWER}"
        )]
    );
}

#[test]
fn empty_values_group_per_column() {
    let errors = [
        empty(2, ColumnKind::Ring),
        empty(2, ColumnKind::Sector),
        empty(3, ColumnKind::Ring),
        empty(4, ColumnKind::Name),
    ];
    assert_eq!(
        render_errors(&errors, 5),
        [
            "error: 2 rows have no value in the ring column — lines 2–3; every row needs a ring, so fill them in or delete the rows",
            "error: line 2, sector: empty; every row needs a sector, so fill it in or delete the row",
            "error: line 4, name: empty; every row needs a name, so fill it in or delete the row",
        ]
    );
}

#[test]
fn one_missing_column_keeps_its_message() {
    assert_eq!(
        render_errors(&[missing(1, ColumnKind::Ring)], 0),
        [
            "error: line 1: the header has no 'ring' column; add one, or rename the column that holds it — the header must name at least name, sector, ring and description"
        ]
    );
}

#[test]
fn missing_columns_share_one_line() {
    use ColumnKind::{Description, Name, Ring, Sector};
    let tail = "column; add them, or rename the columns that hold them — the header must name at least name, sector, ring and description";
    let cases = [
        (vec![Sector, Ring], "'sector' or 'ring'"),
        (vec![Name, Sector, Ring], "'name', 'sector' or 'ring'"),
        (
            vec![Name, Sector, Ring, Description],
            "'name', 'sector', 'ring' or 'description'",
        ),
    ];
    for (kinds, list) in cases {
        let errors: Vec<RowError> = kinds.into_iter().map(|k| missing(1, k)).collect();
        assert_eq!(
            render_errors(&errors, 0),
            [format!("error: line 1: the header has no {list} {tail}")]
        );
    }
}

/// The header-first cause is for a table where no row matches.
#[test]
fn the_header_first_cause_only_when_no_row_matches() {
    let errors = [row(2, 1), row(3, 1)];
    let with_good_row = render_errors(&errors, 1);
    assert!(!with_good_row.join("").contains(HEADER_FIRST));
    let none = render_errors(&errors, 0);
    assert!(none.join("").contains(HEADER_FIRST), "{none:?}");
}

#[test]
fn padding_hint_when_every_long_row_is_padded() {
    let line = sole(&[padded(2, 4), padded(3, 3)], 2);
    assert!(line.ends_with(&format!("; {PADDED}")), "{line}");
    assert!(!line.contains("comma"), "{line}");
}

#[test]
fn comma_hint_when_one_long_row_is_not_padded() {
    let line = sole(&[padded(2, 4), row(3, 3)], 2);
    assert!(line.ends_with(&format!("; {MORE}")), "{line}");
    assert!(!line.contains("empty cells"), "{line}");
}

#[test]
fn fewer_and_padded_causes_are_both_named_in_order() {
    let line = sole(&[padded(2, 4), row(3, 1)], 2);
    assert!(line.ends_with(&format!("; {FEWER}; {PADDED}")), "{line}");
}

/// FR-5 (rank 0) precedes FR-4's duplicates (rank 1) on the header line.
#[test]
fn header_line_order_is_missing_before_duplicate() {
    let errors = [repeats(1, 2), missing(1, ColumnKind::Ring), repeats(1, 3)];
    let lines = render_errors(&errors, 0);
    let [first, second, third] = lines.as_slice() else {
        panic!("three lines expected: {lines:?}");
    };
    assert!(first.contains("has no 'ring' column"), "{first}");
    assert!(second.contains("column 3 (C)"), "{second}");
    assert!(third.contains("column 4 (D)"), "{third}");
}

/// Parse's errors come first in the input, the model's second, and the
/// output is by line anyway.
#[test]
fn errors_of_both_stages_sort_by_line() {
    let errors = [row(9, 1), empty(3, ColumnKind::Ring)];
    let lines = render_errors(&errors, 4);
    let [first, second] = lines.as_slice() else {
        panic!("two lines expected: {lines:?}");
    };
    assert!(first.starts_with("error: line 3, ring: empty"), "{first}");
    assert!(second.starts_with("error: line 9: the row has"), "{second}");
}

#[test]
fn a_group_sits_at_its_first_members_line() {
    let errors = [
        empty(2, ColumnKind::Name),
        row(4, 1),
        empty(5, ColumnKind::Ring),
        row(6, 1),
        empty(7, ColumnKind::Ring),
    ];
    let lines = render_errors(&errors, 3);
    let [name, rows, ring] = lines.as_slice() else {
        panic!("three lines expected: {lines:?}");
    };
    assert!(name.starts_with("error: line 2, name: empty"), "{name}");
    assert!(
        rows.starts_with("error: 2 rows have a different number"),
        "{rows}"
    );
    assert!(
        ring.starts_with("error: 2 rows have no value in the ring"),
        "{ring}"
    );
}

#[test]
fn duplicate_headers_are_never_grouped() {
    let lines = render_errors(&[repeats(1, 1), repeats(1, 2)], 0);
    assert_eq!(
        lines,
        [repeats(1, 1).render(), repeats(1, 2).render()],
        "each duplicate renders alone, as it does by itself"
    );
}

/// Both stages on one file: parse's field counts and the model's empty
/// values and missing column, merged as `main` will.
#[test]
fn a_whole_file_with_errors_from_both_stages() {
    let text = "name,sector,ring,description\n\
                a,Tools,Adopt,x\n\
                b,Tools,,x\n\
                c,Tools\n\
                d,Tools,,x\n\
                e,Tools\n\
                f,Tools,Adopt,x,y\n";
    let mut w = Warnings::default();
    let (table, mut errors) = parse(text, &mut w).expect("parses");
    let rings = default_rings();
    errors.extend(
        build_catalogue(&table, &rings, OffRadar::Listed, &mut w).expect_err("rows are empty"),
    );
    let lines = render_errors(&errors, table.records.len());
    assert_eq!(
        lines,
        [
            "error: 2 rows have no value in the ring column — lines 3, 5; every row needs a ring, so fill them in or delete the rows".to_string(),
            format!(
                "error: 3 rows have a different number of values from the header's 4 columns — 2 values (2 rows: lines 4, 6), 5 values (1 row: line 7); {FEWER}; {MORE}"
            ),
        ]
    );
}

#[test]
fn a_group_lists_its_rows_in_line_order_whatever_the_input_order() {
    let errors = [row(9, 1), row(3, 1), padded(4, 3)];
    assert_eq!(
        render_errors(&errors, 2),
        [format!(
            "error: 3 rows have a different number of values from the header's 2 columns — 1 value (2 rows: lines 3, 9), 3 values (1 row: line 4); {FEWER}; {PADDED}"
        )]
    );
}

#[test]
fn a_repeated_line_is_listed_once() {
    let errors = [
        empty(5, ColumnKind::Name),
        empty(5, ColumnKind::Name),
        empty(7, ColumnKind::Name),
    ];
    assert_eq!(
        render_errors(&errors, 3),
        [
            "error: 3 rows have no value in the name column — lines 5, 7; every row needs a name, so fill them in or delete the rows"
        ]
    );
    assert_eq!(
        sole(&[empty(5, ColumnKind::Name), empty(5, ColumnKind::Name)], 3),
        "error: 2 rows have no value in the name column — line 5; every row needs a name, so fill them in or delete the rows"
    );
}
