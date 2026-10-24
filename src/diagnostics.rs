//! The diagnostic taxonomy (architecture §3, §7): what is fatal, what warns,
//! and how warnings reach stderr (NFR-10, NFR-11, NFR-20).

use std::fmt::{self, Write as _};
use std::sync::Arc;

use crate::columns::{ColumnKind, SECTOR_ALIAS};
use crate::csv::decode::MAX_BYTES as MAX_CSV_BYTES;
use crate::options::OutputFormat;
use crate::options::{Paper, Pdf, Table};
use crate::palette::file::{MAX_BYTES, PaletteError};
use crate::rings::RingsError;
use crate::settings::edition::{Fallback, NOT_AN_EDITION, PLACEHOLDER};
use crate::settings::file::{KeyProblem, MAX_BYTES as CONFIG_MAX_BYTES};
use crate::settings::{NamedPath, Origin, Setting, Source, Sourced};
use crate::starter::Starter;
use crate::style::{MIN_TARGET_PX, NARROW_MAX_WIDTH, NARROW_MIN_VIEWPORT_PX, WIDE_MIN_VIEWPORT_PX};
use crate::text::FontError;
use crate::{ECHO_CHARS, clip, is_untagged_format, normalise, trim};

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
    /// (FR-107). `origin` is what the message says to edit (D-61); `rings`
    /// are the declared rings it lists, so the author sees what would match.
    UndeclaredRing {
        /// The ring cell as the CSV had it.
        value: String,
        /// Where the ring list came from: `--rings`, a file's key, or the
        /// default (D-61).
        origin: Origin,
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
    /// FR-104: a text a PNG, SVG or PDF draws, from this cell, holds characters
    /// the bundled font lacks. One per drawn text, however many it lacks.
    MissingGlyphs {
        /// The column the text was read from, as NFR-10 names it: `name`
        /// or `sector`, or, in a full PDF's table, any column it shows.
        column: String,
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
    SectorCount {
        /// How many distinct sectors the CSV used.
        count: usize,
    },
    /// FR-89, one per overfull band.
    OverfullBand {
        /// The sector name the band belongs to.
        sector: String,
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
    /// RG-102: `--legend hidden` on a PNG, SVG or PDF, which always draw the
    /// legend.
    LegendStatic,
    /// FR-106: `--table all` on a PNG, SVG or the radar PDF, which have no
    /// table.
    TableStatic {
        /// The output it was given for.
        format: OutputFormat,
    },
    /// FR-112: `--paper none` from the flag displaced the file's
    /// `pdf = "full"`, so the run writes the radar PDF without the table.
    FullDropped {
        /// Where `full` came from: the file.
        pdf: Origin,
    },
    /// FR-112: `--pdf full` from the flag with an HTML, PNG or SVG output.
    PdfNotPdf {
        /// The value given.
        pdf: Pdf,
    },
    /// FR-112: the full PDF's prose columns are narrower than
    /// `TABLE_PROSE_MIN` even with its label columns at their floor.
    TableCramped {
        /// The table's columns, the number column included.
        columns: usize,
        /// `--table`, and where it came from, which the remedy names.
        table: Sourced<Table>,
    },
    /// FR-111 (rg-i95): a legend row taller than a page, which runs into
    /// the page's foot room since a row never splits.
    LegendRowTall {
        /// Its CSV line (NFR-10).
        line: usize,
        /// Its name.
        name: String,
    },
    /// FR-112: a row taller than a page of the table, split across pages.
    TableRowTall {
        /// Its CSV line (NFR-10).
        line: usize,
        /// Its name.
        name: String,
    },
    /// FR-111: `--paper` from the flag with an HTML, PNG or SVG output.
    PaperNotPdf {
        /// The paper given.
        paper: Paper,
    },
    /// FR-111: `--width` from the flag with a PDF on paper, which sets
    /// the page's size.
    PaperIgnoresWidth {
        /// The width given.
        width: u32,
        /// What puts the PDF on paper, and where it came from.
        paper: PaperFrom,
    },
    /// FR-111: `--legend-columns` from the flag with a PDF on paper, which
    /// sets the legend's columns.
    PaperIgnoresLegendColumns {
        /// The columns given.
        columns: u8,
        /// What puts the PDF on paper, and where it came from.
        paper: PaperFrom,
    },
    /// FR-111: the radar does not fit the first paper page even at its
    /// smallest scale, so it runs past the margins.
    PaperRadarWide {
        /// The paper it is on.
        paper: Paper,
        /// Whether it is a full PDF, which cannot leave paper.
        full: bool,
    },
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
        width_origin: Origin,
    },
    /// FR-90 on a setting's value, from the command line or the file: one
    /// per value, however many characters it lost.
    SettingUndisplayable {
        /// The setting whose value it is, and where that value came from
        /// (NFR-10).
        origin: Origin,
        /// The first character the value lost.
        first: char,
        /// How many characters the value lost in all.
        count: usize,
    },
    /// D-58: `-e today` read the local date; it is echoed so the render
    /// can be reproduced with an explicit `-e`. A warning, since stderr has
    /// no other class for a line the author should act on (NFR-9).
    EditionToday {
        /// The date read.
        date: String,
        /// The zone it is the date in.
        zone: String,
        /// `-e`, or the `edition` key and its file.
        origin: Origin,
    },
    /// Review M-1: `-e today` could not find the local zone, so it used
    /// UTC; said, since a misspelt `TZ` would otherwise pass silently.
    TodayInUtc(Fallback),
    /// FR-92: `--edition` was given, but no value holds `{edition}`.
    EditionUnused {
        /// The source and the date, as a message names them.
        edition: NamedPath,
    },
    /// FR-104 on a setting's value, from the command line or the file: one
    /// per drawn text, so one per ring entry, however many characters it
    /// lacks.
    SettingMissingGlyphs {
        /// The setting whose value it is, and where that value came from
        /// (NFR-10).
        origin: Origin,
        /// The text, the value or one `--rings` entry, and what it lacks.
        lacking: Lacking,
    },
}

/// FR-104: a text as a PNG, SVG or PDF draws it, with characters the bundled
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
/// (D-73, D-94): a sector name from the CSV, or a ring name from
/// `--rings` or the default set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Binding {
    /// A sector label, as written.
    Sector(String),
    /// A ring name, as declared, and where it was declared.
    Ring {
        /// The ring's name.
        name: String,
        /// `--rings`, a file's key, or the default set.
        origin: Origin,
    },
}

