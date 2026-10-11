//! The error block at print time (architecture §7): both stages' row-level
//! errors merged, sorted and grouped into the lines stderr shows.
//!
//! D-62 groups what NFR-20 groups for warnings, with one difference: an error
//! lists every line, as ranges, because an error writes nothing and a line
//! left out is a line the author finds only on the next run. NFR-11 orders the
//! output and NFR-16 is why there are several errors to order.

use std::mem;

use crate::columns::ColumnKind;
use crate::diagnostics::{
    Fatal, Mismatch, error_line, one_row, plural, row_causes, unnamed_clause,
};

// qual:api — PR 14's main.rs is its first production caller
/// The stderr lines for `errors`, each `error: …`, in NFR-11's order.
///
/// `errors` is `parse`'s tier-two errors followed by `build_catalogue`'s, and
/// `well_formed_rows` is the table's record count: FR-4's header-first cause
/// applies when no data row matches the header. FR-4's mismatched rows, FR-5's
/// missing columns and FR-6's empty values each group into one line per class
/// (per column for FR-6) when there are two or more; duplicate headers never
/// group, and any other [`Fatal`] renders as [`Fatal::render`] does.
///
/// Each line sorts by `(first line, rank, input order)`. Rank 0 is an error
/// naming no column position (FR-4's row, FR-5) and rank 1 one that does, so on
/// the header line FR-5 precedes FR-4's duplicates. A grouped line sorts by its
/// first member's line, and an error with no line sorts first. The sort is
/// stable.
#[must_use]
pub fn render_errors(errors: &[Fatal], well_formed_rows: usize) -> Vec<String> {
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

/// The errors of one class: the first by line, then the rest in line order.
///
/// A run holds at least one member by construction, so no code asks for the
/// first of an empty run.
struct Run<T> {
    first: Member<T>,
    rest: Vec<Member<T>>,
}

impl<T> Run<T> {
    const fn new(first: Member<T>) -> Self {
        Self {
            first,
            rest: Vec::new(),
        }
    }

    /// Adds `member` in line order; one on an equal line goes after, so input
    /// order breaks ties (NFR-11).
    fn insert(&mut self, member: Member<T>) {
        if member.line < self.first.line {
            let displaced = mem::replace(&mut self.first, member);
            self.rest.insert(0, displaced);
        } else {
            let at = self.rest.partition_point(|m| m.line <= member.line);
            self.rest.insert(at, member);
        }
    }

    fn members(&self) -> impl Iterator<Item = &Member<T>> {
        std::iter::once(&self.first).chain(&self.rest)
    }

    fn lines(&self) -> Vec<usize> {
        self.members().map(|m| m.line).collect()
    }
}

/// Adds `member` to the run in `slot`, starting it if there is none.
fn add<T>(slot: &mut Option<Run<T>>, member: Member<T>) {
    match slot {
        Some(run) => run.insert(member),
        None => *slot = Some(Run::new(member)),
    }
}

/// A line of output: one error, or the errors of a class that may share one.
enum Group<'a> {
    /// Printed on its own line.
    Alone(Member<&'a Fatal>),
    /// FR-4's mismatched rows.
    Rows(Run<Mismatch>),
    /// FR-5's missing columns, each with the error that names it.
    Missing(Run<(ColumnKind, &'a Fatal)>),
    /// FR-6's empty values in one column.
    Empty(ColumnKind, Run<&'a Fatal>),
}

/// The groups of `errors`, unsorted.
fn groups(errors: &[Fatal]) -> Vec<Group<'_>> {
    let mut alone = Vec::new();
    let mut rows = None;
    let mut missing = None;
    let mut empty: Vec<(ColumnKind, Run<&Fatal>)> = Vec::new();
    for (order, fatal) in errors.iter().enumerate() {
        let line = line_of(fatal);
        match *fatal {
            Fatal::FieldCount {
                line,
                found,
                expected,
                unnamed,
                padded,
            } => add(
                &mut rows,
                Member {
                    order,
                    line,
                    item: Mismatch {
                        line,
                        found,
                        expected,
                        unnamed,
                        padded,
                    },
                },
            ),
            Fatal::MissingColumn { column, .. } => add(
                &mut missing,
                Member {
                    order,
                    line,
                    item: (column, fatal),
                },
            ),
            Fatal::EmptyRequired { column, .. } => {
                let member = Member {
                    order,
                    line,
                    item: fatal,
                };
                match empty.iter_mut().find(|(c, _)| *c == column) {
                    Some((_, run)) => run.insert(member),
                    None => empty.push((column, Run::new(member))),
                }
            }
            Fatal::UnrecognisedExtension { .. }
            | Fatal::FormatNotInThisVersion { .. }
            | Fatal::Rings(_)
            | Fatal::Unreadable { .. }
            | Fatal::NotUtf8 { .. }
            | Fatal::UnterminatedQuote { .. }
            | Fatal::TextAfterQuote { .. }
            | Fatal::NoHeader
            | Fatal::NotCommaSeparated { .. }
            | Fatal::DuplicateHeader { .. } => alone.push(Group::Alone(Member {
                order,
                line,
                item: fatal,
            })),
        }
    }
    let mut groups = alone;
    groups.extend(rows.map(Group::Rows));
    groups.extend(missing.map(Group::Missing));
    groups.extend(empty.into_iter().map(|(c, run)| Group::Empty(c, run)));
    groups
}

/// The line a [`Fatal`] names; an error with none sorts first.
///
/// Every variant is listed, so a new one must say here whether it has a line.
const fn line_of(fatal: &Fatal) -> usize {
    match fatal {
        Fatal::FieldCount { line, .. }
        | Fatal::DuplicateHeader { line, .. }
        | Fatal::MissingColumn { line, .. }
        | Fatal::EmptyRequired { line, .. }
        | Fatal::NotCommaSeparated { line, .. }
        | Fatal::UnterminatedQuote { line, .. }
        | Fatal::TextAfterQuote { line, .. }
        | Fatal::NotUtf8 { line, .. } => *line,
        Fatal::UnrecognisedExtension { .. }
        | Fatal::FormatNotInThisVersion { .. }
        | Fatal::Rings(_)
        | Fatal::Unreadable { .. }
        | Fatal::NoHeader => 0,
    }
}

/// 0 for an error naming no column position, 1 for one that does.
///
/// Every variant is listed, so a new one must pick its rank here.
const fn rank_of(fatal: &Fatal) -> u8 {
    match fatal {
        Fatal::DuplicateHeader { .. } | Fatal::EmptyRequired { .. } => 1,
        Fatal::FieldCount { .. }
        | Fatal::MissingColumn { .. }
        | Fatal::UnrecognisedExtension { .. }
        | Fatal::FormatNotInThisVersion { .. }
        | Fatal::Rings(_)
        | Fatal::Unreadable { .. }
        | Fatal::NotUtf8 { .. }
        | Fatal::UnterminatedQuote { .. }
        | Fatal::TextAfterQuote { .. }
        | Fatal::NoHeader
        | Fatal::NotCommaSeparated { .. } => 0,
    }
}

impl Group<'_> {
    /// NFR-11's sort key: the first member's line, rank and input order.
    const fn position(&self) -> (usize, u8, usize) {
        match self {
            Self::Alone(m) => (m.line, rank_of(m.item), m.order),
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
fn missing_line(run: &Run<(ColumnKind, &Fatal)>) -> String {
    let columns: Vec<&str> = run.members().map(|m| m.item.0.as_str()).collect();
    error_line(&format!(
        "line {}: the header has no {} column; add them, or rename the columns that hold them — the header must name at least name, quadrant, ring and description",
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
fn empty_line(column: ColumnKind, run: &Run<&Fatal>) -> String {
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
