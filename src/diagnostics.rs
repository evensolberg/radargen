//! The diagnostic taxonomy (architecture §3, §7): what is fatal, what warns,
//! and how warnings reach stderr (NFR-10, NFR-11, NFR-20).

use std::fmt;

use crate::normalise;
use crate::rings::RingsError;

/// Where a warning points.
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

impl fmt::Display for DateColumn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::LastAffirmed => "last_affirmed",
            Self::ExitDate => "exit_date",
        })
    }
}

/// Every warning NFR-9 lists. Values are carried as the CSV had them; display
/// trims them and escapes control characters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WarningKind {
    /// FR-17.
    UndeclaredRing { value: String },
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
    /// FR-88. Render-level.
    QuadrantCount { count: usize },
    /// FR-89. Render-level, one per overfull band.
    OverfullBand {
        quadrant: String,
        ring: String,
        count: usize,
        fits: usize,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Warning {
    pub position: Position,
    pub kind: WarningKind,
}

impl WarningKind {
    /// The column a row-level warning concerns, as NFR-10 names it.
    fn column(&self) -> String {
        match self {
            Self::UndeclaredRing { .. } => "ring".into(),
            Self::UnrecognisedQualifier { .. } => "qualifier".into(),
            Self::UnrecognisedStatus { .. } => "status".into(),
            Self::UnparsableDate { column, .. } => column.to_string(),
            Self::EmptyDescription => "description".into(),
            Self::DuplicateName { .. } => "name".into(),
            // A header name is CSV text (FR-18's extra columns); `Display`
            // escapes it with the rest of the line.
            Self::Undisplayable { column, .. } => column.clone(),
            Self::QuadrantCount { .. } | Self::OverfullBand { .. } => String::new(),
        }
    }

    fn message(&self) -> String {
        match self {
            Self::UndeclaredRing { value } => format!(
                "ring '{}' is not declared by --rings; the row is in the table view, not on the radar",
                value.trim()
            ),
            Self::UnrecognisedQualifier { value } => {
                format!("unrecognised qualifier value '{}'", value.trim())
            }
            Self::UnrecognisedStatus { value } => format!(
                "unrecognised status value '{}'; drawn as No Change",
                value.trim()
            ),
            Self::UnparsableDate { value, .. } => format!(
                "'{}' is not a YYYY-MM-DD date; shown as written",
                value.trim()
            ),
            Self::EmptyDescription => "description is empty; the detail panel omits it".into(),
            Self::DuplicateName { value, first_line } => format!(
                "name '{}' repeats line {first_line}; both are drawn",
                value.trim()
            ),
            Self::Undisplayable {
                first, count: 1, ..
            } => {
                format!(
                    "removed U+{:04X}, which the output cannot display",
                    u32::from(*first)
                )
            }
            Self::Undisplayable { first, count, .. } => format!(
                "removed {count} characters the output cannot display, the first U+{:04X}",
                u32::from(*first)
            ),
            Self::QuadrantCount { count } => format!(
                "{count} quadrants; the radar reads best with three to six, so sectors narrow and labels may crowd"
            ),
            Self::OverfullBand {
                quadrant,
                ring,
                count,
                fits,
            } => format!(
                "quadrant '{}', ring '{}': {count} items; about {fits} fit comfortably, so some markers overlap",
                quadrant.trim(),
                ring.trim()
            ),
        }
    }
}

impl fmt::Display for Warning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let line = match self.position {
            Position::Render => self.kind.message(),
            Position::Cell { line, .. } => {
                format!(
                    "line {line}, {}: {}",
                    self.kind.column(),
                    self.kind.message()
                )
            }
        };
        f.write_str(&escape_controls(&line))
    }
}

/// A CSV value can carry a newline inside quotes (NFR-10), and printed raw it
/// would forge a second diagnostic line; an escape sequence could recolour the
/// terminal, and a bidi override could reorder it. Every stderr line passes
/// through here whole, so no value can be forgotten; values are only trimmed.
fn escape_controls(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if c.is_control()
            || matches!(c, '\u{200e}' | '\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
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

/// NFR-20's warning classes. Render-level warnings have none: FR-88 fires
/// once and FR-89 promises one warning per band, so neither groups.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Class {
    Ring,
    Qualifier,
    Status,
    Date(DateColumn),
    Description,
    DuplicateName,
    Undisplayable,
}

impl WarningKind {
    fn class(&self) -> Option<Class> {
        match self {
            Self::UndeclaredRing { .. } => Some(Class::Ring),
            Self::UnrecognisedQualifier { .. } => Some(Class::Qualifier),
            Self::UnrecognisedStatus { .. } => Some(Class::Status),
            Self::UnparsableDate { column, .. } => Some(Class::Date(*column)),
            Self::EmptyDescription => Some(Class::Description),
            Self::DuplicateName { .. } => Some(Class::DuplicateName),
            Self::Undisplayable { .. } => Some(Class::Undisplayable),
            Self::QuadrantCount { .. } | Self::OverfullBand { .. } => None,
        }
    }

    /// The value NFR-20 enumerates; `None` for a class with no value.
    fn value(&self) -> Option<&str> {
        match self {
            Self::UndeclaredRing { value }
            | Self::UnrecognisedQualifier { value }
            | Self::UnrecognisedStatus { value }
            | Self::UnparsableDate { value, .. }
            | Self::DuplicateName { value, .. } => Some(value),
            Self::EmptyDescription
            | Self::Undisplayable { .. }
            | Self::QuadrantCount { .. }
            | Self::OverfullBand { .. } => None,
        }
    }
}

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
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// One string per stderr line, in NFR-11's order, grouped per NFR-20.
    #[must_use]
    pub fn render(&self) -> Vec<String> {
        // Stable: equal positions keep push order.
        let mut sorted: Vec<&Warning> = self.0.iter().collect();
        sorted.sort_by_key(|w| w.position);

        let mut lines = Vec::new();
        let mut printed: Vec<Class> = Vec::new();
        for w in &sorted {
            let Some(class) = w.kind.class() else {
                lines.push(format!("warning: {w}"));
                continue;
            };
            if printed.contains(&class) {
                continue;
            }
            printed.push(class);
            let members: Vec<&Warning> = sorted
                .iter()
                .copied()
                .filter(|m| m.kind.class() == Some(class))
                .collect();
            if members.len() == 1 {
                lines.push(format!("warning: {w}"));
            } else {
                lines.push(format!(
                    "warning: {}",
                    escape_controls(&grouped(class, &members))
                ));
            }
        }
        lines
    }
}

