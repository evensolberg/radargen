//! The diagnostic taxonomy (architecture §3, §7): what is fatal, what warns,
//! and how warnings reach stderr (NFR-10, NFR-11, NFR-20).

use std::fmt;

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
