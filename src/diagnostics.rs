//! The diagnostic taxonomy (architecture §3, §7): what is fatal, what warns,
//! and how warnings reach stderr (NFR-10, NFR-11, NFR-20).

use std::fmt::{self, Write as _};
use std::sync::Arc;

use crate::cli::OutputFormat;
use crate::columns::ColumnKind;
use crate::csv::decode::MAX_BYTES as MAX_CSV_BYTES;
use crate::emit::html::Table;
use crate::palette::file::{MAX_BYTES, PaletteError};
use crate::rings::RingsError;
use crate::settings::{NamedPath, Origin, Setting, Source};
use crate::starter::Starter;
use crate::style::{MIN_TARGET_PX, NARROW_MAX_WIDTH, NARROW_MIN_VIEWPORT_PX, WIDE_MIN_VIEWPORT_PX};
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
    /// FR-17, for a ring neither declared nor named by `--unplotted-rings`
    /// (FR-107). `source` is what the message says to edit (D-61); `rings`
    /// are the declared rings it lists, so the author sees what would match.
    UndeclaredRing {
        /// The ring cell as the CSV had it.
        value: String,
        /// Whether the ring list came from `--rings` or the default (D-61).
        source: Source,
        /// The declared rings, edge-to-centre, shared by every warning of
        /// the run.
        rings: Arc<[String]>,
        /// Where the run puts the row, which the message says (FR-106).
        off_radar: OffRadar,
        /// Whether the message suggests `--unplotted-rings`: only when the
        /// run was not given it, since an author already using the flag
        /// knows it, and the row is then most likely a typo (D-96).
        suggest_unplotted: bool,
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
    /// FR-96: a non-blank `link` that is not an `http` or `https` address.
    RefusedLink {
        /// The link cell as the CSV had it.
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
    /// FR-104: a text a PNG or SVG draws, from this cell, holds characters
    /// the bundled font lacks. One per drawn text, however many it lacks.
    MissingGlyphs {
        /// The column the text was read from: `name` or `quadrant`.
        column: ColumnKind,
        /// The text and what it lacks.
        lacking: Lacking,
    },
    /// FR-108: a header cell, on the header's line, whose name is not a
    /// known column but nearly is one. One per header, not per row.
    NearMiss {
        /// The header cell as the CSV had it.
        heading: String,
        /// The known column it nearly names.
        known: ColumnKind,
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
    /// FR-105: FR-69's breakpoint is past `WIDE_MIN_VIEWPORT_PX`, so a
    /// laptop gets the narrow arrangement, and `binding` sets the wide
    /// document's width (D-94).
    WideBeyondLaptops {
        /// The label whose reach sets the wide radar's width.
        binding: Binding,
        /// The breakpoint: the narrowest screen, in pixels, that shows the
        /// wide arrangement.
        breakpoint: u32,
    },
    /// RG-102: `--legend hidden` on a PNG or SVG, which always draw the
    /// legend.
    LegendStatic,
    /// FR-106: `--table all` on a PNG or SVG, which have no table.
    TableStatic,
    /// D-87: the document is wider than FR-64's ceiling and `--width` was
    /// not given, so the PNG is drawn at the ceiling.
    WidthClamped {
        /// The document's own width, rounded up.
        natural: u32,
        /// The width drawn.
        drawn: u32,
    },
    /// D-87: the PNG's legend text draws below `MIN_LEGIBLE_PX`.
    PngTextSmall {
        /// The width drawn.
        width: u32,
        /// The least width that keeps it legible, if one within FR-64 does.
        legible_from: Option<u32>,
        /// Where the width came from, which the remedy names (NFR-10).
        width_source: Source,
    },
    /// FR-90 on a flag value: one per flag, however many characters it lost.
    FlagUndisplayable {
        /// The setting whose value it is, and where that value came from
        /// (NFR-10).
        flag: Origin,
        /// The first character the value lost.
        first: char,
        /// How many characters the value lost in all.
        count: usize,
    },
    /// FR-104 on a flag value: one per drawn text, so one per `--rings`
    /// entry, however many characters it lacks.
    FlagMissingGlyphs {
        /// The setting whose value it is, and where that value came from
        /// (NFR-10).
        flag: Origin,
        /// The text, the value or one `--rings` entry, and what it lacks.
        lacking: Lacking,
    },
}

/// FR-104: a text as a PNG or SVG draws it, with characters the bundled
/// font lacks. Its `Display` is the message after the position or flag.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lacking {
    /// The text as drawn.
    pub text: String,
    /// The first character the font lacks.
    pub first: char,
    /// How many distinct characters it lacks.
    pub count: usize,
    /// The snapshot being written, which draws the boxes.
    pub format: OutputFormat,
}

impl fmt::Display for Lacking {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            text,
            first,
            count,
            format,
        } = self;
        let (first, ending) = (glyph(*first), boxes(*format, *count));
        if *count == 1 {
            write!(
                f,
                "'{text}' has {first}, which the bundled font lacks; {ending}"
            )
        } else {
            write!(
                f,
                "'{text}' has {count} characters the bundled font lacks, the first {first}; {ending}"
            )
        }
    }
}

