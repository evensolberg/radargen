//! The run, end to end (architecture §2, §7): the flags resolved once into a
//! [`Config`], then the CSV, the catalogue, the order, the layout and the
//! page, then the write.
//!
//! Failures follow NFR-16's tiers. Flag and file errors stop the run at once;
//! row-level errors from both CSV stages are merged and grouped; a failed
//! write is the third failure point (§7). Warnings collected up to any of
//! them still print, before the errors (NFR-11), and nothing is written
//! unless every stage succeeded (NFR-8).
#![deny(clippy::print_stdout)]

use std::io;
use std::path::{Path, PathBuf};

use crate::cli::{Cli, OutputFormat, Task, clean_setting, format};
use crate::csv::decode;
use crate::csv::parse::parse;
use crate::decode::NotUtf8;
use crate::diagnostics::{Fatal, OffRadar, RenderKind, RowError, Warning, Warnings};
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
use crate::palette::Colours;
use crate::palette::file::{self, ReadError};
use crate::rings::{RingSet, parse_rings};
use crate::settings::{NamedPath, Origin, Setting, Source, Sourced};
use crate::text::Fonts;

/// The resolved run (architecture §7), read-only once built. The colours
/// are resolved here, so a bad `--palette` file is a tier-one error
/// (FR-102).
#[derive(Debug)]
pub struct Config {
    /// `-o`, as given.
    pub output: PathBuf,
    /// How a message names the output: its flag or key, and the path as
    /// written.
    pub output_named: NamedPath,
    /// FR-52: what `-o`'s extension asks for.
    pub format: OutputFormat,
    /// FR-56, FR-59: the built-in themes, or a `--palette` file's one set.
    pub colours: Colours,
    /// RG-102: where an HTML page's legend starts; PNG and SVG warn on
    /// `hidden` and draw it anyway.
    pub legend: Legend,
    /// FR-106: what an HTML page's table publishes; PNG and SVG warn on
    /// `all`, having no table.
    pub table: Table,
    /// FR-57: `--width`, which HTML ignores, with where it came from, so
    /// D-87's remedies can name it.
    pub width: Option<Sourced<u32>>,
    /// FR-100: `--legend-columns`, which HTML ignores.
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
    fn render(self) -> Vec<String> {
        match self {
            Self::Alone(errors) => errors.iter().map(Fatal::render).collect(),
            Self::Rows { errors, rows } => render_errors(&errors, rows),
        }
    }
}

impl From<Fatal> for Failure {
    fn from(error: Fatal) -> Self {
        Self::Alone(vec![error])
    }
}

/// What a run printed (NFR-11).
#[derive(Debug)]
pub struct Report {
    /// Rendered warning lines, in print order.
    pub warnings: Vec<String>,
    /// `Ok` when `-o` was written; otherwise the rendered error lines.
    pub outcome: Result<(), Vec<String>>,
}

impl Report {
    /// Every line to print, in order: the warnings, then the errors
    /// (NFR-11).
    pub fn lines(&self) -> impl Iterator<Item = &str> {
        let errors = self.outcome.as_ref().err().into_iter().flatten();
        self.warnings.iter().chain(errors).map(String::as_str)
    }

