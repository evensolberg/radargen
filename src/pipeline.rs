//! The run, end to end (architecture §2, §7): the flags resolved once into a
//! [`Config`], then the CSV, the catalogue, the order, the layout and the
//! page, then the write.
//!
//! Failures follow NFR-16's tiers. A setting's errors are collected by a
//! gate: a failed check on a command-line value stops the run, while one on
//! a value from `--config` lets the later checks run, so one run reports
//! every problem the file causes (FR-91); a file that cannot be read stops
//! it at once. Row-level errors from both CSV stages are merged and
//! grouped; a failed write is the third failure point (§7). Warnings
//! collected up to any of them still print, before the errors (NFR-11), and
//! nothing is written unless every stage succeeded (NFR-8).
#![deny(clippy::print_stdout)]

use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::cli::{Cli, Task, clean_setting};
use crate::csv::decode;
use crate::csv::parse::parse;
use crate::date::is_iso_date;
use crate::decode::NotUtf8;
use crate::diagnostics::{
    Fatal, OffRadar, PaperFrom, ReadFile, RenderKind, RowError, Warning, Warnings,
};
use crate::emit::html::{Page, page};
use crate::emit::write::parent_dir;
use crate::emit::write_atomic;
use crate::emit::{OUTPUT_VERSION, UNTITLED, pdf, png, svg};
use crate::errors::render_errors;
use crate::layout::arrange::{
    Arrangement, Blocks, arrange, narrow_parts, warn_narrow, warn_wide, wide_parts,
};
use crate::layout::geometry::warn_crowding;
use crate::layout::glyphs::{warn_missing_glyphs, warn_table_glyphs};
use crate::layout::legend::LegendBlock;
use crate::layout::paper::{self, Sheets, page_size};
use crate::layout::title::shown;
use crate::model::{Catalogue, build_catalogue};
use crate::options::{Legend, OutputFormat, Paper, Pdf, Table};
use crate::order::{Plotted, resolve};
use crate::palette::file::{self, ReadError};
use crate::palette::{Colours, Theme};
use crate::published::published;
use crate::rings::{RingSet, default_rings, parse_ring_list};
use crate::settings::edition::{Edition, Problem, Today, expand, uses};
use crate::settings::file::{self as config_file, MAX_KEY_PROBLEMS};
use crate::settings::{
    DEFAULT_LEGEND, DEFAULT_LEGEND_COLUMNS, DEFAULT_PDF, DEFAULT_TABLE, DEFAULT_THEME, Effective,
    NamedPath, Origin, Setting, Settings, Source, Sourced, resolve as resolve_path,
};
use crate::text::Fonts;
use crate::{ECHO_CHARS, clip};

/// The resolved run (architecture §7), read-only once built. The colours
/// are resolved here, so a bad `--palette` file is a tier-one error
/// (FR-102).
#[derive(Debug)]
pub struct Config {
    /// The output to write: `-o` as given, or the file's `output` key
    /// resolved against the file's folder (FR-91).
    pub output: PathBuf,
    /// How a message names the output: its flag or key, and the path as
    /// written.
    pub output_named: NamedPath,
    /// FR-52: what the output's extension asks for.
    pub format: OutputFormat,
    /// FR-56, FR-59: the built-in themes, or a `--palette` file's one set.
    pub colours: Colours,
    /// RG-102: where an HTML page's legend starts. PNG, SVG and PDF draw it
    /// anyway, and warn only when `hidden` came from the flag (D-100).
    pub legend: Sourced<Legend>,
    /// FR-106: what an HTML page's table publishes. PNG, SVG and PDF have no
    /// table, and warn only when `all` came from the flag (D-100).
    pub table: Sourced<Table>,
    /// FR-57: the width from `--width` or the file, which HTML ignores,
    /// with where it came from, so D-87's remedies can name it.
    pub width: Option<Sourced<u32>>,
    /// FR-100: the legend's columns from `--legend-columns`, the file or the
    /// default, which HTML ignores, with where they came from, so paper's
    /// warning (FR-111) can tell the flag from the file.
    pub legend_columns: Sourced<u8>,
    /// FR-112: what a PDF holds, with where it came from.
    pub pdf: Sourced<Pdf>,
    /// The file's `pdf = "full"` that `--paper none` displaced.
    pub dropped_full: Option<Origin>,
    /// FR-111: the paper a PDF is laid out on, from `--paper` or the file,
    /// with where it came from; absent, or `none`, is the drawing-sized
    /// page.
    pub paper: Option<Sourced<Paper>>,
    /// FR-65, resolved: cleaned (FR-90), trimmed, and `None` when empty,
    /// with its origin, so FR-104 can name it.
    pub title: Option<Sourced<String>>,
    /// As `title`.
    pub subtitle: Option<Sourced<String>>,
    /// The declared rings and their source (FR-12, FR-17).
    pub rings: RingSet,
    /// The input CSV: the positional path as given (FR-60), or the file's
    /// `input` key resolved against the file's folder (FR-91).
    pub input: PathBuf,
    /// How a message names the input: the path as written, or the key and
    /// file with it (NFR-10).
    pub input_shown: String,
    /// What an unreadable input's message adds, when a config file named
    /// it ([`NamedPath::hint`], rg-37u).
    pub input_hint: String,
}

impl Config {
    /// FR-65's title, if one shows.
    #[must_use]
    pub fn title(&self) -> Option<&str> {
        self.title.as_ref().map(|t| t.value.as_str())
    }

    /// The title a PDF draws and names itself with: FR-65's, or
    /// *Technology Radar*, since a PDF always draws one (FR-114, D-105).
    #[must_use]
    pub fn pdf_title(&self) -> &str {
        self.title().unwrap_or(UNTITLED)
    }

    /// FR-65's subtitle, if one shows.
    #[must_use]
    pub fn subtitle(&self) -> Option<&str> {
        self.subtitle.as_ref().map(|s| s.value.as_str())
    }

    /// FR-111: the paper's landscape page in points, when the output is a
    /// PDF on paper; FR-112: a full PDF with no paper is on Letter.
    #[must_use]
    pub fn paper(&self) -> Option<(f64, f64)> {
        self.sheet().and_then(Paper::points)
    }

    /// The paper a PDF is laid out on: `--paper letter` or `a4`, or Letter
    /// for a full PDF that names none; `None` for the drawing-sized page or
    /// another format.
    #[must_use]
    pub fn sheet(&self) -> Option<Paper> {
        if self.format != OutputFormat::Pdf {
            return None;
        }
        match &self.paper {
            Some(paper) if paper.value != Paper::None => Some(paper.value),
            Some(_) => None,
            None => self.full().then_some(Paper::Letter),
        }
    }

    /// FR-112: whether the output is a full PDF, the table after the radar.
    #[must_use]
    pub fn full(&self) -> bool {
        self.format == OutputFormat::Pdf && self.pdf.value == Pdf::Full
    }
}

