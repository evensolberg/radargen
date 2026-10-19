//! The run, end to end (architecture §2, §7): the flags resolved once into a
//! [`Config`], then the CSV, the catalogue, the order, the layout and the
//! page, then the write.
//!
//! Failures follow NFR-16's tiers. A setting's errors are collected by a
//! gate: a failed check on a command-line value stops the run, while one on
//! a value from `--config` lets the later checks run, so one run reports
//! every problem the file causes (FR-91); a file that cannot be read stops
//! it at once. Row-level errors from both CSV stages are merged and
//! grouped; a failed
//! write is the third failure point (§7). Warnings collected up to any of
//! them still print, before the errors (NFR-11), and nothing is written
//! unless every stage succeeded (NFR-8).
#![deny(clippy::print_stdout)]

use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::cli::{Cli, OutputFormat, Task, clean_setting, format};
use crate::csv::decode;
use crate::csv::parse::parse;
use crate::decode::NotUtf8;
use crate::diagnostics::{
    Fatal, MAX_KEY_PROBLEMS, OffRadar, ReadFile, RenderKind, RowError, Warning, Warnings,
};
use crate::emit::html::{Legend, Page, Table, page};
use crate::emit::write::parent_dir;
use crate::emit::write_atomic;
use crate::emit::{png, svg};
use crate::errors::render_errors;
use crate::layout::arrange::{
    Arrangement, Blocks, arrange, narrow_parts, warn_narrow, warn_wide, wide_parts,
};
use crate::layout::geometry::warn_crowding;
use crate::layout::glyphs::warn_missing_glyphs;
use crate::layout::title::shown;
use crate::model::{Catalogue, build_catalogue};
use crate::order::resolve;
use crate::palette::file::{self, ReadError};
use crate::palette::{Colours, Theme};
use crate::rings::{RingSet, default_rings, parse_ring_list};
use crate::settings::file as config_file;
use crate::settings::{
    DEFAULT_LEGEND, DEFAULT_LEGEND_COLUMNS, DEFAULT_TABLE, DEFAULT_THEME, Effective, NamedPath,
    Origin, Setting, Settings, Source, Sourced, resolve as resolve_path,
};
use crate::text::Fonts;

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
    /// RG-102: where an HTML page's legend starts; PNG and SVG warn on
    /// `hidden` and draw it anyway.
    pub legend: Sourced<Legend>,
    /// FR-106: what an HTML page's table publishes; PNG and SVG warn on
    /// `all`, having no table.
    pub table: Sourced<Table>,
    /// FR-57: the width from `--width` or the file, which HTML ignores, with
    /// where it came from, so
    /// D-87's remedies can name it.
    pub width: Option<Sourced<u32>>,
    /// FR-100: the legend's columns from `--legend-columns`, the file or the
    /// default, which HTML ignores.
    pub legend_columns: u8,
    /// FR-65, resolved: cleaned (FR-90), trimmed, and `None` when empty,
    /// with its origin, so FR-104 can name it.
    pub title: Option<Sourced<String>>,
    /// As `title`.
    pub subtitle: Option<Sourced<String>>,
    /// The declared rings and their source (FR-12, FR-17).
    pub rings: RingSet,
    /// The input CSV, as given (FR-60).
    pub input: PathBuf,
}

impl Config {
    /// FR-65's title, if one shows.
    #[must_use]
    pub fn title(&self) -> Option<&str> {
        self.title.as_ref().map(|t| t.value.as_str())
    }

