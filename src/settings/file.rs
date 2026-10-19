//! The `--config` file (FR-91): read, parsed as TOML, and every key checked.
//!
//! Each key is checked as clap checks its flag, whether or not a flag
//! overrides it (plan RG-019A, Ruling 2). Every problem is collected, so
//! one run reports them all (NFR-16 tier one).
//!
//! Imports nothing from `diagnostics`: the pipeline names each problem by
//! its key and file (plan RG-018A, R1).

use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use clap::ValueEnum;
use toml::Value;

use super::Setting;
use crate::decode::{NotUtf8, decode, line_at, read_capped};
use crate::options::{Legend, Table};
use crate::palette::Theme;
use crate::style::{MAX_EXPORT_WIDTH, MAX_LEGEND_COLUMNS, MIN_EXPORT_WIDTH};

/// The largest file read. A config is a few hundred bytes, so 1 MiB stops
/// a wrong path such as `/dev/zero` (NFR-15), as a palette's cap does.
pub const MAX_BYTES: usize = 1 << 20;

/// The values of the keys that passed their checks.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct FileValues {
    /// `output`, as written; resolved against the file's directory later.
    pub output: Option<String>,
    /// `title`.
    pub title: Option<String>,
    /// `subtitle`.
    pub subtitle: Option<String>,
    /// `rings`' entries joined with commas, as `--rings` holds them.
    pub rings: Option<String>,
    /// `unplotted_rings`, as `rings`.
    pub unplotted_rings: Option<String>,
    /// `theme` (FR-56).
    pub theme: Option<Theme>,
    /// `width` (FR-64).
    pub width: Option<u32>,
    /// `legend_columns` (FR-100).
    pub legend_columns: Option<u8>,
    /// `legend` (FR-103).
    pub legend: Option<Legend>,
    /// `table` (FR-106).
    pub table: Option<Table>,
    /// `palette`, as written; resolved later.
    pub palette: Option<String>,
}

/// A read and checked file.
#[derive(Debug)]
pub struct Loaded {
    /// The file as named on the command line, which messages quote (FR-17).
    pub file: Arc<str>,
    /// Its directory, which a relative path in it resolves against (FR-91).
    pub dir: PathBuf,
    /// Every key that passed.
    pub values: FileValues,
    /// Every key that did not, in FR-91's order, then unknown keys sorted.
    pub problems: Vec<KeyProblem>,
}

impl Loaded {
    /// Whether `setting`'s key had a problem, so it has no value to use.
    #[must_use]
    pub fn is_broken(&self, setting: Setting) -> bool {
        self.problems.iter().any(|p| p.setting() == Some(setting))
    }
}

/// One key's problem (FR-91), each a usage error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyProblem {
    /// A key FR-91 does not list, so a typo is not ignored.
    Unknown {
        /// The key as written.
        key: String,
        /// The key it is one or two edits from, a hyphen for an underscore
        /// counting as none, as FR-108 guesses a column (UX review 3).
        near: Option<Setting>,
        /// Whether its value is a table, such as `[radar]`, which puts
        /// every key under it.
        table: bool,
    },
    /// An empty `output` or `palette`, which would name the file's own
    /// folder (Rust review M6).
    Empty {
        /// The key's setting.
        setting: Setting,
    },
    /// The value is not of the key's type; `found` is TOML's name for it.
    WrongType {
        /// The key's setting.
        setting: Setting,
        /// `string`, `integer`, `float`, `boolean`, `datetime`, `array` or `table`.
        found: &'static str,
    },
    /// A list entry that is not a string.
    EntryType {
        /// The key's setting.
        setting: Setting,
        /// The entry's 1-based position.
        index: usize,
        /// TOML's name for what it is.
        found: &'static str,
    },
    /// A list entry with a comma, which the flag could not express (FR-91).
    Comma {
        /// The key's setting.
        setting: Setting,
        /// The entry's 1-based position.
        index: usize,
        /// The entry as written.
        entry: String,
    },
    /// A name `theme`, `legend` or `table` does not accept (FR-56, FR-103,
    /// FR-106).
    NotAValue {
        /// The key's setting.
        setting: Setting,
        /// The value as written.
        value: String,
        /// What it accepts, as clap lists them.
        possible: Vec<String>,
    },
    /// An integer outside FR-64's or FR-100's range.
    OutOfRange {
        /// The key's setting.
        setting: Setting,
        /// The value.
        value: i64,
        /// The least accepted.
        min: i64,
        /// The most accepted.
        max: i64,
    },
}

