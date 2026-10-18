//! Command-line surface (§6.6).
//!
//! Argument *behaviour* is fixed by the PRD; exact spelling is OQ-2. There is
//! no subcommand (D-28): one CSV in, one file out, or `--create-template`
//! or `--create-palette` alone to write a file to start from (FR-99,
//! FR-101).

use clap::Parser;
use std::path::{Path, PathBuf};

use crate::diagnostics::{Fatal, Flag, RenderKind, Warning, Warnings};
pub use crate::emit::html::{Legend, Table};
use crate::escape::remove_undisplayable;
pub use crate::palette::Theme;
use crate::starter::Starter;
use crate::style::{MAX_EXPORT_WIDTH, MIN_EXPORT_WIDTH};

/// The output format, inferred from the `-o` extension (FR-52).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    /// `.html`: the shared document the radar is embedded in (FR-52).
    Html,
    /// `.png`: the wide arrangement as a raster image (FR-55, FR-71).
    Png,
    /// `.svg`: the wide arrangement as a standalone SVG, its text outlined
    /// (FR-54, D-88).
    Svg,
}

impl OutputFormat {
    /// Whether this is a static snapshot, a PNG or an SVG: it draws the
    /// legend whatever `--legend` says (RG-102), and with the bundled font
    /// alone (FR-104).
    #[must_use]
    pub const fn is_snapshot(self) -> bool {
        matches!(self, Self::Png | Self::Svg)
    }
}

/// The format as a message names it: `PNG`.
impl std::fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Html => "HTML",
            Self::Png => "PNG",
            Self::Svg => "SVG",
        })
    }
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
    about = "Render a technology radar from a CSV file to a self-contained HTML page, a PNG image or an SVG file.",
    // clap's own usage would show the CSV as optional and leave out -o,
    // since each is required only without a --create-* flag.
    override_usage = "radargen <CSV_PATH> -o <OUTPUT> [OPTIONS]\n       radargen --create-template <PATH>\n       radargen --create-palette <PATH>",
    long_about = None
)]
pub struct Cli {
    // §6.1; exactly one file, with no glob or multi-file form (FR-60),
    // and none at all with --create-template or --create-palette (FR-99,
    // FR-101).
    /// The CSV file to read.
    ///
    /// A header row, then one row per item, with at least name, quadrant,
    /// ring and description columns. --create-template writes one to start
    /// from.
    #[arg(required_unless_present = STARTER)]
    pub csv_path: Option<PathBuf>,

    // FR-52: the format comes from the extension, matched ignoring case.
    /// Where to write the radar.
    ///
    /// The extension picks the format: .html, .png or .svg, in any case.
    #[arg(short, long, required_unless_present = STARTER)]
    pub output: Option<PathBuf>,

    // FR-12 to FR-15: the list is edge-to-centre and replaces the default
    // set (FR-13, `rings::DEFAULT_RINGS`); a test checks the help names it.
    /// The rings, outermost first, separated by commas.
    ///
    /// For example "Hold,Assess,Trial,Adopt". The list replaces the default
    /// set, which is Assess, Trial, Adopt, Retire. A row's ring matches in any
    /// case; a row whose ring is not in the list is left off the radar, with
    /// a warning unless --unplotted-rings names it, and an HTML page's table
    /// lists it only with --table all.
    #[arg(short, long)]
    pub rings: Option<String>,

    // FR-107, D-96: rings kept off the radar on purpose, so FR-17 does not
    // warn about their rows. Naming them, rather than a switch that ignores
    // every unknown ring, keeps the warning for a typo. Parsed as --rings is
    // (FR-15, FR-63). No short form (OQ-2): it is set once per radar.
    /// Rings to leave off the radar without a warning, separated by commas.
    ///
    /// For example "In Use,Rejected". Their rows are treated like any row
    /// whose ring is not drawn, minus the warning; a ring in neither
    /// list still warns, so typos show. Case-insensitive. A ring cannot be
    /// in both lists, nor, without --rings, be a default ring.
    #[arg(long, value_name = "RINGS")]
    pub unplotted_rings: Option<String>,

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
    /// The theme for PNG and SVG.
    ///
    /// Not with --palette, which gives the colours itself. Written in any
    /// case. An HTML page carries both themes and follows the reader's
    /// setting, with a button to switch.
    #[arg(short = 'T', long, value_enum, default_value_t = Theme::Light, ignore_case = true)]
    pub theme: Theme,

    // RG-102: the HTML page's starting state. PNG and SVG have no button to
    // bring a hidden legend back, so they keep it and the pipeline warns.
    // `show` and `hide` read naturally too, so they are accepted without
    // being advertised. No short form: a page's start is set once per
    // edition, and FR-100's --legend-columns may want one.
    /// Where an HTML page's legend starts.
    ///
    /// Accepted in any letter case. The page has a button to show or hide
    /// it either way. PNG and SVG always draw the legend.
    #[arg(long, value_enum, default_value_t = Legend::Shown, ignore_case = true)]
    pub legend: Legend,

