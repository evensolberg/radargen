//! radargen — CSV to technology radar.
//!
//! See `docs/product/radargen-prd.md`. Requirement IDs in comments refer to it.

pub mod cli;
pub mod csv;
pub mod diagnostics;
pub mod escape;
pub mod model;
pub mod num;
pub mod order;
pub mod palette;
pub mod rings;
pub mod style;
pub mod text;

use unicode_properties::{GeneralCategory, UnicodeGeneralCategory};

/// FR-76's trim: any leading or trailing run of whitespace and Unicode
/// format characters (Cf), in any mix.
///
/// A stray byte-order mark or zero-width space is invisible in an editor, so
/// it must not stop `name` matching `name`.
///
/// The tag characters are kept: they end a subdivision flag such as
/// England's, and trimming them would turn every such flag into the same bare
/// black flag (D-61). Format characters inside a value are content and kept.
///
/// Every display trim uses this too — FR-9's quadrant label, FR-15's ring
/// name, and the values diagnostics quote — so no site trims differently.
#[must_use]
pub fn trim(value: &str) -> &str {
    value.trim_matches(trimmable)
}

/// Whitespace, or a format character that is not a tag character.
fn trimmable(c: char) -> bool {
    c.is_whitespace()
        || (c.general_category() == GeneralCategory::Format
            && !matches!(c, '\u{e0001}' | '\u{e0020}'..='\u{e007f}'))
}

/// FR-76: **one normalisation rule**, applied to every value read from the CSV
/// or from `--rings`.
///
/// Both operands of a comparison get [`trim`] and are then folded
/// case-insensitively. Folding is Unicode's default lowercase mapping —
/// `str::to_lowercase`, not full case folding and not locale-aware (D-53).
///
/// This never edits displayed text — callers compare on the result and render
/// the original.
#[must_use]
pub fn normalise(value: &str) -> String {
    trim(value).to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// England's flag: U+1F3F4, the tags g b e n g, then CANCEL TAG.
    const ENGLAND: &str = "\u{1f3f4}\u{e0067}\u{e0062}\u{e0065}\u{e006e}\u{e0067}\u{e007f}";
    /// Scotland's: the same black flag, tags g b s c t.
    const SCOTLAND: &str = "\u{1f3f4}\u{e0067}\u{e0062}\u{e0073}\u{e0063}\u{e0074}\u{e007f}";

    #[test]
    fn trims_a_stray_bom_and_zero_width_space() {
        assert_eq!(trim("\u{feff}name\u{200b}"), "name");
    }

    #[test]
    fn trims_a_mixed_run_of_spaces_and_format_characters() {
        assert_eq!(trim("\u{200b} \u{feff}name \u{200b}\t"), "name");
    }

    #[test]
    fn keeps_format_characters_inside_a_value() {
        // A zero-width joiner inside an emoji sequence is content (D-61).
        let family = "\u{1f469}\u{200d}\u{1f467}";
        assert_eq!(trim(family), family);
    }

    #[test]
    fn keeps_the_tag_characters_that_end_a_flag() {
        let england = format!("Team {ENGLAND}");
        let scotland = format!("Team {SCOTLAND}");
        assert_eq!(trim(&england), england);
        assert_ne!(normalise(&england), normalise(&scotland));
        // The whole tag range, not only CANCEL TAG (D-61).
        for tag in ['\u{e0001}', '\u{e0020}', '\u{e0067}', '\u{e007f}'] {
            let value = format!("x{tag}");
            assert_eq!(trim(&value), value);
        }
        // A zero-width space pasted after a flag goes; the flag's tags stay.
        assert_eq!(trim(&format!("{ENGLAND}\u{200b}")), ENGLAND);
    }

    #[test]
    fn keeps_variation_selectors_keycaps_and_combining_marks() {
        // Mn and Me, not Cf: content, even though VS16 is default-ignorable.
        for kept in ["\u{2764}\u{fe0f}", "1\u{fe0f}\u{20e3}", "e\u{301}"] {
            assert_eq!(trim(kept), kept);
        }
    }

    #[test]
    fn a_value_of_only_format_characters_is_empty() {
        assert_eq!(trim("\u{200b}\u{feff}\u{2060}"), "");
    }

    #[test]
    fn normalise_folds_after_trimming() {
        assert_eq!(normalise("\u{feff} Name\u{200b}"), "name");
    }
}