/// The label as FR-95 and FR-105 name it: trimmed and single-quoted, a
/// ring with where to rename it (D-73).
impl fmt::Display for Binding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Sector(name) => write!(f, "sector '{}'", trim(name)),
            Self::Ring { name, origin } if origin.source == Source::Default => write!(
                f,
                "ring '{}' (a default ring, so pass --rings to rename it)",
                trim(name)
            ),
            Self::Ring { name, origin } => write!(f, "ring '{}' in {origin}", trim(name)),
        }
    }
}

/// A file the run reads, which the output must not replace (D-74).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReadFile {
    /// The input CSV; with the `--config` file when its `input` key named
    /// it (NFR-10).
    Csv(Option<Arc<str>>),
    /// The `--config` file.
    Config,
    /// The palette, from `--palette` or the file's key.
    Palette,
}

/// How D-74's message names it.
impl fmt::Display for ReadFile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Csv(None) => f.write_str("the input CSV"),
            Self::Csv(Some(file)) => write!(f, "the input CSV (the input key in {file})"),
            Self::Config => f.write_str("the --config file"),
            Self::Palette => f.write_str("the palette file"),
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
    /// One run declares one ring set, so its origin never splits a class;
    /// it rides here so the grouped line can name it.
    Ring(Origin),
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
            Self::UndeclaredRing { origin, .. } => Class::Ring(origin.clone()),
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
            // A header name is CSV text (FR-18's extra columns); it is escaped
            // with the rest of the line. A missing glyph's column is named as
            // the warning was raised.
            Self::MissingGlyphs { column, .. }
            | Self::Undisplayable { column, .. }
            | Self::UnnamedValue { column } => column,
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
                origin,
                rings,
                off_radar,
                suggest_unplotted,
                ..
            } => off_radar.ring_row(v, origin, &rings.join(", "), *suggest_unplotted),
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
            Self::SectorCount { count } => format!(
                "{count} sectors; the radar reads best with 3 to 6, so sectors narrow and labels may crowd"
            ),
            Self::OverfullBand {
                sector,
                ring,
                count,
                fits,
            } => format!(
                "sector '{}', ring '{}': {count} items; about {fits} fit comfortably, so some markers overlap",
                trim(sector),
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
                "--legend hidden applies to HTML pages; PNG, SVG and PDF always draw the legend"
                    .to_owned()
            }
            Self::TableStatic { format } => table_static(*format).to_owned(),
            Self::PdfNotPdf { pdf } => pdf_not_pdf(*pdf),
            Self::FullDropped { pdf } => full_dropped(pdf),
            Self::TableCramped { columns, table } => cramped(*columns, table),
            Self::TableRowTall { line, name } => tall_row(*line, name),
            Self::LegendRowTall { line, name } => tall_legend_row(*line, name),
            Self::PaperNotPdf { paper } => format!(
                "--paper {paper} applies to PDF; HTML pages, PNG and SVG have no paper size"
            ),
            Self::PaperIgnoresWidth { width, paper } => ignores_width(*width, paper),
            Self::PaperIgnoresLegendColumns { columns, paper } => ignores_columns(*columns, paper),
            Self::PaperRadarWide { paper, full } => radar_wide(*paper, *full),
            Self::WidthClamped { natural, drawn } => format!(
                "the radar is {natural} pixels wide, past --width's limit; the PNG is drawn {drawn} wide, or pass --width to choose"
            ),
            Self::PngTextSmall {
                width,
                legible_from,
                width_origin,
            } => small_text(*width, *legible_from, width_origin),
            Self::SettingUndisplayable {
                origin,
                first,
                count: 1,
            } => format!(
                "{origin}: removed {}, which the output cannot display",
                code_point(*first)
            ),
            Self::SettingUndisplayable {
                origin,
                first,
                count,
            } => format!(
                "{origin}: removed {count} characters the output cannot display, the first {}",
                code_point(*first)
            ),
            Self::EditionToday { date, zone, origin } => {
                // From the flag the author typed `today`; from the key, the
                // echo says where it came from (maintainer, 2026-10-22).
                let from = if origin.source.is_config() {
                    format!("{origin}: ")
                } else {
                    String::new()
                };
                format!(
                    "{from}edition {date} (today, {}); pass -e {date} to reproduce this render",
                    escape_controls(zone)
                )
            }
            Self::TodayInUtc(fallback) => {
                let why = match fallback {
                    Fallback::BadTz(tz) => format!(
                        "TZ '{}' names no time zone radargen can find",
                        escape_controls(&clip(tz, ECHO_CHARS))
                    ),
                    Fallback::NoSystemZone => {
                        "the system's time zone cannot be determined".to_owned()
                    }
                };
                format!("{why}, so -e today used UTC; set TZ to a zone such as America/Vancouver")
            }
            Self::EditionUnused { edition } => format!(
                "{edition}: no value uses it; put {PLACEHOLDER} in the output, title or subtitle"
            ),
            Self::SettingMissingGlyphs { origin, lacking } => format!("{origin}: {lacking}"),
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

