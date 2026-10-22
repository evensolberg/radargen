//! Command-line surface (§6.6).
//!
//! Argument *behaviour* is fixed by the PRD; exact spelling is OQ-2. There is
//! no subcommand (D-28): one CSV in, one file out, or `--create-template`,
//! `--create-palette` or `--create-config` alone to write a file to start
//! from (FR-99, FR-101, FR-109).

use clap::Parser;
use std::path::{Path, PathBuf};

use crate::diagnostics::{Fatal, RenderKind, Warning, Warnings};
use crate::escape::remove_undisplayable;
use crate::options::{Legend, Table};
pub use crate::palette::Theme;
use crate::settings::edition::{self, Edition};
use crate::settings::file::Loaded;
use crate::settings::{Origin, Setting, Settings, pick};
use crate::starter::Starter;
use crate::style::{MAX_EXPORT_WIDTH, MAX_LEGEND_COLUMNS, MIN_EXPORT_WIDTH};

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
    override_usage = "radargen <CSV_PATH> -o <OUTPUT> [OPTIONS]\n       radargen [CSV_PATH] -c <FILE> [OPTIONS]\n       radargen --create-template <PATH>\n       radargen --create-palette <PATH>\n       radargen --create-config <PATH>",
    long_about = None
)]
pub struct Cli {
    // §6.1; exactly one file, with no glob or multi-file form (FR-60),
    // and none at all with a --create-* flag (FR-99, FR-101, FR-109).
    // Optional with --config, whose input key can name it (FR-91); given
    // here, it overrides the key.
    /// The CSV file to read.
    ///
    /// A header row, then one row per item, with at least name, sector,
    /// ring and description columns. --create-template writes one to start
    /// from. With --config, it can come from the file's input key instead.
    #[arg(required_unless_present_any = [STARTER, "config"])]
    pub csv_path: Option<PathBuf>,

    // FR-52: the format comes from the extension, matched ignoring case.
    // Required unless a --create-* flag stands in for the render, or from
    // --config's output key (FR-91).
    /// Where to write the radar.
    ///
    /// The extension picks the format: .html, .png or .svg, in any case.
    #[arg(short, long, required_unless_present_any = [STARTER, "config"])]
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

    // rg-ahj: no flag carries a clap default, so `Some` means given on the
    // command line, which `--config`'s merge reads; the defaults are
    // `settings::DEFAULT_*`, and each help line names its default in prose.

    // FR-56: resolves a static snapshot's colours at generation time; HTML
    // carries both palettes and ignores it. `ignore_case` gives `--theme Dark`
    // the courtesy FR-52 gives `.PNG`. `-T`, not `-t`, which is `--title`:
    // both appear in §6.6's canonical PNG invocation, so both earn a short
    // form (OQ-2).
    /// The theme for PNG and SVG; the default is light.
    ///
    /// Not with --palette, which gives the colours itself. Written in any
    /// case. An HTML page carries both themes and follows the reader's
    /// setting, with a button to switch.
    #[arg(short = 'T', long, value_enum, ignore_case = true)]
    pub theme: Option<Theme>,

    // RG-102: the HTML page's starting state. PNG and SVG have no button to
    // bring a hidden legend back, so they keep it and the pipeline warns.
    // `show` and `hide` read naturally too, so they are accepted without
    // being advertised. No short form: a page's start is set once per
    // edition, and FR-100's --legend-columns may want one.
    /// How an HTML page's legend starts; the default is shown.
    ///
    /// Accepted in any letter case. The page has a button to show or hide
    /// it either way. PNG and SVG always draw the legend.
    #[arg(long, value_enum, ignore_case = true)]
    pub legend: Option<Legend>,

    // FR-106, D-95: the page publishes what the radar draws unless asked
    // for everything, so a CSV that doubles as an internal register is not
    // published whole by default. PNG and SVG have no table, so the pipeline
    // warns on `all` there, as it does on `--legend hidden`. No aliases and
    // no short form: it is set once per radar, like --legend.
    /// What an HTML page's table publishes; the default is radar.
    ///
    /// It also decides what the detail panels show. Accepted in any letter
    /// case. PNG and SVG have no table.
    #[arg(long, value_enum, ignore_case = true)]
    pub table: Option<Table>,