impl KeyProblem {
    /// The setting the problem breaks; `None` for an unknown key.
    #[must_use]
    pub const fn setting(&self) -> Option<Setting> {
        match self {
            Self::Unknown { .. } => None,
            Self::WrongType { setting, .. }
            | Self::EntryType { setting, .. }
            | Self::Comma { setting, .. }
            | Self::NotAValue { setting, .. }
            | Self::OutOfRange { setting, .. }
            | Self::Empty { setting } => Some(*setting),
        }
    }
}

/// Why there is no checked file: each a runtime failure (NFR-15).
#[derive(Debug)]
pub enum ReadError {
    /// It does not exist, is a directory, or cannot be read.
    Unreadable(io::Error),
    /// It is larger than [`MAX_BYTES`].
    TooLarge,
    /// It is not UTF-8, or holds a NUL.
    NotUtf8(NotUtf8),
    /// It is not TOML; `line` where the parser gives one.
    Syntax {
        /// The 1-based line, counted as an editor counts it.
        line: Option<usize>,
        /// The parser's message, on one line.
        message: String,
    },
}

/// FR-91: the file at `path`, read, decoded, parsed and checked.
///
/// Read through a cap, so `-c /dev/zero` ends; not checked to be a regular
/// file, so a process substitution works.
///
/// # Errors
///
/// A [`ReadError`] when the file cannot be read, is too large, is not
/// UTF-8, or is not TOML. Key problems are not errors here: they are in
/// [`Loaded::problems`], so the run can report them with everything else.
pub fn read(path: &Path) -> Result<Loaded, ReadError> {
    let bytes = read_capped(path, MAX_BYTES)
        .map_err(ReadError::Unreadable)?
        .ok_or(ReadError::TooLarge)?;
    let text = decode(&bytes).map_err(ReadError::NotUtf8)?;
    load(&text, path)
}

/// [`read`] from text already decoded, `path` naming the file.
///
/// # Errors
///
/// [`ReadError::Syntax`] when `text` is not TOML.
pub fn load(text: &str, path: &Path) -> Result<Loaded, ReadError> {
    let table: toml::Table = text
        .parse()
        .map_err(|e: toml::de::Error| ReadError::Syntax {
            line: e.span().map(|span| line_at(text.as_bytes(), span.start)),
            message: one_line(e.message()),
        })?;
    let (values, problems) = check(&table);
    Ok(Loaded {
        file: file_name(path),
        dir: path.parent().unwrap_or_else(|| Path::new("")).to_path_buf(),
        values,
        problems,
    })
}

/// The file as messages name it: the path as given on the command line
/// (FR-17), the one spelling every message about it uses.
#[must_use]
pub fn file_name(path: &Path) -> Arc<str> {
    Arc::from(path.display().to_string())
}

/// A parser message as one line: its lines joined with `; `.
fn one_line(message: &str) -> String {
    message
        .lines()
        .map(crate::trim)
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join("; ")
}

/// Every key FR-91 lists, in its order, then every other key, sorted
/// (`toml::Table` is ordered by key without `preserve_order`).
fn check(table: &toml::Table) -> (FileValues, Vec<KeyProblem>) {
    let mut values = FileValues::default();
    let mut problems: Vec<KeyProblem> = Setting::ALL
        .into_iter()
        .filter_map(|setting| table.get(setting.key()).map(|value| (setting, value)))
        .filter_map(|(setting, value)| take(&mut values, setting, value).err())
        .flatten()
        .collect();
    problems.extend(unknown(table));
    (values, problems)
}