/// D-87's small-text warning, its remedy naming the width by its origin
/// (plan RG-019A, Ruling 9).
fn small_text(width: u32, legible_from: Option<u32>, origin: &Origin) -> String {
    legible_from.map_or_else(
        || {
            format!(
                "at {width} pixels wide the PNG's text is too small to read, at any {}; fewer or shorter names would help",
                origin.name()
            )
        },
        |from| {
            format!(
                "at {width} pixels wide the PNG's text is too small to read; {} keeps it legible",
                origin.set_to(from, " or more")
            )
        },
    )
}

impl Fatal {
    /// Whether this is a usage error, which exits with clap's status: the
    /// file's key problems, FR-59 across sources, and a missing output
    /// (FR-91, plan RG-019A).
    #[must_use]
    pub const fn is_usage(&self) -> bool {
        matches!(
            self,
            Self::ConfigKey { .. }
                | Self::ConfigMore { .. }
                | Self::Conflict { .. }
                | Self::FullNeedsPaper { .. }
                | Self::NoOutput { .. }
                | Self::NoInput { .. }
        )
    }
}

/// `name, line 3: message`, or `name: message` without a line: a
/// config file's syntax error and a palette's problem.
fn at_line(name: impl fmt::Display, line: Option<usize>, message: impl fmt::Display) -> String {
    line.map_or_else(
        || format!("{name}: {message}"),
        |line| format!("{name}, line {line}: {message}"),
    )
}

/// `a string`, `an integer`: TOML's type names with their article.
fn article(found: &str) -> String {
    let an = matches!(found.chars().next(), Some('a' | 'e' | 'i' | 'o' | 'u'));
    format!("{} {found}", if an { "an" } else { "a" })
}

/// FR-91's keys, in its order, as a sentence lists them: `output, …, table
/// and edition`.
fn known_keys() -> String {
    let keys: Vec<&str> = Setting::ALL.iter().map(|s| s.key()).collect();
    let (last, rest) = keys.split_last().map_or(("", &[][..]), |(l, r)| (*l, r));
    format!("{} and {last}", rest.join(", "))
}

/// The type FR-91 gives a key, as its wrong-type message asks for it.
const fn wanted(setting: Setting) -> &'static str {
    match setting {
        Setting::Width | Setting::LegendColumns => "an integer",
        Setting::Rings | Setting::UnplottedRings => {
            "an array of strings, such as [\"Adopt\", \"Trial\"]"
        }
        Setting::Input
        | Setting::Output
        | Setting::Edition
        | Setting::Title
        | Setting::Subtitle
        | Setting::Theme
        | Setting::Legend
        | Setting::Table
        | Setting::Paper
        | Setting::Pdf
        | Setting::Palette => "a string",
    }
}