/// Why a run stopped (NFR-16): file and flag errors, each on its own line
/// (tier one), or row errors, grouped against the table's row count (tier
/// two). Every stage returns it through `?`, and the run renders it once.
enum Failure {
    Alone(Vec<Fatal>),
    Rows { errors: Vec<RowError>, rows: usize },
}

impl Failure {
    fn render(self) -> Failed {
        match self {
            Self::Alone(errors) => Failed {
                usage: errors.iter().any(Fatal::is_usage),
                lines: errors.iter().map(Fatal::render).collect(),
            },
            Self::Rows { errors, rows } => Failed {
                lines: render_errors(&errors, rows),
                usage: false,
            },
        }
    }
}

impl From<Fatal> for Failure {
    fn from(error: Fatal) -> Self {
        Self::Alone(vec![error])
    }
}

/// clap's exit status for a usage error. FR-91 gives the file's key
/// problems, FR-59 across sources and a missing output the same status as
/// the flag errors they mirror.
const USAGE_STATUS: u8 = 2;

/// A run that wrote nothing: its error lines, and whether any error was a
/// usage error.
#[derive(Debug)]
pub struct Failed {
    /// Rendered error lines, in print order.
    pub lines: Vec<String>,
    /// Whether the run exits with clap's usage status.
    pub usage: bool,
}

/// What a run printed (NFR-11).
#[derive(Debug)]
pub struct Report {
    /// Rendered warning lines, in print order.
    pub warnings: Vec<String>,
    /// `Ok` when the output was written; otherwise the rendered error
    /// lines, and whether any was a usage error.
    pub outcome: Result<(), Failed>,
}

impl Report {
    /// Every line to print, in order: the warnings, then the errors
    /// (NFR-11).
    pub fn lines(&self) -> impl Iterator<Item = &str> {
        let errors = self
            .outcome
            .as_ref()
            .err()
            .into_iter()
            .flat_map(|f| &f.lines);
        self.warnings.iter().chain(errors).map(String::as_str)
    }

    /// NFR-8's exit status: 0 when the output was written, 2 for a usage
    /// error (clap's, and FR-91's), 1 otherwise.
    #[must_use]
    pub const fn status(&self) -> u8 {
        match &self.outcome {
            Ok(()) => 0,
            Err(failed) if failed.usage => USAGE_STATUS,
            Err(_) => 1,
        }
    }
}

/// The whole pipeline for `cli` (architecture §2). Warnings are rendered
/// last, so every outcome carries the warnings gathered before it.
///
/// `today` is the clock `-e today` reads, the local date and its zone:
/// [`crate::settings::edition::local_today`] in production, a fixed date
/// in tests. It is called at most once, and only for `-e today` (D-24,
/// D-58).
#[must_use]
pub fn run(cli: &Cli, today: impl FnOnce() -> Today) -> Report {
    let mut warnings = Warnings::default();
    let outcome = steps(cli, today, &mut warnings).map_err(Failure::render);
    Report {
        warnings: warnings.render(),
        outcome,
    }
}

/// The stages in order, each stopping the run on failure.
fn steps(cli: &Cli, today: impl FnOnce() -> Today, warnings: &mut Warnings) -> Result<(), Failure> {
    if let Task::Create(starter, path) = cli.task()? {
        return Ok(starter.create(path)?);
    }
    let config = config(cli, today, warnings).map_err(Failure::Alone)?;
    let fonts = Fonts::embedded().map_err(Fatal::from)?;
    // FR-106: the flags say where an off-radar row goes, so its warning
    // can say so when it is raised.
    let off_radar = OffRadar::of(
        config.format,
        config.full(),
        config.table.value,
        config.table.origin.is_flag(),
    );
    let catalogue = catalogue(&config, off_radar, warnings)?;
    let bytes = render(&catalogue, &config, &fonts, warnings)?;
    Ok(write_atomic(&config.output, &config.output_named, &bytes)?)
}

/// The effective edition's date (FR-92): as given, from `-e` or the
/// `edition` key; or for `today` the local date, read once and echoed with
/// its zone and source, so the render can be reproduced with an explicit
/// `-e` (D-58); a fallback to UTC is said first (review M-1).
fn edition_date(
    edition: Sourced<Edition>,
    today: impl FnOnce() -> Today,
    warnings: &mut Warnings,
) -> Sourced<String> {
    let value = match edition.value {
        Edition::Date(date) => date,
        Edition::Today => {
            let Today {
                date,
                zone,
                fallback,
            } = today();
            debug_assert!(is_iso_date(&date), "the clock gives FR-92's shape");
            if let Some(fallback) = fallback {
                warnings.push(Warning::Render(RenderKind::TodayInUtc(fallback)));
            }
            warnings.push(Warning::Render(RenderKind::EditionToday {
                date: date.clone(),
                zone,
                origin: edition.origin.clone(),
            }));
            date
        }
    };
    Sourced {
        value,
        origin: edition.origin,
    }
}