    /// FR-65's subtitle, if one shows.
    #[must_use]
    pub fn subtitle(&self) -> Option<&str> {
        self.subtitle.as_ref().map(|s| s.value.as_str())
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
#[must_use]
pub fn run(cli: &Cli) -> Report {
    let mut warnings = Warnings::default();
    let outcome = steps(cli, &mut warnings).map_err(Failure::render);
    Report {
        warnings: warnings.render(),
        outcome,
    }
}

/// The stages in order, each stopping the run on failure.
fn steps(cli: &Cli, warnings: &mut Warnings) -> Result<(), Failure> {
    let input = match cli.task()? {
        Task::Create(starter, path) => return Ok(starter.create(path)?),
        Task::Render { input } => input,
    };
    let config = config(cli, input, warnings).map_err(Failure::Alone)?;
    let fonts = Fonts::embedded().map_err(Fatal::from)?;
    // FR-106: the flags say where an off-radar row goes, so its warning
    // can say so when it is raised.
    let off_radar = OffRadar::of(config.format, config.table.value);
    let catalogue = catalogue(&config.input, &config.rings, off_radar, warnings)?;
    let bytes = render(&catalogue, &config, &fonts, warnings)?;
    Ok(write_atomic(&config.output, &config.output_named, &bytes)?)
}

/// The run's settings, in FR-92's order: the merge (FR-91), FR-90 on the
/// text values, paths resolved against their sources, then NFR-16 tier
/// one's checks: FR-52, FR-62 and D-74 on the output, FR-63 on the rings,
/// FR-107, FR-102 on the palette.
fn config(cli: &Cli, input: &Path, warnings: &mut Warnings) -> Result<Config, Vec<Fatal>> {
    let (settings, mut gate) = merged(cli)?;
    let Settings {
        output,
        title,
        subtitle,
        rings,
        unplotted_rings,
        theme,
        width,
        legend_columns,
        legend,
        table,
        palette,
        dir,
    } = settings;
    // FR-90 first, every warning naming the value's source.
    let title = title.given().and_then(|t| resolved(t, warnings));
    let subtitle = subtitle.given().and_then(|s| resolved(s, warnings));
    let rings = rings.map(|r| cleaned(r, warnings));
    let unplotted = unplotted_rings.map(|r| cleaned(r, warnings));
    // FR-92 (PR 19b): `{edition}` expands in the output, title and subtitle
    // here, after FR-90 and before any path resolves.
    let output = output.as_given().map(|o| resolve_path(dir.as_deref(), o));
    let palette_path = palette.as_given().map(|p| resolve_path(dir.as_deref(), p));
    let format = match &output {
        Some((path, named)) => {
            let mut reads = vec![(input, ReadFile::Csv)];
            reads.extend(cli.config.as_deref().map(|c| (c, ReadFile::Config)));
            reads.extend(
                palette_path
                    .as_ref()
                    .map(|(p, _)| (p.as_path(), ReadFile::Palette)),
            );
            gate.check(&named.origin.source, output_checks(path, named, &reads))?
        }
        None => None,
    };
    let ring_set = checked_rings(&mut gate, &rings, &unplotted)?;
    let palette_broken = palette == Effective::Broken;
    let colours = checked_colours(&mut gate, palette_path.as_ref(), palette_broken, theme)?;
    // With no error recorded, no setting is broken and every check passed,
    // so each value below is present; the `else` returns the errors that
    // explain why one is not.
    let (true, Some(format), Some((output, output_named)), Some(rings), Some(colours)) =
        (gate.errors.is_empty(), format, output, ring_set, colours)
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
        legend: legend.or(Setting::Legend, DEFAULT_LEGEND),
        table: table.or(Setting::Table, DEFAULT_TABLE),
        width: width.given(),
        legend_columns: legend_columns
            .or(Setting::LegendColumns, DEFAULT_LEGEND_COLUMNS)
            .value,
        title,
        subtitle,
        rings,
        input: input.to_path_buf(),
    })
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
    let (settings, merged) = cli.settings(loaded.as_ref());
    let mut gate = Gate::default();
    gate.errors.extend(loaded.iter().flat_map(key_errors));
    gate.errors.extend(merged.into_iter().map(Fatal::from));
    Ok((settings, gate))
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
    palette_broken: bool,
    theme: Effective<Theme>,
) -> Result<Option<Colours>, Vec<Fatal>> {
    Ok(match (path, palette_broken) {
        (Some((path, named)), _) => gate
            .check(
                &named.origin.source,
                regular(path, named)
                    .map_err(|e| vec![e])
                    .and_then(|()| file::read(path).map_err(|e| palette_errors(named, e))),
            )?
            .map(Colours::from),
        (None, true) => None,
        (None, false) => Some(Colours::BuiltIn(
            theme.or(Setting::Theme, DEFAULT_THEME).value,
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
        || {
            (
                Ok(default_rings()),
                Origin::new(Setting::Rings, Source::Default),
            )
        },
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
                if matches!(source, Source::Config(_)) {
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
        ReadError::Invalid(errors) => errors
            .into_iter()
            .map(|error| Fatal::Palette {
                palette: palette.clone(),
                error,
            })
            .collect(),
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
        let format = format(path).ok_or_else(|| Fatal::UnrecognisedExtension {
            output: named.clone(),
        })?;
        missing_directory(path, named)?;
        regular(path, named)?;
        reads
            .iter()
            .try_for_each(|&(read, which)| output_is_input(path, named, read, which))?;
        Ok(format)
    };
    checked().map_err(|e| vec![e])
}

/// The maintainer's ruling B (security review F4): a path from a
/// `--config` file that exists must be a regular file, so a pipe, a
/// terminal or a folder is refused before anything opens it. A path that
/// does not exist is left to the read or the write; a flag's path is the
/// author's own, so it is not checked.
fn regular(path: &Path, named: &NamedPath) -> Result<(), Fatal> {
    match (&named.origin.source, std::fs::metadata(path)) {
        (Source::Config(_), Ok(meta)) if !meta.is_file() => Err(Fatal::NotRegular {
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

/// D-74: the output must not be a file the run reads: the input CSV, the
/// `--config` file or the palette (security review F2). Files are compared
/// by identity
/// (device and inode, or volume and file index on Windows), so `./`, `..`,
/// symlinks, case and hard links are all caught, and no path spelling needs
/// resolving. A path that cannot be opened is not the input.
fn output_is_input(
    output: &Path,
    named: &NamedPath,
    input: &Path,
    which: ReadFile,
) -> Result<(), Fatal> {
    if same_file::is_same_file(output, input).unwrap_or(false) {
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
    path: &Path,
    rings: &RingSet,
    off_radar: OffRadar,
    warnings: &mut Warnings,
) -> Result<Catalogue, Failure> {
    let text = decode::read(path)?;
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
    let from_flag = |origin: &Origin| origin.source == Source::Flag;
    if !config.format.is_snapshot() {
        return;
    }
    if config.legend.value == Legend::Hidden && from_flag(&config.legend.origin) {
        warnings.push(Warning::Render(RenderKind::LegendStatic));
    }
    if config.table.value == Table::All && from_flag(&config.table.origin) {
        warnings.push(Warning::Render(RenderKind::TableStatic));
    }
}

/// The order, the render warnings (FR-88, FR-89, FR-95, FR-104, FR-105, D-87), the blocks
/// built once, and the output: the page around both arrangements (FR-68),
/// or the wide arrangement as a PNG or an SVG (FR-71).
fn render(
    catalogue: &Catalogue,
    config: &Config,
    fonts: &Fonts,
    warnings: &mut Warnings,
) -> Result<Vec<u8>, Fatal> {
    let plotted = resolve(catalogue);
    let blocks = Blocks::new(
        catalogue,
        &plotted,
        config.title(),
        config.subtitle(),
        fonts,
    );
    // FR-89 at the growth the radar is drawn with (D-86): only what still
    // overlaps is reported.
    warn_crowding(catalogue, &plotted, blocks.radar.growth(), warnings);
    warn_static_flags(config, warnings);
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
    let blocks = if config.format.is_snapshot() {
        blocks.with_legend_columns(usize::from(config.legend_columns))
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
            let width_source = config
                .width
                .as_ref()
                .map_or(Source::Default, |w| w.origin.source.clone());
            let size = png::pixel_size(&wide, config.width.as_ref().map(|w| w.value)).map_err(
                |png::TooLarge { width, height }| Fatal::ImageTooLarge {
                    width,
                    height,
                    width_source: width_source.clone(),
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
                    width_source,
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
