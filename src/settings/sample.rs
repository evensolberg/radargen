//! FR-109: the starter `--config` file `--create-config` writes (D-102).
//!
//! The text is `docs/product/config.sample.toml`, so the docs and the tests
//! read the bytes the flag writes, as the palette sample is (FR-101). Every
//! key is commented out, so the file as written changes nothing; the tests
//! below hold it to FR-91's key list, its order and each key's type.

/// The file `--create-config` writes: every FR-91 key, commented out, with
/// its default where it has one and a line saying what it does.
pub const SAMPLE: &str = include_str!("../../docs/product/config.sample.toml");

/// The commented-out key line's prefix; removing it uses the key.
pub const COMMENT: &str = "# ";

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::rings::DEFAULT_RINGS;
    use crate::settings::file::{FileValues, Loaded, load};
    use crate::settings::{
        DEFAULT_LEGEND, DEFAULT_LEGEND_COLUMNS, DEFAULT_TABLE, DEFAULT_THEME, Setting,
    };

    /// The setting a line sets once [`COMMENT`] is removed, if it is a key
    /// line: `# key = value`, the key one of FR-91's.
    fn key_of(line: &str) -> Option<Setting> {
        let (key, _) = line.strip_prefix(COMMENT)?.split_once(" = ")?;
        Setting::ALL.into_iter().find(|s| s.key() == key)
    }

    /// The sample with every key line but `except`'s uncommented.
    fn uncommented(except: Setting) -> String {
        SAMPLE
            .lines()
            .map(|line| match key_of(line) {
                Some(setting) if setting != except => line.strip_prefix(COMMENT).unwrap_or(line),
                _ => line,
            })
            .flat_map(|line| [line, "\n"])
            .collect()
    }

    fn loaded(text: &str) -> Loaded {
        load(text, Path::new("radar.toml")).expect("parses")
    }

    /// FR-109: as written, the file is an empty file.
    #[test]
    fn as_written_it_sets_nothing() {
        let file = loaded(SAMPLE);
        assert!(file.problems.is_empty(), "{:?}", file.problems);
        assert_eq!(file.values, FileValues::default());
    }

    /// FR-109: every FR-91 key, once each, in `Setting::ALL`'s order, so
    /// the starter cannot drift from the key list.
    #[test]
    fn it_lists_every_key_once_in_order() {
        let keys: Vec<Setting> = SAMPLE.lines().filter_map(key_of).collect();
        assert_eq!(keys, Setting::ALL);
    }

    /// FR-109: every line uncommented is a valid file, so each example has
    /// its key's type; theme or palette is left out, since both together
    /// are FR-59's conflict.
    #[test]
    fn every_line_uncommented_has_no_key_problem() {
        for except in [Setting::Theme, Setting::Palette] {
            let file = loaded(&uncommented(except));
            assert!(file.problems.is_empty(), "{except:?}: {:?}", file.problems);
            for setting in Setting::ALL {
                let set = match setting {
                    Setting::Input => file.values.input.is_some(),
                    Setting::Output => file.values.output.is_some(),
                    Setting::Title => file.values.title.is_some(),
                    Setting::Subtitle => file.values.subtitle.is_some(),
                    Setting::Rings => file.values.rings.is_some(),
                    Setting::UnplottedRings => file.values.unplotted_rings.is_some(),
                    Setting::Theme => file.values.theme.is_some(),
                    Setting::Width => file.values.width.is_some(),
                    Setting::LegendColumns => file.values.legend_columns.is_some(),
                    Setting::Legend => file.values.legend.is_some(),
                    Setting::Table => file.values.table.is_some(),
                    Setting::Paper => file.values.paper.is_some(),
                    Setting::Pdf => file.values.pdf.is_some(),
                    Setting::Palette => file.values.palette.is_some(),
                    Setting::Edition => file.values.edition.is_some(),
                };
                assert_eq!(set, setting != except, "{setting:?}");
            }
        }
    }

    /// FR-109: a key with a default shows that default.
    #[test]
    fn each_key_with_a_default_shows_it() {
        let values = loaded(&uncommented(Setting::Palette)).values;
        assert_eq!(values.rings, Some(DEFAULT_RINGS.join(",")));
        assert_eq!(values.theme, Some(DEFAULT_THEME));
        assert_eq!(values.legend_columns, Some(DEFAULT_LEGEND_COLUMNS));
        assert_eq!(values.legend, Some(DEFAULT_LEGEND));
        assert_eq!(values.table, Some(DEFAULT_TABLE));
        assert_eq!(values.paper, Some(crate::options::Paper::None));
        assert_eq!(values.pdf, Some(super::super::DEFAULT_PDF));
    }

    /// The ranges the comments state are the code's, so neither can move
    /// without the other.
    #[test]
    fn the_stated_ranges_are_the_codes() {
        use crate::style::{MAX_EXPORT_WIDTH, MAX_LEGEND_COLUMNS, MIN_EXPORT_WIDTH};
        assert!(SAMPLE.contains(&format!("from {MIN_EXPORT_WIDTH} to {MAX_EXPORT_WIDTH}")));
        assert!(SAMPLE.contains(&format!("into 1 to {MAX_LEGEND_COLUMNS} columns")));
    }

    /// The maintainer's lines (rg-2ss): the CSV, today's edition and a
    /// subtitle that shows it off, with the note on what it needs.
    #[test]
    fn it_has_the_maintainer_s_lines() {
        for line in [
            "# input = \"radar.csv\"",
            "# edition = \"today\"",
            "# subtitle = \"Created {edition}\"",
        ] {
            assert!(SAMPLE.lines().any(|l| l == line), "{line}");
        }
        for needs in ["-e YYYY-MM-DD", "-e today", "edition key"] {
            assert!(SAMPLE.contains(needs), "{needs}");
        }
    }

    /// NFR-17 and D-101: plain ASCII, LF endings, no byte-order mark, and
    /// sectors, never quadrants; it reads as a user's file, with no
    /// requirement numbers.
    #[test]
    fn it_reads_as_a_users_file() {
        assert!(SAMPLE.is_ascii());
        assert!(!SAMPLE.contains('\r') && SAMPLE.ends_with('\n'));
        assert!(!SAMPLE.to_lowercase().contains("quadrant"));
        assert!(!SAMPLE.contains("FR-") && !SAMPLE.contains("D-"));
        for said in ["-c", "overrides", "folder"] {
            assert!(SAMPLE.contains(said), "{said}");
        }
    }
}
