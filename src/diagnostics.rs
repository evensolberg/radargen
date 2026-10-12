//! The diagnostic taxonomy (architecture §3, §7): what is fatal, what warns,
//! and how warnings reach stderr (NFR-10, NFR-11, NFR-20).

use std::fmt;
use std::sync::Arc;

use crate::columns::ColumnKind;
use crate::rings::{RingSource, RingsError};
use crate::style::{MIN_TARGET_PX, NARROW_MAX_WIDTH, NARROW_MIN_VIEWPORT_PX};
use crate::text::FontError;
use crate::{is_untagged_format, normalise, trim};

/// Where a warning points: its sort key.
///
/// `Render` concerns the render as a whole (FR-88, FR-89) and has no line.
/// It is declared first so the derived order puts it before every row
/// warning, which is NFR-11's order once FR-88 is placed (architecture §7) —
/// a sort key, not a special case.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Position {
    /// The render as a whole; sorts before every cell.
    Render,
    /// `line` is the 1-based physical line the record begins on, counting the
    /// header as line 1 (NFR-10, D-37). `column` is the header index, which
    /// orders warnings within a line (NFR-11).
    Cell {
        /// The record's first physical line.
        line: usize,
        /// The header index of the cell.
        column: usize,
    },
}

/// FR-24 and FR-25 share a parsing rule but are separate columns, and NFR-10
/// requires the message to name the column — so they are separate classes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DateColumn {
    /// The `last_affirmed` column (FR-24).
    LastAffirmed,
    /// The `exit_date` column (FR-25).
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
    /// FR-17. `source` is what the message says to edit (D-61); `rings` are
    /// the declared rings it lists, so the author sees what would match.
    UndeclaredRing {
        /// The ring cell as the CSV had it.
        value: String,
        /// Whether the ring list came from `--rings` or the default (D-61).
        source: RingSource,
        /// The declared rings, edge-to-centre, shared by every warning of
        /// the run.
        rings: Arc<[String]>,
    },
    /// FR-20 — non-blank only; a blank qualifier claims nothing (D-35).
    UnrecognisedQualifier {
        /// The qualifier cell as the CSV had it.
        value: String,
    },
    /// FR-26 — non-blank only, as FR-20.
    UnrecognisedStatus {
        /// The status cell as the CSV had it.
        value: String,
    },
    /// FR-24, FR-25.
    UnparsableDate {
        /// Which of the two date columns the cell was under.
        column: DateColumn,
        /// The date cell as the CSV had it.
        value: String,
    },
    /// FR-7.
    EmptyDescription,
    /// FR-28, reported at the later line naming the earlier (NFR-10).
    DuplicateName {
        /// The repeated name as the CSV had it.
        value: String,
        /// The 1-based physical line of the earlier occurrence (D-37).
        first_line: usize,
        /// The earlier occurrence's name as the CSV had it.
        first_value: String,
    },
    /// FR-90: one per value, however many characters it lost. `column` is the
    /// canonical header name, since any column can carry one.
    Undisplayable {
        /// The canonical header name of the affected column; an unnamed
        /// column is named by [`column_label`].
        column: String,
        /// The first character the value lost.
        first: char,
        /// How many characters the value lost in all.
        count: usize,
    },
    /// FR-93: a value under a column whose header cell is empty. `column`
    /// is [`column_label`], since there is no name.
    UnnamedValue {
        /// [`column_label`] of the nameless column: `column 6 (F)`.
        column: String,
    },
}

/// The warnings that concern the render as a whole. They never group: FR-88
/// fires once and FR-89 promises one warning per band.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderKind {
    /// FR-88.
    QuadrantCount {
        /// How many distinct quadrants the CSV used.
        count: usize,
    },
    /// FR-89, one per overfull band.
    OverfullBand {
        /// The quadrant name the band belongs to.
        quadrant: String,
        /// The ring name the band belongs to.
        ring: String,
        /// How many items the band holds.
        count: usize,
        /// How many of them the band has room for.
        fits: usize,
    },
    /// FR-95: the narrow arrangement is wider than FR-94's targets are
    /// sized for (D-71), and `binding` sets its width (D-73).
    NarrowWidened {
        /// The label whose reach sets the radar's width.
        binding: Binding,
        /// The narrow document's width, rounded up to a whole unit.
        width: u32,
    },
    /// FR-90 on a flag value: one per flag, however many characters it lost.
    FlagUndisplayable {
        /// The flag whose value lost characters.
        flag: Flag,
        /// The first character the value lost.
        first: char,
        /// How many characters the value lost in all.
        count: usize,
    },
}

