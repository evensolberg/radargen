//! The diagnostic taxonomy (architecture §3, §7): what is fatal, what warns,
//! and how warnings reach stderr (NFR-10, NFR-11, NFR-20).

use std::fmt;

use crate::rings::{RingSource, RingsError};
use crate::{normalise, trim};

/// Where a warning points: its sort key.
///
/// `Render` concerns the render as a whole (FR-88, FR-89) and has no line.
/// It is declared first so the derived order puts it before every row
/// warning, which is NFR-11's order once FR-88 is placed (architecture §7) —
/// a sort key, not a special case.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Position {
    Render,
    /// `line` is the 1-based physical line the record begins on, counting the
    /// header as line 1 (NFR-10, D-37). `column` is the header index, which
    /// orders warnings within a line (NFR-11).
    Cell {
        line: usize,
        column: usize,
    },
}

/// FR-24 and FR-25 share a parsing rule but are separate columns, and NFR-10
/// requires the message to name the column — so they are separate classes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DateColumn {
    LastAffirmed,
    ExitDate,
}

impl DateColumn {
    /// The canonical header name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::LastAffirmed => "last_affirmed",
            Self::ExitDate => "exit_date",
        }
    }
}

impl fmt::Display for DateColumn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The warnings NFR-9 lists that concern one CSV cell. Values are carried as
/// the CSV had them; display trims them (FR-76) and escapes control characters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RowKind {
    /// FR-17. `source` is what the message says to edit (D-61).
    UndeclaredRing { value: String, source: RingSource },
    /// FR-20 — non-blank only; a blank qualifier claims nothing (D-35).
    UnrecognisedQualifier { value: String },
    /// FR-26 — non-blank only, as FR-20.
    UnrecognisedStatus { value: String },
    /// FR-24, FR-25.
    UnparsableDate { column: DateColumn, value: String },
    /// FR-7.
    EmptyDescription,
    /// FR-28, reported at the later line naming the earlier (NFR-10).
    DuplicateName { value: String, first_line: usize },
    /// FR-90: one per field, however many characters it lost. `column` is the
    /// canonical header name, since any column can carry one.
    Undisplayable {
        column: String,
        first: char,
        count: usize,
    },
    /// FR-93: a value under a column whose header cell is empty. `column`
    /// is `column N`, the 1-based position, since there is no name.
    UnnamedValue { column: String },
}

/// The warnings that concern the render as a whole. They never group: FR-88
/// fires once and FR-89 promises one warning per band.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderKind {
    /// FR-88.
    QuadrantCount { count: usize },
    /// FR-89, one per overfull band.
    OverfullBand {
        quadrant: String,
        ring: String,
        count: usize,
        fits: usize,
    },
}

/// One warning. Split by kind, so a row warning always has a line and a
/// render-level one never claims one (NFR-10) — by type, not by a check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Warning {
    Render(RenderKind),
    /// `line` and `column` as in [`Position::Cell`].
    Row {
        line: usize,
        column: usize,
        kind: RowKind,
    },
}

impl Warning {
    #[must_use]
    pub const fn position(&self) -> Position {
        match *self {
            Self::Render(_) => Position::Render,
            Self::Row { line, column, .. } => Position::Cell { line, column },
        }
    }
}

impl fmt::Display for Warning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            Self::Render(kind) => kind.message(),
            Self::Row { line, kind, .. } => Member { line: *line, kind }.per_row(),
        };
        f.write_str(&escape_controls(&text))
    }
}

/// NFR-20's warning classes: one per row kind, except that the two date
/// columns are separate classes, so each grouped line names its column.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Class {
    /// One run declares one ring set, so the source never splits a class;
    /// it rides here so the grouped line can name it.
    Ring(RingSource),
    Qualifier,
    Status,
    Date(DateColumn),
    Description,
    DuplicateName,
    Undisplayable,
    Unnamed,
}