/// The label whose reach sets the radar's width, for FR-95 and FR-105
/// (D-73, D-94): a quadrant name from the CSV, or a ring name from
/// `--rings` or the default set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Binding {
    /// A quadrant label, as written.
    Quadrant(String),
    /// A ring name, as declared, and where it was declared.
    Ring {
        /// The ring's name.
        name: String,
        /// `--rings`, or the default set.
        source: Source,
    },
}

/// The label as FR-95 and FR-105 name it: trimmed and single-quoted, a
/// ring with where to rename it (D-73).
impl fmt::Display for Binding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Quadrant(name) => write!(f, "quadrant '{}'", trim(name)),
            Self::Ring {
                name,
                source: Source::Default,
            } => write!(
                f,
                "ring '{}' (a default ring, so pass --rings to rename it)",
                trim(name)
            ),
            Self::Ring { name, source } => {
                write!(f, "ring '{}' in {}", trim(name), ring_list(source))
            }
        }
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
#[derive(Debug, Clone, PartialEq, Eq)]
enum Class {
    /// One run declares one ring set, so the source never splits a class;
    /// it rides here so the grouped line can name it.
    Ring(Source),
    Qualifier,
    Status,
    Date(DateColumn),
    Link,
    Description,
    DuplicateName,
    Undisplayable,
    Unnamed,
    NearMiss,
    /// The format rides here, as a ring set's source does: one run writes
    /// one format.
    MissingGlyphs(OutputFormat),
}

impl RowKind {
    fn class(&self) -> Class {
        match self {
            Self::UndeclaredRing { source, .. } => Class::Ring(source.clone()),
            Self::UnrecognisedQualifier { .. } => Class::Qualifier,
            Self::UnrecognisedStatus { .. } => Class::Status,
            Self::UnparsableDate { column, .. } => Class::Date(*column),
            Self::RefusedLink { .. } => Class::Link,
            Self::EmptyDescription => Class::Description,
            Self::DuplicateName { .. } => Class::DuplicateName,
            Self::Undisplayable { .. } => Class::Undisplayable,
            Self::UnnamedValue { .. } => Class::Unnamed,
            Self::MissingGlyphs { lacking, .. } => Class::MissingGlyphs(lacking.format),
            Self::NearMiss { .. } => Class::NearMiss,
        }
    }

    /// The column this warning concerns, as NFR-10 names it.
    fn column_name(&self) -> &str {
        match self {
            Self::UndeclaredRing { .. } => ColumnKind::Ring.as_str(),
            Self::UnrecognisedQualifier { .. } => ColumnKind::Qualifier.as_str(),
            Self::UnrecognisedStatus { .. } => ColumnKind::Status.as_str(),
            Self::UnparsableDate { column, .. } => column.as_str(),
            Self::RefusedLink { .. } => ColumnKind::Link.as_str(),
            Self::EmptyDescription => ColumnKind::Description.as_str(),
            Self::DuplicateName { .. } => ColumnKind::Name.as_str(),
            Self::MissingGlyphs { column, .. } => column.as_str(),
            // A header name is CSV text (FR-18's extra columns); it is escaped
            // with the rest of the line.
            Self::Undisplayable { column, .. } | Self::UnnamedValue { column } => column,
            // The header as written, so the author finds the cell to fix.
            Self::NearMiss { heading, .. } => trim(heading),
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
            | Self::RefusedLink { value }
            | Self::DuplicateName { value, .. }
            | Self::UnnamedValue { column: value } => Some(value),
            Self::EmptyDescription
            | Self::Undisplayable { .. }
            | Self::MissingGlyphs { .. }
            | Self::NearMiss { .. } => None,
        }
    }

