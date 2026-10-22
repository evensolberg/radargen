//! The error block at print time (architecture §7): both stages' row-level
//! errors merged, sorted and grouped into the lines stderr shows.
//!
//! D-62 groups what NFR-20 groups for warnings, with one difference: an error
//! lists every line, as ranges, because an error writes nothing and a line
//! left out is a line the author finds only on the next run. NFR-11 orders the
//! output and NFR-16 is why there are several errors to order.

use std::collections::BTreeMap;

use crate::columns::ColumnKind;
use crate::diagnostics::{
    Mismatch, RowError, error_line, one_row, plural, row_causes, unnamed_clause,
};

/// The stderr lines for `errors`, each `error: …`, in NFR-11's order.
///
/// `errors` is `parse`'s tier-two errors followed by `build_catalogue`'s, and
/// `well_formed_rows` is the table's record count: FR-4's header-first cause
/// applies when no data row matches the header. FR-4's mismatched rows, FR-5's
/// missing columns and FR-6's empty values each group into one line per class
/// (per column for FR-6) when there are two or more; duplicate headers never
/// group, and each renders as [`RowError::render`] does.
///
/// Each line sorts by `(first line, rank, input order)`. Rank 0 is an error
/// naming no column position (FR-4's row, FR-5) and rank 1 one that does, so on
/// the header line FR-5 precedes FR-4's duplicates. A grouped line sorts by its
/// first member's line. The sort is stable.
#[must_use]
pub fn render_errors(errors: &[RowError], well_formed_rows: usize) -> Vec<String> {
    let mut groups = groups(errors);
    groups.sort_by_key(Group::position);
    groups
        .iter()
        .map(|group| group.render(well_formed_rows))
        .collect()
}

/// One error with its place in the input and what its class needs of it.
struct Member<T> {
    order: usize,
    line: usize,
    item: T,
}

impl<T> Member<T> {
    /// `item` standing for the `order`th error, on `line`.
    const fn at(order: usize, line: usize, item: T) -> Self {
        Self { order, line, item }
    }
}

/// The errors of one class: the first by line, then the rest in line order.
///
/// A run holds at least one member by construction, so no code asks for the
/// first of an empty run.
struct Run<T> {
    first: Member<T>,
    rest: Vec<Member<T>>,
}

impl<T> Run<T> {
    /// The run of `members` in line order, or `None` for none. The sort is
    /// stable, so input order breaks ties (NFR-11).
    fn of(mut members: Vec<Member<T>>) -> Option<Self> {
        members.sort_by_key(|m| m.line);
        let mut sorted = members.into_iter();
        sorted.next().map(|first| Self {
            first,
            rest: sorted.collect(),
        })
    }

    fn members(&self) -> impl Iterator<Item = &Member<T>> {
        std::iter::once(&self.first).chain(&self.rest)
    }

    fn lines(&self) -> Vec<usize> {
        self.members().map(|m| m.line).collect()
    }
}