impl RowKind {
    const fn class(&self) -> Class {
        match self {
            Self::UndeclaredRing { source, .. } => Class::Ring(*source),
            Self::UnrecognisedQualifier { .. } => Class::Qualifier,
            Self::UnrecognisedStatus { .. } => Class::Status,
            Self::UnparsableDate { column, .. } => Class::Date(*column),
            Self::EmptyDescription => Class::Description,
            Self::DuplicateName { .. } => Class::DuplicateName,
            Self::Undisplayable { .. } => Class::Undisplayable,
            Self::UnnamedValue { .. } => Class::Unnamed,
        }
    }

    /// The column this warning concerns, as NFR-10 names it.
    fn column_name(&self) -> &str {
        match self {
            Self::UndeclaredRing { .. } => "ring",
            Self::UnrecognisedQualifier { .. } => "qualifier",
            Self::UnrecognisedStatus { .. } => "status",
            Self::UnparsableDate { column, .. } => column.as_str(),
            Self::EmptyDescription => "description",
            Self::DuplicateName { .. } => "name",
            // A header name is CSV text (FR-18's extra columns); it is escaped
            // with the rest of the line.
            Self::Undisplayable { column, .. } | Self::UnnamedValue { column } => column,
        }
    }

    /// The value NFR-20 enumerates; `None` for a class that lists positions.
    /// FR-93's class enumerates its columns, so its value is the column.
    fn value(&self) -> Option<&str> {
        match self {
            Self::UndeclaredRing { value, .. }
            | Self::UnrecognisedQualifier { value }
            | Self::UnrecognisedStatus { value }
            | Self::UnparsableDate { value, .. }
            | Self::DuplicateName { value, .. }
            | Self::UnnamedValue { column: value } => Some(value),
            Self::EmptyDescription | Self::Undisplayable { .. } => None,
        }
    }

    fn message(&self) -> String {
        let v = self.value().map(trim).unwrap_or_default();
        match self {
            Self::UndeclaredRing { source, .. } => format!(
                "'{v}' is not declared by {source}; the row is in the table view, not on the radar"
            ),
            Self::UnrecognisedQualifier { .. } => {
                format!("unrecognised qualifier value '{v}'; drawn solid")
            }
            Self::UnrecognisedStatus { .. } => {
                format!("unrecognised status value '{v}'; drawn as No Change")
            }
            Self::UnparsableDate { .. } => {
                format!("'{v}' is not a YYYY-MM-DD date; shown as written")
            }
            Self::EmptyDescription => "empty; the detail panel omits it".into(),
            Self::DuplicateName { first_line, .. } => {
                format!("'{v}' repeats line {first_line}; both are drawn")
            }
            Self::Undisplayable {
                first, count: 1, ..
            } => format!(
                "removed {}, which the output cannot display",
                code_point(*first)
            ),
            Self::Undisplayable { first, count, .. } => format!(
                "removed {count} characters the output cannot display, the first {}",
                code_point(*first)
            ),
            Self::UnnamedValue { .. } => {
                "this column has no name in the header row, so its value is not shown; add a header name, or delete the value".into()
            }
        }
    }
}

impl RenderKind {
    fn message(&self) -> String {
        match self {
            Self::QuadrantCount { count } => format!(
                "{count} quadrants; the radar reads best with 3 to 6, so sectors narrow and labels may crowd"
            ),
            Self::OverfullBand {
                quadrant,
                ring,
                count,
                fits,
            } => format!(
                "quadrant '{}', ring '{}': {count} items; about {fits} fit comfortably, so some markers overlap",
                trim(quadrant),
                trim(ring)
            ),
        }
    }
}

fn code_point(c: char) -> String {
    format!("U+{:04X}", u32::from(c))
}

fn comma_list(items: impl Iterator<Item = String>) -> String {
    items.collect::<Vec<_>>().join(", ")
}