    // FR-57: the height always follows the document's aspect ratio and is
    // never a second flag. The range is FR-64's, enforced by clap so the
    // message is standard usage output. HTML ignores it.
    /// The width in pixels for PNG and SVG.
    ///
    /// From 320 to 10000; the height follows. An HTML page fits the reader's
    /// window.
    #[arg(short, long, value_parser = clap::value_parser!(u32).range(i64::from(MIN_EXPORT_WIDTH)..=i64::from(MAX_EXPORT_WIDTH)))]
    pub width: Option<u32>,

    // FR-100, D-90: opt-in, so every existing edition renders as it did.
    // The range is enforced by clap, as --width's is. HTML ignores it
    // without a word, as it does --width: its legend scrolls beside the
    // radar. No short form until it earns one (OQ-2).
    /// Flow a PNG or SVG legend into this many columns beside the radar; the default is 1.
    ///
    /// From 1 to 4; columns break only between sectors. An HTML page
    /// ignores it.
    #[arg(long, value_name = "N", value_parser = clap::value_parser!(u8).range(1..=i64::from(MAX_LEGEND_COLUMNS)))]
    pub legend_columns: Option<u8>,

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

    // FR-91, D-57: read only when named, never discovered. `-c` (OQ-2,
    // provisional in D-57). The keys mean what their flags mean; a flag
    // here overrides its key, and a relative path in the file is read from
    // the file's folder. `exclusive` on the --create-* flags keeps it from
    // them.
    // The long help is a constant, since its TOML example would read as an
    // intra-doc link in a doc comment.
    /// Read settings from a TOML file.
    #[arg(short, long, value_name = "FILE", long_help = CONFIG_HELP)]
    pub config: Option<PathBuf>,

    // FR-92, D-58: also the `edition` key (FR-91), which this overrides.
    // `-e` (OQ-2).
    // The date expands exactly as written, so dated filenames sort; the
    // tool reads the clock only for `today`, the maintainer's opt-in for
    // scheduled jobs, and echoes the date it read.
    /// The edition's date, which {edition} in the output, title or subtitle becomes.
    ///
    /// Written YYYY-MM-DD, such as 2026-09-01, or today for the local date,
    /// which the run prints so it can be repeated. A value holding
    /// {edition} needs this flag; while it is in use, any other {name} in
    /// that value is an error, since it is likely a typo. Overrides a
    /// config file's edition key.
    #[arg(short, long, value_name = "YYYY-MM-DD", value_parser = edition::parse)]
    pub edition: Option<Edition>,

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

    // FR-109, D-102: as --create-template, for a --config file. Every key
    // is commented out, so the file as written changes nothing.
    /// Write a starter config file for --config, and render nothing.
    ///
    /// It has every key, commented out, with its default and what it does,
    /// so it changes nothing until a line is uncommented. An existing file
    /// is never replaced. Give this flag alone.
    #[arg(long, value_name = "PATH", exclusive = true, group = STARTER)]
    pub create_config: Option<PathBuf>,
}

/// `--config`'s long help (UX review 8).
const CONFIG_HELP: &str = "Read settings from a TOML file.\n\n\
Its keys are input, output, title, subtitle, rings, unplotted_rings, theme, width, \
legend_columns, legend, table, palette and edition, each meaning what its flag \
means; input names the CSV. rings and unplotted_rings are arrays of strings, such as [\"Adopt\", \"Trial\"]; \
width and legend_columns are integers; the rest are strings, edition a date \
written YYYY-MM-DD or \"today\". Any other key is \
an error. A flag on the command line, or the CSV given there, overrides its key. \
A relative path in the \
file is read from the file's own folder; ~ is not expanded. --create-config \
writes one to start from.";

/// The group of flags that each stand in for a render (FR-60).
const STARTER: &str = "starter";