/// The label whose reach sets the radar's width (FR-95, D-73): a quadrant
/// name from the CSV, or a ring name from `--rings`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Binding {
    /// A quadrant label, as written.
    Quadrant(String),
    /// A ring name, as declared, and where it was declared.
    Ring {
        /// The ring's name.
        name: String,
        /// `--rings`, or the default set.
        source: RingSource,
    },
}

/// A command-line flag whose value FR-90 cleans. `--config` keys arrive with
/// PR 19 and will name themselves the same way.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Flag {
    /// `--title` (FR-65).
    Title,
    /// `--subtitle` (FR-65).
    Subtitle,
    /// `--rings` (FR-12).
    Rings,
}

impl fmt::Display for Flag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Title => "--title",
            Self::Subtitle => "--subtitle",
            Self::Rings => "--rings",
        })
    }
}

/// One warning. Split by kind, so a row warning always has a line and a
/// render-level one never claims one (NFR-10) — by type, not by a check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Warning {
    /// A warning about the render as a whole (FR-88, FR-89).
    Render(RenderKind),
    /// A warning about one CSV cell.
    Row {
        /// As in [`Position::Cell`].
        line: usize,
        /// As in [`Position::Cell`].
        column: usize,
        /// What was wrong with the cell.
        kind: RowKind,
    },
}

impl Warning {
    /// Where the warning sorts (NFR-11).
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
            Self::UndeclaredRing { .. } => ColumnKind::Ring.as_str(),
            Self::UnrecognisedQualifier { .. } => ColumnKind::Qualifier.as_str(),
            Self::UnrecognisedStatus { .. } => ColumnKind::Status.as_str(),
            Self::UnparsableDate { column, .. } => column.as_str(),
            Self::EmptyDescription => ColumnKind::Description.as_str(),
            Self::DuplicateName { .. } => ColumnKind::Name.as_str(),
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
            Self::UndeclaredRing { source, rings, .. } => {
                let list = rings.join(", ");
                match source {
                    RingSource::Default => format!(
                        "'{v}' is not one of the default rings ({list}); the row is in the table view, not on the radar; to plot it, pass --rings with every ring you want, since --rings replaces the default set"
                    ),
                    RingSource::Flag => format!(
                        "'{v}' is not one of the rings --rings declares ({list}); the row is in the table view, not on the radar; to plot it, add it to --rings"
                    ),
                }
            }
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
            Self::DuplicateName {
                first_line,
                first_value,
                ..
            } => format!(
                "'{v}' repeats line {first_line}'s name '{}' (names match ignoring case and surrounding spaces); both are drawn, so rename or delete one if they are the same item",
                trim(first_value)
            ),
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
            Self::NarrowWidened { binding, width } => {
                let fix = match binding {
                    Binding::Quadrant(name) => format!("quadrant '{}'", trim(name)),
                    Binding::Ring {
                        name,
                        source: RingSource::Flag,
                    } => format!("ring '{}' in --rings", trim(name)),
                    Binding::Ring {
                        name,
                        source: RingSource::Default,
                    } => format!("ring '{}', one of the default rings,", trim(name)),
                };
                format!(
                    "the narrow arrangement is {width} units wide, past the {NARROW_MAX_WIDTH} its legend rows and marker targets are sized for, so on a {NARROW_MIN_VIEWPORT_PX}-pixel screen they fall below {MIN_TARGET_PX} pixels; a shorter name for {fix} would fix it"
                )
            }
            Self::FlagUndisplayable {
                flag,
                first,
                count: 1,
            } => format!(
                "{flag}: removed {}, which the output cannot display",
                code_point(*first)
            ),
            Self::FlagUndisplayable { flag, first, count } => format!(
                "{flag}: removed {count} characters the output cannot display, the first {}",
                code_point(*first)
            ),
        }
    }
}

/// An I/O failure's reason as a message shows it: the OS's own words, without
/// the "(os error N)" suffix or a trailing full stop, starting lowercase, so
/// every path that fails to read or write reads the same way (rg-bsb).
fn io_reason(error: &std::io::Error) -> String {
    let text = error.to_string();
    let words = text
        .split_once(" (os error ")
        .map_or(text.as_str(), |(words, _)| words)
        .trim_end_matches('.');
    let mut chars = words.chars();
    chars.next().map_or_else(String::new, |first| {
        first.to_lowercase().chain(chars).collect()
    })
}

