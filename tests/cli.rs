//! CLI surface (§6.6). Parsed via clap's `try_parse_from`, so the tests assert
//! the contract rather than spawning a process.

use clap::Parser;
use radargen::cli::{Cli, FatalError, OutputFormat, Theme};

fn parse(args: &[&str]) -> Result<Cli, clap::Error> {
    Cli::try_parse_from(std::iter::once("radargen").chain(args.iter().copied()))
}

#[test]
fn no_arguments_is_a_usage_error() {
    // FR-60: invoked with no arguments, the tool prints usage and exits non-zero.
    assert!(parse(&[]).is_err());
}

#[test]
fn a_csv_path_and_an_output_are_enough() {
    let cli = parse(&["entries.csv", "-o", "edition.html"]).expect("parses");
    assert_eq!(cli.csv_path.to_str(), Some("entries.csv"));
    assert_eq!(cli.output.to_str(), Some("edition.html"));
}

#[test]
fn output_is_required() {
    // FR-60/§6.6: -o carries no default.
    assert!(parse(&["entries.csv"]).is_err());
}

#[test]
fn exactly_one_csv_is_accepted() {
    // FR-60: no glob, no multi-file form.
    assert!(parse(&["a.csv", "b.csv", "-o", "x.html"]).is_err());
}

// ---- FR-52: format inferred from the -o extension, matched case-insensitively

#[test]
fn html_is_inferred_from_the_output_extension() {
    let cli = parse(&["in.csv", "-o", "e.html"]).expect("parses");
    assert_eq!(cli.format().expect("recognised"), OutputFormat::Html);
}

#[test]
fn extension_matching_ignores_case() {
    // FR-52 extends this courtesy explicitly; `.HTML` is the v1.0 case.
    let cli = parse(&["in.csv", "-o", "e.HTML"]).expect("parses");
    assert_eq!(cli.format().expect("recognised"), OutputFormat::Html);
}

#[test]
fn png_and_svg_say_not_in_this_version_rather_than_unrecognised() {
    // §14.1: v1.0 ships HTML only, and UC-1's documented workflow is one HTML
    // run and one PNG run — so this is a path a maintainer takes on purpose.
    // "Unrecognised extension" would send them hunting for a typo in a
    // correct path.
    for ext in ["png", "svg", "PNG"] {
        let cli = parse(&["in.csv", "-o", &format!("e.{ext}")]).expect("parses");
        match cli.format() {
            Err(FatalError::FormatNotInThisVersion { arrives_in, .. }) => {
                assert_eq!(arrives_in, "1.1")
            }
            other => panic!("{ext} gave {other:?}"),
        }
    }
}

#[test]
fn an_unrecognised_or_absent_extension_is_a_different_error() {
    for path in ["e.jpg", "e", "e."] {
        let cli = parse(&["in.csv", "-o", path]).expect("parses");
        assert!(
            matches!(cli.format(), Err(FatalError::UnrecognisedExtension { .. })),
            "{path} should be unrecognised, not deferred"
        );
    }
}

// ---- FR-64: --width accepts 320..=10000 inclusive

#[test]
fn width_accepts_its_documented_range() {
    for w in ["320", "1920", "10000"] {
        assert!(parse(&["in.csv", "-o", "e.png", "--width", w]).is_ok(), "{w}");
    }
}

#[test]
fn width_outside_the_range_is_a_usage_error() {
    for w in ["319", "10001", "0"] {
        assert!(parse(&["in.csv", "-o", "e.png", "--width", w]).is_err(), "{w}");
    }
}

// ---- FR-56: --theme, case-insensitive, default light

#[test]
fn theme_defaults_to_light_and_ignores_case() {
    let cli = parse(&["in.csv", "-o", "e.png"]).expect("parses");
    assert_eq!(cli.theme, Theme::Light);
    let cli = parse(&["in.csv", "-o", "e.png", "--theme", "Dark"]).expect("parses");
    assert_eq!(cli.theme, Theme::Dark);
}

#[test]
fn an_unknown_theme_is_a_usage_error() {
    assert!(parse(&["in.csv", "-o", "e.png", "--theme", "sepia"]).is_err());
}

// ---- OQ-2: short forms where practical. Settled at implementation planning,
// which §6.6 says is where this belongs.

#[test]
fn every_flag_has_a_short_form() {
    let cli = parse(&[
        "in.csv", "-o", "e.html", "-r", "Adopt,Trial", "-t", "Radar", "-s", "2026-09",
    ])
    .expect("short forms parse");
    assert_eq!(cli.output.to_str(), Some("e.html"));
    assert_eq!(cli.rings.as_deref(), Some("Adopt,Trial"));
    assert_eq!(cli.title.as_deref(), Some("Radar"));
    assert_eq!(cli.subtitle.as_deref(), Some("2026-09"));
}

#[test]
fn theme_takes_capital_t_and_title_takes_lowercase() {
    // Both appear together in §6.6's canonical PNG invocation, so both earn a
    // short form and one of them has to be the capital.
    let cli = parse(&["in.csv", "-o", "e.png", "-t", "Radar", "-T", "dark", "-w", "1920"])
        .expect("parses");
    assert_eq!(cli.title.as_deref(), Some("Radar"));
    assert_eq!(cli.theme, Theme::Dark);
    assert_eq!(cli.width, Some(1920));
}
