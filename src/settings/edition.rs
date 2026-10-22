//! FR-92, D-58: `-e/--edition <YYYY-MM-DD>`, and the `{edition}`
//! placeholder it fills in `output`, `title` and `subtitle`.
//!
//! The date comes from the author (D-24), so re-rendering an old edition
//! with its own date reproduces it byte for byte (NFR-4). The one
//! exception is `-e today`, the maintainer's opt-in for scheduled jobs
//! (D-58): the local date, read once through the clock the caller injects
//! and echoed, so the render can still be reproduced with an explicit `-e`.

use crate::date::is_iso_date;

/// The one placeholder FR-92 defines, braces and all.
pub const PLACEHOLDER: &str = "{edition}";

/// Its name, as [`placeholders`] finds it.
const NAME: &str = "edition";

/// The word that asks for the local date (D-58), matched exactly.
pub const TODAY: &str = "today";

/// What `--edition` was given (FR-92).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Edition {
    /// A date, `YYYY-MM-DD`, exactly as written.
    Date(String),
    /// `today`: the local date, which only the run's clock knows.
    Today,
}

/// clap's value parser for `--edition`: `today`, or a date written
/// `YYYY-MM-DD` exactly, a real calendar date (FR-24's shape). Anything
/// else is clap's usage error, in its words.
///
/// # Errors
///
/// The reason clap prints after the value.
pub fn parse(text: &str) -> Result<Edition, String> {
    if text == TODAY {
        Ok(Edition::Today)
    } else if is_iso_date(text) {
        Ok(Edition::Date(text.to_owned()))
    } else {
        Err("not a calendar date written YYYY-MM-DD, or today".to_owned())
    }
}

/// The machine's local date, `YYYY-MM-DD`, in the time zone `TZ` names or
/// else the system's: the production clock `-e today` reads (D-58). The
/// one place radargen reads the clock.
#[must_use]
pub fn local_today() -> String {
    jiff::Zoned::now().date().to_string()
}

/// What is wrong with one value's placeholders (FR-92).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Problem {
    /// It holds `{edition}`, and no `--edition` was given.
    NoEdition,
    /// Placeholders are in use, and it holds this other one, braces and
    /// all.
    Unknown(String),
}

/// `value` with `{edition}` expanded to `edition`, and whether it held one.
///
/// Placeholders are in use when `edition` is given or `value` holds
/// `{edition}`; only then is another placeholder an error, so a value
/// such as `Radar {beta}` without `--edition` stays as written (NFR-12).
///
/// # Errors
///
/// Every [`Problem`] in `value`, in order: [`Problem::NoEdition`] first,
/// then each unknown placeholder where it appears.
pub fn expand(value: &str, edition: Option<&str>) -> Result<(String, bool), Vec<Problem>> {
    let names = placeholders(value);
    let uses = names.contains(&NAME);
    if edition.is_none() && !uses {
        return Ok((value.to_owned(), false));
    }
    let missing = (uses && edition.is_none()).then_some(Problem::NoEdition);
    let problems: Vec<Problem> = missing
        .into_iter()
        .chain(
            names
                .into_iter()
                .filter(|name| *name != NAME)
                .map(|name| Problem::Unknown(format!("{{{name}}}"))),
        )
        .collect();
    match edition {
        Some(date) if problems.is_empty() => Ok((value.replace(PLACEHOLDER, date), uses)),
        _ => Err(problems),
    }
}