/// The run's settings, in FR-92's order: the merge (FR-91), FR-90 on the
/// text values, `{edition}` expanded, paths resolved against their
/// sources, then NFR-16 tier one's checks: FR-52, FR-62 and D-74 on the
/// output, FR-63 on the rings, FR-107, FR-102 on the palette.
fn config(
    cli: &Cli,
    today: impl FnOnce() -> Today,
    warnings: &mut Warnings,
) -> Result<Config, Vec<Fatal>> {
    let (settings, mut gate) = merged(cli)?;
    // FR-92: the clock is read only when the effective edition is `today`,
    // from either source (D-58).
    let edition = settings
        .edition
        .map(|given| edition_date(given, today, warnings));
    let dir = settings.dir.as_deref();
    // FR-90 first, every warning naming the value's source.
    let title = settings.title.given().and_then(|t| resolved(t, warnings));
    let subtitle = settings
        .subtitle
        .given()
        .and_then(|s| resolved(s, warnings));
    let rings = settings.rings.map(|r| cleaned(r, warnings));
    let unplotted = settings.unplotted_rings.map(|r| cleaned(r, warnings));
    // FR-92: `{edition}` expands in the output, title and subtitle, in
    // FR-91's order, after FR-90 and before any path resolves. The title
    // and subtitle arrive trimmed by `shown`, which cannot matter: a
    // placeholder holds no whitespace. Each is a check through the gate,
    // so a value from the file that fails drops out, as a broken key does,
    // and nothing later reports its unexpanded text (rg-u6u, M-9a).
    let mut expander = Expander {
        edition: &edition,
        file: cli.config.as_deref().map(config_file::file_name),
        used: false,
    };
    let output = expander.path(&mut gate, settings.output.given())?;
    let title = expander.text(&mut gate, title)?;
    let subtitle = expander.text(&mut gate, subtitle)?;
    if let (Effective::Given(date), false) = (&edition, expander.used) {
        warnings.push(Warning::Render(RenderKind::EditionUnused {
            edition: NamedPath {
                origin: date.origin.clone(),
                shown: date.value.clone(),
            },
        }));
    }
    let output = output.as_ref().map(|o| resolve_path(dir, o));
    let palette_path = settings.palette.as_given().map(|p| resolve_path(dir, p));
    // FR-91: the input, from the command line or the file's key, resolved
    // as `output` is; ruling B holds a key's path to a regular file.
    let input = settings.input.as_given().map(|i| resolve_path(dir, i));
    let input = match input {
        Some((path, named)) => gate
            .check(
                &named.origin.source,
                regular(&path, &named).map_err(|e| vec![e]),
            )?
            .map(|()| (path, named)),
        None => None,
    };
    let reads = read_files(input.as_ref(), cli.config.as_deref(), palette_path.as_ref());
    let format = checked_output(&mut gate, output.as_ref(), &reads)?;
    gate.errors
        .extend(full_needs_paper(format, &settings.pdf, &settings.paper));
    let ring_set = checked_rings(&mut gate, &rings, &unplotted)?;
    let colours = checked_colours(
        &mut gate,
        palette_path.as_ref(),
        &settings.palette,
        settings.theme,
    )?;
    // With no error recorded, no setting is broken and every check passed,
    // so each value below is present; the `else` returns the errors that
    // explain why one is not.
    let (
        true,
        Some((input, input_named)),
        Some(format),
        Some((output, output_named)),
        Some(rings),
        Some(colours),
    ) = (
        gate.errors.is_empty(),
        input,
        format,
        output,
        ring_set,
        colours,
    )
    else {
        // Every path to a missing value records its error first, so an
        // empty gate here would be a silent failure (Rust review M2).
        debug_assert!(
            !gate.errors.is_empty(),
            "a setting is missing with no error recorded"
        );
        return Err(gate.errors);
    };
    Ok(Config {
        output,
        output_named,
        format,
        colours,
        legend: settings.legend.or(Setting::Legend, DEFAULT_LEGEND),
        table: settings.table.or(Setting::Table, DEFAULT_TABLE),
        width: settings.width.given(),
        legend_columns: settings
            .legend_columns
            .or(Setting::LegendColumns, DEFAULT_LEGEND_COLUMNS),
        paper: settings.paper.given(),
        pdf: settings.pdf.clone().or(Setting::Pdf, DEFAULT_PDF),
        dropped_full: settings.dropped_full.clone(),
        title,
        subtitle,
        rings,
        input_shown: input_shown(&input_named),
        input_hint: input_named.hint(),
        input,
    })
}

/// FR-92's expansion of the output, title and subtitle against the
/// effective edition, recording whether any value holds `{edition}`.
struct Expander<'a> {
    /// The date from `-e` or the `edition` key; `Broken` when the key has
    /// a problem, already recorded.
    edition: &'a Effective<String>,
    /// The `--config` file, which a missing edition's remedy names.
    file: Option<Arc<str>>,
    /// Whether a value holds `{edition}`, expanded or not (review I-1).
    used: bool,
}

impl Expander<'_> {
    /// Records whether `text` holds `{edition}`, and says whether it must
    /// be dropped: it needs the `edition` key, and the key is broken. The
    /// value then goes as a broken key's dependants do, and only the key's
    /// problem is reported (plan RG-019A, Ruling 3).
    fn dropped(&mut self, text: &str) -> bool {
        let holds = uses(text);
        self.used |= holds;
        holds && *self.edition == Effective::Broken
    }

    /// The date `{edition}` expands to, if an edition was given.
    fn date(&self) -> Option<&str> {
        self.edition.as_given().map(|date| date.value.as_str())
    }

    /// FR-92 on the output path. A path that is not UTF-8, which only `-o`
    /// can give, is read for placeholders through its lossy form, exact for
    /// them since they are ASCII; it is used as given unless `{edition}`
    /// would have to change it, which is an error, since replacing the text
    /// would change its other bytes (rg-u6u, M-9b).
    fn path(
        &mut self,
        gate: &mut Gate,
        path: Option<Sourced<PathBuf>>,
    ) -> Result<Option<Sourced<PathBuf>>, Vec<Fatal>> {
        let Some(path) = path else {
            return Ok(None);
        };
        let text = path.value.to_string_lossy();
        if self.dropped(&text) {
            return Ok(None);
        }
        let named = NamedPath::new(path.origin.clone(), &path.value);
        let utf8 = path.value.to_str().is_some();
        let expanded = expand(&text, self.date())
            .map_err(|problems| self.errors(&named, problems))
            .and_then(|expanded| match (utf8, uses(&text)) {
                (true, _) => Ok(PathBuf::from(expanded)),
                (false, true) => Err(vec![Fatal::EditionNotUtf8 { output: named }]),
                (false, false) => Ok(path.value.clone()),
            });
        Ok(gate
            .check(&path.origin.source, expanded)?
            .map(|value| Sourced {
                value,
                origin: path.origin,
            }))
    }

    /// FR-92 on a title or subtitle, named by its source with its text
    /// quoted, a value from the file cut at [`ECHO_CHARS`] (FR-91).
    fn text(
        &mut self,
        gate: &mut Gate,
        text: Option<Sourced<String>>,
    ) -> Result<Option<Sourced<String>>, Vec<Fatal>> {
        let Some(text) = text else {
            return Ok(None);
        };
        if self.dropped(&text.value) {
            return Ok(None);
        }
        let shown = if text.origin.source.is_config() {
            clip(&text.value, ECHO_CHARS)
        } else {
            text.value.clone()
        };
        let named = NamedPath {
            origin: text.origin.clone(),
            shown: format!("'{shown}'"),
        };
        let expanded =
            expand(&text.value, self.date()).map_err(|problems| self.errors(&named, problems));
        Ok(gate
            .check(&text.origin.source, expanded)?
            .map(|value| Sourced {
                value,
                origin: text.origin,
            }))
    }

    /// FR-92's problems in one value as tier-one errors, every one
    /// reported (NFR-16).
    fn errors(&self, value: &NamedPath, problems: Vec<Problem>) -> Vec<Fatal> {
        problems
            .into_iter()
            .map(|problem| match problem {
                Problem::NoEdition => Fatal::NoEdition {
                    value: value.clone(),
                    file: self.file.clone(),
                },
                Problem::Unknown(placeholder) => Fatal::UnknownPlaceholder {
                    value: value.clone(),
                    placeholder,
                },
            })
            .collect()
    }
}

/// How a message names the input CSV: as written when given on the
/// command line, as every message about it always has; with its key and
/// file when the file's `input` key named it (NFR-10).
fn input_shown(named: &NamedPath) -> String {
    if named.origin.source.is_config() {
        named.to_string()
    } else {
        named.shown.clone()
    }
}