/// A CSV value can carry a newline inside quotes (NFR-10), and printed raw it
/// would forge a second diagnostic line; an escape sequence could recolour the
/// terminal, a bidi control could reorder it, and U+2028/U+2029 break a line
/// in some terminals and log viewers. Every stderr line passes through here
/// whole, so no value can be forgotten; values are only trimmed.
fn escape_controls(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if c.is_control()
            || matches!(
                c,
                '\u{061c}'
                    | '\u{200e}'
                    | '\u{200f}'
                    | '\u{2028}'..='\u{202e}'
                    | '\u{2066}'..='\u{2069}'
            )
        {
            out.extend(c.escape_default());
        } else {
            out.push(c);
        }
    }
    out
}

/// Warnings collected over a run (NFR-16 tier three).
///
/// A plain list, grouped only when printed: NFR-20's grouping needs the whole
/// run, and grouping at push time would need a map keyed by value, which §6
/// rules out of output-affecting code (architecture §7).
#[derive(Debug, Default)]
pub struct Warnings(Vec<Warning>);

impl Warnings {
    pub fn push(&mut self, w: Warning) {
        self.0.push(w);
    }

    /// The raw warnings in push order, so a stage's tests can assert on
    /// kinds and positions rather than on rendered English.
    #[must_use]
    pub fn as_slice(&self) -> &[Warning] {
        &self.0
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// One string per stderr line, in NFR-11's order, grouped per NFR-20.
    #[must_use]
    pub fn render(&self) -> Vec<String> {
        // Stable: equal positions keep push order.
        let mut sorted: Vec<&Warning> = self.0.iter().collect();
        sorted.sort_by_key(|w| w.position());

        // Render-level warnings sort first and print as they are; row
        // warnings collect into classes in the order each class first
        // appears, which places every grouped line at its first member.
        let mut lines = Vec::new();
        let mut classes: Vec<(Class, Vec<Member>)> = Vec::new();
        for w in sorted {
            match w {
                Warning::Render(_) => lines.push(format!("warning: {w}")),
                Warning::Row { line, kind, .. } => {
                    let member = Member { line: *line, kind };
                    match classes.iter_mut().find(|(c, _)| *c == kind.class()) {
                        Some((_, members)) => members.push(member),
                        None => classes.push((kind.class(), vec![member])),
                    }
                }
            }
        }
        for (class, members) in &classes {
            let text = match members.as_slice() {
                [only] => only.per_row(),
                _ => grouped(*class, members),
            };
            lines.push(format!("warning: {}", escape_controls(&text)));
        }
        lines
    }
}

/// A row warning as grouping sees it.
struct Member<'a> {
    line: usize,
    kind: &'a RowKind,
}

impl Member<'_> {
    fn per_row(&self) -> String {
        format!(
            "line {}, {}: {}",
            self.line,
            self.kind.column_name(),
            self.kind.message()
        )
    }
}

fn grouped(class: Class, members: &[Member]) -> String {
    let n = members.len();
    match class {
        Class::Ring(source) => format!(
            "{n} rows use a ring not declared by {source} — {}; all appear in the table view",
            tallied(members, normalise)
        ),
        Class::Qualifier => format!(
            "{n} rows have an unrecognised qualifier — {}; each is drawn solid",
            tallied(members, normalise)
        ),
        Class::Status => format!(
            "{n} rows have an unrecognised status — {}; each is drawn as No Change",
            tallied(members, normalise)
        ),
        // FR-76 folds case only for the comparisons it lists; a date is
        // parsed, so only trimmed, and two spellings are two values.
        Class::Date(column) => format!(
            "{n} rows have {column} values that are not YYYY-MM-DD dates — {}; each is shown as written",
            tallied(members, |v| trim(v).to_string())
        ),
        Class::DuplicateName => format!(
            "{n} rows repeat an earlier name — {}; every row is drawn",
            tallied(members, normalise)
        ),
        // No value to enumerate, so NFR-20 lists positions, uncapped.
        Class::Description => format!(
            "{n} rows have an empty description — lines {}; each detail panel omits it",
            comma_list(members.iter().map(|m| m.line.to_string()))
        ),
        // Positions again, each with the code point: a removed character is
        // usually invisible in an editor, so it is what tells the author what
        // to look for.
        Class::Undisplayable => format!(
            "{n} fields had characters the output cannot display — {}; each was removed",
            comma_list(members.iter().filter_map(|m| {
                let RowKind::Undisplayable {
                    column,
                    first,
                    count,
                } = m.kind
                else {
                    return None;
                };
                Some(match count {
                    1 => format!("{column} on line {} ({})", m.line, code_point(*first)),
                    c => format!(
                        "{column} on line {} ({c}, first {})",
                        m.line,
                        code_point(*first)
                    ),
                })
            }))
        ),
        // FR-93 counts values, each column with its count and first line.
        Class::Unnamed => format!(
            "{n} values are under columns with no name in the header row — {}; none is shown, so add a header name, or delete the values",
            tallied(members, str::to_string)
        ),
    }
}