/// The name of every placeholder in `value`, in order, without its
/// braces. A placeholder is `{`, an ASCII letter, any ASCII letters,
/// digits or underscores, then `}`; any other brace is literal text.
fn placeholders(value: &str) -> Vec<&str> {
    let mut names = Vec::new();
    let mut rest = value;
    while let Some((_, after)) = rest.split_once('{') {
        let lettered = after
            .bytes()
            .next()
            .is_some_and(|b| b.is_ascii_alphabetic());
        let length = after
            .bytes()
            .take_while(|b| b.is_ascii_alphanumeric() || *b == b'_')
            .count();
        // The run is ASCII, so `length` falls on a character boundary.
        let closed = after
            .split_at_checked(length)
            .and_then(|(name, tail)| Some((name, tail.strip_prefix('}')?)));
        match closed {
            Some((name, tail)) if lettered => {
                names.push(name);
                rest = tail;
            }
            // Not a placeholder: this brace is literal, and one may start
            // at the next.
            _ => rest = after,
        }
    }
    names
}

#[cfg(test)]
mod tests {
    use super::*;

    fn found(value: &str) -> Vec<String> {
        placeholders(value)
            .into_iter()
            .map(|name| format!("{{{name}}}"))
            .collect()
    }

    #[test]
    fn a_placeholder_is_a_letter_then_letters_digits_or_underscores() {
        assert_eq!(found("{edition}"), ["{edition}"]);
        assert_eq!(found("a{x}b{X_1}c{e9}"), ["{x}", "{X_1}", "{e9}"]);
        for literal in [
            "{}", "{1x}", "{_x}", "{a-b}", "{a b}", "{é}", "{a", "a}", "{ a}",
        ] {
            assert_eq!(found(literal), Vec::<String>::new(), "{literal}");
        }
        // A stray brace before one does not hide it, nor one after.
        assert_eq!(found("{{edition}}"), ["{edition}"]);
        assert_eq!(found("{a{b}"), ["{b}"]);
        assert_eq!(found("é{edition}é"), ["{edition}"]);
    }

    #[test]
    fn edition_expands_everywhere_and_reports_its_use() {
        assert_eq!(
            expand("{edition}/r-{edition}.html", Some("2026-09-01")),
            Ok(("2026-09-01/r-2026-09-01.html".to_owned(), true))
        );
        assert_eq!(
            expand("Radar", Some("2026-09-01")),
            Ok(("Radar".to_owned(), false))
        );
    }

    #[test]
    fn placeholders_matched_case_sensitively() {
        assert_eq!(
            expand("{Edition}", Some("2026-09-01")),
            Err(vec![Problem::Unknown("{Edition}".to_owned())])
        );
    }

    /// FR-92's "in use": `--edition` given, or `{edition}` in the value.
    #[test]
    fn unknown_placeholders_fail_only_when_in_use() {
        assert_eq!(
            expand("Radar {beta}", None),
            Ok(("Radar {beta}".to_owned(), false))
        );
        assert_eq!(
            expand("Radar {beta}", Some("2026-09-01")),
            Err(vec![Problem::Unknown("{beta}".to_owned())])
        );
        assert_eq!(
            expand("{beta} {edition} {beta}", None),
            Err(vec![
                Problem::NoEdition,
                Problem::Unknown("{beta}".to_owned()),
                Problem::Unknown("{beta}".to_owned()),
            ])
        );
        assert_eq!(expand("{edition}", None), Err(vec![Problem::NoEdition]));
    }

    /// FR-92, D-58: a real date exactly as written, or `today` exactly.
    #[test]
    fn an_edition_is_a_date_written_exactly_or_today() {
        assert_eq!(
            parse("2024-02-29"),
            Ok(Edition::Date("2024-02-29".to_owned()))
        );
        assert_eq!(parse("today"), Ok(Edition::Today));
        for bad in [
            "2026-02-30",
            "2026-2-1",
            "2026-09-01T00:00",
            "",
            " 2026-09-01",
            "Today",
            "TODAY",
            "yesterday",
            " today",
        ] {
            assert_eq!(
                parse(bad),
                Err("not a calendar date written YYYY-MM-DD, or today".to_owned()),
                "{bad:?}"
            );
        }
    }

    /// The production clock gives FR-92's shape; its value is not
    /// asserted, so no test depends on the real date.
    #[test]
    fn the_local_date_is_a_date() {
        assert!(is_iso_date(&local_today()));
    }
}