/// Every file the run reads, which D-74 keeps the output from replacing:
/// the CSV, the `--config` file and the palette (security review F2).
fn read_files<'a>(
    input: Option<&'a (PathBuf, NamedPath)>,
    config: Option<&'a Path>,
    palette: Option<&'a (PathBuf, NamedPath)>,
) -> Vec<(&'a Path, ReadFile)> {
    input
        .map(|(path, named)| {
            let file = match &named.origin.source {
                Source::Config(file) => Some(Arc::clone(file)),
                Source::Flag | Source::Default => None,
            };
            (path.as_path(), ReadFile::Csv(file))
        })
        .into_iter()
        .chain(config.map(|c| (c, ReadFile::Config)))
        .chain(palette.map(|(p, _)| (p.as_path(), ReadFile::Palette)))
        .collect()
}

/// FR-52, FR-62, ruling B and D-74 on the output, when there is one: a
/// broken or missing output has its error recorded already.
fn checked_output(
    gate: &mut Gate,
    output: Option<&(PathBuf, NamedPath)>,
    reads: &[(&Path, ReadFile)],
) -> Result<Option<OutputFormat>, Vec<Fatal>> {
    match output {
        Some((path, named)) => gate.check(&named.origin.source, output_checks(path, named, reads)),
        None => Ok(None),
    }
}

/// FR-91: the `--config` file, read only when named, merged under the
/// command line, with the file's key problems and the merge's own as the
/// gate's first errors. A file that cannot be read or is not TOML stops the
/// run alone (NFR-15).
fn merged(cli: &Cli) -> Result<(Settings, Gate), Vec<Fatal>> {
    let loaded = match &cli.config {
        Some(path) => Some(config_file::read(path).map_err(|e| vec![config_error(path, e)])?),
        None => None,
    };
    let settings = cli.settings(loaded.as_ref());
    let mut gate = Gate::default();
    gate.errors.extend(loaded.iter().flat_map(key_errors));
    gate.errors
        .extend(loaded.iter().flat_map(|l| merge_errors(&settings, l)));
    Ok((settings, gate))
}

/// FR-112: a full PDF needs paper. `none` against `full` from the same
/// source is a usage error; across sources the flag has already won
/// (`Settings::displace`).
fn full_needs_paper(
    format: Option<OutputFormat>,
    pdf: &Effective<Pdf>,
    paper: &Effective<Paper>,
) -> Option<Fatal> {
    match (pdf, paper) {
        (Effective::Given(pdf), Effective::Given(paper))
            if format == Some(OutputFormat::Pdf)
                && pdf.value == Pdf::Full
                && paper.value == Paper::None =>
        {
            Some(Fatal::FullNeedsPaper {
                pdf: pdf.origin.clone(),
                paper: paper.origin.clone(),
            })
        }
        _ => None,
    }
}

/// What the merge finds wrong beyond each key's own problems, usage errors
/// as clap's would be: FR-59 within the file, then neither `-o` nor an
/// `output` key (FR-91). The conflict is judged from the file's own values,
/// not the merge's, so a file holding both keys is an error whatever flags
/// are given, as a file is valid or not on its own (D-100, final review
/// I-2); across sources the flag wins ([`Settings::displace`]), and clap
/// refuses both flags.
fn merge_errors(settings: &Settings, loaded: &config_file::Loaded) -> Vec<Fatal> {
    let both = loaded.values.theme.is_some() && loaded.values.palette.is_some();
    let no_input = settings.input == Effective::Absent;
    let none = settings.output == Effective::Absent;
    let file = || Arc::clone(&loaded.file);
    both.then(|| Fatal::Conflict { file: file() })
        .into_iter()
        .chain(no_input.then(|| Fatal::NoInput { file: file() }))
        .chain(none.then(|| Fatal::NoOutput { file: file() }))
        .collect()
}

/// The file's key problems, each named by its key and the file (FR-91).
///
/// Past [`MAX_KEY_PROBLEMS`] the rest are counted, not listed (security
/// review F5).
fn key_errors(loaded: &config_file::Loaded) -> impl Iterator<Item = Fatal> + '_ {
    let more = loaded.problems.len().saturating_sub(MAX_KEY_PROBLEMS);
    let listed = loaded
        .problems
        .iter()
        .take(MAX_KEY_PROBLEMS)
        .map(|problem| Fatal::ConfigKey {
            file: Arc::clone(&loaded.file),
            problem: problem.clone(),
        });
    listed.chain((more > 0).then(|| Fatal::ConfigMore {
        file: Arc::clone(&loaded.file),
        more,
    }))
}

/// FR-63 on the ring set, then FR-107 on the unplotted rings against it;
/// a broken `rings` key skips both, since the ring set is checked first.
fn checked_rings(
    gate: &mut Gate,
    rings: &Effective<String>,
    unplotted: &Effective<String>,
) -> Result<Option<RingSet>, Vec<Fatal>> {
    let declared = match rings {
        Effective::Broken => None,
        other => {
            let given = other.as_given();
            gate.check(
                given.map_or(&Source::Default, |r| &r.origin.source),
                ring_set(given),
            )?
        }
    };
    Ok(match (declared, unplotted) {
        (Some(set), Effective::Given(list)) => {
            gate.check(&list.origin.source, unplotted_set(set, list))?
        }
        (Some(set), Effective::Absent) => Some(set),
        _ => None,
    })
}

/// FR-102 on the palette, from the flag or the file; else the built-in
/// themes with FR-56's theme. A broken `palette` key gives none: its key
/// problem is already recorded.
fn checked_colours(
    gate: &mut Gate,
    path: Option<&(PathBuf, NamedPath)>,
    palette: &Effective<PathBuf>,
    theme: Effective<Theme>,
) -> Result<Option<Colours>, Vec<Fatal>> {
    Ok(match (path, palette) {
        (Some((path, named)), _) => gate
            .check(
                &named.origin.source,
                regular(path, named)
                    .map_err(|e| vec![e])
                    .and_then(|()| file::read(path).map_err(|e| palette_errors(named, e))),
            )?
            .map(Colours::from),
        (None, Effective::Broken) => None,
        (None, _) => Some(Colours::BuiltIn(
            theme.given().map_or(DEFAULT_THEME, |t| t.value),
        )),
    })
}

/// FR-90 on a comma list, before FR-63 reads it.
fn cleaned(list: Sourced<String>, warnings: &mut Warnings) -> Sourced<String> {
    Sourced {
        value: clean_setting(&list.origin, &list.value, warnings),
        origin: list.origin,
    }
}

