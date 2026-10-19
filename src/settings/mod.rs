//! Where each setting's value came from (NFR-10, D-61, D-64), and the
//! shape of one setting after the `--config` merge (FR-91, D-57).
//!
//! The merge itself is `Cli::settings`, beside the flags it reads
//! (architecture §3); `file` reads the TOML. Nothing here imports `cli` or
//! `diagnostics`, so the taxonomy can name a source without a cycle.

pub mod file;

use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::options::{Legend, Table};
use crate::palette::Theme;
use file::{FileValues, Loaded};

/// FR-56's default.
pub const DEFAULT_THEME: Theme = Theme::Light;
/// FR-103's default.
pub const DEFAULT_LEGEND: Legend = Legend::Shown;
/// FR-106's default.
pub const DEFAULT_TABLE: Table = Table::Radar;
/// FR-100's default: the single column FR-71 exports.
pub const DEFAULT_LEGEND_COLUMNS: u8 = 1;

/// Where a setting's value came from (NFR-10, D-61).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    /// The command line.
    Flag,
    /// A key in the `--config` file, named as given on the command line
    /// (FR-17, FR-91).
    Config(Arc<str>),
    /// The built-in default; for the rings, FR-13's set.
    Default,
}

/// The settings FR-91 lists, each a flag and a key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Setting {
    /// `-o`, `output`.
    Output,
    /// `--title`, `title`.
    Title,
    /// `--subtitle`, `subtitle`.
    Subtitle,
    /// `--rings`, `rings`.
    Rings,
    /// `--unplotted-rings`, `unplotted_rings`.
    UnplottedRings,
    /// `--theme`, `theme`.
    Theme,
    /// `--width`, `width`.
    Width,
    /// `--legend-columns`, `legend_columns`.
    LegendColumns,
    /// `--legend`, `legend`.
    Legend,
    /// `--table`, `table`.
    Table,
    /// `--palette`, `palette`.
    Palette,
}

impl Setting {
    /// Every setting, in FR-91's order: the order a file's problems are
    /// reported in.
    pub const ALL: [Self; 11] = [
        Self::Output,
        Self::Title,
        Self::Subtitle,
        Self::Rings,
        Self::UnplottedRings,
        Self::Theme,
        Self::Width,
        Self::LegendColumns,
        Self::Legend,
        Self::Table,
        Self::Palette,
    ];

    /// The flag as a message names it.
    #[must_use]
    pub const fn flag(self) -> &'static str {
        self.names().0
    }

    /// The key in a `--config` file (FR-91).
    #[must_use]
    pub const fn key(self) -> &'static str {
        self.names().1
    }

    /// The flag and the key, together, so the two cannot drift apart.
    const fn names(self) -> (&'static str, &'static str) {
        match self {
            Self::Output => ("-o", "output"),
            Self::Title => ("--title", "title"),
            Self::Subtitle => ("--subtitle", "subtitle"),
            Self::Rings => ("--rings", "rings"),
            Self::UnplottedRings => ("--unplotted-rings", "unplotted_rings"),
            Self::Theme => ("--theme", "theme"),
            Self::Width => ("--width", "width"),
            Self::LegendColumns => ("--legend-columns", "legend_columns"),
            Self::Legend => ("--legend", "legend"),
            Self::Table => ("--table", "table"),
            Self::Palette => ("--palette", "palette"),
        }
    }
}

/// A setting and where its value came from: its effective source
/// (NFR-10).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Origin {
    /// Which setting.
    pub setting: Setting,
    /// Where its value came from.
    pub source: Source,
}

impl Origin {
    /// `setting` from `source`.
    #[must_use]
    pub const fn new(setting: Setting, source: Source) -> Self {
        Self { setting, source }
    }

    /// `setting` from the command line.
    #[must_use]
    pub const fn flag(setting: Setting) -> Self {
        Self::new(setting, Source::Flag)
    }
}

/// How a message names it: `--title`, or `the title key in radar.toml`
/// (NFR-10). The default names the flag, which is what to pass to change
/// it; FR-17 and FR-107 word the default ring set themselves.
impl fmt::Display for Origin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.source {
            Source::Config(file) => write!(f, "the {} key in {file}", self.setting.key()),
            Source::Flag | Source::Default => f.write_str(self.setting.flag()),
        }
    }
}

/// A value and where it came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sourced<T> {
    /// The value, as given.
    pub value: T,
    /// Its setting and source.
    pub origin: Origin,
}

/// A path setting as a message names it, with the path as the author wrote
/// it rather than as resolved (plan RG-018A, R1): `-o radar.html`, or
/// `the output key in radar.toml (editions/radar.html)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamedPath {
    /// Its setting and source.
    pub origin: Origin,
    /// The path as written.
    pub shown: String,
}

impl NamedPath {
    /// `path` given to `setting`'s flag.
    #[must_use]
    pub fn flag(setting: Setting, path: &Path) -> Self {
        Self {
            origin: Origin::flag(setting),
            shown: path.display().to_string(),
        }
    }
}

