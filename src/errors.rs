//! The error block at print time (architecture §7): both stages' row-level
//! errors merged, sorted and grouped into the lines stderr shows.
//!
//! D-62 groups what NFR-20 groups for warnings, with one difference: an error
//! lists every line, as ranges, because an error writes nothing and a line
//! left out is a line the author finds only on the next run. NFR-11 orders the
//! output and NFR-16 is why there are several errors to order.

use crate::columns::ColumnKind;
use crate::diagnostics::{Fatal, error_line, plural};

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

/// What may share a line: errors of one class.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Class {
    /// FR-4's mismatched rows.
    Rows,
    /// FR-5's missing columns.
    Missing,
    /// FR-6's empty values in one column.
    Empty(ColumnKind),
    /// Printed on its own line.
    Alone,
}

impl Class {
    const fn of(fatal: &Fatal) -> Self {
        match fatal {
            Fatal::FieldCount { .. } => Self::Rows,
            Fatal::MissingColumn { .. } => Self::Missing,
            Fatal::EmptyRequired { column, .. } => Self::Empty(*column),
            _ => Self::Alone,
        }
    }
}

/// One error with its place in the input.
struct Member<'a> {
    order: usize,
    fatal: &'a Fatal,
}

/// The errors of one class, in line order.
struct Group<'a> {
    class: Class,
    members: Vec<Member<'a>>,
}

/// Collects `errors` into classes in the order each first appears.
fn groups(errors: &[Fatal]) -> Vec<Group<'_>> {
    let mut groups: Vec<Group<'_>> = Vec::new();
    for (order, fatal) in errors.iter().enumerate() {
        let class = Class::of(fatal);
        let member = Member { order, fatal };
        match groups
            .iter_mut()
            .find(|g| class != Class::Alone && g.class == class)
        {
            Some(group) => group.members.push(member),
            None => groups.push(Group {
                class,
                members: vec![member],
            }),
        }
    }
    for group in &mut groups {
        group.members.sort_by_key(|m| line_of(m.fatal));
    }
    groups
}

/// The line a [`Fatal`] names; an error with none sorts first.
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
        _ => 0,
    }
}

/// 0 for an error naming no column position, 1 for one that does.
const fn rank_of(fatal: &Fatal) -> u8 {
    match fatal {
        Fatal::DuplicateHeader { .. } | Fatal::EmptyRequired { .. } => 1,
        _ => 0,
    }
}

impl Group<'_> {
    /// NFR-11's sort key: the first member's line, rank and input order.
    fn position(&self) -> (usize, u8, usize) {
        self.members
            .first()
            .map_or((0, 0, 0), |m| (line_of(m.fatal), rank_of(m.fatal), m.order))
    }

    fn render(&self, well_formed_rows: usize) -> String {
        let fatals: Vec<&Fatal> = self.members.iter().map(|m| m.fatal).collect();
        match (self.class, fatals.as_slice()) {
            (Class::Rows, [_, _, ..]) => rows_line(&fatals, well_formed_rows == 0),
            (Class::Missing, [_, _, ..]) => missing_line(&fatals),
            (Class::Empty(column), [_, _, ..]) => empty_line(column, &fatals),
            (Class::Rows, [one]) => Mismatch::of(one).map_or_else(
                || one.render(),
                |m| error_line(&one_row(&m, well_formed_rows == 0)),
            ),
            (_, [one, ..]) => one.render(),
            (_, []) => String::new(),
        }
    }
}

/// A mismatched row, as FR-4 records it.
pub(crate) struct Mismatch {
    pub(crate) line: usize,
    pub(crate) found: usize,
    pub(crate) expected: usize,
    pub(crate) unnamed: usize,
    pub(crate) padded: bool,
}

impl Mismatch {
    const fn of(fatal: &Fatal) -> Option<Self> {
        match *fatal {
            Fatal::FieldCount {
                line,
                found,
                expected,
                unnamed,
                padded,
            } => Some(Self {
                line,
                found,
                expected,
                unnamed,
                padded,
            }),
            _ => None,
        }
    }
}

