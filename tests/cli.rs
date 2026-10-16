//! CLI surface (§6.6). Parsed via clap's `try_parse_from`, so the tests assert
//! the contract rather than spawning a process.

use clap::Parser;
use radargen::cli::{Cli, OutputFormat, Theme};
use radargen::cli::{Task, clean_flag, format};
use radargen::diagnostics::{Fatal, Flag, RowKind, Warning, Warnings};
use radargen::rings::{RingsError, parse_rings};
use std::path::Path;

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
    assert_eq!(
        cli.csv_path.as_deref().and_then(|p| p.to_str()),
        Some("entries.csv")
    );
    assert_eq!(
        cli.output.as_deref().and_then(|p| p.to_str()),
        Some("edition.html")
    );
    assert!(matches!(cli.task(), Ok(Task::Render { .. })));
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
    assert_eq!(
        format(cli.output.as_deref().expect("-o")).expect("recognised"),
        OutputFormat::Html
    );
}

#[test]
fn extension_matching_ignores_case() {
    // FR-52 extends this courtesy explicitly; `.HTML` is the v1.0 case.
    let cli = parse(&["in.csv", "-o", "e.HTML"]).expect("parses");
    assert_eq!(
        format(cli.output.as_deref().expect("-o")).expect("recognised"),
        OutputFormat::Html
    );
}

#[test]
fn png_and_svg_are_formats() {
    for path in ["r.png", "R.PNG", "r.Png"] {
        assert!(
            matches!(format(Path::new(path)), Ok(OutputFormat::Png)),
            "{path}"
        );
    }
    for path in ["e.svg", "E.SVG", "e.Svg"] {
        assert!(
            matches!(format(Path::new(path)), Ok(OutputFormat::Svg)),
            "{path}"
        );
    }
}

#[test]
fn an_unrecognised_or_absent_extension_is_a_different_error() {
    for path in ["e.jpg", "e", "e."] {
        let cli = parse(&["in.csv", "-o", path]).expect("parses");
        match format(cli.output.as_deref().expect("-o")) {
            Err(Fatal::UnrecognisedExtension { path: named }) => assert_eq!(named, path),
            other => panic!("{path} should be unrecognised: {other:?}"),
        }
    }
}

// ---- FR-64: --width accepts 320..=10000 inclusive

#[test]
fn width_accepts_its_documented_range() {
    for w in ["320", "1920", "10000"] {
        assert!(
            parse(&["in.csv", "-o", "e.png", "--width", w]).is_ok(),
            "{w}"
        );
    }
}