impl NamedPath {
    /// What a message adds when a path from the file starts with `~`, which
    /// no shell expanded (FR-91, UX review 5); empty otherwise.
    #[must_use]
    pub fn tilde_note(&self) -> &'static str {
        let tilde = self.shown == "~" || self.shown.starts_with("~/");
        match self.origin.source {
            Source::Config(_) if tilde => {
                "; ~ is not expanded in a config file, so write the full path"
            }
            _ => "",
        }
    }
}

impl fmt::Display for NamedPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.origin.source {
            Source::Config(_) => write!(f, "{} ({})", self.origin, self.shown),
            Source::Flag | Source::Default => write!(f, "{} {}", self.origin, self.shown),
        }
    }
}

/// One setting after the merge (FR-91).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Effective<T> {
    /// From the command line, or from the file where the command line
    /// left it out.
    Given(Sourced<T>),
    /// From neither: the default applies.
    Absent,
    /// The file's key has a problem and no flag overrides it. The problem
    /// fails the run, and every check that would read the value is
    /// skipped, so none describes a value nobody wrote (plan RG-019A,
    /// Ruling 3).
    Broken,
}

impl<T> Effective<T> {
    /// The given value, if there is one.
    #[must_use]
    pub fn given(self) -> Option<Sourced<T>> {
        match self {
            Self::Given(value) => Some(value),
            Self::Absent | Self::Broken => None,
        }
    }

    /// As [`Effective::given`], borrowed.
    #[must_use]
    pub const fn as_given(&self) -> Option<&Sourced<T>> {
        match self {
            Self::Given(value) => Some(value),
            Self::Absent | Self::Broken => None,
        }
    }

    /// The given value through `f`; `Absent` and `Broken` stay as they
    /// are.
    #[must_use]
    pub fn map<U>(self, f: impl FnOnce(Sourced<T>) -> Sourced<U>) -> Effective<U> {
        match self {
            Self::Given(value) => Effective::Given(f(value)),
            Self::Absent => Effective::Absent,
            Self::Broken => Effective::Broken,
        }
    }

    /// The given value, or `default` from [`Source::Default`]. `Broken`
    /// gives the default too: a broken setting has its key's problem
    /// recorded, so the run fails before this value is used.
    #[must_use]
    pub fn or(self, setting: Setting, default: T) -> Sourced<T> {
        self.given().unwrap_or_else(|| Sourced {
            value: default,
            origin: Origin::new(setting, Source::Default),
        })
    }
}

/// Every setting after the merge, paths not yet resolved, so FR-92's
/// `{edition}` (PR 19b) can expand first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settings {
    /// `-o` or `output`.
    pub output: Effective<PathBuf>,
    /// `--title` or `title`.
    pub title: Effective<String>,
    /// `--subtitle` or `subtitle`.
    pub subtitle: Effective<String>,
    /// `--rings` or `rings`, as a comma list.
    pub rings: Effective<String>,
    /// `--unplotted-rings` or `unplotted_rings`, as a comma list.
    pub unplotted_rings: Effective<String>,
    /// `--theme` or `theme`.
    pub theme: Effective<Theme>,
    /// `--width` or `width`.
    pub width: Effective<u32>,
    /// `--legend-columns` or `legend_columns`.
    pub legend_columns: Effective<u8>,
    /// `--legend` or `legend`.
    pub legend: Effective<Legend>,
    /// `--table` or `table`.
    pub table: Effective<Table>,
    /// `--palette` or `palette`.
    pub palette: Effective<PathBuf>,
    /// The file's directory, which a relative path from it resolves
    /// against (FR-91); `None` without `--config`.
    pub dir: Option<PathBuf>,
}

/// What the merge finds wrong beyond each key's own problems: usage
/// errors, as clap's would be (FR-59, FR-91).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MergeProblem {
    /// FR-59 within the file: a `theme` key and a `palette` key. Across
    /// sources the flag wins instead ([`Settings::displace`]), and clap
    /// refuses the pair on the command line.
    Conflict {
        /// The file, as named on the command line.
        file: Arc<str>,
    },
    /// FR-91: neither `-o` nor an `output` key.
    NoOutput {
        /// The file, as named on the command line.
        file: Arc<str>,
    },
}

/// One setting from its flag, else its key in `file`, else absent. A key
/// with a problem is `Broken`, unless the flag overrides it (FR-91).
pub fn pick<T>(
    setting: Setting,
    flag: Option<T>,
    file: Option<&Loaded>,
    from_file: impl FnOnce(&FileValues) -> Option<T>,
) -> Effective<T> {
    if let Some(value) = flag {
        return Effective::Given(Sourced {
            value,
            origin: Origin::flag(setting),
        });
    }
    let Some(loaded) = file else {
        return Effective::Absent;
    };
    if loaded.is_broken(setting) {
        return Effective::Broken;
    }
    from_file(&loaded.values).map_or(Effective::Absent, |value| {
        Effective::Given(Sourced {
            value,
            origin: Origin::new(setting, Source::Config(Arc::clone(&loaded.file))),
        })
    })
}