/// FR-111: `flag`, given on the command line, does not apply on `paper`,
/// for `why`. A paper from a file is named with its file, and how to undo
/// it for one run.
fn paper_ignores(flag: &str, from: &PaperFrom, why: &str, instead: &str) -> String {
    let fact = match (&from.paper, &from.full) {
        (Some(paper), _) if paper.origin.source.is_config() => format!(
            "{flag} does not apply on paper ({})",
            paper.origin.assignment(paper.value, "")
        ),
        (Some(paper), _) => format!(
            "{flag} does not apply with {}",
            paper.origin.assignment(paper.value, "")
        ),
        (None, Some(full)) if full.source.is_config() => format!(
            "{flag} does not apply on paper ({})",
            full.assignment(Pdf::Full, "")
        ),
        (None, _) => format!("{flag} does not apply with --pdf full, which is always on paper"),
    };
    // The remedy must leave the drawing-sized page: a full PDF needs
    // --pdf radar, and paper given beside it --paper none as well, since
    // --pdf full --paper none is a usage error (final review, QA 1).
    let remedy = match (&from.paper, &from.full) {
        (Some(_), Some(_)) => "pass --pdf radar --paper none".to_owned(),
        (None, Some(_)) => "pass --pdf radar".to_owned(),
        (Some(paper), None) if paper.origin.source.is_config() => "pass --paper none".to_owned(),
        (_, None) => "leave out --paper".to_owned(),
    };
    format!("{fact}; {why}; {remedy} for {instead}")
}

/// FR-111: the radar does not fit `paper`'s first page even at its
/// smallest; the drawing-sized page is the way out, except for a full PDF.
fn radar_wide(paper: Paper, full: bool) -> String {
    let way_out = if full { "" } else { ", or pass --paper none" };
    let name = match paper {
        Paper::A4 => "A4",
        Paper::Letter | Paper::None => "Letter",
    };
    format!(
        "the radar does not fit on the {name} page even at its smallest, so it runs past the margins; a shorter title, or shorter sector or ring names, would fix it{way_out}"
    )
}

/// FR-111: `--width` given for a PDF on paper.
fn ignores_width(width: u32, paper: &PaperFrom) -> String {
    paper_ignores(
        &format!("--width {width}"),
        paper,
        "the paper sets the page's size",
        &format!("a page --width {width} wide"),
    )
}

/// FR-111: `--legend-columns` given for a PDF on paper.
fn ignores_columns(columns: u8, paper: &PaperFrom) -> String {
    paper_ignores(
        &format!("--legend-columns {columns}"),
        paper,
        "on paper the legend takes as many columns as fit each page",
        &format!("a legend in {columns} columns"),
    )
}

/// FR-112: `--paper none` displaced the file's `pdf = "full"`.
fn full_dropped(pdf: &Origin) -> String {
    format!(
        "--paper none: {} needs paper, so this writes the radar PDF without the table; pass --pdf radar to say so, or leave out --paper none for the table",
        pdf.assignment(Pdf::Full, "")
    )
}

/// FR-112: `--pdf full` from the flag on an output that is not a PDF.
fn pdf_not_pdf(pdf: Pdf) -> String {
    format!(
        "--pdf {pdf} applies to PDF and is ignored here; an HTML page already has the table, and PNG and SVG have none"
    )
}

/// FR-112: the row on `line`, named `name`, is taller than a table page.
fn tall_row(line: usize, name: &str) -> String {
    format!(
        "line {line}: '{name}' is taller than a page of the table, so it runs on across pages, marked (continued); shorter text in its row keeps it on one page"
    )
}

/// rg-i95: the legend row on `line`, named `name`, is taller than a page.
fn tall_legend_row(line: usize, name: &str) -> String {
    format!(
        "line {line}: '{name}' in the legend is taller than the room on its page, so it runs over the page's footer; a shorter name fixes it"
    )
}

/// FR-106's warning for `--table all` on an output without a table: the
/// radar PDF names the full PDF that has one (FR-112).
fn table_static(format: OutputFormat) -> &'static str {
    if format == OutputFormat::Pdf {
        "--table all applies to HTML pages and the full PDF; the radar PDF has no table, so add --pdf full for one"
    } else {
        "--table all applies to HTML pages and the full PDF (--pdf full); PNG and SVG have no table"
    }
}

/// FR-112's cramped table, its remedy naming `--table` by its origin when
/// it is `all`: under `radar` there is no column to drop.
fn cramped(columns: usize, table: &Sourced<Table>) -> String {
    let fact = format!(
        "--pdf full: the table has {columns} columns, so its prose columns are narrower than about 21 characters"
    );
    if table.value != Table::All {
        return format!(
            "{fact}; --paper a4 is a little wider, and the HTML page shows the table in full"
        );
    }
    let radar = table.origin.set_to("radar", "");
    format!("{fact}; {radar} leaves out the columns radargen does not read")
}

