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

use crate::cli::{Cli, clean_flag};
use crate::csv::decode;
use crate::csv::parse::parse;
use crate::diagnostics::{Fatal, Flag, Warnings};
use crate::emit::html::page;
use crate::emit::write::parent_dir;
use crate::emit::write_atomic;
use crate::errors::render_errors;
use crate::layout::arrange::{Arrangement, Blocks, arrange, warn_narrow};
use crate::layout::geometry::warn_crowding;
use crate::layout::title::shown;
use crate::model::{Catalogue, build_catalogue};
use crate::order::resolve;
use crate::rings::{RingSet, parse_rings};
use crate::text::Fonts;

/// The resolved run (architecture §7), read-only once built.
#[derive(Debug)]
pub struct Config {
    /// `-o`, as given.
    pub output: PathBuf,
    /// FR-65, resolved: cleaned (FR-90), trimmed, and `None` when empty.
    pub title: Option<String>,
    /// As `title`.
    pub subtitle: Option<String>,
    /// The declared rings and their source (FR-12, FR-17).
    pub rings: RingSet,
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
    let outcome = steps(cli, &mut warnings);
    Report {
        warnings: warnings.render(),
        outcome,
    }
}

/// The stages in order, each stopping the run on failure.
fn steps(cli: &Cli, warnings: &mut Warnings) -> Result<(), Vec<String>> {
    let config = config(cli, warnings).map_err(|errors| alone(&errors))?;
    let fonts = Fonts::embedded().map_err(|e| alone(&[Fatal::from(e)]))?;
    let catalogue = catalogue(&cli.csv_path, &config.rings, warnings)?;
    let html = render(&catalogue, &config, &fonts, warnings);
    write_atomic(&config.output, html.as_bytes()).map_err(|e| alone(&[e]))
}

/// Tier-one errors, each its own line.
fn alone(errors: &[Fatal]) -> Vec<String> {
    errors.iter().map(Fatal::render).collect()
}

/// NFR-16 tier one, in order: FR-52's extension, FR-62's directory, D-74's
/// same-file check, then the flag values, cleaned (FR-90), and the rings
/// (FR-63, every violation together).
fn config(cli: &Cli, warnings: &mut Warnings) -> Result<Config, Vec<Fatal>> {
    cli.format()
        .and_then(|_| missing_directory(&cli.output))
        .and_then(|()| output_is_input(&cli.output, &cli.csv_path))
        .map_err(|e| vec![e])?;
    let title = resolved(Flag::Title, cli.title.as_deref(), warnings);
    let subtitle = resolved(Flag::Subtitle, cli.subtitle.as_deref(), warnings);
    let declared = cli
        .rings
        .as_deref()
        .map(|rings| clean_flag(Flag::Rings, rings, warnings));
    let rings = parse_rings(declared.as_deref())
        .map_err(|errors| errors.into_iter().map(Fatal::from).collect::<Vec<_>>())?;
    Ok(Config {
        output: cli.output.clone(),
        title,
        subtitle,
        rings,
    })
}

/// FR-62: `-o`'s directory must exist. Only a directory that is not there
/// is this error; one that is a file or cannot be searched is left to the
/// write, which stays the final authority, so a directory removed after
/// this check still fails cleanly there.
fn missing_directory(output: &Path) -> Result<(), Fatal> {
    let dir = parent_dir(output);
    match std::fs::metadata(dir) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => Err(Fatal::MissingDirectory {
            path: output.display().to_string(),
            dir: dir.display().to_string(),
        }),
        _ => Ok(()),
    }
}

/// D-74: `-o` must not be the input CSV. Files are compared by identity
/// (device and inode, or volume and file index on Windows), so `./`, `..`,
/// symlinks, case and hard links are all caught, and no path spelling needs
/// resolving. A path that cannot be opened is not the input.
fn output_is_input(output: &Path, input: &Path) -> Result<(), Fatal> {
    if same_file::is_same_file(output, input).unwrap_or(false) {
        Err(Fatal::OutputIsInput {
            path: output.display().to_string(),
        })
    } else {
        Ok(())
    }
}

/// FR-65's resolved value: cleaned (FR-90), trimmed, `None` when empty.
fn resolved(flag: Flag, value: Option<&str>, warnings: &mut Warnings) -> Option<String> {
    let cleaned = clean_flag(flag, value?, warnings);
    shown(Some(&cleaned)).map(str::to_owned)
}

/// Read, parse and build. A file-level failure is one line (tier one);
/// row-level errors from both stages are merged and grouped (tier two),
/// and `parse`'s alone fail the run, since it drops the rows it rejects.
fn catalogue(
    path: &Path,
    rings: &RingSet,
    warnings: &mut Warnings,
) -> Result<Catalogue, Vec<String>> {
    let text = decode::read(path).map_err(|e| alone(&[e]))?;
    let (table, mut errors) = parse(&text, warnings).map_err(|e| alone(&[e]))?;
    let built = build_catalogue(&table, rings, warnings).map_err(|more| errors.extend(more));
    match built {
        Ok(catalogue) if errors.is_empty() => Ok(catalogue),
        _ => Err(render_errors(&errors, table.records.len())),
    }
}

/// The order, the render warnings (FR-88, FR-89, FR-95), the blocks built
/// once, and the page around both arrangements of them (FR-68).
fn render(
    catalogue: &Catalogue,
    config: &Config,
    fonts: &Fonts,
    warnings: &mut Warnings,
) -> String {
    let plotted = resolve(catalogue);
    warn_crowding(catalogue, &plotted, warnings);
    let blocks = Blocks::new(
        catalogue,
        &plotted,
        config.title.as_deref(),
        config.subtitle.as_deref(),
        fonts,
    );
    warn_narrow(&blocks, warnings);
    page(
        &arrange(Arrangement::Wide, &blocks),
        &arrange(Arrangement::Narrow, &blocks),
        config.title.as_deref(),
    )
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