/// FR-91: a path from the file resolves against `dir`, the file's own
/// directory, unless it is absolute.
///
/// One from the command line is used as given, against the working
/// directory. `~` is never expanded: no shell
/// reads the file. Returns the path to open, and the path as a message
/// names it, as written.
#[must_use]
pub fn resolve(dir: Option<&Path>, path: &Sourced<PathBuf>) -> (PathBuf, NamedPath) {
    let opened = match (&path.origin.source, dir) {
        (Source::Config(_), Some(dir)) if path.value.is_relative() => dir.join(&path.value),
        _ => path.value.clone(),
    };
    let named = NamedPath {
        origin: path.origin.clone(),
        shown: path.value.display().to_string(),
    };
    (opened, named)
}

/// Whether `value` came from the command line.
fn by_flag<T>(value: &Effective<T>) -> bool {
    value
        .as_given()
        .is_some_and(|given| given.origin.source == Source::Flag)
}

impl Settings {
    /// FR-59 across sources (rg-ahj; the maintainer's ruling, 2026-10-18):
    /// the command line wins over the opposing key, so `--palette` drops
    /// the file's `theme` and `--theme` drops its `palette`. The dropped key
    /// is still checked (FR-91), but nothing reads its value.
    pub fn displace(&mut self) {
        if by_flag(&self.palette) && !by_flag(&self.theme) {
            self.theme = Effective::Absent;
        }
        if by_flag(&self.theme) && !by_flag(&self.palette) {
            self.palette = Effective::Absent;
        }
    }

    /// FR-59 within the file, then a missing output (FR-91). Call after
    /// [`Settings::displace`], so a theme and palette both still given
    /// both came from the file.
    #[must_use]
    pub fn problems(&self, file: Option<&Loaded>) -> Vec<MergeProblem> {
        let mut problems = Vec::new();
        if let (Effective::Given(_), Effective::Given(_), Some(loaded)) =
            (&self.theme, &self.palette, file)
        {
            problems.push(MergeProblem::Conflict {
                file: Arc::clone(&loaded.file),
            });
        }
        if let (Effective::Absent, Some(loaded)) = (&self.output, file) {
            problems.push(MergeProblem::NoOutput {
                file: Arc::clone(&loaded.file),
            });
        }
        problems
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_setting_has_fr_91_s_key_in_its_order() {
        let keys: Vec<&str> = Setting::ALL.iter().map(|s| s.key()).collect();
        assert_eq!(
            keys,
            [
                "output",
                "title",
                "subtitle",
                "rings",
                "unplotted_rings",
                "theme",
                "width",
                "legend_columns",
                "legend",
                "table",
                "palette"
            ]
        );
        let flags: Vec<&str> = Setting::ALL.iter().map(|s| s.flag()).collect();
        assert_eq!(
            flags,
            [
                "-o",
                "--title",
                "--subtitle",
                "--rings",
                "--unplotted-rings",
                "--theme",
                "--width",
                "--legend-columns",
                "--legend",
                "--table",
                "--palette"
            ]
        );
    }

    #[test]
    fn an_origin_names_the_flag_or_the_key_and_its_file() {
        let file = Source::Config(Arc::from("conf/radar.toml"));
        assert_eq!(Origin::flag(Setting::Title).to_string(), "--title");
        assert_eq!(
            Origin::new(Setting::Rings, Source::Default).to_string(),
            "--rings"
        );
        assert_eq!(
            Origin::new(Setting::UnplottedRings, file).to_string(),
            "the unplotted_rings key in conf/radar.toml"
        );
    }

    #[test]
    fn a_named_path_shows_the_path_as_written() {
        assert_eq!(
            NamedPath::flag(Setting::Output, Path::new("out/radar.html")).to_string(),
            "-o out/radar.html"
        );
        let named = NamedPath {
            origin: Origin::new(Setting::Palette, Source::Config(Arc::from("radar.toml"))),
            shown: "brand.css".into(),
        };
        assert_eq!(
            named.to_string(),
            "the palette key in radar.toml (brand.css)"
        );
    }

    #[test]
    fn or_keeps_a_given_value_and_supplies_the_default() {
        let given = Effective::Given(Sourced {
            value: 3_u8,
            origin: Origin::flag(Setting::LegendColumns),
        });
        assert_eq!(given.or(Setting::LegendColumns, 1).value, 3);
        let absent: Effective<u8> = Effective::Absent;
        let default = absent.or(Setting::LegendColumns, 1);
        assert_eq!((default.value, default.origin.source), (1, Source::Default));
        assert_eq!(Effective::<u8>::Broken.given(), None);
    }
}