/// NFR-20's enumeration: each distinct value under `key`, in first-occurrence
/// order, shown as its first occurrence wrote it, with its count and first
/// line.
fn tallied(members: &[Member], key: fn(&str) -> String) -> String {
    // (key, first occurrence, count)
    let mut tallies: Vec<(String, &Member, usize)> = Vec::new();
    for m in members {
        let k = key(m.kind.value().unwrap_or_default());
        match tallies.iter_mut().find(|(t, _, _)| *t == k) {
            Some((_, _, count)) => *count += 1,
            None => tallies.push((k, m, 1)),
        }
    }
    comma_list(tallies.iter().map(|(_, first, count)| {
        let shown = trim(first.kind.value().unwrap_or_default());
        let line = first.line;
        match (first.kind, count) {
            // A duplicate leads with the line it repeats, so "first" can only
            // mean the first repeat.
            (RowKind::DuplicateName { first_line, .. }, 1) => {
                format!("{shown} (line {first_line}; repeated on line {line})")
            }
            (RowKind::DuplicateName { first_line, .. }, c) => {
                format!("{shown} (line {first_line}; repeated on {c} rows, first line {line})")
            }
            (_, 1) => format!("{shown} (1, line {line})"),
            (_, c) => format!("{shown} ({c}, first line {line})"),
        }
    }))
}

/// FR-1: a separator radargen refuses, found where a comma belongs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Separator {
    Semicolon,
    Tab,
}

impl Separator {
    /// The separator `c` is, if it is one the message can name.
    #[must_use]
    pub const fn of(c: char) -> Option<Self> {
        match c {
            ';' => Some(Self::Semicolon),
            '\t' => Some(Self::Tab),
            _ => None,
        }
    }
}

/// How the message names it.
// qual:allow(dry, boilerplate) reason: "deriving Display needs a crate (strum, derive_more) for one two-variant enum"
impl fmt::Display for Separator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Semicolon => "semicolons",
            Self::Tab => "tabs",
        })
    }
}