/// A distinct value within a class, in first-occurrence order.
struct Tally {
    key: String,
    shown: String,
    count: usize,
    first_line: usize,
    /// FR-28's earlier line, for a duplicate name; `None` for every other class.
    original: Option<usize>,
}

fn line_of(w: &Warning) -> usize {
    match w.position {
        Position::Cell { line, .. } => line,
        Position::Render => 0, // unreachable for a classed warning; 0 is never a CSV line
    }
}

fn grouped(class: Class, members: &[&Warning]) -> String {
    let n = members.len();
    let list = match class {
        // No value to enumerate, so NFR-20 lists positions, uncapped.
        Class::Description => {
            let lines: Vec<String> = members.iter().map(|m| line_of(m).to_string()).collect();
            format!("lines {}", lines.join(", "))
        }
        Class::Undisplayable => members
            .iter()
            .map(|m| format!("line {} {}", line_of(m), m.kind.column()))
            .collect::<Vec<_>>()
            .join(", "),
        _ => tallied(members),
    };
    match class {
        Class::Ring => format!(
            "{n} rows use a ring --rings does not declare — {list}; all appear in the table view"
        ),
        Class::Qualifier => {
            format!("{n} rows have an unrecognised qualifier — {list}; each is drawn solid")
        }
        Class::Status => {
            format!("{n} rows have an unrecognised status — {list}; each is drawn as No Change")
        }
        Class::Date(column) => format!(
            "{n} rows have {column} values that are not YYYY-MM-DD dates — {list}; each is shown as written"
        ),
        Class::Description => {
            format!("{n} rows have an empty description — {list}; the detail panel omits each")
        }
        Class::DuplicateName => {
            format!("{n} rows repeat an earlier name — {list}; every row is drawn")
        }
        Class::Undisplayable => {
            format!(
                "{n} fields held characters the output cannot display — {list}; each was removed"
            )
        }
    }
}

/// NFR-20's enumeration: each distinct value, in first-occurrence order, with
/// its count and first line.
fn tallied(members: &[&Warning]) -> String {
    let mut tallies: Vec<Tally> = Vec::new();
    for m in members {
        let value = m.kind.value().unwrap_or_default();
        let key = normalise(value);
        if let Some(t) = tallies.iter_mut().find(|t| t.key == key) {
            t.count += 1;
        } else {
            let original = match m.kind {
                WarningKind::DuplicateName { first_line, .. } => Some(first_line),
                _ => None,
            };
            tallies.push(Tally {
                key,
                shown: value.trim().to_string(),
                count: 1,
                first_line: line_of(m),
                original,
            });
        }
    }
    tallies
        .iter()
        .map(|t| {
            let at = match t.count {
                1 => format!("1, line {}", t.first_line),
                c => format!("{c}, first line {}", t.first_line),
            };
            match t.original {
                Some(o) => format!("{} ({at}; original line {o})", t.shown),
                None => format!("{} ({at})", t.shown),
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// Everything the tool refuses to run on: non-zero exit, nothing written
/// (NFR-8, NFR-9). One taxonomy for the whole tool (architecture §1); each
/// later stage adds its variants beside the code that raises them.
#[derive(Debug, thiserror::Error)]
pub enum Fatal {
    /// FR-52: the `-o` extension is absent or names a format we do not write.
    #[error("-o {path}: the output file name must end in .html")]
    UnrecognisedExtension { path: String },
    /// §14.1: the extension is one v1.1 adds, so say that rather than
    /// "unrecognised", which sends the caller hunting for a typo in a path
    /// that is correct.
    #[error("-o: .{ext} output arrives in radargen {arrives_in}; this version writes .html")]
    FormatNotInThisVersion {
        ext: String,
        arrives_in: &'static str,
    },
    /// FR-63, NFR-16 tier one.
    #[error("--rings: {0}")]
    Rings(#[from] RingsError),
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
