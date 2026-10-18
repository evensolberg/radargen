//! Where each setting's value came from (NFR-10, D-61, D-64), and the
//! shape of one setting after the `--config` merge (FR-91, D-57).
//!
//! The merge itself is `Cli::settings`, beside the flags it reads
//! (architecture §3); `file` reads the TOML. Nothing here imports `cli` or
//! `diagnostics`, so the taxonomy can name a source without a cycle.

pub mod file;

use std::fmt;
use std::path::Path;
use std::sync::Arc;

use crate::emit::html::{Legend, Table};
use crate::palette::Theme;

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
        match self {
            Self::Output => "-o",
            Self::Title => "--title",
            Self::Subtitle => "--subtitle",
            Self::Rings => "--rings",
            Self::UnplottedRings => "--unplotted-rings",
            Self::Theme => "--theme",
            Self::Width => "--width",
            Self::LegendColumns => "--legend-columns",
            Self::Legend => "--legend",
            Self::Table => "--table",
            Self::Palette => "--palette",
        }
    }

    /// The key in a `--config` file (FR-91).
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::Output => "output",
            Self::Title => "title",
            Self::Subtitle => "subtitle",
            Self::Rings => "rings",
            Self::UnplottedRings => "unplotted_rings",
            Self::Theme => "theme",
            Self::Width => "width",
            Self::LegendColumns => "legend_columns",
            Self::Legend => "legend",
            Self::Table => "table",
            Self::Palette => "palette",
        }
    }

    /// The flag as clap's usage errors write it, value name included, so a
    /// message about the file reads as clap's would (FR-59, FR-91).
    #[must_use]
    pub const fn usage(self) -> &'static str {
        match self {
            Self::Output => "--output <OUTPUT>",
            Self::Title => "--title <TITLE>",
            Self::Subtitle => "--subtitle <SUBTITLE>",
            Self::Rings => "--rings <RINGS>",
            Self::UnplottedRings => "--unplotted-rings <RINGS>",
            Self::Theme => "--theme <THEME>",
            Self::Width => "--width <WIDTH>",
            Self::LegendColumns => "--legend-columns <N>",
            Self::Legend => "--legend <LEGEND>",
            Self::Table => "--table <TABLE>",
            Self::Palette => "--palette <FILE>",
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

    /// How clap-style usage errors name it: `'--theme <THEME>'` for a
    /// flag, as clap does, and the key with its file otherwise (FR-59).
    #[must_use]
    pub fn usage(&self) -> String {
        match self.source {
            Source::Config(_) => self.to_string(),
            Source::Flag | Source::Default => format!("'{}'", self.setting.usage()),
        }
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
            Origin::new(Setting::UnplottedRings, file.clone()).to_string(),
            "the unplotted_rings key in conf/radar.toml"
        );
        // clap's spelling on the command line; the key's in the file.
        assert_eq!(Origin::flag(Setting::Palette).usage(), "'--palette <FILE>'");
        assert_eq!(
            Origin::new(Setting::Theme, file).usage(),
            "the theme key in conf/radar.toml"
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
