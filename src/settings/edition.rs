//! FR-92, D-58: `-e/--edition <YYYY-MM-DD>`, and the `{edition}`
//! placeholder it fills in `output`, `title` and `subtitle`.
//!
//! The date comes from the author (D-24), so re-rendering an old edition
//! with its own date reproduces it byte for byte (NFR-4). The one
//! exception is `-e today`, the maintainer's opt-in for scheduled jobs
//! (D-58): the local date, read once through the clock the caller injects
//! and echoed, so the render can still be reproduced with an explicit `-e`.

use std::ffi::OsStr;

use jiff::Timestamp;
use jiff::tz::TimeZone;

use crate::date::is_iso_date;

/// The one placeholder FR-92 defines, braces and all.
pub const PLACEHOLDER: &str = "{edition}";

/// Its name, as [`placeholders`] finds it.
const NAME: &str = "edition";

/// The word that asks for the local date (D-58), matched exactly.
const TODAY: &str = "today";

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

/// What `-e today` read (D-58): the date, `YYYY-MM-DD`, and the zone it
/// is the date in, which the run echoes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Today {
    /// The local date.
    pub date: String,
    /// The zone, as the echo names it: its IANA name, else `TZ` as
    /// written, else `local time`; `UTC` after a fallback.
    pub zone: String,
    /// Why UTC was used instead of the local zone, if it was.
    pub fallback: Option<Fallback>,
}

/// Why `-e today` used UTC (review M-1): jiff alone would fall back
/// silently, so a misspelt `TZ` in a scheduled job would date the edition
/// in UTC without a word.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fallback {
    /// `TZ` is set, as written, and names no zone jiff can find.
    BadTz(String),
    /// `TZ` is not set, and the system's zone cannot be determined.
    NoSystemZone,
}

/// The machine's local date: the production clock `-e today` reads
/// (D-58), in the zone `TZ` names, else the system's, else UTC with a
/// [`Fallback`] saying why.
#[must_use]
#[expect(
    clippy::disallowed_methods,
    reason = "D-58: -e today's one clock read; nothing else may read the clock (D-24)"
)]
pub fn local_today() -> Today {
    today_in(
        TimeZone::try_system(),
        std::env::var_os("TZ").as_deref(),
        Timestamp::now(),
    )
}

/// [`local_today`] for a given zone lookup, `TZ` and instant, so the
/// fallback is testable without the clock or the environment.
fn today_in(zone: Result<TimeZone, jiff::Error>, tz: Option<&OsStr>, now: Timestamp) -> Today {
    let (zone, fallback) = zone.map_or_else(
        |_| {
            let why = tz.map_or(Fallback::NoSystemZone, |tz| {
                Fallback::BadTz(tz.to_string_lossy().into_owned())
            });
            (TimeZone::UTC, Some(why))
        },
        |zone| (zone, None),
    );
    let name = zone.iana_name().map_or_else(
        || {
            tz.map_or_else(
                || "local time".to_owned(),
                |tz| tz.to_string_lossy().into_owned(),
            )
        },
        str::to_owned,
    );
    Today {
        date: now.to_zoned(zone).date().to_string(),
        zone: name,
        fallback,
    }
}

/// Whether `value` holds `{edition}`: a use of `--edition`, whether or not
/// the value then expands (review I-1).
#[must_use]
pub fn uses(value: &str) -> bool {
    placeholders(value).contains(&NAME)
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

/// `value` with `{edition}` expanded to `edition`.
///
/// Placeholders are in use when `edition` is given or `value` holds
/// `{edition}`; only then is another placeholder an error, so a value
/// such as `Radar {beta}` without `--edition` stays as written (NFR-12).
///
/// # Errors
///
/// Every [`Problem`] in `value`, in order: [`Problem::NoEdition`] first,
/// then each unknown placeholder where it appears.
pub fn expand(value: &str, edition: Option<&str>) -> Result<String, Vec<Problem>> {
    let names = placeholders(value);
    let uses = names.contains(&NAME);
    if edition.is_none() && !uses {
        return Ok(value.to_owned());
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
        Some(date) if problems.is_empty() => Ok(value.replace(PLACEHOLDER, date)),
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
            Ok("2026-09-01/r-2026-09-01.html".to_owned())
        );
        assert_eq!(expand("Radar", Some("2026-09-01")), Ok("Radar".to_owned()));
        assert!(uses("{edition}-{editon}") && !uses("{Edition} {editon}"));
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
        assert_eq!(expand("Radar {beta}", None), Ok("Radar {beta}".to_owned()));
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
        assert!(is_iso_date(&local_today().date), "FR-92's shape");
    }

    /// Review M-1: the zone is named, and a lookup that fails gives UTC
    /// with the reason, never silently.
    #[test]
    fn today_names_its_zone_or_why_it_used_utc() {
        // 2026-10-23T03:00Z: still the 22nd in Vancouver.
        let now: Timestamp = "2026-10-23T03:00:00Z".parse().expect("an instant");
        let tz = |name: &str| TimeZone::get(name);
        assert_eq!(
            today_in(
                tz("America/Vancouver"),
                Some(OsStr::new("America/Vancouver")),
                now
            ),
            Today {
                date: "2026-10-22".into(),
                zone: "America/Vancouver".into(),
                fallback: None,
            }
        );
        assert_eq!(
            today_in(
                tz("America/Vancuover"),
                Some(OsStr::new("America/Vancuover")),
                now
            ),
            Today {
                date: "2026-10-23".into(),
                zone: "UTC".into(),
                fallback: Some(Fallback::BadTz("America/Vancuover".into())),
            }
        );
        assert_eq!(
            today_in(tz("Nowhere/Else"), None, now).fallback,
            Some(Fallback::NoSystemZone)
        );
        let posix = TimeZone::posix("EST5EDT,M3.2.0,M11.1.0").expect("a POSIX zone");
        assert_eq!(
            today_in(Ok(posix), Some(OsStr::new("EST5EDT,M3.2.0,M11.1.0")), now).zone,
            "EST5EDT,M3.2.0,M11.1.0"
        );
    }
}