    fn message(&self) -> String {
        let v = self.value().map(trim).unwrap_or_default();
        match self {
            Self::UndeclaredRing {
                source,
                rings,
                off_radar,
                suggest_unplotted,
                ..
            } => off_radar.ring_row(v, source, &rings.join(", "), *suggest_unplotted),
            Self::UnrecognisedQualifier { .. } => {
                format!("unrecognised qualifier value '{v}'; drawn solid")
            }
            Self::UnrecognisedStatus { .. } => {
                format!("unrecognised status value '{v}'; drawn as No Change")
            }
            Self::UnparsableDate { .. } => {
                format!("'{v}' is not a YYYY-MM-DD date; shown as written")
            }
            Self::RefusedLink { .. } => {
                format!("'{v}' is not an http or https address; shown as text, not a link")
            }
            Self::EmptyDescription => "empty; the detail panel omits it".into(),
            Self::DuplicateName {
                first_line,
                first_value,
                ..
            } => format!(
                "'{v}' repeats line {first_line}'s name '{}' (names match ignoring case and extra spaces); both are drawn, so rename or delete one if they are the same item",
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
            Self::MissingGlyphs { lacking, .. } => lacking.to_string(),
            // True under either --table: the column is never read, though
            // `--table all` publishes it (FR-106, D-98).
            Self::NearMiss { known, .. } => format!(
                "not a column radargen knows; did you mean '{}'?",
                known.as_str()
            ),
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
            Self::NarrowWidened { binding, width } => format!(
                "the narrow layout is {width} units wide, past the {NARROW_MAX_WIDTH} its legend rows and marker targets are sized for, so on a {NARROW_MIN_VIEWPORT_PX}-pixel screen they fall below {MIN_TARGET_PX} pixels; a shorter name for {binding} would fix it"
            ),
            Self::WideBeyondLaptops {
                binding,
                breakpoint,
            } => format!(
                "the wide layout needs a screen at least {breakpoint} pixels wide, so {WIDE_MIN_VIEWPORT_PX}-pixel laptops get the narrow layout; a shorter name for {binding} would fix it"
            ),
            Self::LegendStatic => {
                "--legend hidden applies to HTML pages; PNG and SVG always draw the legend"
                    .to_owned()
            }
            Self::TableStatic => {
                "--table all applies to HTML pages; PNG and SVG have no table".to_owned()
            }
            Self::WidthClamped { natural, drawn } => format!(
                "the radar is {natural} pixels wide, past --width's limit; the PNG is drawn {drawn} wide, or pass --width to choose"
            ),
            Self::PngTextSmall {
                width,
                legible_from: Some(from),
                width_source: Source::Config(file),
            } => format!(
                "at {width} pixels wide the PNG's text is too small to read; setting the width key in {file} to {from} or more keeps it legible"
            ),
            Self::PngTextSmall {
                width,
                legible_from: None,
                width_source: Source::Config(_),
            } => format!(
                "at {width} pixels wide the PNG's text is too small to read, at any width; fewer or shorter names would help"
            ),
            Self::PngTextSmall {
                width,
                legible_from: Some(from),
                width_source: _,
            } => format!(
                "at {width} pixels wide the PNG's text is too small to read; --width {from} or more keeps it legible"
            ),
            Self::PngTextSmall {
                width,
                legible_from: None,
                width_source: _,
            } => format!(
                "at {width} pixels wide the PNG's text is too small to read, at any --width; fewer or shorter names would help"
            ),
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
            Self::FlagMissingGlyphs { flag, lacking } => format!("{flag}: {lacking}"),
        }
    }
}

/// An I/O failure's reason as a message shows it: the OS's own words, without
/// the "(os error N)" suffix or a trailing full stop, starting lowercase, so
/// every read and write failure takes the same form (rg-bsb). Lowercasing is
/// deliberate even where a message opens with a proper noun ("windows
/// cannot…"): it reads as a clause after the colon. One path differs in
/// words, not form: tempfile hides the OS error, so a failure creating the
/// temporary file gives Rust's name for its kind.
fn io_reason(error: &std::io::Error) -> String {
    let text = error.to_string();
    let words = text
        .rsplit_once(" (os error ")
        .map_or(text.as_str(), |(words, _)| words);
    let words = words.strip_suffix('.').unwrap_or(words);
    let mut chars = words.chars();
    chars.next().map_or_else(String::new, |first| {
        first.to_lowercase().chain(chars).collect()
    })
}

/// D-87's remedy for an image too large to draw.
fn smaller_width(source: &Source) -> String {
    match source {
        Source::Config(file) => format!("set the width key in {file} smaller"),
        Source::Flag | Source::Default => "pass a smaller --width".to_owned(),
    }
}

/// A palette problem as stderr shows it: located when it has a line.
fn palette_problem(palette: &NamedPath, error: &PaletteError) -> String {
    error.line().map_or_else(
        || format!("{palette}: {error}"),
        |line| format!("{palette}, line {line}: {error}"),
    )
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

/// FR-104's ending, which says what to do about `count` boxes in `format`.
fn boxes(format: OutputFormat, count: usize) -> String {
    let (them, boxes) = (plural(count, "it", "them"), plural(count, "a box", "boxes"));
    format!(
        "the {format} draws {them} as {boxes} — an HTML page shows {them}, or transliterate {them}"
    )
}

/// A missing character as FR-104 names it: `U+4E2D 中`, or the code point
/// alone where the character shows nothing a reader can match. A mark would
/// combine with the space before it, and whitespace or a control, format,
/// unassigned or private-use character is invisible.
fn glyph(c: char) -> String {
    use unicode_properties::{GeneralCategoryGroup, UnicodeGeneralCategory as _};
    let hidden = c.is_whitespace()
        || matches!(
            c.general_category_group(),
            GeneralCategoryGroup::Mark | GeneralCategoryGroup::Other
        );
    if hidden {
        code_point(c)
    } else {
        format!("{} {c}", code_point(c))
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

/// Where a row FR-17 leaves off the radar ends up, which its warning says
/// (FR-106).
///
/// It depends on the output and on `--table`, both known before any row is
/// read, so each warning carries it, as it carries its source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OffRadar {
    /// An HTML page under `--table all`: the table lists it, flagged.
    Listed,
    /// An HTML page under the default `--table radar`: nowhere on the page.
    Unlisted,
    /// A PNG or SVG, which has no table.
    Undrawn,
}

impl OffRadar {
    /// Where a run writing `format` with `--table` set to `table` puts the
    /// row: in the page's table only under `--table all`, and nowhere in a
    /// PNG or SVG, which has no table.
    #[must_use]
    pub const fn of(format: OutputFormat, table: Table) -> Self {
        match (format, table) {
            (OutputFormat::Html, Table::All) => Self::Listed,
            (OutputFormat::Html, Table::Radar) => Self::Unlisted,
            (OutputFormat::Png | OutputFormat::Svg, _) => Self::Undrawn,
        }
    }

    /// FR-17's message for one row whose ring `v` is not in `list`,
    /// declared by `source` (D-61): the fact, then the remedies.
    fn ring_row(self, v: &str, source: &Source, list: &str, suggest: bool) -> String {
        let fact = match source {
            Source::Default => format!("'{v}' is not one of the default rings ({list})"),
            named => format!("'{v}' is not in {} ({list})", ring_list(named)),
        };
        let went = match self {
            Self::Listed => "; the row is in the table view, not on the radar",
            Self::Unlisted => ", so the row is left off the page",
            Self::Undrawn => "; the row is not on the radar",
        };
        format!("{fact}{went}{}", self.remedies(source, suggest, "it", "it"))
    }

    /// FR-17's grouped line for `n` rows, their values `tallies`.
    fn ring_rows(
        self,
        n: usize,
        source: &Source,
        list: &str,
        tallies: &str,
        suggest: bool,
    ) -> String {
        let fact = match source {
            Source::Default => {
                format!("{n} rows use a ring that is not one of the default rings ({list})")
            }
            named => format!("{n} rows use a ring not in {} ({list})", ring_list(named)),
        };
        let went = match self {
            Self::Listed => "all appear in the table view",
            Self::Unlisted => "they are left off the page",
            Self::Undrawn => "none is on the radar",
        };
        format!(
            "{fact} — {tallies}; {went}{}",
            self.remedies(source, suggest, "one", "them")
        )
    }

    /// FR-17's remedies, each after a `; `, in one order: how to plot the
    /// row, how to keep it off quietly (only when `suggest`, D-96), and,
    /// last, what `--table all` would also publish (FR-106). `one` names
    /// the row and `them` the rows `--table all` lists.
    fn remedies(self, source: &Source, suggest: bool, one: &str, them: &str) -> String {
        // `write!` rather than `push_str(&format!(..))`, which clippy's
        // `format_push_string` refuses; writing to a `String` cannot fail.
        let mut text = match source {
            Source::Default => format!(
                "; to plot {one}, pass --rings with every ring to draw (it replaces the defaults)"
            ),
            named => format!("; to plot {one}, add it to {}", ring_list(named)),
        };
        if suggest {
            write!(
                text,
                "; to leave {one} off without this warning, add it to --unplotted-rings"
            )
            .expect("a String takes any write");
        }
        if self == Self::Unlisted {
            write!(
                text,
                "; --table all lists {them}, but also publishes every other column"
            )
            .expect("a String takes any write");
        }
        text
    }
}

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
                _ => grouped(class, members),
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

fn grouped(class: &Class, members: &[Member]) -> String {
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
        // A URL's path is case-sensitive, so two spellings are two values,
        // as for a date.
        Class::Link => format!(
            "{n} rows have a link that is not an http or https address — {}; shown as text, not links",
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
                        "{column} on line {} ({c} characters, first {})",
                        m.line,
                        code_point(*first)
                    ),
                })
            }))
        ),
        // Positions, as for FR-90, each with its text: unlike a removed
        // character, the box is drawn, so the text is what to look for.
        Class::MissingGlyphs(format) => format!(
            "{n} values have characters the bundled font lacks — {}; {}",
            comma_list(members.iter().filter_map(|m| {
                let RowKind::MissingGlyphs {
                    column,
                    lacking:
                        Lacking {
                            text, first, count, ..
                        },
                } = m.kind
                else {
                    return None;
                };
                let (column, first) = (column.as_str(), glyph(*first));
                Some(match count {
                    1 => format!("{column} '{text}' on line {} ({first})", m.line),
                    c => format!(
                        "{column} '{text}' on line {} ({c} characters, first {first})",
                        m.line
                    ),
                })
            })),
            boxes(*format, n)
        ),
        // FR-93 counts values, each column with its count and first line.
        Class::Unnamed => format!(
            "{n} values are under columns with no name in the header row — {}; none is shown, so add a header name, or delete the values",
            tallied(members, str::to_string)
        ),
        // Every member is on the header's line, so each is named by its
        // header, in column order, with its suggestion.
        Class::NearMiss => format!(
            "{n} header names are not columns radargen knows — {}",
            comma_list(members.iter().filter_map(|m| match m.kind {
                RowKind::NearMiss { heading, known } => Some(format!(
                    "{} (did you mean {}?)",
                    trim(heading),
                    known.as_str()
                )),
                _ => None,
            }))
        ),
    }
}

