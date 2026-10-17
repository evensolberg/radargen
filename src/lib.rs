//! radargen — CSV to technology radar.
//!
//! See `docs/product/radargen-prd.md`. Requirement IDs in comments refer to it.

pub mod cli;
pub mod columns;
pub mod csv;
pub mod decode;
pub mod diagnostics;
pub mod emit;
pub mod errors;
pub mod escape;
pub mod layout;
pub mod link;
pub mod model;
pub mod num;
pub mod order;
pub mod palette;
pub mod pipeline;
pub mod rings;
pub mod style;
pub mod svg;
pub mod template;
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
/// The values diagnostics quote use this too, so no site trims differently;
/// a label that is shown is [`tidy`], which starts with this.
#[must_use]
pub fn trim(value: &str) -> &str {
    value.trim_matches(trimmable)
}

/// [`trim`], then every run of whitespace inside the value as one space.
///
/// That is how a wrapped line joins its words ([`text::words`]), so a label
/// reads the same wrapped or not (rg-cuj). It is the display form of
/// FR-76's three trimmed values, and the first step of [`normalise`].
#[must_use]
pub fn tidy(value: &str) -> String {
    text::words(trim(value))
}

/// `value` with its line breaks as plain newlines: CR LF and a lone CR each
/// become LF (FR-97). A quoted CSV cell can hold Excel's Alt+Enter break.
#[must_use]
pub fn plain_newlines(value: &str) -> String {
    value.replace("\r\n", "\n").replace('\r', "\n")
}

/// `value` as one line: every run of line breaks becomes one space (FR-97),
/// for a field that is a label rather than prose.
#[must_use]
pub fn single_line(value: &str) -> String {
    value
        .split(['\r', '\n'])
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Whitespace, or a format character that is not a tag character.
fn trimmable(c: char) -> bool {
    c.is_whitespace() || is_untagged_format(c)
}

/// A format character (Cf) that is not a tag character: the one rule that
/// trimming and escaping share.
pub(crate) fn is_untagged_format(c: char) -> bool {
    c.general_category() == GeneralCategory::Format
        && !matches!(c, '\u{e0001}' | '\u{e0020}'..='\u{e007f}')
}

/// FR-76: **one normalisation rule**, applied to every value read from the CSV
/// or from `--rings`.
///
/// Both operands of a comparison get [`tidy`] and are then folded
/// case-insensitively. Folding is Unicode's default lowercase mapping —
/// `str::to_lowercase`, not full case folding and not locale-aware (D-53).
///
/// This never edits displayed text — callers compare on the result and render
/// the original.
#[must_use]
pub fn normalise(value: &str) -> String {
    tidy(value).to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// FR-97: a run of line breaks is one space; breaks at either end go;
    /// only breaks is empty; spaces beside a break are kept as written.
    #[test]
    fn single_line_collapses_runs_of_breaks() {
        assert_eq!(single_line("Kafka\r\n\r\nStreams"), "Kafka Streams");
        assert_eq!(single_line("\nKafka\r"), "Kafka");
        assert_eq!(single_line("\r\n\n"), "");
        assert_eq!(single_line("a \n b"), "a   b");
        assert_eq!(single_line("plain"), "plain");
    }

    /// FR-97: CR LF and a lone CR each become one LF; LF stays.
    #[test]
    fn plain_newlines_makes_every_break_a_newline() {
        assert_eq!(plain_newlines("a\r\nb\rc\nd"), "a\nb\nc\nd");
        assert_eq!(plain_newlines("\r\r\n"), "\n\n");
    }

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

    /// rg-cuj: a run of whitespace inside a value is one space, the way a
    /// wrapped line joins its words, so a label reads the same wrapped or
    /// not. A format character inside is content, as for [`trim`].
    #[test]
    fn tidy_collapses_internal_runs_of_whitespace() {
        assert_eq!(tidy("\u{200b} Tool  \t Kit\u{a0} x "), "Tool Kit x");
        assert_eq!(tidy("a\u{200d}b"), "a\u{200d}b");
        assert_eq!(tidy(" \u{feff} "), "");
    }

    #[test]
    fn normalise_collapses_internal_runs_too() {
        assert_eq!(normalise("Tool  Kit"), normalise("tool kit"));
    }
}