/// FR-63 on the declared rings, from the flag or the file, or FR-13's
/// default set when neither gave any.
fn ring_set(declared: Option<&Sourced<String>>) -> Result<RingSet, Vec<Fatal>> {
    let (parsed, origin) = declared.map_or_else(
        || (Ok(default_rings()), Origin::rings(Source::Default)),
        |list| {
            (
                parse_ring_list(&list.value, list.origin.source.clone()),
                list.origin.clone(),
            )
        },
    );
    parsed.map_err(|errors| {
        errors
            .into_iter()
            .map(|error| Fatal::Rings {
                origin: origin.clone(),
                error,
            })
            .collect()
    })
}

/// FR-107 on `list`, against `set`, whatever the sources of each.
fn unplotted_set(set: RingSet, list: &Sourced<String>) -> Result<RingSet, Vec<Fatal>> {
    set.with_unplotted(&list.value).map_err(|errors| {
        errors
            .into_iter()
            .map(|error| Fatal::UnplottedRings {
                origin: list.origin.clone(),
                error,
            })
            .collect()
    })
}

/// A `--config` file's read failure, named by the file (NFR-15).
fn config_error(path: &Path, error: config_file::ReadError) -> Fatal {
    let file = config_file::file_name(path);
    match error {
        config_file::ReadError::Unreadable(source) => Fatal::ConfigUnreadable { file, source },
        config_file::ReadError::TooLarge => Fatal::ConfigTooLarge { file },
        config_file::ReadError::NotUtf8(NotUtf8 { line }) => Fatal::ConfigNotUtf8 { file, line },
        config_file::ReadError::Syntax { line, message } => Fatal::ConfigSyntax {
            file,
            line,
            message,
        },
    }
}

/// NFR-16 tier one across sources (FR-91): every error is kept. A failed
/// check on a command-line value stops the run, as the first flag to fail
/// always has. One on a value from the file lets the checks go on, so
/// every problem the file causes is reported in one run (plan RG-019A,
/// Ruling 3).
#[derive(Default)]
struct Gate {
    errors: Vec<Fatal>,
}

impl Gate {
    /// `result`'s value; or, recording its errors, `None` to go on, or
    /// every error so far to stop.
    fn check<T>(
        &mut self,
        source: &Source,
        result: Result<T, Vec<Fatal>>,
    ) -> Result<Option<T>, Vec<Fatal>> {
        match result {
            Ok(value) => Ok(Some(value)),
            Err(errors) => {
                self.errors.extend(errors);
                if source.is_config() {
                    Ok(None)
                } else {
                    Err(std::mem::take(&mut self.errors))
                }
            }
        }
    }
}

/// A palette file's read failure as tier-one errors, named by its flag or
/// key, with the path as written rather than as opened (NFR-10, plan
/// RG-018A, R1).
fn palette_errors(palette: &NamedPath, error: ReadError) -> Vec<Fatal> {
    match error {
        ReadError::Unreadable(source) => vec![Fatal::PaletteUnreadable {
            palette: palette.clone(),
            source,
        }],
        ReadError::TooLarge => vec![Fatal::PaletteTooLarge {
            palette: palette.clone(),
        }],
        ReadError::NotUtf8(NotUtf8 { line }) => vec![Fatal::PaletteNotUtf8 {
            palette: palette.clone(),
            line,
        }],
        ReadError::Invalid(errors) => {
            // A palette the config names may be any file, so its errors
            // are capped as key problems are (final review M-7, ruling B);
            // a flag's palette is the author's own and lists every one.
            let cap = if palette.origin.source.is_config() {
                MAX_KEY_PROBLEMS
            } else {
                usize::MAX
            };
            let more = errors.len().saturating_sub(cap);
            errors
                .into_iter()
                .take(cap)
                .map(|error| Fatal::Palette {
                    palette: palette.clone(),
                    error,
                })
                .chain((more > 0).then(|| Fatal::PaletteMore {
                    palette: palette.clone(),
                    more,
                }))
                .collect()
        }
    }
}

/// FR-52's extension, FR-62's directory, ruling B's regular file and
/// D-74's same-file check against every file the run `reads`, the first
/// to fail alone (NFR-16 tier one).
fn output_checks(
    path: &Path,
    named: &NamedPath,
    reads: &[(&Path, ReadFile)],
) -> Result<OutputFormat, Vec<Fatal>> {
    let checked = || -> Result<OutputFormat, Fatal> {
        let format = OutputFormat::from_path(path).ok_or_else(|| Fatal::UnrecognisedExtension {
            output: named.clone(),
        })?;
        missing_directory(path, named)?;
        regular(path, named)?;
        // One handle on the output, compared with each file the run reads
        // (efficiency review 4).
        let handle = is_regular(path)
            .then(|| same_file::Handle::from_path(path).ok())
            .flatten();
        reads.iter().try_for_each(|(read, which)| {
            output_is_input(handle.as_ref(), named, read, which.clone())
        })?;
        Ok(format)
    };
    checked().map_err(|e| vec![e])
}

/// The maintainer's ruling B (security review F4): a path from a
/// `--config` file that exists must be a regular file, so a pipe, a
/// terminal or a folder is refused rather than read or waited on. Nothing
/// before it opens a path that is not a regular file: D-74's check skips
/// one ([`output_is_input`]). A path that does not exist is left to the
/// read or the write; a flag's path is the author's own, so it is not
/// checked.
fn regular(path: &Path, named: &NamedPath) -> Result<(), Fatal> {
    // The source first, so a run without a config pays no stat
    // (efficiency review 3).
    if !named.origin.source.is_config() {
        return Ok(());
    }
    match std::fs::metadata(path) {
        Ok(meta) if !meta.is_file() => Err(Fatal::NotRegular {
            path: named.clone(),
        }),
        _ => Ok(()),
    }
}

/// FR-62: the output's directory must exist. Only a directory that is not there
/// is this error; one that is a file or cannot be searched is left to the
/// write, which stays the final authority, so a directory removed after
/// this check still fails cleanly there.
fn missing_directory(output: &Path, named: &NamedPath) -> Result<(), Fatal> {
    let dir = parent_dir(output);
    match std::fs::metadata(dir) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => Err(Fatal::MissingDirectory {
            output: named.clone(),
            dir: dir.display().to_string(),
        }),
        _ => Ok(()),
    }
}

/// Whether `path` names a regular file, following symlinks; `false` when
/// it cannot be read.
fn is_regular(path: &Path) -> bool {
    std::fs::metadata(path).is_ok_and(|meta| meta.is_file())
}

/// D-74: the output must not be a file the run reads: the input CSV, the
/// `--config` file or the palette (security review F2). Files are compared
/// by identity (device and inode, or volume and file index on Windows), so
/// `./`, `..`, symlinks, case and hard links are all caught, and no path
/// spelling needs resolving. A path that cannot be opened is not the input,
/// and an output that cannot be opened (`output` is `None`) is none of
/// them.
///
/// Only a regular file is opened: opening a pipe blocks until a writer
/// comes, and a pipe, a terminal or a folder can never be the same file as
/// a regular output, so it is skipped (final review I-1).
fn output_is_input(
    output: Option<&same_file::Handle>,
    named: &NamedPath,
    input: &Path,
    which: ReadFile,
) -> Result<(), Fatal> {
    let same = output.is_some_and(|out| {
        is_regular(input) && same_file::Handle::from_path(input).is_ok_and(|read| &read == out)
    });
    if same {
        Err(Fatal::OutputIsInput {
            output: named.clone(),
            input: which,
        })
    } else {
        Ok(())
    }
}