/// FR-4's closing causes for the rows seen, each a clause, joined with `; `
/// (NFR-20).
///
/// `no_match` is "no data row matches the header" and says to look at the
/// header first. The padding clause needs every row with more values to end in
/// empty cells; one that does not sends them all to the comma clause.
fn row_causes(no_match: bool, rows: &[Mismatch]) -> String {
    let fewer = rows.iter().any(|m| m.found < m.expected);
    let mut longer = rows.iter().filter(|m| m.found > m.expected).peekable();
    let more = longer.peek().is_some();
    let padded = longer.all(|m| m.padded);
    [
        (no_match, "no row matches the header, so check the header first"),
        (
            fewer,
            "a row with fewer values is usually missing one, or an unquoted line break split it",
        ),
        (
            more && padded,
            "rows with more values end in empty cells: delete them, or give the header matching empty columns",
        ),
        (
            more && !padded,
            "a row with more values usually has a comma that needs the value in double quotes",
        ),
    ]
    .into_iter()
    .filter_map(|(applies, cause)| applies.then_some(cause))
    .collect::<Vec<_>>()
    .join("; ")
}

/// `, 1 of them unnamed` when the header has unnamed columns (FR-4, FR-93).
fn unnamed_clause(unnamed: usize) -> String {
    if unnamed == 0 {
        String::new()
    } else {
        format!(", {unnamed} of them unnamed")
    }
}

/// FR-4's message for one mismatched row.
///
/// `no_match` adds the header-first cause; [`Fatal`]'s own message cannot know
/// it, since it sees one row and not the table.
pub(crate) fn one_row(m: &Mismatch, no_match: bool) -> String {
    format!(
        "line {}: the row has {} {} but the header has {} {}{}; {}",
        m.line,
        m.found,
        plural(m.found, "value", "values"),
        m.expected,
        plural(m.expected, "column", "columns"),
        unnamed_clause(m.unnamed),
        row_causes(no_match, std::slice::from_ref(m))
    )
}

/// FR-4's one line for two or more mismatched rows (NFR-20's shape).
fn rows_line(fatals: &[&Fatal], no_match: bool) -> String {
    let rows: Vec<Mismatch> = fatals.iter().filter_map(|f| Mismatch::of(f)).collect();
    let (expected, unnamed) = rows.first().map_or((0, 0), |m| (m.expected, m.unnamed));
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
fn value_counts(rows: &[Mismatch]) -> String {
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
fn missing_line(fatals: &[&Fatal]) -> String {
    let columns: Vec<&str> = fatals
        .iter()
        .filter_map(|f| match f {
            Fatal::MissingColumn { column, .. } => Some(column.as_str()),
            _ => None,
        })
        .collect();
    let line = fatals.first().map_or(0, |f| line_of(f));
    error_line(&format!(
        "line {line}: the header has no {} column; add them, or rename the columns that hold them — the header must name at least name, quadrant, ring and description",
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
fn empty_line(column: ColumnKind, fatals: &[&Fatal]) -> String {
    let lines: Vec<usize> = fatals.iter().map(|f| line_of(f)).collect();
    let name = column.as_str();
    error_line(&format!(
        "{} rows have no value in the {name} column — {}; every row needs a {name}, so fill them in or delete the rows",
        lines.len(),
        line_list(&lines)
    ))
}

/// `line 7`; `lines 3, 5, 8`; `lines 2–4, 9, 11–12`. Ascending lines are
/// expected, as `groups` sorts them.
fn line_list(lines: &[usize]) -> String {
    let mut runs: Vec<(usize, usize)> = Vec::new();
    for &line in lines {
        match runs.last_mut() {
            Some((_, end)) if *end + 1 == line => *end = line,
            _ => runs.push((line, line)),
        }
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
    format!("{} {text}", plural(lines.len(), "line", "lines"))
}
