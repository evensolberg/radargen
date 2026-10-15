//! Command-line surface (§6.6).
//!
//! Argument *behaviour* is fixed by the PRD; exact spelling is OQ-2. There is
//! no subcommand (D-28): one CSV in, one file out, or `--create-template`
//! alone to write a CSV to start from (FR-99).

use clap::Parser;
use std::path::{Path, PathBuf};

use crate::diagnostics::{Fatal, Flag, RenderKind, Warning, Warnings};
use crate::escape::remove_undisplayable;
pub use crate::palette::Theme;

/// The output format, inferred from the `-o` extension (FR-52).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    /// `.html`: the shared document the radar is embedded in (FR-52).
    Html,
    /// `.png`: the raster export, arriving in v1.1 (§14.1).
    Png,
    /// `.svg`: the bare vector radar, arriving in v1.1 (§14.1).
    Svg,
}

/// The parsed command line: one CSV in, one file out (D-28, §6.6).
//
// clap shows every `///` comment below as `--help` text, so those speak to
// the person running the tool. The reasoning behind each flag lives in the
// `//` comments, which clap ignores; a test keeps requirement numbers out of
// the help.
#[derive(Debug, Parser)]
#[command(
    name = "radargen",
    version,
    about = "Render a technology radar from a CSV file to a self-contained HTML page.",
    // clap's own usage would show the CSV as optional and leave out -o,
    // since each is required only without --create-template.
    override_usage = "radargen <CSV_PATH> -o <OUTPUT> [OPTIONS]\n       radargen --create-template <PATH>",
    long_about = None
)]
pub struct Cli {
    // §6.1; exactly one file, with no glob or multi-file form (FR-60),
    // and none at all with --create-template (FR-99).
    /// The CSV file to read.
    ///
    /// A header row, then one row per item, with at least name, quadrant,
    /// ring and description columns. --create-template writes one to start
    /// from.
    #[arg(required_unless_present = "create_template")]
    pub csv_path: Option<PathBuf>,

    // FR-52: the format comes from the extension, matched ignoring case.
    /// Where to write the radar.
    ///
    /// The extension picks the format: .html, in any case (PNG and SVG arrive
    /// in version 1.1).
    #[arg(short, long, required_unless_present = "create_template")]
    pub output: Option<PathBuf>,

    // FR-12 to FR-15: the list is edge-to-centre and replaces the default
    // set (FR-13, `rings::DEFAULT_RINGS`); a test checks the help names it.
    /// The rings, outermost first, separated by commas.
    ///
    /// For example "Hold,Assess,Trial,Adopt". The list replaces the default
    /// set, which is Assess, Trial, Adopt, Retire. A row's ring matches in any
    /// case; a row whose ring is not in the list is left off the radar and
    /// listed in the table, with a warning.
    #[arg(short, long)]
    pub rings: Option<String>,

    // FR-65: omitted renders nothing at all, with no placeholder or space.
    /// A title shown above the radar.
    #[arg(short, long)]
    pub title: Option<String>,

    // FR-65, D-24: the tool attaches no meaning to it and never supplies one.
    /// A second line under the title, such as the edition's date.
    #[arg(short, long)]
    pub subtitle: Option<String>,

    // FR-56: resolves a static snapshot's colours at generation time; HTML
    // carries both palettes and ignores it. `ignore_case` gives `--theme Dark`
    // the courtesy FR-52 gives `.PNG`. `-T`, not `-t`, which is `--title`:
    // both appear in §6.6's canonical PNG invocation, so both earn a short
    // form (OQ-2).
    /// The theme for PNG and SVG (version 1.1).
    ///
    /// Written in any case. An HTML page carries both themes and follows the
    /// reader's setting, with a button to switch.
    #[arg(short = 'T', long, value_enum, default_value_t = Theme::Light, ignore_case = true)]
    pub theme: Theme,

    // FR-57: the height always follows the document's aspect ratio and is
    // never a second flag. The range is FR-64's, enforced by clap so the
    // message is standard usage output. HTML ignores it.
    /// The width in pixels for PNG and SVG (version 1.1).
    ///
    /// From 320 to 10000; the height follows. An HTML page fits the reader's
    /// window.
    #[arg(short, long, value_parser = clap::value_parser!(u32).range(320..=10_000))]
    pub width: Option<u32>,

    // FR-99, D-83: the tool writes the template, so it always matches the
    // parser that reads it. `exclusive`, so it never half-renders; no short
    // form, since it is run once per radar.
    /// Write a starter CSV to fill in, and render nothing.
    ///
    /// It has every column the tool reads and a few example rows to replace
    /// or delete. An existing file is never replaced. Give this flag alone.
    #[arg(long, value_name = "PATH", exclusive = true)]
    pub create_template: Option<PathBuf>,
}

/// What a command line asks for (FR-60, FR-99).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Task<'a> {
    /// `--create-template <path>`: write the starter CSV there.
    Template(&'a Path),
    /// Render `input` to `output`.
    Render {
        /// The CSV, as given.
        input: &'a Path,
        /// `-o`, as given.
        output: &'a Path,
    },
}

impl Cli {
    /// The task this command line asks for.
    ///
    /// # Errors
    ///
    /// [`Fatal::NoTask`] when it asks for neither or both, which clap
    /// refuses before a `Cli` is built; only one built by hand gets here.
    pub fn task(&self) -> Result<Task<'_>, Fatal> {
        match (&self.create_template, &self.csv_path, &self.output) {
            (Some(path), None, None) => Ok(Task::Template(path)),
            (None, Some(input), Some(output)) => Ok(Task::Render { input, output }),
            _ => Err(Fatal::NoTask),
        }
    }
}

/// FR-52: format from the `-o` extension, matched case-insensitively.
///
/// # Errors
///
/// [`Fatal::FormatNotInThisVersion`] for `.png` and `.svg`, which
/// v1.1 adds; [`Fatal::UnrecognisedExtension`] for any other or
/// missing extension.
pub fn format(output: &Path) -> Result<OutputFormat, Fatal> {
    let ext = output
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    match ext.as_str() {
        "html" => Ok(OutputFormat::Html),
        // §14.1: v1.0 ships HTML only, and these must not read as typos.
        "png" | "svg" => Err(Fatal::FormatNotInThisVersion {
            path: output.display().to_string(),
            ext,
            arrives_in: "1.1",
        }),
        _ => Err(Fatal::UnrecognisedExtension {
            path: output.display().to_string(),
        }),
    }
}

/// FR-90 on a flag value: removes every character the output cannot display
/// (the set [`displayable`] rejects, the one FR-90 removes from CSV fields) and
/// warns once, naming `flag`, when it removed any.
///
/// For `--rings` the caller cleans before [`crate::rings::parse_rings`], so
/// FR-63's empty and duplicate checks see the cleaned list; FR-65's
/// empty-or-whitespace rule likewise applies to the cleaned title.
pub fn clean_flag(flag: Flag, value: &str, warnings: &mut Warnings) -> String {
    let mut cleaned = value.to_string();
    if let Some((first, count)) = remove_undisplayable(&mut cleaned) {
        warnings.push(Warning::Render(RenderKind::FlagUndisplayable {
            flag,
            first,
            count,
        }));
    }
    cleaned
}