/// FR-112's same-source conflict: `full` needs paper, `none` refuses it.
fn needs_paper(pdf: &Origin, paper: &Origin) -> String {
    let full = pdf.assignment(Pdf::Full, "");
    let none = paper.assignment(Paper::None, "");
    if paper.source.is_config() {
        format!(
            "{full} needs paper, but {none} asks for the drawing-sized page; set paper to \"letter\" or \"a4\", remove the paper line, or set pdf = \"radar\""
        )
    } else {
        format!(
            "{full} needs paper, but {none} asks for the drawing-sized page; use --paper letter or --paper a4, or --pdf radar for the drawing-sized page without the table"
        )
    }
}

/// FR-91: one key's problem, named by the key and the file (NFR-10).
fn key_problem(file: &Arc<str>, problem: &KeyProblem) -> String {
    let key = |setting: Setting| Origin::new(setting, Source::Config(Arc::clone(file)));
    match problem {
        KeyProblem::Unknown { key, near, table } => unknown_key(file, key, *near, *table),
        KeyProblem::WrongType { setting, found } => format!(
            "{}: must be {}, not {}{}",
            key(*setting),
            wanted(*setting),
            article(found),
            quotes(*setting, found)
        ),
        KeyProblem::Empty { setting } => format!(
            "{}: is empty; give a file name, or remove the key",
            key(*setting)
        ),
        KeyProblem::EntryType {
            setting,
            index,
            found,
        } => {
            format!(
                "{}: entry {index} must be a string, not {}",
                key(*setting),
                article(found)
            )
        }
        KeyProblem::Comma {
            setting,
            index,
            entry,
        } => format!(
            "{}: entry {index} '{}' holds a comma; give each ring as its own entry",
            key(*setting),
            entry
        ),
        KeyProblem::BadEdition { value } => format!(
            "invalid value '{value}' for {}: {NOT_AN_EDITION}",
            key(Setting::Edition)
        ),
        KeyProblem::NotAValue {
            setting,
            value,
            possible,
        } => format!(
            "invalid value '{}' for {} [possible values: {}]",
            value,
            key(*setting),
            possible.join(", ")
        ),
        KeyProblem::OutOfRange {
            setting,
            value,
            min,
            max,
        } => format!(
            "{}: {value} is out of range; use {min} to {max}",
            key(*setting)
        ),
    }
}

/// FR-91's unknown key, with the key it is near (UX review 3) and, for a
/// table such as `[radar]`, where keys go.
fn unknown_key(file: &str, key: &str, near: Option<Setting>, table: bool) -> String {
    let guess = near.map_or_else(
        || "; the keys".to_owned(),
        |s| format!("; did you mean '{}'? The keys", s.key()),
    );
    let section = if table {
        format!("; keys go at the top of the file, not under [{key}]")
    } else {
        String::new()
    };
    format!(
        "--config {file}: unknown key '{key}'{guess} are {}{section}",
        known_keys()
    )
}

/// UX review 11: a wrong type that missing or extra quotes explain.
fn quotes(setting: Setting, found: &str) -> &'static str {
    let integer = matches!(setting, Setting::Width | Setting::LegendColumns);
    let list = matches!(setting, Setting::Rings | Setting::UnplottedRings);
    match found {
        "integer" | "float" | "boolean" | "datetime" if !integer && !list => {
            "; put the value in quotes"
        }
        "string" if integer => "; remove the quotes",
        _ => "",
    }
}

