//! Where each setting's value came from (NFR-10, D-61, D-64), and the
//! shape of one setting after the `--config` merge (FR-91, D-57).
//!
//! The merge itself is `Cli::settings`, beside the flags it reads
//! (architecture §3); `file` reads the TOML. Nothing here imports `cli` or
//! `diagnostics`, so the taxonomy can name a source without a cycle.

pub mod edition;
pub mod file;
pub mod sample;

use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::options::{Legend, Paper, Pdf, Table};
use crate::palette::Theme;
use edition::Edition;
use file::{FileValues, Loaded};

/// FR-56's default.
pub const DEFAULT_THEME: Theme = Theme::Light;
/// FR-103's default.
pub const DEFAULT_LEGEND: Legend = Legend::Shown;
/// FR-106's default.
pub const DEFAULT_TABLE: Table = Table::Radar;
/// FR-112: the radar PDF, unless `--pdf full` asks for the table too.
pub const DEFAULT_PDF: Pdf = Pdf::Radar;
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

impl Source {
    /// Whether the value came from a `--config` file.
    #[must_use]
    pub const fn is_config(&self) -> bool {
        matches!(self, Self::Config(_))
    }
}

/// The settings FR-91 lists, each a flag and a key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Setting {
    /// The positional CSV, `input` (a key at the maintainer's decision,
    /// 2026-10-22, so a scheduled job runs `radargen -c radar.toml`).
    Input,
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
    /// `--paper`, `paper` (FR-111).
    Paper,
    /// `--pdf`, `pdf` (FR-112).
    Pdf,
    /// `--palette`, `palette`.
    Palette,
    /// `-e/--edition`, `edition` (FR-92; a key at the maintainer's
    /// decision, 2026-10-22, so a scheduled job's file can say `today`).
    Edition,
}