/// A column as NFR-10 names it: `column 6 (F)`.
///
/// The 1-based number and the spreadsheet letters, from the 0-based header
/// `index`. The one place an index becomes a label, so no message counts
/// differently.
#[must_use]
pub fn column_label(index: usize) -> String {
    format!("column {} ({})", index + 1, column_letters(index + 1))
}

/// Spreadsheet letters for a 1-based column number: A..Z, AA..AZ, BA..
// qual:allow(complexity, error_handling) reason: "a remainder of 26 always fits in u8"
fn column_letters(number: usize) -> String {
    const ALPHABET: usize = 26;
    let mut letters = Vec::new();
    let mut rest = number;
    while rest > 0 {
        rest -= 1;
        let offset = u8::try_from(rest % ALPHABET).expect("a remainder of 26 is below 26");
        letters.push(char::from(b'A' + offset));
        rest /= ALPHABET;
    }
    letters.into_iter().rev().collect()
}

/// `one` when `n` is 1, otherwise `many`: the count-agreeing noun in a message.
#[must_use]
pub const fn plural<'a>(n: usize, one: &'a str, many: &'a str) -> &'a str {
    if n == 1 { one } else { many }
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
        if needs_escape(c) {
            out.extend(c.escape_default());
        } else {
            out.push(c);
        }
    }
    out
}

/// Control characters; the Unicode line and paragraph separators, which break
/// a line in some terminals and log viewers; and every format character (Cf),
/// bidi controls included, so an invisible character inside a value is shown.
/// The joiners and the tag characters are kept: they build emoji sequences and
/// subdivision flags, which would otherwise be unreadable (D-61, D-64).
fn needs_escape(c: char) -> bool {
    c.is_control()
        || matches!(c, '\u{2028}' | '\u{2029}')
        || (is_untagged_format(c) && !matches!(c, '\u{200c}' | '\u{200d}'))
}

/// Warnings collected over a run (NFR-16 tier three).
///
/// A plain list, grouped only when printed: NFR-20's grouping needs the whole
/// run, and grouping at push time would need a map keyed by value, which §6
/// rules out of output-affecting code (architecture §7).
#[derive(Debug, Default)]
pub struct Warnings(Vec<Warning>);

impl Warnings {
    /// Records a warning; order is kept until printing (NFR-11).
    pub fn push(&mut self, w: Warning) {
        self.0.push(w);
    }

    /// The raw warnings in push order, so a stage's tests can assert on
    /// kinds and positions rather than on rendered English.
    #[must_use]
    pub fn as_slice(&self) -> &[Warning] {
        &self.0
    }

    /// Whether the run raised no warnings.
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
        Class::Ring(source) => grouped_rings(source, members),
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
            "{n} rows repeat an earlier name — {}; every row is drawn, so rename or delete the repeats that are the same item",
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
            "{n} values had characters the output cannot display — {}; each was removed",
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

/// FR-17's grouped line: it lists the declared rings and says what to edit,
/// which depends on where the ring set came from (D-61).
// qual:allow(complexity, error_handling) reason: "a Ring class is built only from UndeclaredRing rows, so a member is always found"
fn grouped_rings(source: RingSource, members: &[Member]) -> String {
    let n = members.len();
    let list = members
        .iter()
        .find_map(|m| match m.kind {
            RowKind::UndeclaredRing { rings, .. } => Some(rings.join(", ")),
            _ => None,
        })
        .expect("a Ring class holds only UndeclaredRing members");
    let tallies = tallied(members, normalise);
    match source {
        RingSource::Default => format!(
            "{n} rows use a ring that is not one of the default rings ({list}) — {tallies}; all appear in the table view; to plot one, pass --rings with every ring you want, since --rings replaces the default set"
        ),
        RingSource::Flag => format!(
            "{n} rows use a ring --rings does not declare ({list}) — {tallies}; all appear in the table view; to plot one, add it to --rings"
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
        let line = first.line;
        match (first.kind, count) {
            // A duplicate leads with the earlier row's spelling and line, so
            // "first" can only mean the first repeat.
            (
                RowKind::DuplicateName {
                    first_line,
                    first_value,
                    ..
                },
                c,
            ) => {
                let shown = trim(first_value);
                if *c == 1 {
                    format!("{shown} (line {first_line}; repeated on line {line})")
                } else {
                    format!("{shown} (line {first_line}; repeated on {c} rows, first line {line})")
                }
            }
            (kind, c) => {
                let shown = trim(kind.value().unwrap_or_default());
                if *c == 1 {
                    format!("{shown} (1, line {line})")
                } else {
                    format!("{shown} ({c}, first line {line})")
                }
            }
        }
    }))
}