/// A palette problem as stderr shows it: located when it has a line.
///
/// A palette from a `--config` file quotes nothing from it: the key may
/// name any file, so its text could be a secret (ruling B).
fn palette_problem(palette: &NamedPath, error: &PaletteError) -> String {
    let message = if palette.origin.source.is_config() {
        error.redacted().to_string()
    } else {
        error.to_string()
    };
    at_line(palette, error.line(), message)
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
/// A PDF's box carries its character (FR-113), so its warning says a
/// screen reader still reads it; whether it copies depends on the viewer.
fn boxes(format: OutputFormat, count: usize) -> String {
    let (them, boxes) = (plural(count, "it", "them"), plural(count, "a box", "boxes"));
    let searched = if format == OutputFormat::Pdf {
        plural(
            count,
            ", though a screen reader still reads it as itself",
            ", though a screen reader still reads them as themselves",
        )
        .to_owned()
    } else {
        String::new()
    };
    format!(
        "the {format} draws {them} as {boxes}{searched} — an HTML page shows {them}, or transliterate {them}"
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

/// What puts a PDF on paper (FR-111, FR-112), so a flag paper ignores is
/// warned about in words that name it, with a remedy that works.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaperFrom {
    /// `--paper letter` or `a4`, from the flag or the file; `None` for a
    /// full PDF on its default Letter.
    pub paper: Option<Sourced<Paper>>,
    /// Where `--pdf full` came from, when the PDF is full.
    pub full: Option<Origin>,
}

/// Where a row FR-17 leaves off the radar ends up, which its warning says
/// (FR-106).
///
/// It depends on the output and on `--table`, both known before any row is
/// read, so each warning carries it, as it carries its source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OffRadar {
    /// An HTML page under `--table all`: the table lists it, flagged.
    Listed,
    /// An HTML page under `--table radar` from a file or the default:
    /// nowhere on the page.
    Unlisted,
    /// As [`OffRadar::Unlisted`], with `--table radar` given on the command
    /// line, which overrides any table key, so the table remedy names the
    /// flag (final review M-3).
    UnlistedByFlag,
    /// A PNG, SVG or PDF, which has no table.
    Undrawn,
}

impl OffRadar {
    /// Where a run writing `format` with `--table` set to `table` puts the
    /// row: in the table of an HTML page or a `full` PDF only under
    /// `--table all` (FR-112), and nowhere in a PNG, SVG or radar PDF, which
    /// has no table. `by_flag` says the table came from the command line.
    #[must_use]
    pub const fn of(format: OutputFormat, full: bool, table: Table, by_flag: bool) -> Self {
        let tabled = matches!(format, OutputFormat::Html) || full;
        match (tabled, table) {
            (true, Table::All) => Self::Listed,
            (true, Table::Radar) if by_flag => Self::UnlistedByFlag,
            (true, Table::Radar) => Self::Unlisted,
            (false, _) => Self::Undrawn,
        }
    }

    /// FR-17's message for one row whose ring `v` is not in `list`,
    /// declared by `origin` (D-61): the fact, then the remedies.
    fn ring_row(self, v: &str, origin: &Origin, list: &str, suggest: bool) -> String {
        let fact = if origin.source == Source::Default {
            format!("'{v}' is not one of the default rings ({list})")
        } else {
            format!("'{v}' is not in {origin} ({list})")
        };
        let went = match self {
            Self::Listed => "; the row is in the table view, not on the radar",
            Self::Unlisted | Self::UnlistedByFlag => ", so the row is left off the page",
            Self::Undrawn => "; the row is not on the radar",
        };
        format!("{fact}{went}{}", self.remedies(origin, suggest, "it", "it"))
    }

    /// FR-17's grouped line for `n` rows, their values `tallies`.
    fn ring_rows(
        self,
        n: usize,
        origin: &Origin,
        list: &str,
        tallies: &str,
        suggest: bool,
    ) -> String {
        let fact = if origin.source == Source::Default {
            format!("{n} rows use a ring that is not one of the default rings ({list})")
        } else {
            format!("{n} rows use a ring not in {origin} ({list})")
        };
        let went = match self {
            Self::Listed => "all appear in the table view",
            Self::Unlisted | Self::UnlistedByFlag => "they are left off the page",
            Self::Undrawn => "none is on the radar",
        };
        format!(
            "{fact} — {tallies}; {went}{}",
            self.remedies(origin, suggest, "one", "them")
        )
    }

    /// FR-17's remedies, each after a `; `, in one order: how to plot the
    /// row, how to keep it off quietly (only when `suggest`, D-96), and,
    /// last, what `--table all` would also publish (FR-106). `one` names
    /// the row and `them` the rows `--table all` lists.
    fn remedies(self, origin: &Origin, suggest: bool, one: &str, them: &str) -> String {
        // `write!` rather than `push_str(&format!(..))`, which clippy's
        // `format_push_string` refuses; writing to a `String` cannot fail.
        // The sibling settings come from the ring set's source, so a ring
        // set from the file names the file's keys (UX review 4). Only the
        // default set's plot remedy is its own; a file's says "that list",
        // since the fact before it already named the rings key.
        let plot = match origin.source {
            Source::Default => {
                "pass --rings with every ring to draw (it replaces the defaults)".to_owned()
            }
            Source::Flag => format!("add it to {origin}"),
            Source::Config(_) => "add it to that list".to_owned(),
        };
        let quiet = origin.sibling(Setting::UnplottedRings);
        // The table remedy names the table's own flag when the command
        // line set it, since that overrides any key (final review M-3);
        // otherwise the table follows the ring set's source.
        let table = if self == Self::UnlistedByFlag {
            Origin::flag(Setting::Table)
        } else {
            origin.sibling(Setting::Table)
        };
        // A file's table remedy is a suggestion, worded as one (final
        // review M-1); the flag's stays "--table all lists".
        let lists = if table.source.is_config() {
            "would list"
        } else {
            "lists"
        };
        let all = table.set_to("all", "");
        let mut text = format!("; to plot {one}, {plot}");
        if suggest {
            write!(
                text,
                "; to leave {one} off without this warning, add it to {quiet}"
            )
            .expect("a String takes any write");
        }
        if matches!(self, Self::Unlisted | Self::UnlistedByFlag) {
            write!(
                text,
                "; {all} {lists} {them}, but also publishes every other column"
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
                    let class = kind.class();
                    match classes.iter_mut().find(|(c, _)| *c == class) {
                        Some((_, members)) => members.push(member),
                        None => classes.push((class, vec![member])),
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
        Class::Ring(origin) => grouped_rings(origin, members),
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

/// FR-17's grouped line: it lists the declared rings and says what to edit,
/// which depends on where the ring set came from (D-61).
// qual:allow(complexity, error_handling) reason: "a Ring class is built only from UndeclaredRing rows, so a member is always found"
fn grouped_rings(origin: &Origin, members: &[Member]) -> String {
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
    off_radar.ring_rows(n, origin, &list, &tallied(members, normalise), suggest)
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
    #[error("{output}: the output file name must end in .html, .png, .svg or .pdf")]
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
    #[error(
        "{palette}: cannot be read: {}{}",
        io_reason(source),
        palette.hint()
    )]
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
    #[error("{output}: the directory {dir} does not exist{}", output.hint())]
    MissingDirectory {
        /// The output, by its flag or key, as written.
        output: NamedPath,
        /// The directory part of it.
        dir: String,
    },
    /// FR-62, D-74: the output is a file the run reads, which the write
    /// would replace: the CSV, the `--config` file or the palette.
    #[error("{output}: is {input}; choose another output path")]
    OutputIsInput {
        /// The output, by its flag or key, as written.
        output: NamedPath,
        /// Which file it is.
        input: ReadFile,
    },
    /// The maintainer's ruling B (security review F4): a path from a
    /// `--config` file names a regular file, so a pipe, a terminal or a
    /// folder is never read or waited on.
    #[error(
        "{path}: not a regular file; a path in a config file must name a file, not a device, pipe or folder"
    )]
    NotRegular {
        /// The path, by its key, as written.
        path: NamedPath,
    },
    /// FR-92: a value holds `{edition}`, and no edition was given by `-e`
    /// or the `edition` key.
    #[error(
        "{value}: uses {PLACEHOLDER}, but no --edition was given; pass --edition YYYY-MM-DD{}",
        file.as_ref().map_or_else(String::new, |f| format!(", or add an edition key to {f}"))
    )]
    NoEdition {
        /// The value, by its flag or key, as written.
        value: NamedPath,
        /// The `--config` file, if one was read, which could hold the key.
        file: Option<Arc<str>>,
    },
    /// FR-92: placeholders are in use, and a value holds another one.
    #[error("{value}: {placeholder} is not a placeholder; the only one is {PLACEHOLDER}")]
    UnknownPlaceholder {
        /// The value, by its flag or key, as written.
        value: NamedPath,
        /// The placeholder, braces and all.
        placeholder: String,
    },
    /// FR-92 (rg-u6u, M-9b): `-o` is not UTF-8, so `{edition}` cannot be
    /// replaced in it without changing its other bytes.
    #[error("{output}: is not UTF-8 text, so {PLACEHOLDER} cannot expand in it")]
    EditionNotUtf8 {
        /// The output, by its flag, as written.
        output: NamedPath,
    },
    /// Security review F5: past [`crate::settings::file::MAX_KEY_PROBLEMS`],
    /// the rest are counted.
    #[error("--config {file}: and {more} more key problems")]
    ConfigMore {
        /// The file as named.
        file: Arc<str>,
        /// How many were not listed.
        more: usize,
    },
    /// Final review M-7 (ruling B): a palette a `--config` file names may
    /// be any file, so past the key problems' cap its errors are counted.
    #[error("{palette}: and {more} more palette problems")]
    PaletteMore {
        /// The palette, by its key, as written.
        palette: NamedPath,
        /// How many were not listed.
        more: usize,
    },
    /// NFR-15: an embedded font face does not parse.
    #[error(transparent)]
    Font(#[from] FontError),
    /// FR-114 (rg-969): a count the PDF writes as an integer is past the
    /// largest one it holds, so writing it would repeat a number.
    #[error(
        "the PDF would number {count} {what}, more than a PDF can number ({})",
        i32::MAX
    )]
    PdfTooLarge {
        /// What is counted.
        what: &'static str,
        /// How many.
        count: usize,
    },
    /// FR-61: the input does not exist, cannot be read, or is a directory.
    #[error("{path}: cannot be read: {}{hint}", io_reason(source))]
    Unreadable {
        /// The input path as given.
        path: String,
        /// What a config's `input` key adds ([`NamedPath::hint`]); empty
        /// for the command line's.
        hint: String,
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
    #[error("{output}: cannot be written: {}{}", io_reason(source), output.hint())]
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
        width_origin.smaller()
    )]
    ImageTooLarge {
        /// The width in pixels.
        width: u32,
        /// The height in pixels.
        height: u32,
        /// Where the width came from, which the remedy names (NFR-10).
        width_origin: Origin,
    },
    /// FR-60, FR-99, FR-101, FR-109: a `Cli` asking for no task, or for more than
    /// one. clap refuses such a command line, so only a `Cli` built by hand
    /// reaches this.
    #[error(
        "give a CSV and -o or --config to render, or --create-template, --create-palette or --create-config alone"
    )]
    NoTask,
    /// NFR-15, FR-91: the `--config` file does not exist or cannot be read.
    #[error("--config {file}: cannot be read: {}", io_reason(source))]
    ConfigUnreadable {
        /// The file as named.
        file: Arc<str>,
        /// The underlying I/O failure.
        #[source]
        source: std::io::Error,
    },
    /// NFR-15: the `--config` file passes its cap.
    #[error("--config {file}: larger than {} MiB; a config file is a few hundred bytes, so check the path", CONFIG_MAX_BYTES >> 20)]
    ConfigTooLarge {
        /// The file as named.
        file: Arc<str>,
    },
    /// NFR-15: the `--config` file is not UTF-8, or holds a NUL.
    #[error("--config {file}, line {line}: not UTF-8 text; re-save the file as UTF-8")]
    ConfigNotUtf8 {
        /// The file as named.
        file: Arc<str>,
        /// The line holding the first bad byte.
        line: usize,
    },
    /// NFR-15, FR-91: the file is not TOML, at the parser's line if any.
    #[error("{}", at_line(format_args!("--config {file}"), *line, message))]
    ConfigSyntax {
        /// The file as named.
        file: Arc<str>,
        /// The parser's line, if it gives one.
        line: Option<usize>,
        /// The parser's message, on one line.
        message: String,
    },
    /// FR-91: one key's problem, a usage error.
    #[error("{}", key_problem(file, problem))]
    ConfigKey {
        /// The file as named.
        file: Arc<str>,
        /// What is wrong with which key.
        problem: KeyProblem,
    },
    /// FR-59 within the file: a `theme` key and a `palette` key, a usage
    /// error. Across sources the flag wins, so only the file conflicts.
    #[error(
        "--config {file}: the theme and palette keys cannot be used together; a palette replaces the theme, so remove one"
    )]
    Conflict {
        /// The file as named.
        file: Arc<str>,
    },
    /// FR-112: `--pdf full` and `--paper none` from one source, a usage
    /// error; across sources the command line wins.
    #[error("{}", needs_paper(pdf, paper))]
    FullNeedsPaper {
        /// Where `full` came from.
        pdf: Origin,
        /// Where `none` came from.
        paper: Origin,
    },
    /// FR-91: neither `-o` nor `output`, in clap's words, a usage error.
    #[error(
        "no output: pass -o <OUTPUT>, or add an output key to {file}, such as output = \"radar.html\""
    )]
    NoOutput {
        /// The file as named.
        file: Arc<str>,
    },
    /// FR-91: neither a positional CSV nor `input`, worded as
    /// [`Fatal::NoOutput`] is; a usage error.
    #[error(
        "no input: name the CSV on the command line, or add an input key to {file}, such as input = \"radar.csv\""
    )]
    NoInput {
        /// The file as named.
        file: Arc<str>,
    },
    /// FR-99, FR-101, FR-109: a file to start from never replaces one (D-83,
    /// D-91, D-102).
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
    /// FR-99, FR-101, FR-109: the file cannot be written, for any of the reasons
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
        "the CSV has no header row (it is empty or blank); the header must name at least name, sector, ring and description"
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
    /// D-101, as FR-4's repeat: the header names the sector column twice,
    /// once by its former name, [`SECTOR_ALIAS`]. The earlier column is
    /// named first; both names are as written, trimmed.
    #[error(
        "line {line}: {} '{first_name}' and {} '{name}' both name the {} column ({SECTOR_ALIAS} is its former name); keep one, and remove or rename the other",
        column_label(*first_index),
        column_label(*index),
        ColumnKind::Sector.as_str()
    )]
    SectorTwice {
        /// The header's line.
        line: usize,
        /// The 0-based header index of the later column.
        index: usize,
        /// Its name, as written and trimmed.
        name: String,
        /// The 0-based header index of the earlier column.
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
        "line {line}: the header has no '{}' column; add one, or rename the column that holds it — the header must name at least name, sector, ring and description",
        column.as_str()
    )]
    MissingColumn {
        /// The header's line.
        line: usize,
        /// The missing column.
        column: ColumnKind,
    },
    /// FR-6: an empty `name`, `sector` or `ring`, empty under FR-76's
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