/// A line of output: one error, or the errors of a class that may share one.
enum Group<'a> {
    /// Printed on its own line.
    Alone(Member<&'a RowError>),
    /// FR-4's mismatched rows.
    Rows(Run<Mismatch>),
    /// FR-5's missing columns, each with the error that names it.
    Missing(Run<(ColumnKind, &'a RowError)>),
    /// FR-6's empty values in one column.
    Empty(ColumnKind, Run<&'a RowError>),
}

/// An error with the class it belongs to.
enum Classified<'a> {
    Alone(Member<&'a RowError>),
    Row(Member<Mismatch>),
    Missing(Member<(ColumnKind, &'a RowError)>),
    Empty(ColumnKind, Member<&'a RowError>),
}

/// Which class `error` belongs to. Every variant is listed, so a new one must
/// say whether its errors may share a line.
const fn classify(order: usize, error: &RowError) -> Classified<'_> {
    match *error {
        RowError::FieldCount {
            line,
            found,
            expected,
            unnamed,
            padded,
        } => Classified::Row(Member::at(
            order,
            line,
            Mismatch {
                line,
                found,
                expected,
                unnamed,
                padded,
            },
        )),
        RowError::MissingColumn { line, column } => {
            Classified::Missing(Member::at(order, line, (column, error)))
        }
        RowError::EmptyRequired { line, column } => {
            Classified::Empty(column, Member::at(order, line, error))
        }
        RowError::DuplicateHeader { line, .. } | RowError::SectorTwice { line, .. } => {
            Classified::Alone(Member::at(order, line, error))
        }
    }
}

/// The members of each class, before they become groups.
#[derive(Default)]
struct Buckets<'a> {
    alone: Vec<Member<&'a RowError>>,
    rows: Vec<Member<Mismatch>>,
    missing: Vec<Member<(ColumnKind, &'a RowError)>>,
    empty: BTreeMap<ColumnKind, Vec<Member<&'a RowError>>>,
}

impl<'a> Buckets<'a> {
    fn add(&mut self, classified: Classified<'a>) {
        match classified {
            Classified::Alone(m) => self.alone.push(m),
            Classified::Row(m) => self.rows.push(m),
            Classified::Missing(m) => self.missing.push(m),
            Classified::Empty(column, m) => self.empty.entry(column).or_default().push(m),
        }
    }

    fn into_groups(self) -> Vec<Group<'a>> {
        let empty = self
            .empty
            .into_iter()
            .filter_map(|(column, ms)| Run::of(ms).map(|run| Group::Empty(column, run)));
        self.alone
            .into_iter()
            .map(Group::Alone)
            .chain(Run::of(self.rows).map(Group::Rows))
            .chain(Run::of(self.missing).map(Group::Missing))
            .chain(empty)
            .collect()
    }
}

/// The groups of `errors`, unsorted.
fn groups(errors: &[RowError]) -> Vec<Group<'_>> {
    let mut buckets = Buckets::default();
    for (order, error) in errors.iter().enumerate() {
        buckets.add(classify(order, error));
    }
    buckets.into_groups()
}

impl Group<'_> {
    /// NFR-11's sort key: the first member's line, rank and input order.
    const fn position(&self) -> (usize, u8, usize) {
        match self {
            // Only duplicate headers stand alone, and they name a position.
            Self::Alone(m) => (m.line, 1, m.order),
            Self::Rows(run) => (run.first.line, 0, run.first.order),
            Self::Missing(run) => (run.first.line, 0, run.first.order),
            Self::Empty(_, run) => (run.first.line, 1, run.first.order),
        }
    }

    fn render(&self, well_formed_rows: usize) -> String {
        match self {
            Self::Alone(m) => m.item.render(),
            Self::Rows(run) if run.rest.is_empty() => {
                error_line(&one_row(&run.first.item, well_formed_rows == 0))
            }
            Self::Rows(run) => rows_line(run, well_formed_rows == 0),
            Self::Missing(run) if run.rest.is_empty() => run.first.item.1.render(),
            Self::Missing(run) => missing_line(run),
            Self::Empty(_, run) if run.rest.is_empty() => run.first.item.render(),
            Self::Empty(column, run) => empty_line(*column, run),
        }
    }
}

/// FR-4's one line for two or more mismatched rows (NFR-20's shape).
fn rows_line(run: &Run<Mismatch>, no_match: bool) -> String {
    let rows: Vec<&Mismatch> = run.members().map(|m| &m.item).collect();
    let Mismatch {
        expected, unnamed, ..
    } = run.first.item;
    let causes = row_causes(no_match, &rows);
    error_line(&format!(
        "{} rows have a different number of values from the header's {expected} {}{} — {}; {causes}",
        rows.len(),
        plural(expected, "column", "columns"),
        unnamed_clause(unnamed),
        value_counts(&rows),
    ))
}

/// Each distinct number of values with its rows, in order of its first line.
fn value_counts(rows: &[&Mismatch]) -> String {
    let mut counts: Vec<(usize, Vec<usize>)> = Vec::new();
    for m in rows {
        match counts.iter_mut().find(|(found, _)| *found == m.found) {
            Some((_, lines)) => lines.push(m.line),
            None => counts.push((m.found, vec![m.line])),
        }
    }
    counts
        .iter()
        .map(|(found, lines)| {
            format!(
                "{found} {} ({} {}: {})",
                plural(*found, "value", "values"),
                lines.len(),
                plural(lines.len(), "row", "rows"),
                line_list(lines)
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// FR-5's one line for two or more missing columns, all on the header's line.
fn missing_line(run: &Run<(ColumnKind, &RowError)>) -> String {
    let columns: Vec<&str> = run.members().map(|m| m.item.0.as_str()).collect();
    error_line(&format!(
        "line {}: the header has no {} column; add them, or rename the columns that hold them — the header must name at least name, sector, ring and description",
        run.first.line,
        or_list(&columns)
    ))
}

/// `'a' or 'b'`; `'a', 'b' or 'c'`.
fn or_list(names: &[&str]) -> String {
    let quoted: Vec<String> = names.iter().map(|n| format!("'{n}'")).collect();
    match quoted.split_last() {
        Some((last, rest)) if !rest.is_empty() => format!("{} or {last}", rest.join(", ")),
        Some((last, _)) => last.clone(),
        None => String::new(),
    }
}

/// FR-6's one line for two or more empty values in one column.
fn empty_line(column: ColumnKind, run: &Run<&RowError>) -> String {
    let lines = run.lines();
    let name = column.as_str();
    error_line(&format!(
        "{} rows have no value in the {name} column — {}; every row needs a {name}, so fill them in or delete the rows",
        lines.len(),
        line_list(&lines)
    ))
}

/// `line 7`; `lines 3, 5, 8`; `lines 2–4, 9, 11–12`.
///
/// `lines` is ascending, as a [`Run`] keeps it. An equal line is skipped, so
/// a repeat cannot print `5, 5`, and the noun agrees with the lines shown.
fn line_list(lines: &[usize]) -> String {
    let mut runs: Vec<(usize, usize)> = Vec::new();
    let mut shown = 0;
    for &line in lines {
        match runs.last_mut() {
            Some((_, end)) if *end == line => continue,
            Some((_, end)) if *end + 1 == line => *end = line,
            _ => runs.push((line, line)),
        }
        shown += 1;
    }
    let text = runs
        .iter()
        .map(|&(start, end)| {
            if start == end {
                start.to_string()
            } else {
                format!("{start}–{end}")
            }
        })
        .collect::<Vec<_>>()
        .join(", ");
    format!("{} {text}", plural(shown, "line", "lines"))
}