/// `setting`'s key, `value`, checked and stored in `values`; or its
/// problems, leaving `values` without it.
fn take(v: &mut FileValues, setting: Setting, value: &Value) -> Result<(), Vec<KeyProblem>> {
    match setting {
        Setting::Output => path(setting, value).map(|s| v.output = Some(s)),
        Setting::Title => string(setting, value).map(|s| v.title = Some(s)),
        Setting::Subtitle => string(setting, value).map(|s| v.subtitle = Some(s)),
        Setting::Palette => path(setting, value).map(|s| v.palette = Some(s)),
        Setting::Rings => list(setting, value).map(|s| v.rings = Some(s)),
        Setting::UnplottedRings => list(setting, value).map(|s| v.unplotted_rings = Some(s)),
        Setting::Theme => named(setting, value).map(|t| v.theme = Some(t)),
        Setting::Legend => named(setting, value).map(|l| v.legend = Some(l)),
        Setting::Table => named(setting, value).map(|t| v.table = Some(t)),
        Setting::Width => integer(
            setting,
            value,
            i64::from(MIN_EXPORT_WIDTH),
            i64::from(MAX_EXPORT_WIDTH),
        )
        .map(|n| v.width = u32::try_from(n).ok()),
        Setting::LegendColumns => integer(setting, value, 1, i64::from(MAX_LEGEND_COLUMNS))
            .map(|n| v.legend_columns = u8::try_from(n).ok()),
    }
}

/// Every key FR-91 does not list, so a typo is not ignored.
fn unknown(table: &toml::Table) -> impl Iterator<Item = KeyProblem> + '_ {
    table
        .iter()
        .filter(|(key, _)| !Setting::ALL.iter().any(|s| s.key() == key.as_str()))
        .map(|(key, value)| KeyProblem::Unknown {
            key: key.clone(),
            near: near_miss(key),
            table: value.is_table(),
        })
}

/// The key `key` is one or two edits from, as FR-108 guesses a column
/// (D-98): a hyphen for an underscore and a capital count as no edit, a
/// transposition as one, and only a key longer than four characters, given
/// longer still, allows two. Of several, the fewest edits wins, then
/// FR-91's order.
fn near_miss(key: &str) -> Option<Setting> {
    /// The longest key that allows only one edit.
    const SHORT: usize = 4;
    let spelled = key.replace('-', "_").to_lowercase();
    let length = spelled.chars().count();
    Setting::ALL
        .into_iter()
        .map(|s| (strsim::damerau_levenshtein(&spelled, s.key()), s))
        .filter(|&(edits, s)| {
            let allowed = if s.key().len() > SHORT && length > s.key().len() {
                2
            } else {
                1
            };
            edits <= allowed
        })
        .min_by_key(|&(edits, _)| edits)
        .map(|(_, s)| s)
}

/// A path key's string, refused when empty (Rust review M6).
fn path(setting: Setting, value: &Value) -> Result<String, Vec<KeyProblem>> {
    let text = string(setting, value)?;
    Some(text)
        .filter(|t| !t.is_empty())
        .ok_or_else(|| vec![KeyProblem::Empty { setting }])
}

fn wrong(setting: Setting, value: &Value) -> Vec<KeyProblem> {
    vec![KeyProblem::WrongType {
        setting,
        found: value.type_str(),
    }]
}

fn string(setting: Setting, value: &Value) -> Result<String, Vec<KeyProblem>> {
    match value {
        Value::String(s) => Ok(s.clone()),
        other => Err(wrong(setting, other)),
    }
}

/// FR-64, FR-100: clap's ranged parser, so the message can be its.
fn integer(setting: Setting, value: &Value, min: i64, max: i64) -> Result<i64, Vec<KeyProblem>> {
    match value {
        Value::Integer(n) if (min..=max).contains(n) => Ok(*n),
        Value::Integer(n) => Err(vec![KeyProblem::OutOfRange {
            setting,
            value: *n,
            min,
            max,
        }]),
        other => Err(wrong(setting, other)),
    }
}

/// FR-56, FR-103, FR-106: matched as clap's `ignore_case` matches the flag,
/// aliases included, so `legend = "hide"` works as `--legend hide` does.
fn named<T: ValueEnum>(setting: Setting, value: &Value) -> Result<T, Vec<KeyProblem>> {
    let text = string(setting, value)?;
    T::from_str(&text, true).map_err(|_| {
        vec![KeyProblem::NotAValue {
            setting,
            value: text,
            possible: T::value_variants()
                .iter()
                .filter_map(ValueEnum::to_possible_value)
                .filter(|p| !p.is_hide_set())
                .map(|p| p.get_name().to_owned())
                .collect(),
        }]
    })
}