#[test]
fn width_outside_the_range_is_a_usage_error() {
    for w in ["319", "10001", "0", "abc", "-5"] {
        assert!(
            parse(&["in.csv", "-o", "e.png", "--width", w]).is_err(),
            "{w}"
        );
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
fn every_render_flag_has_a_short_form() {
    let cli = parse(&[
        "in.csv",
        "-o",
        "e.html",
        "-r",
        "Adopt,Trial",
        "-t",
        "Radar",
        "-s",
        "2026-09",
    ])
    .expect("short forms parse");
    assert_eq!(
        cli.output.as_deref().and_then(|p| p.to_str()),
        Some("e.html")
    );
    assert_eq!(cli.rings.as_deref(), Some("Adopt,Trial"));
    assert_eq!(cli.title.as_deref(), Some("Radar"));
    assert_eq!(cli.subtitle.as_deref(), Some("2026-09"));
}

#[test]
fn theme_takes_capital_t_and_title_takes_lowercase() {
    // Both appear together in §6.6's canonical PNG invocation, so both earn a
    // short form and one of them has to be the capital.
    let cli = parse(&[
        "in.csv", "-o", "e.png", "-t", "Radar", "-T", "dark", "-w", "1920",
    ])
    .expect("parses");
    assert_eq!(cli.title.as_deref(), Some("Radar"));
    assert_eq!(cli.theme, Theme::Dark);
    assert_eq!(cli.width, Some(1920));
}

#[test]
fn each_flag_names_itself_in_the_one_character_warning() {
    for (flag, name) in [
        (Flag::Title, "--title"),
        (Flag::Subtitle, "--subtitle"),
        (Flag::Rings, "--rings"),
    ] {
        let mut warnings = Warnings::default();
        assert_eq!(clean_flag(flag, "a\u{1}b", &mut warnings), "ab");
        assert_eq!(
            warnings.render(),
            [format!(
                "warning: {name}: removed U+0001, which the output cannot display"
            )]
        );
    }
}

#[test]
fn each_flag_names_itself_in_the_several_characters_warning() {
    for (flag, name) in [
        (Flag::Title, "--title"),
        (Flag::Subtitle, "--subtitle"),
        (Flag::Rings, "--rings"),
    ] {
        let mut warnings = Warnings::default();
        assert_eq!(clean_flag(flag, "\u{1}a\u{7f}b\u{85}", &mut warnings), "ab");
        assert_eq!(
            warnings.render(),
            [format!(
                "warning: {name}: removed 3 characters the output cannot display, the first U+0001"
            )]
        );
    }
}

#[test]
fn a_clean_value_is_unchanged_and_silent() {
    let mut warnings = Warnings::default();
    let value = "Tab\there, Ærø 日本\nline";
    assert_eq!(clean_flag(Flag::Title, value, &mut warnings), value);
    assert!(warnings.is_empty());
}

#[test]
fn a_title_of_only_undisplayable_characters_cleans_to_empty() {
    let mut warnings = Warnings::default();
    assert_eq!(clean_flag(Flag::Title, "\u{1}", &mut warnings), "");
    assert_eq!(warnings.as_slice().len(), 1);
}

#[test]
fn rings_are_cleaned_before_fr_63_checks_them() {
    let mut warnings = Warnings::default();
    let cleaned = clean_flag(Flag::Rings, "Adopt,\u{1},Hold", &mut warnings);
    assert_eq!(
        parse_rings(Some(&cleaned)),
        Err(vec![RingsError::EmptyEntry { index: 2 }])
    );
}

#[test]
fn a_flag_warning_renders_before_row_warnings() {
    let mut warnings = Warnings::default();
    warnings.push(Warning::Row {
        line: 2,
        column: 0,
        kind: RowKind::EmptyDescription,
    });
    clean_flag(Flag::Subtitle, "x\u{1}", &mut warnings);
    let lines = warnings.render();
    assert_eq!(lines.len(), 2, "{lines:?}");
    assert!(
        lines
            .first()
            .is_some_and(|l| l.starts_with("warning: --subtitle: removed")),
        "{lines:?}"
    );
}

/// D-72: a flag value follows FR-90's rule for bidi formatting characters,
/// as a CSV cell does.
#[test]
fn a_bidi_override_is_removed_from_a_title() {
    let mut warnings = Warnings::default();
    assert_eq!(
        clean_flag(Flag::Title, "Radar\u{202e}2026", &mut warnings),
        "Radar2026"
    );
    assert_eq!(warnings.as_slice().len(), 1);
}

/// `--help` is for the person running the tool: no requirement numbers,
/// PRD sections or open questions, which belong in the code's comments.
#[test]
fn help_speaks_to_users() {
    use clap::CommandFactory;
    let help = Cli::command().render_long_help().to_string();
    assert!(!help.contains('§'), "a PRD section in --help:\n{help}");
    // A reference is a capital-letter prefix, a hyphen and a digit: FR-5,
    // NFR-13, OQ-2, D-28, A-4.
    for (at, _) in help.match_indices('-') {
        let before = help.get(..at).unwrap_or_default();
        let prefix = before.len()
            - before
                .trim_end_matches(|c: char| c.is_ascii_uppercase())
                .len();
        let digit = help.get(at + 1..).and_then(|rest| rest.chars().next());
        assert!(
            prefix == 0 || !digit.is_some_and(|c| c.is_ascii_digit()),
            "a reference in --help near {:?}:\n{help}",
            help.get(at - prefix..at + 2)
        );
    }
    let about = help.lines().next().unwrap_or_default();
    assert!(
        about.contains("PNG") && about.contains("SVG"),
        "the about line promises only what this version writes:\n{help}"
    );
}

/// `--rings` names the rings used without it, from the same list the tool
/// uses, so the two cannot drift apart.
#[test]
fn rings_help_names_the_default_rings() {
    use clap::CommandFactory;
    let help = Cli::command().render_long_help().to_string();
    let defaults = radargen::rings::DEFAULT_RINGS.join(", ");
    assert!(
        help.contains(&defaults),
        "--help must name {defaults}:\n{help}"
    );
}

/// clap's derive cannot put a constant in a doc comment, so the help's
/// range is written out; this keeps it FR-64's.
#[test]
fn width_help_names_the_range() {
    use clap::CommandFactory;
    use radargen::style::{MAX_EXPORT_WIDTH, MIN_EXPORT_WIDTH};
    let help = Cli::command().render_long_help().to_string();
    let range = format!("From {MIN_EXPORT_WIDTH} to {MAX_EXPORT_WIDTH}");
    assert!(help.contains(&range), "--help must say {range}:\n{help}");
}

/// `-h` is the short form: clap takes each comment's first paragraph, so
/// each flag gets one sentence there, under 120 columns with clap's
/// defaults, and the rest is left to `--help`.
#[test]
fn short_help_is_one_line_per_flag() {
    use clap::CommandFactory;
    let help = Cli::command().render_help().to_string();
    for line in help.lines() {
        assert!(line.chars().count() <= 120, "a long -h line:\n{line}");
    }
}

/// The help says where case does not matter, so no one retypes `.HTML`.
#[test]
fn help_says_case_does_not_matter() {
    use clap::CommandFactory;
    let help = Cli::command().render_long_help().to_string();
    assert!(help.contains("any case"), "--help:\n{help}");
}

// ---- FR-99: --create-template, given alone

#[test]
fn create_template_stands_alone() {
    let cli = parse(&["--create-template", "radar.csv"]).expect("parses");
    assert_eq!(
        cli.create_template.as_deref().and_then(|p| p.to_str()),
        Some("radar.csv")
    );
    assert!(cli.csv_path.is_none() && cli.output.is_none());
    assert!(matches!(cli.task(), Ok(Task::Template(_))));
}

#[test]
fn create_template_with_anything_else_is_a_usage_error() {
    for extra in [
        &["in.csv"][..],
        &["-o", "e.html"],
        &["in.csv", "-o", "e.html"],
        &["-t", "Radar"],
        &["-T", "dark"],
        // The default value, given explicitly, is still given.
        &["-T", "light"],
        &["-r", "Adopt"],
        &["-s", "2026-09"],
        &["-w", "1920"],
    ] {
        let args: Vec<&str> = ["--create-template", "radar.csv"]
            .into_iter()
            .chain(extra.iter().copied())
            .collect();
        let kind = parse(&args).map(|_| ()).map_err(|e| e.kind());
        assert_eq!(
            kind,
            Err(clap::error::ErrorKind::ArgumentConflict),
            "{args:?}"
        );
    }
}

#[test]
fn a_render_still_needs_both_paths() {
    assert!(parse(&["-o", "e.html"]).is_err());
    assert!(parse(&["in.csv"]).is_err());
}

/// The usage names both forms; clap's own would show the CSV as optional
/// and leave out -o.
#[test]
fn usage_names_both_forms() {
    use clap::CommandFactory;
    let help = Cli::command().render_help().to_string();
    assert!(help.contains("radargen <CSV_PATH> -o <OUTPUT>"), "{help}");
    assert!(help.contains("radargen --create-template <PATH>"), "{help}");
}

/// A `Cli` built by hand, which clap would have refused, asking for
/// neither task or for both.
#[test]
fn a_cli_with_neither_task_or_both_is_an_error() {
    let mut cli = parse(&["in.csv", "-o", "e.html"]).expect("parses");
    cli.create_template = Some("radar.csv".into());
    assert!(matches!(cli.task(), Err(Fatal::NoTask)));
    cli.create_template = None;
    cli.output = None;
    assert!(matches!(cli.task(), Err(Fatal::NoTask)));
}