/// What a command line asks for (FR-60, FR-99, FR-101, FR-109).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Task<'a> {
    /// `--create-template`, `--create-palette` or `--create-config <path>`:
    /// write that file there.
    Create(Starter, &'a Path),
    /// Render; the CSV is `input` or the file's `input` key, and the
    /// output is `-o` or the file's `output` key (FR-91).
    Render {
        /// The CSV, as given on the command line, if it was.
        input: Option<&'a Path>,
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
        let renders = self.output.is_some() || self.config.is_some();
        let mut starters = [
            (Starter::Template, &self.create_template),
            (Starter::Palette, &self.create_palette),
            (Starter::Config, &self.create_config),
        ]
        .into_iter()
        .filter_map(|(starter, path)| Some((starter, path.as_deref()?)));
        match (starters.next(), starters.next(), &self.csv_path) {
            (Some((starter, path)), None, None) if !renders => Ok(Task::Create(starter, path)),
            (None, None, Some(input)) if renders => Ok(Task::Render { input: Some(input) }),
            (None, None, None) if self.config.is_some() => Ok(Task::Render { input: None }),
            _ => Err(Fatal::NoTask),
        }
    }

    /// FR-91: every setting from its flag, else from `file`, with where it
    /// came from (NFR-10); a flag displaces the opposing key (FR-59).
    #[must_use]
    pub fn settings(&self, file: Option<&Loaded>) -> Settings {
        let mut settings = Settings {
            input: pick(Setting::Input, self.csv_path.clone(), file, |v| {
                v.input.clone()
            }),
            output: pick(Setting::Output, self.output.clone(), file, |v| {
                v.output.clone()
            }),
            title: pick(Setting::Title, self.title.clone(), file, |v| {
                v.title.clone()
            }),
            subtitle: pick(Setting::Subtitle, self.subtitle.clone(), file, |v| {
                v.subtitle.clone()
            }),
            rings: pick(Setting::Rings, self.rings.clone(), file, |v| {
                v.rings.clone()
            }),
            unplotted_rings: pick(
                Setting::UnplottedRings,
                self.unplotted_rings.clone(),
                file,
                |v| v.unplotted_rings.clone(),
            ),
            theme: pick(Setting::Theme, self.theme, file, |v| v.theme),
            width: pick(Setting::Width, self.width, file, |v| v.width),
            legend_columns: pick(Setting::LegendColumns, self.legend_columns, file, |v| {
                v.legend_columns
            }),
            legend: pick(Setting::Legend, self.legend, file, |v| v.legend),
            table: pick(Setting::Table, self.table, file, |v| v.table),
            palette: pick(Setting::Palette, self.palette.clone(), file, |v| {
                v.palette.clone()
            }),
            edition: pick(Setting::Edition, self.edition.clone(), file, |v| {
                v.edition.clone()
            }),
            dir: file.map(|loaded| loaded.dir.clone()),
        };
        settings.displace();
        settings
    }
}

/// FR-90 on a setting's value, from the command line or the file.
///
/// Removes every character the output cannot display (the set
/// [`displayable`] rejects, the one FR-90 removes from CSV fields) and warns
/// once, naming the value's setting and source (NFR-10), when it removed
/// any.
///
/// For the rings the caller cleans before [`crate::rings::parse_ring_list`],
/// and for `--unplotted-rings` before
/// [`crate::rings::RingSet::with_unplotted`], so FR-63's checks see the
/// cleaned list; FR-65's empty-or-whitespace rule likewise applies to the
/// cleaned title.
pub fn clean_setting(origin: &Origin, value: &str, warnings: &mut Warnings) -> String {
    let mut cleaned = value.to_string();
    if let Some((first, count)) = remove_undisplayable(&mut cleaned) {
        warnings.push(Warning::Render(RenderKind::SettingUndisplayable {
            origin: origin.clone(),
            first,
            count,
        }));
    }
    cleaned
}

#[cfg(test)]
mod tests {
    use super::*;

    /// FR-91: `--config`'s help lists every key, `edition` included, in
    /// FR-91's order, so the help and the key list cannot drift apart.
    #[test]
    fn the_config_help_lists_every_key() {
        let keys: Vec<&str> = Setting::ALL.iter().map(|s| s.key()).collect();
        let (last, rest) = keys.split_last().expect("keys");
        let listed = format!("{} and {last}", rest.join(", "));
        let help = CONFIG_HELP.replace(" \\\n", " ").replace('\n', " ");
        assert!(help.contains(&listed), "{listed} in {help}");
        assert_eq!(*last, "edition", "edition is FR-91's twelfth key");
    }
}