    /// The exit status NFR-8 gives the run: 0 when `-o` was written, 1
    /// otherwise.
    #[must_use]
    pub const fn status(&self) -> u8 {
        if self.outcome.is_ok() { 0 } else { 1 }
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
    let (input, output) = match cli.task()? {
        Task::Create(starter, path) => return Ok(starter.create(path)?),
        Task::Render { input, output } => (input, output),
    };
    let config = config(cli, input, output, warnings).map_err(Failure::Alone)?;
    let fonts = Fonts::embedded().map_err(Fatal::from)?;
    // FR-106: the flags say where an off-radar row goes, so its warning
    // can say so when it is raised.
    let off_radar = OffRadar::of(config.format, config.table);
    let catalogue = catalogue(&config.input, &config.rings, off_radar, warnings)?;
    let bytes = render(&catalogue, &config, &fonts, warnings)?;
    Ok(write_atomic(&config.output, &config.output_named, &bytes)?)
}

/// NFR-16 tier one, in order: FR-52's extension, FR-62's directory, D-74's
/// same-file check, then the flag values, cleaned (FR-90), the rings
/// (FR-63, every violation together) and the unplotted rings (FR-107).
fn config(
    cli: &Cli,
    input: &Path,
    output: &Path,
    warnings: &mut Warnings,
) -> Result<Config, Vec<Fatal>> {
    let output_named = NamedPath::flag(Setting::Output, output);
    let format = output_checks(output, &output_named, input)?;
    let title = cli
        .title
        .clone()
        .map(|value| Sourced {
            value,
            origin: Origin::flag(Setting::Title),
        })
        .and_then(|t| resolved(t, warnings));
    let subtitle = cli
        .subtitle
        .clone()
        .map(|value| Sourced {
            value,
            origin: Origin::flag(Setting::Subtitle),
        })
        .and_then(|t| resolved(t, warnings));
    let declared = cli
        .rings
        .as_deref()
        .map(|rings| clean_setting(&Origin::flag(Setting::Rings), rings, warnings));
    let unplotted = cli
        .unplotted_rings
        .as_deref()
        .map(|rings| clean_setting(&Origin::flag(Setting::UnplottedRings), rings, warnings));
    let mut rings = parse_rings(declared.as_deref()).map_err(|errors| {
        errors
            .into_iter()
            .map(|error| Fatal::Rings {
                origin: Origin::flag(Setting::Rings),
                error,
            })
            .collect::<Vec<_>>()
    })?;
    // FR-107: checked against the declared set, so only once that parsed.
    if let Some(list) = &unplotted {
        rings = rings.with_unplotted(list).map_err(|errors| {
            errors
                .into_iter()
                .map(|error| Fatal::UnplottedRings {
                    origin: Origin::flag(Setting::UnplottedRings),
                    error,
                })
                .collect::<Vec<_>>()
        })?;
    }
    // Plan RG-018A Ruling 7: the next flag after --rings, every problem in
    // the file at once (FR-102), each its own line.
    let colours = match &cli.palette {
        Some(path) => file::read(path)
            .map_err(|e| palette_errors(&NamedPath::flag(Setting::Palette, path), e))?
            .into(),
        None => Colours::BuiltIn(cli.theme),
    };
    Ok(Config {
        output: output.to_path_buf(),
        output_named,
        format,
        colours,
        legend: cli.legend,
        table: cli.table,
        width: cli.width.map(|value| Sourced {
            value,
            origin: Origin::flag(Setting::Width),
        }),
        legend_columns: cli.legend_columns,
        title,
        subtitle,
        rings,
        input: input.to_path_buf(),
    })
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

/// FR-52's extension, FR-62's directory and D-74's same-file check on the
/// output, the first to fail alone (NFR-16 tier one).
fn output_checks(path: &Path, named: &NamedPath, input: &Path) -> Result<OutputFormat, Vec<Fatal>> {
    let checked = || -> Result<OutputFormat, Fatal> {
        let format = format(path).ok_or_else(|| Fatal::UnrecognisedExtension {
            output: named.clone(),
        })?;
        missing_directory(path, named)?;
        output_is_input(path, named, input)?;
        Ok(format)
    };
    checked().map_err(|e| vec![e])
}

/// FR-62: `-o`'s directory must exist. Only a directory that is not there
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

/// D-74: `-o` must not be the input CSV. Files are compared by identity
/// (device and inode, or volume and file index on Windows), so `./`, `..`,
/// symlinks, case and hard links are all caught, and no path spelling needs
/// resolving. A path that cannot be opened is not the input.
fn output_is_input(output: &Path, named: &NamedPath, input: &Path) -> Result<(), Fatal> {
    if same_file::is_same_file(output, input).unwrap_or(false) {
        Err(Fatal::OutputIsInput {
            output: named.clone(),
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
    // RG-102: a static snapshot has no button to bring a hidden legend
    // back, so it keeps the legend and says so.
    if config.legend == Legend::Hidden && config.format.is_snapshot() {
        warnings.push(Warning::Render(RenderKind::LegendStatic));
    }
    // FR-106: nor have they a table to list everything in.
    if config.table == Table::All && config.format.is_snapshot() {
        warnings.push(Warning::Render(RenderKind::TableStatic));
    }
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
                legend: config.legend,
                table: config.table,
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

    fn report(outcome: Result<(), Vec<String>>) -> Report {
        Report {
            warnings: vec!["warning: w".into()],
            outcome,
        }
    }

    #[test]
    fn warnings_print_before_errors() {
        let failed = report(Err(vec!["error: e".into()]));
        assert_eq!(
            failed.lines().collect::<Vec<_>>(),
            ["warning: w", "error: e"]
        );
        assert_eq!(report(Ok(())).lines().collect::<Vec<_>>(), ["warning: w"]);
    }

    #[test]
    fn only_a_written_run_exits_zero() {
        assert_eq!(report(Ok(())).status(), 0);
        assert_eq!(report(Err(Vec::new())).status(), 1);
    }
}