/// FR-1: a separator radargen refuses, found where a comma belongs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Separator {
    /// `;`, which Excel writes where a comma is the decimal mark.
    Semicolon,
    /// `\t`, which spreadsheets export as tab-separated values.
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
    UnrecognisedExtension {
        /// The `-o` argument as given.
        path: String,
    },
    /// §14.1: the extension is one v1.1 adds, so say that rather than
    /// "unrecognised", which sends the caller hunting for a typo in a path
    /// that is correct.
    #[error("-o {path}: .{ext} output arrives in radargen {arrives_in}; this version writes .html")]
    FormatNotInThisVersion {
        /// The `-o` argument as given.
        path: String,
        /// The extension, without its dot.
        ext: String,
        /// The release that adds the format (§14.1).
        arrives_in: &'static str,
    },
    /// FR-63, NFR-16 tier one.
    #[error("--rings: {0}")]
    Rings(#[from] RingsError),
    /// FR-62: `-o`'s directory does not exist; the tool does not create it.
    #[error("-o {path}: the directory {dir} does not exist")]
    MissingDirectory {
        /// The `-o` argument as given.
        path: String,
        /// The directory part of it.
        dir: String,
    },
    /// FR-62, D-74: `-o` is the input CSV, which the page would replace.
    #[error("-o {path}: is the input CSV; choose another output path")]
    OutputIsInput {
        /// The `-o` argument as given.
        path: String,
    },
    /// NFR-15: an embedded font face does not parse.
    #[error(transparent)]
    Font(#[from] FontError),
    /// FR-61: the input does not exist, cannot be read, or is a directory.
    #[error("{path}: cannot be read: {}", io_reason(source))]
    Unreadable {
        /// The input path as given.
        path: String,
        /// The underlying I/O failure.
        #[source]
        source: std::io::Error,
    },
    /// NFR-15: the output cannot be written: its directory is missing or
    /// unwritable, the path names a directory or a read-only file, or the
    /// disk is full. Nothing at `-o` has changed (NFR-14).
    #[error("-o {path}: cannot be written: {}", io_reason(source))]
    Unwritable {
        /// The `-o` argument as given.
        path: String,
        /// The underlying I/O failure.
        #[source]
        source: std::io::Error,
    },
    /// FR-61, D-27: the input is not UTF-8, or holds a NUL byte (FR-61, D-65).
    /// Say what to do — the likely cause is an Excel export in a legacy code
    /// page, and "invalid utf-8" alone does not tell the author that. `line`
    /// is the physical line holding the first bad byte.
    #[error(
        "{path}, line {line}: not UTF-8 text; re-save the file as UTF-8 (in Excel, Save As \"CSV UTF-8\")"
    )]
    NotUtf8 {
        /// The input path as given.
        path: String,
        /// The physical line holding the first bad byte, or the first NUL
        /// (FR-61, D-65).
        line: usize,
    },
    /// NFR-15: an unterminated quote swallows the rest of the file into one
    /// field, so nothing after it parses. `line` is where the quote opened,
    /// which may be after the record's first line (plan RG-005, Decisions).
    #[error(
        "line {line}, {}: a quote opens this value and never closes; add the closing quote, and write any quote inside the value as \"\"",
        column_label(*column)
    )]
    UnterminatedQuote {
        /// The 1-based line where the quote opened.
        line: usize,
        /// The 0-based header index of the column it opened in.
        column: usize,
    },
    /// NFR-15: text after a closing quote. RFC 4180 doubles a quote inside a
    /// quoted field; keeping or dropping the stray text would both guess.
    /// `line` is the closing quote's.
    #[error(
        "line {line}, {}: text follows a closing quote; inside quotes, write a quote as \"\"",
        column_label(*column)
    )]
    TextAfterQuote {
        /// The 1-based line of the closing quote.
        line: usize,
        /// The 0-based header index of the column it closed.
        column: usize,
    },
    /// FR-3: the file has no non-blank records, so no header names a column.
    /// The required columns are FR-5's, named so the message is actionable.
    #[error(
        "the CSV has no header row (it is empty or blank); the header must name at least name, quadrant, ring and description"
    )]
    NoHeader,
    /// FR-1: a header with `;` or a tab between its names, or after a closing
    /// quote, is a file with another separator — Excel writes `;` where a
    /// comma is the decimal mark. Without this, every row would mis-report
    /// its field count.
    #[error(
        "line {line}: the header has {separator} between its names, not commas; radargen reads comma-separated files only, so export the file with commas as the separator"
    )]
    NotCommaSeparated {
        /// The header's line.
        line: usize,
        /// The separator found where a comma belongs.
        separator: Separator,
    },
    /// FR-4: a header name repeated under FR-76's normalisation, so
    /// `Name` and `name ` collide. Indices are 0-based, as
    /// [`column_label`] takes them; names are as
    /// written, trimmed, so the author can find both cells.
    #[error(
        "line {line}: {} '{name}' repeats {} '{first_name}'; header names match ignoring case and surrounding spaces, so rename or remove one",
        column_label(*index),
        column_label(*first_index)
    )]
    DuplicateHeader {
        /// The header's line.
        line: usize,
        /// The 0-based header index of the repeating column.
        index: usize,
        /// Its name, as written and trimmed.
        name: String,
        /// The 0-based header index of the earlier column it collides with.
        first_index: usize,
        /// The earlier column's name, as written and trimmed.
        first_name: String,
    },
    /// FR-4: a record whose field count is not the header's, with the usual
    /// cause for each direction (FR-61's precedent: say what to do). This is
    /// the one-row message; [`crate::errors::render_errors`] groups several
    /// rows (D-62) and adds the header-first cause, which needs the whole table.
    #[error(
        "{}",
        one_row(
            &Mismatch {
                line: *line,
                found: *found,
                expected: *expected,
                unnamed: *unnamed,
                padded: *padded,
            },
            false
        )
    )]
    FieldCount {
        /// The record's first line.
        line: usize,
        /// How many values the record has.
        found: usize,
        /// How many columns the header has.
        expected: usize,
        /// How many of the header's cells are empty under FR-76 (FR-93).
        unnamed: usize,
        /// Whether the record has more values than the header and every one
        /// past the header's last column is empty under FR-76's trim.
        padded: bool,
    },
    /// FR-5: the header lacks a required column. Every missing one is
    /// reported (NFR-16 tier two). `line` is the header's.
    #[error(
        "line {line}: the header has no '{}' column; add one, or rename the column that holds it — the header must name at least name, quadrant, ring and description",
        column.as_str()
    )]
    MissingColumn {
        /// The header's line.
        line: usize,
        /// The missing column.
        column: ColumnKind,
    },
    /// FR-6: an empty `name`, `quadrant` or `ring`, empty under FR-76's
    /// trim, so a cell of spaces or a lone zero-width space counts.
    #[error(
        "line {line}, {0}: empty; every row needs a {0}, so fill it in or delete the row",
        column.as_str()
    )]
    EmptyRequired {
        /// The row's line.
        line: usize,
        /// The empty column.
        column: ColumnKind,
    },
}

impl Fatal {
    /// The stderr line.
    ///
    /// Tier-two row errors (`FieldCount`, `DuplicateHeader`, `MissingColumn`,
    /// `EmptyRequired`) must be printed through [`crate::errors::render_errors`],
    /// which groups them (D-62) and adds FR-4's header-first cause that a
    /// single error cannot know.
    #[must_use]
    pub fn render(&self) -> String {
        // Escaped as a whole, so no variant can forget to: `-o` and `--rings`
        // values come from argv, and later variants carry CSV text (NFR-10).
        error_line(&self.to_string())
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

/// FR-4's closing causes for the rows seen, each a clause, joined with `; `
/// (NFR-20).
///
/// `no_match` is "no data row matches the header" and says to look at the
/// header first. The padding clause needs every row with more values to end in
/// empty cells; one that does not sends them all to the comma clause.
pub(crate) fn row_causes(no_match: bool, rows: &[&Mismatch]) -> String {
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
pub(crate) fn unnamed_clause(unnamed: usize) -> String {
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
        row_causes(no_match, &[m])
    )
}

/// A stderr line for `message`: `error: ` and the message, escaped whole.
#[must_use]
pub(crate) fn error_line(message: &str) -> String {
    format!("error: {}", escape_controls(message))
}