/// FR-65's resolved value: cleaned (FR-90), trimmed, `None` when empty;
/// it keeps its origin, so FR-104 can name it.
fn resolved(text: Sourced<String>, warnings: &mut Warnings) -> Option<Sourced<String>> {
    let cleaned = clean_setting(&text.origin, &text.value, warnings);
    shown(Some(&cleaned)).map(|value| Sourced {
        value: value.to_owned(),
        origin: text.origin,
    })
}

/// Read, parse and build. A file-level failure is one line (tier one);
/// row-level errors from both stages are merged and grouped (tier two),
/// and `parse`'s alone fail the run, since it drops the rows it rejects.
fn catalogue(
    config: &Config,
    off_radar: OffRadar,
    warnings: &mut Warnings,
) -> Result<Catalogue, Failure> {
    let rings = &config.rings;
    let text = decode::read_as(&config.input, &config.input_shown, &config.input_hint)?;
    let (table, mut errors) = parse(&text, warnings)?;
    let built =
        build_catalogue(&table, rings, off_radar, warnings).map_err(|more| errors.extend(more));
    match built {
        Ok(catalogue) if errors.is_empty() => Ok(catalogue),
        _ => Err(Failure::Rows {
            errors,
            rows: table.records.len(),
        }),
    }
}

/// RG-102, FR-106: a static snapshot has no button to bring a hidden
/// legend back, nor a table, so a flag asking for either is warned about.
/// The same value from --config is silent, since one file serves every
/// format a radar is rendered to (D-100, rg-qag).
fn warn_static_flags(config: &Config, warnings: &mut Warnings) {
    if !config.format.is_snapshot() {
        return;
    }
    if config.legend.value == Legend::Hidden && config.legend.origin.is_flag() {
        warnings.push(Warning::Render(RenderKind::LegendStatic));
    }
    // FR-112: the full PDF has the table.
    if config.table.value == Table::All && config.table.origin.is_flag() && !config.full() {
        warnings.push(Warning::Render(RenderKind::TableStatic {
            format: config.format,
        }));
    }
}

/// FR-111: `--paper` from the flag on an output with no paper, and
/// `--width` or `--legend-columns` from the flag on a PDF on paper, which
/// sets both, are warned about; so is `--pdf full` from the flag on an
/// output that is not a PDF (FR-112). The same values from `--config` are
/// silent, since one file serves every format (D-100).
fn warn_paper_flags(config: &Config, warnings: &mut Warnings) {
    if config.format != OutputFormat::Pdf {
        warn_no_paper(config, warnings);
        return;
    }
    if let Some(pdf) = config.dropped_full.as_ref() {
        warnings.push(Warning::Render(RenderKind::FullDropped {
            pdf: pdf.clone(),
        }));
    }
    let paper = config.paper.as_ref().filter(|p| p.value != Paper::None);
    if paper.is_none() && !config.full() {
        return;
    }
    let from = PaperFrom {
        paper: paper.cloned(),
        full: config.full().then(|| config.pdf.origin.clone()),
    };
    if let Some(width) = config.width.as_ref().filter(|w| w.origin.is_flag()) {
        warnings.push(Warning::Render(RenderKind::PaperIgnoresWidth {
            width: width.value,
            paper: from.clone(),
        }));
    }
    if config.legend_columns.origin.is_flag() {
        warnings.push(Warning::Render(RenderKind::PaperIgnoresLegendColumns {
            columns: config.legend_columns.value,
            paper: from,
        }));
    }
}

/// `--pdf full` and `--paper` from the flag on an output that is not a
/// PDF (FR-111, FR-112).
fn warn_no_paper(config: &Config, warnings: &mut Warnings) {
    if config.pdf.value == Pdf::Full && config.pdf.origin.is_flag() {
        warnings.push(Warning::Render(RenderKind::PdfNotPdf {
            pdf: config.pdf.value,
        }));
    }
    let flagged = config
        .paper
        .as_ref()
        .filter(|p| p.value != Paper::None && p.origin.is_flag());
    if let Some(paper) = flagged {
        warnings.push(Warning::Render(RenderKind::PaperNotPdf {
            paper: paper.value,
        }));
    }
}

/// The order, the render warnings (FR-88, FR-89, FR-95, FR-104, FR-105, D-87), the blocks
/// built once, and the output: the page around both arrangements (FR-68),
/// or the wide arrangement as a PNG, an SVG or a PDF (FR-71).
fn render(
    catalogue: &Catalogue,
    config: &Config,
    fonts: &Fonts,
    warnings: &mut Warnings,
) -> Result<Vec<u8>, Fatal> {
    let plotted = resolve(catalogue);
    // A PDF always draws its title, so a tagged PDF opens with its H1
    // (FR-65, FR-114).
    let title = if matches!(config.format, OutputFormat::Pdf) {
        Some(config.pdf_title())
    } else {
        config.title()
    };
    let blocks = Blocks::new(catalogue, &plotted, title, config.subtitle(), fonts);
    // FR-89 at the growth the radar is drawn with (D-86): only what still
    // overlaps is reported.
    warn_crowding(catalogue, &plotted, blocks.radar.growth(), warnings);
    warn_static_flags(config, warnings);
    warn_paper_flags(config, warnings);
    // FR-104: a static snapshot draws every text with the bundled font
    // alone, so a character it lacks is a box; the page falls back to the
    // reader's fonts, so it is not warned (D-93).
    if config.format.is_snapshot() {
        warn_missing_glyphs(
            catalogue,
            &plotted,
            config.title.as_ref(),
            config.subtitle.as_ref(),
            config.format,
            fonts,
            warnings,
        );
    }
    // FR-100: only a static output flows its legend; the page's scrolls
    // beside the radar, so HTML ignores the flag, silently (D-90).
    // FR-111: on paper the page sets the legend's columns instead.
    let blocks = if config.format.is_snapshot() && config.paper().is_none() {
        blocks.with_legend_columns(usize::from(config.legend_columns.value))
    } else {
        blocks
    };
    match config.format {
        OutputFormat::Html => {
            // FR-105 and FR-95 concern the switch between the arrangements
            // and the narrow one, which only HTML carries.
            let wide = wide_parts(&blocks);
            warn_wide(&blocks, &wide, warnings);
            warn_narrow(&blocks, warnings);
            Ok(page(&Page {
                wide: &wide,
                narrow: &narrow_parts(&blocks),
                catalogue,
                plotted: &plotted,
                title: config.title(),
                colours: &config.colours,
                legend: config.legend.value,
                table: config.table.value,
            })
            .into_bytes())
        }
        OutputFormat::Png => {
            let wide = arrange(Arrangement::Wide, &blocks);
            let width_origin = config.width.as_ref().map_or_else(
                || Origin::new(Setting::Width, Source::Default),
                |w| w.origin.clone(),
            );
            let size = png::pixel_size(&wide, config.width.as_ref().map(|w| w.value)).map_err(
                |png::TooLarge { width, height }| Fatal::ImageTooLarge {
                    width,
                    height,
                    width_origin: width_origin.clone(),
                },
            )?;
            if let Some(natural) = size.clamped() {
                warnings.push(Warning::Render(RenderKind::WidthClamped {
                    natural,
                    drawn: size.width(),
                }));
            }
            if let png::Legibility::Small { legible_from } = size.legibility() {
                warnings.push(Warning::Render(RenderKind::PngTextSmall {
                    width: size.width(),
                    legible_from,
                    width_origin,
                }));
            }
            png::rasterise(&wide, &config.colours.snapshot(), size)
        }
        OutputFormat::Svg => Ok(svg::export(
            &arrange(Arrangement::Wide, &blocks),
            &svg::Export {
                palette: &config.colours.snapshot(),
                fonts,
                width: config.width.as_ref().map(|w| w.value),
                title: config.title(),
                desc: &blocks.radar.name(),
            },
        )?
        .into_bytes()),
        OutputFormat::Pdf if let Some(points) = config.paper() => on_paper(
            catalogue, &plotted, &blocks, config, points, fonts, warnings,
        ),
        OutputFormat::Pdf => Ok(pdf::export(
            &arrange(Arrangement::Wide, &blocks),
            config.width.as_ref().map(|w| w.value),
            &pdf::Export {
                palette: &config.colours.snapshot(),
                fonts,
                title: Some(config.pdf_title()),
                desc: &blocks.radar.name(),
                producer: OUTPUT_VERSION,
            },
        )?),
    }
}