impl Setting {
    /// Every setting, in FR-91's order: the order a file's problems are
    /// reported in.
    pub const ALL: [Self; 15] = [
        Self::Input,
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
        Self::Paper,
        Self::Pdf,
        Self::Palette,
        Self::Edition,
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
            Self::Input => ("<CSV_PATH>", "input"),
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
            Self::Paper => ("--paper", "paper"),
            Self::Pdf => ("--pdf", "pdf"),
            Self::Palette => ("--palette", "palette"),
            Self::Edition => ("--edition", "edition"),
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

    /// The ring set from `source` (FR-17, D-61).
    #[must_use]
    pub const fn rings(source: Source) -> Self {
        Self::new(Setting::Rings, source)
    }

    /// Another setting from the same source, for a remedy that names a
    /// sibling: `--unplotted-rings`, or the `unplotted_rings` key in the
    /// same file (UX review 4).
    #[must_use]
    pub fn sibling(&self, setting: Setting) -> Self {
        Self::new(setting, self.source.clone())
    }

    /// Whether the value came from the command line.
    #[must_use]
    pub fn is_flag(&self) -> bool {
        self.source == Source::Flag
    }

    /// The bare name an author edits: the flag, or the key in a file.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        if self.source.is_config() {
            self.setting.key()
        } else {
            self.setting.flag()
        }
    }

    /// How to set it to `value`, with `tail` (such as " or more") after
    /// the value: `--width 1200 or more`, or `setting width = 1200 or more
    /// in radar.toml`. A string value is quoted, with `"` and `\` escaped
    /// as a TOML basic string needs. The one place a remedy's wording
    /// depends on the source; the caller places the verb that follows.
    #[must_use]
    pub fn set_to(&self, value: impl fmt::Display, tail: &str) -> String {
        let assignment = self.assignment(value, tail);
        if self.source.is_config() {
            format!("setting {assignment}")
        } else {
            assignment
        }
    }

    /// The setting at `value`, with `tail` after the value, as its source
    /// holds it: `--paper a4`, or `paper = "a4" in radar.toml`, a string
    /// quoted as [`set_to`](Self::set_to) quotes it.
    #[must_use]
    pub fn assignment(&self, value: impl fmt::Display, tail: &str) -> String {
        match &self.source {
            Source::Config(file) => {
                let key = self.setting.key();
                match self.setting {
                    Setting::Width | Setting::LegendColumns => {
                        format!("{key} = {value}{tail} in {file}")
                    }
                    _ => {
                        let text = value.to_string().replace('\\', "\\\\").replace('"', "\\\"");
                        format!("{key} = \"{text}\"{tail} in {file}")
                    }
                }
            }
            Source::Flag | Source::Default => format!("{} {value}{tail}", self.setting.flag()),
        }
    }

    /// How to make it smaller: `pass a smaller --width`, or `set a smaller
    /// width in radar.toml`.
    #[must_use]
    pub fn smaller(&self) -> String {
        match &self.source {
            Source::Config(file) => format!("set a smaller {} in {file}", self.setting.key()),
            Source::Flag | Source::Default => format!("pass a smaller {}", self.setting.flag()),
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

/// A path as every message shows it: as written, never resolved (plan
/// RG-018A, R1). The one spelling, for a path setting and the config file.
#[must_use]
pub fn as_written(path: &Path) -> String {
    path.display().to_string()
}

impl NamedPath {
    /// `path` from `origin`, shown as written.
    #[must_use]
    pub fn new(origin: Origin, path: &Path) -> Self {
        Self {
            origin,
            shown: as_written(path),
        }
    }

    /// What a message adds when a path from the file resolved somewhere
    /// its author likely did not mean (FR-91): a `~`, which no shell
    /// expanded (UX review 5), or a relative path under the folder a
    /// process substitution names, such as `/dev/fd` (rg-37u); empty
    /// otherwise.
    #[must_use]
    pub fn hint(&self) -> String {
        let Source::Config(file) = &self.origin.source else {
            return String::new();
        };
        if self.shown == "~" || self.shown.starts_with("~/") {
            return "; ~ is not expanded in a config file, so write the full path".to_owned();
        }
        match Path::new(&**file).parent() {
            Some(folder) if Path::new(&self.shown).is_relative() && descriptors(folder) => {
                format!(
                    "; a config file read through a process substitution resolves relative paths under {}, so write the full path",
                    folder.display()
                )
            }
            _ => String::new(),
        }
    }
}

/// Whether `folder` is where a shell's process substitution names its
/// pipe: `/dev/fd`, or Linux's `/proc/<pid>/fd` (`/proc/self/fd` among
/// them). A relative path read from a file there cannot name a real one.
fn descriptors(folder: &Path) -> bool {
    let under_proc = folder.starts_with("/proc")
        && folder.components().count() == 4
        && folder.file_name().is_some_and(|name| name == "fd");
    folder == Path::new("/dev/fd") || under_proc
}

impl fmt::Display for NamedPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.origin.source.is_config() {
            write!(f, "{} ({})", self.origin, self.shown)
        } else {
            write!(f, "{} {}", self.origin, self.shown)
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
    /// The positional CSV or `input`.
    pub input: Effective<PathBuf>,
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
    /// `paper` (FR-111).
    pub paper: Effective<Paper>,
    /// `--pdf` or `pdf` (FR-112).
    pub pdf: Effective<Pdf>,
    /// The file's `pdf = "full"` that `--paper none` displaced, so the run
    /// can say the table was dropped (the maintainer's ruling after the
    /// final review, 2026-10-23).
    pub dropped_full: Option<Origin>,
    /// `--palette` or `palette`.
    pub palette: Effective<PathBuf>,
    /// `--edition` or `edition` (FR-92).
    pub edition: Effective<Edition>,
    /// The file's directory, which a relative path from it resolves
    /// against (FR-91); `None` without `--config`.
    pub dir: Option<PathBuf>,
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
/// directory. `~` is never expanded: no shell reads the file. Returns the
/// path to open, and the path as a message names it, as written.
#[must_use]
pub fn resolve(dir: Option<&Path>, path: &Sourced<PathBuf>) -> (PathBuf, NamedPath) {
    let opened = match dir {
        Some(dir) if path.origin.source.is_config() && path.value.is_relative() => {
            dir.join(&path.value)
        }
        _ => path.value.clone(),
    };
    let named = NamedPath::new(path.origin.clone(), &path.value);
    (opened, named)
}

/// Whether `value` came from the command line.
fn by_flag<T>(value: &Effective<T>) -> bool {
    value.as_given().is_some_and(|given| given.origin.is_flag())
}

impl Settings {
    /// FR-112's half of [`displace`](Self::displace): `--pdf full` drops
    /// the file's `paper = "none"`, and `--paper none` drops its
    /// `pdf = "full"`, recording that the table was dropped.
    fn displace_pdf_paper(&mut self) {
        let full = self.pdf.as_given().is_some_and(|p| p.value == Pdf::Full);
        let none = self
            .paper
            .as_given()
            .is_some_and(|p| p.value == Paper::None);
        if !(full && none) {
            return;
        }
        match (by_flag(&self.pdf), by_flag(&self.paper)) {
            (true, false) => self.paper = Effective::Absent,
            (false, true) => {
                self.dropped_full = self.pdf.as_given().map(|pdf| pdf.origin.clone());
                self.pdf = Effective::Absent;
            }
            _ => {}
        }
    }

    /// FR-59 across sources (rg-ahj; the maintainer's ruling, 2026-10-18):
    /// the command line wins over the opposing key, so `--palette` drops
    /// the file's `theme` and `--theme` drops its `palette`. The dropped key
    /// is still checked (FR-91), but nothing reads its value.
    ///
    /// FR-112 likewise: `--pdf full` drops the file's `paper = "none"`, and
    /// `--paper none` drops its `pdf = "full"`, since a full PDF is always
    /// on paper (the maintainer's ruling, 2026-10-23).
    pub fn displace(&mut self) {
        if by_flag(&self.palette) && !by_flag(&self.theme) {
            self.theme = Effective::Absent;
        }
        if by_flag(&self.theme) && !by_flag(&self.palette) {
            self.palette = Effective::Absent;
        }
        self.displace_pdf_paper();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// rg-37u: where a process substitution's pipe lives, and nowhere else.
    #[test]
    fn descriptor_folders_are_dev_fd_and_proc_fd() {
        for yes in ["/dev/fd", "/proc/self/fd", "/proc/4242/fd"] {
            assert!(descriptors(Path::new(yes)), "{yes}");
        }
        for no in [
            "/dev",
            "/dev/fdx",
            "/proc/fd",
            "/proc/self/fd/x",
            "dev/fd",
            "/home/fd",
        ] {
            assert!(!descriptors(Path::new(no)), "{no}");
        }
    }

    /// rg-37u: only a relative path from a file read under /dev/fd gets
    /// the hint; `~` keeps its own.
    #[test]
    fn the_hint_names_the_folder_a_path_resolved_under() {
        let from = |file: &str, path: &str| {
            NamedPath::new(
                Origin::new(Setting::Output, Source::Config(Arc::from(file))),
                Path::new(path),
            )
            .hint()
        };
        assert_eq!(
            from("/dev/fd/63", "r.html"),
            "; a config file read through a process substitution resolves relative paths under /dev/fd, so write the full path"
        );
        assert_eq!(from("/dev/fd/63", "/tmp/r.html"), "");
        assert_eq!(from("radar.toml", "r.html"), "");
        assert!(from("/dev/fd/63", "~/r.html").starts_with("; ~ is not expanded"));
        let flag = NamedPath::new(
            Origin::new(Setting::Output, Source::Flag),
            Path::new("r.html"),
        );
        assert_eq!(flag.hint(), "");
    }

    #[test]
    fn every_setting_has_fr_91_s_key_in_its_order() {
        let keys: Vec<&str> = Setting::ALL.iter().map(|s| s.key()).collect();
        assert_eq!(
            keys,
            [
                "input",
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
                "paper",
                "pdf",
                "palette",
                "edition"
            ]
        );
        let flags: Vec<&str> = Setting::ALL.iter().map(|s| s.flag()).collect();
        assert_eq!(
            flags,
            [
                "<CSV_PATH>",
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
                "--paper",
                "--pdf",
                "--palette",
                "--edition"
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

    /// Final review M-1, M-2 and M-5: the caller places the tail, a
    /// string is quoted and escaped, and the flag form is unchanged.
    #[test]
    fn set_to_and_smaller_word_each_source() {
        let file = Origin::new(Setting::Width, Source::Config(Arc::from("r.toml")));
        assert_eq!(
            file.set_to(1200, " or more"),
            "setting width = 1200 or more in r.toml"
        );
        assert_eq!(file.smaller(), "set a smaller width in r.toml");
        let flag = Origin::flag(Setting::Width);
        assert_eq!(flag.set_to(1200, " or more"), "--width 1200 or more");
        assert_eq!(flag.smaller(), "pass a smaller --width");
        let title = Origin::new(Setting::Title, Source::Config(Arc::from("r.toml")));
        assert_eq!(
            title.set_to(r#"a "b" \c"#, ""),
            r#"setting title = "a \"b\" \\c" in r.toml"#
        );
    }

    #[test]
    fn a_named_path_shows_the_path_as_written() {
        assert_eq!(
            NamedPath::new(Origin::flag(Setting::Output), Path::new("out/radar.html")).to_string(),
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