/// FR-91: an array of strings, none holding a comma, joined with commas
/// into the list the flag would hold, so FR-15's tidy, FR-63's checks and
/// their entry numbers are the flag's (plan RG-019A, Ruling 5).
fn list(setting: Setting, value: &Value) -> Result<String, Vec<KeyProblem>> {
    let Value::Array(entries) = value else {
        return Err(wrong(setting, value));
    };
    let mut names = Vec::new();
    let mut problems = Vec::new();
    for (i, entry) in entries.iter().enumerate() {
        match entry {
            Value::String(s) if s.contains(',') => {
                problems.push(KeyProblem::Comma {
                    setting,
                    index: i + 1,
                    entry: s.clone(),
                });
            }
            Value::String(s) => names.push(s.as_str()),
            other => problems.push(KeyProblem::EntryType {
                setting,
                index: i + 1,
                found: other.type_str(),
            }),
        }
    }
    if problems.is_empty() {
        Ok(names.join(","))
    } else {
        Err(problems)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn loaded(text: &str) -> Loaded {
        load(text, Path::new("conf/radar.toml")).expect("parses")
    }

    #[test]
    fn every_key_of_its_type_is_read() {
        let l = loaded(
            "output = \"out/r.png\"\ntitle = \"Radar\"\nsubtitle = \"Q3\"\nrings = [\" Hold \", \"Adopt\"]\n\
             unplotted_rings = [\"In Use\"]\ntheme = \"Dark\"\nwidth = 1920\nlegend_columns = 3\n\
             legend = \"HIDE\"\ntable = \"All\"\npalette = \"brand.css\"\n",
        );
        assert_eq!(l.problems, []);
        assert_eq!(&*l.file, "conf/radar.toml");
        assert_eq!(l.dir, Path::new("conf"));
        let v = l.values;
        assert_eq!(
            v.rings.as_deref(),
            Some(" Hold ,Adopt"),
            "joined as the flag holds it"
        );
        assert_eq!(v.unplotted_rings.as_deref(), Some("In Use"));
        // FR-56, FR-103, FR-106: any case, and --legend's aliases.
        assert_eq!(
            (v.theme, v.legend, v.table),
            (Some(Theme::Dark), Some(Legend::Hidden), Some(Table::All))
        );
        assert_eq!((v.width, v.legend_columns), (Some(1920), Some(3)));
        assert_eq!(v.palette.as_deref(), Some("brand.css"));
    }

    /// FR-91: every problem, in FR-91's key order, then unknown keys sorted.
    #[test]
    fn every_problem_is_collected() {
        let l = loaded(
            "zebra = 1\nring = [\"Adopt\"]\nwidth = 1920.0\nrings = \"Adopt,Trial\"\ntheme = \"sepia\"\n\
             legend_columns = 5\nunplotted_rings = [\"In Use\", 3, \"A,B\"]\n",
        );
        assert_eq!(
            l.problems,
            [
                KeyProblem::WrongType {
                    setting: Setting::Rings,
                    found: "string"
                },
                KeyProblem::EntryType {
                    setting: Setting::UnplottedRings,
                    index: 2,
                    found: "integer"
                },
                KeyProblem::Comma {
                    setting: Setting::UnplottedRings,
                    index: 3,
                    entry: "A,B".into()
                },
                KeyProblem::NotAValue {
                    setting: Setting::Theme,
                    value: "sepia".into(),
                    possible: vec!["light".into(), "dark".into()],
                },
                KeyProblem::WrongType {
                    setting: Setting::Width,
                    found: "float"
                },
                KeyProblem::OutOfRange {
                    setting: Setting::LegendColumns,
                    value: 5,
                    min: 1,
                    max: 4
                },
                KeyProblem::Unknown {
                    key: "ring".into(),
                    near: Some(Setting::Rings),
                    table: false,
                },
                KeyProblem::Unknown {
                    key: "zebra".into(),
                    near: None,
                    table: false,
                },
            ]
        );
        assert!(l.is_broken(Setting::Rings) && l.is_broken(Setting::Width));
        assert!(!l.is_broken(Setting::Title));
        assert_eq!(l.values.rings, None, "a broken key gives no value");
    }

    /// FR-91: `[]` is FR-63's (or FR-107's) no-rings error, raised where
    /// the list is parsed; here it is an empty list.
    #[test]
    fn an_empty_list_is_an_empty_comma_list() {
        let l = loaded("rings = []\nunplotted_rings = []\n");
        assert_eq!(l.problems, []);
        assert_eq!(
            (
                l.values.rings.as_deref(),
                l.values.unplotted_rings.as_deref()
            ),
            (Some(""), Some(""))
        );
    }

    /// Review Focus 3: no overflow, no panic, FR-64's and FR-100's message.
    #[test]
    fn integers_at_the_extremes_are_range_errors() {
        for (text, setting, value, min, max) in [
            (
                "width = 9223372036854775807",
                Setting::Width,
                i64::MAX,
                320,
                10_000,
            ),
            ("width = -1", Setting::Width, -1, 320, 10_000),
            ("legend_columns = 0", Setting::LegendColumns, 0, 1, 4),
            (
                "legend_columns = -9223372036854775808",
                Setting::LegendColumns,
                i64::MIN,
                1,
                4,
            ),
        ] {
            assert_eq!(
                loaded(text).problems,
                [KeyProblem::OutOfRange {
                    setting,
                    value,
                    min,
                    max
                }],
                "{text}"
            );
        }
        assert_eq!(loaded("width = 320").values.width, Some(320));
        assert_eq!(loaded("width = 10000").values.width, Some(10_000));
    }

    /// Review Focus 4: a table, dotted key or datetime where a value is
    /// expected is its key's wrong type, and nothing else.
    #[test]
    fn a_table_or_datetime_is_the_wrong_type() {
        for (text, setting, found) in [
            ("[theme]\nname = \"dark\"", Setting::Theme, "table"),
            ("rings.extra = \"x\"", Setting::Rings, "table"),
            ("title = { text = \"x\" }", Setting::Title, "table"),
            ("title = 2026-09-01", Setting::Title, "datetime"),
            ("output = true", Setting::Output, "boolean"),
        ] {
            assert_eq!(
                loaded(text).problems,
                [KeyProblem::WrongType { setting, found }],
                "{text}"
            );
        }
    }

    /// A duplicate key is TOML's own error, with its line (FR-91).
    #[test]
    fn a_syntax_error_names_its_line() {
        let error =
            load("title = \"a\"\n\ntitle = \"b\"\n", Path::new("r.toml")).expect_err("duplicate");
        let ReadError::Syntax { line, message } = error else {
            panic!("{error:?}")
        };
        assert_eq!(line, Some(3));
        assert!(
            !message.contains('\n') && message.contains("duplicate"),
            "{message}"
        );
    }

    /// QA review I-2, Rust review M7: every key's type is checked where it
    /// is wired, and `legend` and `table` refuse a name as `theme` does.
    #[test]
    fn every_key_refuses_the_wrong_type() {
        for (text, setting, found) in [
            ("output = 1", Setting::Output, "integer"),
            ("title = 1", Setting::Title, "integer"),
            ("subtitle = 1", Setting::Subtitle, "integer"),
            ("rings = 1", Setting::Rings, "integer"),
            (
                "unplotted_rings = \"Hold\"",
                Setting::UnplottedRings,
                "string",
            ),
            ("theme = 1", Setting::Theme, "integer"),
            ("width = \"1920\"", Setting::Width, "string"),
            ("legend_columns = \"3\"", Setting::LegendColumns, "string"),
            ("legend = true", Setting::Legend, "boolean"),
            ("table = 1.5", Setting::Table, "float"),
            ("palette = 3", Setting::Palette, "integer"),
        ] {
            let l = loaded(text);
            assert_eq!(
                l.problems,
                [KeyProblem::WrongType { setting, found }],
                "{text}"
            );
            assert!(l.is_broken(setting), "{text}");
        }
        for (text, setting, possible) in [
            ("legend = \"nope\"", Setting::Legend, ["shown", "hidden"]),
            ("table = \"nope\"", Setting::Table, ["radar", "all"]),
        ] {
            assert_eq!(
                loaded(text).problems,
                [KeyProblem::NotAValue {
                    setting,
                    value: "nope".into(),
                    possible: possible.map(String::from).to_vec(),
                }],
                "{text}"
            );
        }
        assert_eq!(
            Setting::LegendColumns.expected(),
            super::super::Expected::Integer
        );
        assert_eq!(
            Setting::UnplottedRings.expected(),
            super::super::Expected::Strings
        );
    }

    /// QA review M-2: a file at the cap reads; one byte more does not.
    #[test]
    fn a_file_at_the_cap_reads() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("radar.toml");
        std::fs::write(&path, vec![b'#'; MAX_BYTES]).expect("writes");
        assert!(read(&path).is_ok());
    }

    /// QA review M-3: the file is decoded as the CSV is, so a NUL or a
    /// bad byte past the first line is named by its own line.
    #[test]
    fn a_nul_or_bad_byte_is_named_by_its_line() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("radar.toml");
        for bytes in [
            &b"title = \"a\"\n\nsubtitle = \"\0\"\n"[..],
            &b"title = \"a\"\n\nsubtitle = \"\xff\"\n"[..],
        ] {
            std::fs::write(&path, bytes).expect("writes");
            assert!(matches!(
                read(&path),
                Err(ReadError::NotUtf8(NotUtf8 { line: 3 }))
            ));
        }
    }

    /// Review Focus 1: Notepad's BOM and CRLF parse, and a syntax error
    /// after CRLF lines names the line an editor shows. (TOML refuses a
    /// lone CR, so CRLF is the only line end that differs from LF.)
    #[test]
    fn a_bom_and_crlf_parse_and_lines_count_as_an_editor_does() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("radar.toml");
        std::fs::write(&path, b"\xEF\xBB\xBFtitle = \"Radar\"\r\nwidth = 1920\r\n")
            .expect("writes");
        let l = read(&path).expect("parses");
        assert_eq!(
            (l.values.title.as_deref(), l.values.width),
            (Some("Radar"), Some(1920))
        );
        std::fs::write(
            &path,
            b"\xEF\xBB\xBFtitle = \"Radar\"\r\n\r\nwidth = = 3\r\n",
        )
        .expect("writes");
        assert!(matches!(
            read(&path),
            Err(ReadError::Syntax { line: Some(3), .. })
        ));
    }

    #[test]
    fn an_empty_file_has_no_values_and_no_problems() {
        let l = loaded("");
        assert_eq!((l.values, l.problems), (FileValues::default(), vec![]));
    }

    /// UX review 3: an unknown key names the key it is near, a hyphen for
    /// an underscore included, and says when it is a table.
    #[test]
    fn an_unknown_key_names_its_near_miss() {
        let unknown = |text: &str| loaded(text).problems;
        assert_eq!(
            unknown("unplotted-rings = [\"In Use\"]"),
            [KeyProblem::Unknown {
                key: "unplotted-rings".into(),
                near: Some(Setting::UnplottedRings),
                table: false,
            }]
        );
        assert_eq!(
            unknown("titel = \"x\""),
            [KeyProblem::Unknown {
                key: "titel".into(),
                near: Some(Setting::Title),
                table: false,
            }]
        );
        assert_eq!(
            unknown("[radar]\ntitle = \"x\""),
            [KeyProblem::Unknown {
                key: "radar".into(),
                near: None,
                table: true,
            }]
        );
        assert_eq!(
            unknown("csv = \"r.csv\""),
            [KeyProblem::Unknown {
                key: "csv".into(),
                near: None,
                table: false,
            }]
        );
    }

    /// Rust review M6: an empty path key is its own problem, not a path
    /// that resolves to the file's folder.
    #[test]
    fn an_empty_path_key_is_a_problem() {
        assert_eq!(
            loaded("output = \"\"\npalette = \"\"\ntitle = \"\"").problems,
            [
                KeyProblem::Empty {
                    setting: Setting::Output
                },
                KeyProblem::Empty {
                    setting: Setting::Palette
                },
            ],
            "an empty title is allowed: it shows no title (FR-65)"
        );
    }

    /// Security review F3: deep nesting is TOML's own error, not a crash.
    #[test]
    fn deep_nesting_is_a_syntax_error() {
        let text = format!("title = {}", "[".repeat(600_000));
        assert!(text.len() < MAX_BYTES);
        assert!(matches!(
            load(&text, Path::new("r.toml")),
            Err(ReadError::Syntax { .. })
        ));
    }

    #[test]
    fn a_large_or_unreadable_file_is_refused() {
        let dir = tempfile::tempdir().expect("tempdir");
        assert!(
            matches!(read(dir.path()), Err(ReadError::Unreadable(_))),
            "a directory"
        );
        let big = dir.path().join("big.toml");
        std::fs::write(&big, vec![b'#'; MAX_BYTES + 1]).expect("writes");
        assert!(matches!(read(&big), Err(ReadError::TooLarge)));
        let latin1 = dir.path().join("l.toml");
        std::fs::write(&latin1, b"title = \"\xE9\"\n").expect("writes");
        assert!(matches!(
            read(&latin1),
            Err(ReadError::NotUtf8(NotUtf8 { line: 1 }))
        ));
    }
}