/// The PDF on paper (FR-111): the leading pages as the paper lays them
/// out, the legend at paper's size, each page footed; a radar that does
/// not fit the first page even at its smallest is warned about.
fn on_paper(
    catalogue: &Catalogue,
    plotted: &Plotted,
    blocks: &Blocks,
    config: &Config,
    points: (f64, f64),
    fonts: &Fonts,
    warnings: &mut Warnings,
) -> Result<Vec<u8>, Fatal> {
    let legend = LegendBlock::on_paper(catalogue, plotted, fonts);
    // FR-112: a full PDF's table, as the HTML page publishes it.
    let table = config
        .full()
        .then(|| published(catalogue, plotted, config.table.value));
    if let Some(table) = &table {
        warn_table_glyphs(catalogue, table, config.format, fonts, warnings);
    }
    let sheets = Sheets {
        radar: &blocks.radar,
        key: &blocks.key,
        legend: &legend,
        title: config.pdf_title(),
        subtitle: config.subtitle(),
        size: page_size(points),
        table: table.as_ref(),
    };
    let bodies = paper::bodies(&sheets, fonts);
    if bodies.radar_too_wide {
        warnings.push(Warning::Render(RenderKind::PaperRadarWide {
            paper: config.sheet().unwrap_or(Paper::Letter),
            full: config.full(),
        }));
    }
    if let Some(columns) = bodies.table_cramped {
        warnings.push(Warning::Render(RenderKind::TableCramped {
            columns,
            table: config.table.clone(),
        }));
    }
    for (line, name) in &bodies.tall_legend_rows {
        warnings.push(Warning::Render(RenderKind::LegendRowTall {
            line: *line,
            name: name.clone(),
        }));
    }
    for (line, name) in &bodies.tall_table_rows {
        warnings.push(Warning::Render(RenderKind::TableRowTall {
            line: *line,
            name: name.clone(),
        }));
    }
    pdf::export_pages(
        &paper::footed(bodies, sheets.footing(), fonts),
        &pdf::Export {
            palette: &config.colours.snapshot(),
            fonts,
            title: Some(config.pdf_title()),
            desc: &blocks.radar.name(),
            producer: OUTPUT_VERSION,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::edition::Fallback;

    fn report(outcome: Result<(), Failed>) -> Report {
        Report {
            warnings: vec!["warning: w".into()],
            outcome,
        }
    }

    #[test]
    fn warnings_print_before_errors() {
        let failed = report(Err(Failed {
            lines: vec!["error: e".into()],
            usage: false,
        }));
        assert_eq!(
            failed.lines().collect::<Vec<_>>(),
            ["warning: w", "error: e"]
        );
        assert_eq!(report(Ok(())).lines().collect::<Vec<_>>(), ["warning: w"]);
    }

    #[test]
    fn only_a_written_run_exits_zero() {
        assert_eq!(report(Ok(())).status(), 0);
        assert_eq!(
            report(Err(Failed {
                lines: vec!["error: e".into()],
                usage: false,
            }))
            .status(),
            1
        );
        assert_eq!(
            report(Err(Failed {
                lines: vec![],
                usage: true,
            }))
            .status(),
            2
        );
    }

    /// `radargen args…` in a directory holding a one-row CSV, with the
    /// clock `today` gives; the report and the directory.
    fn edition_run(args: &[&str], today: impl FnOnce() -> Today) -> (Report, tempfile::TempDir) {
        edition_run_with(None, args, today)
    }

    /// As [`edition_run`], with `toml`, when given, at `@radar.toml`.
    fn edition_run_with(
        toml: Option<&str>,
        args: &[&str],
        today: impl FnOnce() -> Today,
    ) -> (Report, tempfile::TempDir) {
        use clap::Parser;
        let dir = tempfile::tempdir().expect("tempdir");
        let csv = dir.path().join("radar.csv");
        std::fs::write(&csv, "name,sector,ring,description\nKafka,Tool,Adopt,d\n").expect("seeds");
        if let Some(toml) = toml {
            std::fs::write(dir.path().join("radar.toml"), toml).expect("seeds");
        }
        let out = |name: &str| dir.path().join(name).display().to_string();
        let args: Vec<String> = args
            .iter()
            .map(|a| a.strip_prefix('@').map_or_else(|| (*a).to_owned(), out))
            .collect();
        let cli = Cli::try_parse_from(
            std::iter::once("radargen".to_owned())
                .chain(std::iter::once(out("radar.csv")))
                .chain(args),
        )
        .expect("parses");
        (run(&cli, today), dir)
    }

    /// D-58: `-e today` expands to the injected date, exactly as that date
    /// typed would, and echoes it once.
    #[test]
    fn today_reads_the_injected_clock_once_and_echoes_it() {
        let (report, dir) = edition_run(
            &[
                "-o",
                "@{edition}.svg",
                "-t",
                "Radar {edition}",
                "-e",
                "today",
            ],
            || vancouver(None),
        );
        assert_eq!(report.status(), 0, "{:?}", report.outcome);
        assert_eq!(
            report.warnings,
            [
                "warning: edition 2026-10-22 (today, America/Vancouver); pass -e 2026-10-22 to reproduce this render"
            ]
        );
        let svg = std::fs::read_to_string(dir.path().join("2026-10-22.svg")).expect("written");
        assert!(svg.contains("<title>Radar 2026-10-22</title>"), "{svg}");
    }

    /// The fixed clock: 2026-10-22 in Vancouver, or with `fallback`, in UTC.
    fn vancouver(fallback: Option<Fallback>) -> Today {
        Today {
            date: "2026-10-22".into(),
            zone: if fallback.is_some() {
                "UTC"
            } else {
                "America/Vancouver"
            }
            .into(),
            fallback,
        }
    }

    /// Review M-1: a fallback to UTC is said before the echo, which names
    /// UTC; a bad `TZ` is cut and escaped as any echoed text is.
    #[test]
    fn a_fallback_to_utc_is_said() {
        let bad = format!("Bad\u{1}{}", "z".repeat(50));
        let (report, _dir) = edition_run(&["-o", "@{edition}.svg", "-e", "today"], || {
            vancouver(Some(Fallback::BadTz(bad)))
        });
        assert_eq!(report.status(), 0, "{:?}", report.outcome);
        assert_eq!(
            report.warnings,
            [
                format!(
                    "warning: TZ 'Bad\\u{{1}}{}…' names no time zone radargen can find, so -e today used UTC; set TZ to a zone such as America/Vancouver",
                    "z".repeat(36)
                ),
                "warning: edition 2026-10-22 (today, UTC); pass -e 2026-10-22 to reproduce this render".to_owned(),
            ]
        );
        let (report, _dir) = edition_run(&["-o", "@{edition}.svg", "-e", "today"], || {
            vancouver(Some(Fallback::NoSystemZone))
        });
        assert_eq!(
            report.warnings.first().map(String::as_str),
            Some(
                "warning: the system's time zone cannot be determined, so -e today used UTC; set TZ to a zone such as America/Vancouver"
            )
        );
    }

    /// D-58 (maintainer, 2026-10-22): `edition = "today"` reads the clock
    /// with no flag, and the echo names the key and its file.
    #[test]
    fn a_today_key_reads_the_clock_and_names_itself() {
        let (report, dir) = edition_run_with(
            Some(
                "output = \"{edition}.svg\"\nsubtitle = \"Created {edition}\"\nedition = \"today\"\n",
            ),
            &["-c", "@radar.toml"],
            || vancouver(None),
        );
        assert_eq!(report.status(), 0, "{:?}", report.outcome);
        let file = dir.path().join("radar.toml").display().to_string();
        assert_eq!(
            report.warnings,
            [format!(
                "warning: the edition key in {file}: edition 2026-10-22 (today, America/Vancouver); pass -e 2026-10-22 to reproduce this render"
            )]
        );
        let svg = std::fs::read_to_string(dir.path().join("2026-10-22.svg")).expect("written");
        assert!(svg.contains("Created 2026-10-22"), "{svg}");
    }

    /// FR-91: `-e` overrides the key, so `edition = "today"` under `-e`
    /// never reads the clock.
    #[test]
    fn the_flag_overrides_a_today_key() {
        let never = || -> Today { panic!("the clock was read") };
        let (report, dir) = edition_run_with(
            Some("output = \"{edition}.svg\"\nedition = \"today\"\n"),
            &["-c", "@radar.toml", "-e", "2026-09-01"],
            never,
        );
        assert_eq!(report.status(), 0, "{:?}", report.outcome);
        assert!(report.warnings.is_empty(), "{:?}", report.warnings);
        assert!(dir.path().join("2026-09-01.svg").exists());
    }

    /// D-24: without `today`, the clock is never read, a date and no
    /// `--edition` alike.
    #[test]
    fn no_other_run_reads_the_clock() {
        let never = || -> Today { panic!("the clock was read") };
        let (dated, _dir) = edition_run(&["-o", "@{edition}.svg", "-e", "2026-09-01"], never);
        assert_eq!(dated.status(), 0, "{:?}", dated.outcome);
        let (plain, _dir) = edition_run(&["-o", "@a.svg"], never);
        assert_eq!(plain.status(), 0, "{:?}", plain.outcome);
    }

    /// `today` is checked as any date is: unused, it warns as one would.
    #[test]
    fn an_unused_today_warns_as_a_date_would() {
        let (report, _dir) = edition_run(&["-o", "@a.svg", "-e", "today"], || vancouver(None));
        assert_eq!(report.status(), 0, "{:?}", report.outcome);
        assert_eq!(
            report.warnings,
            [
                "warning: edition 2026-10-22 (today, America/Vancouver); pass -e 2026-10-22 to reproduce this render",
                "warning: --edition 2026-10-22: no value uses it; put {edition} in the output, title or subtitle",
            ]
        );
    }

    /// rg-u6u, M-9b: an output path that is not UTF-8 is used as given
    /// unless `{edition}` would have to change it.
    #[cfg(unix)]
    #[test]
    fn a_non_utf8_output_cannot_expand() {
        use std::ffi::OsStr;
        use std::os::unix::ffi::OsStrExt;
        let sourced = |bytes: &[u8]| {
            Some(Sourced {
                value: PathBuf::from(OsStr::from_bytes(bytes)),
                origin: Origin::flag(Setting::Output),
            })
        };
        let none = Effective::Absent;
        let mut plain_run = Expander {
            edition: &none,
            file: None,
            used: false,
        };
        let plain = plain_run.path(&mut Gate::default(), sourced(b"a\xff{x}.svg"));
        assert_eq!(
            plain.ok().flatten().map(|p| p.value),
            Some(PathBuf::from(OsStr::from_bytes(b"a\xff{x}.svg")))
        );
        let date = Effective::Given(Sourced {
            value: "2026-09-01".to_owned(),
            origin: Origin::flag(Setting::Edition),
        });
        let mut dated_run = Expander {
            edition: &date,
            file: None,
            used: false,
        };
        let dated = dated_run.path(&mut Gate::default(), sourced(b"a\xff{edition}.svg"));
        let lines: Vec<String> = dated
            .err()
            .unwrap_or_default()
            .iter()
            .map(Fatal::render)
            .collect();
        assert_eq!(
            lines,
            [
                "error: -o a\u{fffd}{edition}.svg: is not UTF-8 text, so {edition} cannot expand in it"
            ]
        );
        // Review I-1: it holds `{edition}`, so it uses the date though it fails.
        assert!(dated_run.used, "a failing value still uses --edition");
    }
}