/// Everything the tool refuses to run on: non-zero exit, nothing written
/// (NFR-8, NFR-9).
///
/// One taxonomy for the whole tool (architecture §1); each later stage adds its
/// variants beside the code that raises them. An I/O variant should carry the
/// path it failed on and mark the error `#[source]`, not `#[from]`: a bare
/// `From<io::Error>` would let `?` build a `Fatal` that cannot name its path
/// (NFR-10).
#[derive(Debug, thiserror::Error)]
pub enum Fatal {
    /// FR-52: the `-o` extension is absent or names a format we do not write.
    #[error("-o {path}: the output file name must end in .html")]
    UnrecognisedExtension { path: String },
    /// §14.1: the extension is one v1.1 adds, so say that rather than
    /// "unrecognised", which sends the caller hunting for a typo in a path
    /// that is correct.
    #[error("-o {path}: .{ext} output arrives in radargen {arrives_in}; this version writes .html")]
    FormatNotInThisVersion {
        path: String,
        ext: String,
        arrives_in: &'static str,
    },
    /// FR-63, NFR-16 tier one.
    #[error("--rings: {0}")]
    Rings(#[from] RingsError),
    /// FR-61: the input does not exist, cannot be read, or is a directory.
    #[error("{path}: cannot be read: {source}")]
    Unreadable {
        path: String,
        #[source]
        source: std::io::Error,
    },
    /// FR-61, D-27: say what to do — the likely cause is an Excel export in a
    /// legacy code page, and "invalid utf-8" alone does not tell the author
    /// that. `line` is the physical line holding the first bad byte.
    #[error(
        "{path}, line {line}: not UTF-8 text; re-save the file as UTF-8 (in Excel, Save As \"CSV UTF-8\")"
    )]
    NotUtf8 { path: String, line: usize },
    /// NFR-15: an unterminated quote swallows the rest of the file into one
    /// field, so nothing after it parses. `line` is where the quote opened,
    /// which may be after the record's first line (plan RG-005, Decisions).
    #[error(
        "line {line}, column {column}: a quote opens this value and never closes; add the closing quote, and write any quote inside the value as \"\""
    )]
    UnterminatedQuote { line: usize, column: usize },
    /// NFR-15: text after a closing quote. RFC 4180 doubles a quote inside a
    /// quoted field; keeping or dropping the stray text would both guess.
    /// `line` is the closing quote's.
    #[error(
        "line {line}, column {column}: text follows a closing quote; inside quotes, write a quote as \"\""
    )]
    TextAfterQuote { line: usize, column: usize },
    /// FR-3: the file has no non-blank records, so no header names a column.
    /// The required columns are FR-5's, named so the message is actionable.
    #[error(
        "the CSV has no header row (it is empty or blank); the header must name at least name, quadrant, ring and description"
    )]
    NoHeader,
    /// FR-1: a one-column header holding `;` or a tab is a file with another
    /// separator — Excel writes `;` where a comma is the decimal mark.
    /// Without this, every row would mis-report its field count.
    #[error(
        "line {line}: the header is one column holding {separator}; radargen reads comma-separated files only, so export the file with commas as the separator"
    )]
    NotCommaSeparated { line: usize, separator: Separator },
    /// FR-4: a header name repeated under FR-76's normalisation, so
    /// `Name` and `name ` collide. Indices are 1-based; names are as
    /// written, trimmed, so the author can find both cells.
    #[error(
        "line {line}: column {index} '{name}' repeats column {first_index} '{first_name}'; header names match ignoring case and surrounding spaces, so rename or remove one"
    )]
    DuplicateHeader {
        line: usize,
        index: usize,
        name: String,
        first_index: usize,
        first_name: String,
    },
    /// FR-4: a record whose field count is not the header's, with the usual
    /// cause for each direction (FR-61's precedent: say what to do).
    #[error(
        "line {line}: the row's field count is {found} but the header's is {expected}; {}",
        if found > expected {
            "a comma inside a value needs the value in double quotes"
        } else {
            "a value is missing, or an unquoted line break split the row"
        }
    )]
    FieldCount {
        line: usize,
        found: usize,
        expected: usize,
    },
    /// FR-5: the header lacks a required column. Every missing one is
    /// reported (NFR-16 tier two). `line` is the header's.
    #[error(
        "line {line}: the header has no '{column}' column; add one, or rename the column that holds it — the header must name at least name, quadrant, ring and description"
    )]
    MissingColumn { line: usize, column: &'static str },
    /// FR-6: an empty `name`, `quadrant` or `ring`, empty under FR-76's
    /// trim, so a cell of spaces or a lone zero-width space counts.
    #[error(
        "line {line}, {column}: empty; every row needs a {column}, so fill it in or delete the row"
    )]
    EmptyRequired { line: usize, column: &'static str },
}

impl Fatal {
    /// The stderr line.
    #[must_use]
    pub fn render(&self) -> String {
        // Escaped as a whole, so no variant can forget to: `-o` and `--rings`
        // values come from argv, and later variants carry CSV text (NFR-10).
        format!("error: {}", escape_controls(&self.to_string()))
    }
}