    // FR-106, D-95: the page publishes what the radar draws unless asked
    // for everything, so a CSV that doubles as an internal register is not
    // published whole by default. PNG and SVG have no table, so the pipeline
    // warns on `all` there, as it does on `--legend hidden`. No aliases and
    // no short form: it is set once per radar, like --legend.
    /// What an HTML page's table publishes.
    ///
    /// It also decides what the detail panels show. Accepted in any letter
    /// case. PNG and SVG have no table.
    #[arg(long, value_enum, default_value_t = Table::Radar, ignore_case = true)]
    pub table: Table,

    // FR-57: the height always follows the document's aspect ratio and is
    // never a second flag. The range is FR-64's, enforced by clap so the
    // message is standard usage output. HTML ignores it.
    /// The width in pixels for PNG and SVG.
    ///
    /// From 320 to 10000; the height follows. An HTML page fits the reader's
    /// window.
    #[arg(short, long, value_parser = clap::value_parser!(u32).range(i64::from(MIN_EXPORT_WIDTH)..=i64::from(MAX_EXPORT_WIDTH)))]
    pub width: Option<u32>,

    // FR-59, D-44: one set of colours for every format, so --theme has
    // nothing to choose between; clap refuses the pair, so the message is
    // standard usage output. `-p`: a render flag earns a short form (OQ-2,
    // plan RG-018A Ruling 6). FR-102's grammar is checked when the file is
    // read, in `pipeline::config`.
    /// Colours from a CSS file instead of the built-in themes.
    ///
    /// The file is one :root rule declaring --surface-1, --surface-2,
    /// --text-primary, --text-muted, --stroke-ring, --stroke-divider,
    /// --marker-hollow, --focus and --series-1 onward, each a #RRGGBB colour.
    /// The palette guide explains each one:
    /// <https://github.com/evensolberg/radargen/blob/main/docs/palette.md>.
    /// --create-palette writes one to start from. Not with --theme; an HTML
    /// page drawn with it has no theme button.
    #[arg(short, long, value_name = "FILE", conflicts_with = "theme")]
    pub palette: Option<PathBuf>,

    // FR-99, D-83: the tool writes the template, so it always matches the
    // parser that reads it. `exclusive`, so it never half-renders; no short
    // form, since it is run once per radar.
    /// Write a starter CSV to fill in, and render nothing.
    ///
    /// It has every column the tool reads and a few example rows to replace
    /// or delete. An existing file is never replaced. Give this flag alone.
    #[arg(long, value_name = "PATH", exclusive = true, group = STARTER)]
    pub create_template: Option<PathBuf>,

    // FR-101, D-91: as --create-template, for a --palette file. The
    // built-in light colours, since a palette is one set; choosing the set
    // with --theme would make this the one aside not given alone.
    /// Write a palette file to edit for --palette, and render nothing.
    ///
    /// It holds the built-in light colours, with the dark theme's value in a
    /// comment beside each colour the themes do not share. An existing file
    /// is never replaced. Give this flag alone.
    #[arg(long, value_name = "PATH", exclusive = true, group = STARTER)]
    pub create_palette: Option<PathBuf>,
}

/// The group of flags that each stand in for a render (FR-60).
const STARTER: &str = "starter";

/// What a command line asks for (FR-60, FR-99, FR-101).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Task<'a> {
    /// `--create-template <path>` or `--create-palette <path>`: write that
    /// file there.
    Create(Starter, &'a Path),
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
    /// [`Fatal::NoTask`] when it asks for none or more than one, which clap
    /// refuses before a `Cli` is built; only one built by hand gets here.
    pub fn task(&self) -> Result<Task<'_>, Fatal> {
        match (
            &self.create_template,
            &self.create_palette,
            &self.csv_path,
            &self.output,
        ) {
            (Some(path), None, None, None) => Ok(Task::Create(Starter::Template, path)),
            (None, Some(path), None, None) => Ok(Task::Create(Starter::Palette, path)),
            (None, None, Some(input), Some(output)) => Ok(Task::Render { input, output }),
            _ => Err(Fatal::NoTask),
        }
    }
}

/// FR-52: format from the `-o` extension, matched case-insensitively.
///
/// # Errors
///
/// [`Fatal::UnrecognisedExtension`] for any other or missing extension.
pub fn format(output: &Path) -> Result<OutputFormat, Fatal> {
    let ext = output
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    match ext.as_str() {
        "html" => Ok(OutputFormat::Html),
        "png" => Ok(OutputFormat::Png),
        "svg" => Ok(OutputFormat::Svg),
        _ => Err(Fatal::UnrecognisedExtension {
            path: output.display().to_string(),
        }),
    }
}

/// FR-90 on a flag value: removes every character the output cannot display
/// (the set [`displayable`] rejects, the one FR-90 removes from CSV fields) and
/// warns once, naming `flag`, when it removed any.
///
/// For `--rings` the caller cleans before [`crate::rings::parse_rings`], and
/// for `--unplotted-rings` before [`crate::rings::RingSet::with_unplotted`], so
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