/// The declared ring set as FR-17 names the list to edit: `--rings`, or
/// `the rings key in radar.toml`. The default set has its own wording.
fn ring_list(source: &Source) -> Origin {
    Origin::new(Setting::Rings, source.clone())
}

/// FR-17's grouped line: it lists the declared rings and says what to edit,
/// which depends on where the ring set came from (D-61).
// qual:allow(complexity, error_handling) reason: "a Ring class is built only from UndeclaredRing rows, so a member is always found"
fn grouped_rings(source: &Source, members: &[Member]) -> String {
    let n = members.len();
    // Every member of one run carries the same rings, `off_radar` and
    // `suggest_unplotted`, so the first speaks for all.
    let (list, off_radar, suggest) = members
        .iter()
        .find_map(|m| match m.kind {
            RowKind::UndeclaredRing {
                rings,
                off_radar,
                suggest_unplotted,
                ..
            } => Some((rings.join(", "), *off_radar, *suggest_unplotted)),
            _ => None,
        })
        .expect("a Ring class holds only UndeclaredRing members");
    off_radar.ring_rows(n, source, &list, &tallied(members, normalise), suggest)
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

/// A flag, file or runtime error the tool stops on by itself (NFR-16 tier
/// one, and NFR-15's): non-zero exit, nothing written (NFR-8, NFR-9).
///
/// Row-level errors are [`RowError`]s. Each stage adds its variants beside the
/// code that raises them (architecture §1). An I/O variant should carry the
/// path it failed on and mark the error `#[source]`, not `#[from]`: a bare
/// `From<io::Error>` would let `?` build a `Fatal` that cannot name its path
/// (NFR-10).
#[derive(Debug, thiserror::Error)]
pub enum Fatal {
    /// FR-52: the `-o` extension is absent or names a format we do not write.
    #[error("{output}: the output file name must end in .html, .png or .svg")]
    UnrecognisedExtension {
        /// The output, by its flag or key, as written.
        output: NamedPath,
    },
    /// FR-63 on the ring set, from `--rings` or a `--config` file's key
    /// (NFR-16 tier one, NFR-10).
    #[error("{origin}: {error}")]
    Rings {
        /// Where the list came from.
        origin: Origin,
        /// What is wrong with it.
        error: RingsError,
    },
    /// FR-107's checks, FR-63's among them, named by the list's source.
    #[error("{origin}: {error}")]
    UnplottedRings {
        /// Where the list came from.
        origin: Origin,
        /// What is wrong with it.
        error: RingsError,
    },
    /// FR-102, FR-86, FR-87: one problem in the `--palette` file; a file
    /// reports every one (NFR-16 tier one).
    #[error("{}", palette_problem(palette, error))]
    Palette {
        /// The file, by its flag or key, as the author named it.
        palette: NamedPath,
        /// What is wrong, and where.
        error: PaletteError,
    },
    /// NFR-15: the `--palette` file does not exist or cannot be read.
    #[error("{palette}: cannot be read: {}", io_reason(source))]
    PaletteUnreadable {
        /// The file, by its flag or key, as the author named it.
        palette: NamedPath,
        /// The underlying I/O failure.
        #[source]
        source: std::io::Error,
    },
    /// NFR-15, FR-102: the `--palette` file is not UTF-8, or holds a NUL.
    #[error("{palette}, line {line}: not UTF-8 text; re-save the file as UTF-8")]
    PaletteNotUtf8 {
        /// The file, by its flag or key, as the author named it.
        palette: NamedPath,
        /// The physical line holding the first bad byte.
        line: usize,
    },
    /// NFR-15, plan RG-018A R4: the `--palette` file passes the cap, so it
    /// is not a palette, or never ends (`/dev/zero`).
    #[error(
        "{palette}: larger than {} MiB; a palette is a few hundred bytes, so check the path",
        MAX_BYTES >> 20
    )]
    PaletteTooLarge {
        /// The file, by its flag or key, as the author named it.
        palette: NamedPath,
    },
    /// FR-62: `-o`'s directory does not exist; the tool does not create it.
    #[error("{output}: the directory {dir} does not exist")]
    MissingDirectory {
        /// The output, by its flag or key, as written.
        output: NamedPath,
        /// The directory part of it.
        dir: String,
    },
    /// FR-62, D-74: `-o` is the input CSV, which the page would replace.
    #[error("{output}: is the input CSV; choose another output path")]
    OutputIsInput {
        /// The output, by its flag or key, as written.
        output: NamedPath,
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
    /// NFR-15, FR-61, rg-w5k: the input passes the cap, so it is not a
    /// radar's CSV, or never ends (`/dev/zero`).
    #[error(
        "{path}: larger than {} MiB; a radar's CSV is usually a few kilobytes, so check the path",
        MAX_CSV_BYTES >> 20
    )]
    CsvTooLarge {
        /// The input path as given.
        path: String,
    },
    /// NFR-15: the output cannot be written: its directory is missing or
    /// unwritable, the path names a directory or a read-only file, or the
    /// disk is full. Nothing at `-o` has changed (NFR-14).
    #[error("{output}: cannot be written: {}", io_reason(source))]
    Unwritable {
        /// The output, by its flag or key, as written.
        output: NamedPath,
        /// The underlying I/O failure.
        #[source]
        source: std::io::Error,
    },
    /// FR-55: resvg could not parse or rasterise the document, or encode
    /// the image. Nothing at `-o` has changed (NFR-14). The reason is
    /// resvg's or the encoder's, which name positions and kinds, never the
    /// document's text.
    #[error("the PNG cannot be drawn: {reason}")]
    Rasterise {
        /// What failed.
        reason: String,
    },
    /// D-87: the image would pass `MAX_PNG_PIXELS`.
    #[error(
        "a {width} by {height} pixel PNG is too large to draw; {}",
        smaller_width(width_source)
    )]
    ImageTooLarge {
        /// The width in pixels.
        width: u32,
        /// The height in pixels.
        height: u32,
        /// Where the width came from, which the remedy names (NFR-10).
        width_source: Source,
    },
    /// FR-60, FR-99, FR-101: a `Cli` asking for no task, or for more than
    /// one. clap refuses such a command line, so only a `Cli` built by hand
    /// reaches this.
    #[error(
        "give a CSV and -o or --config to render, or --create-template or --create-palette alone"
    )]
    NoTask,
    /// FR-99, FR-101: a file to start from never replaces one (D-83, D-91).
    #[error(
        "{} {path}: already exists; radargen never replaces a file, so choose another path",
        .starter.flag()
    )]
    StarterExists {
        /// Which file, so the message names its flag.
        starter: Starter,
        /// Its argument as given.
        path: String,
    },
    /// FR-99, FR-101: the file cannot be written, for any of the reasons
    /// [`Fatal::Unwritable`] gives `-o`. Nothing at the path has changed.
    #[error("{} {path}: cannot be written: {}", .starter.flag(), io_reason(source))]
    StarterUnwritable {
        /// Which file, so the message names its flag.
        starter: Starter,
        /// Its argument as given.
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
}

/// A row-level hard error (NFR-16 tier two).
///
/// Collected across the whole file and printed together through
/// [`crate::errors::render_errors`], which groups them (D-62) and adds FR-4's
/// header-first cause a single error cannot know.
/// Kept apart from [`Fatal`], whose errors each stop the run on their own, so
/// the grouping code lists only what it groups (rg-w39).
#[derive(Debug, thiserror::Error)]
pub enum RowError {
    /// FR-4: a header name repeated under FR-76's normalisation, so
    /// `Name` and `name ` collide. Indices are 0-based, as
    /// [`column_label`] takes them; names are as
    /// written, trimmed, so the author can find both cells.
    #[error(
        "line {line}: {} '{name}' repeats {} '{first_name}'; header names match ignoring case and extra spaces, so rename or remove one",
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

impl RowError {
    /// The stderr line for this error alone; [`crate::errors::render_errors`]
    /// uses it for an error that does not group.
    #[must_use]
    pub fn render(&self) -> String {
        // Escaped as a whole, as `Fatal::render` is: header names and cells
        // come from the CSV (NFR-10).
        error_line(&self.to_string())
    }
}

impl Fatal {
    /// The stderr line.
    #[must_use]
    pub fn render(&self) -> String {
        // Escaped as a whole, so no variant can forget to: `-o` and `--rings`
        // values come from argv, and some variants carry CSV text (NFR-10).
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
