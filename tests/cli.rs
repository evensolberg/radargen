//! CLI surface (§6.6). Parsed via clap's `try_parse_from`, so the tests assert
//! the contract rather than spawning a process.

#[path = "common/source.rs"]
mod source;

use clap::Parser;
use radargen::cli::{Cli, Task, Theme, clean_setting};
use radargen::diagnostics::{Fatal, RowKind, Warning, Warnings};
use radargen::options::{Legend, OutputFormat, Table};
use radargen::rings::{RingsError, parse_ring_list};
use radargen::settings::file::load;
use radargen::settings::{Effective, Origin, Setting, Source, Sourced, resolve};
use radargen::starter::Starter;
use std::path::{Path, PathBuf};

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
        OutputFormat::from_path(cli.output.as_deref().expect("-o")).expect("recognised"),
        OutputFormat::Html
    );
}

#[test]
fn extension_matching_ignores_case() {
    // FR-52 extends this courtesy explicitly; `.HTML` is the v1.0 case.
    let cli = parse(&["in.csv", "-o", "e.HTML"]).expect("parses");
    assert_eq!(
        OutputFormat::from_path(cli.output.as_deref().expect("-o")).expect("recognised"),
        OutputFormat::Html
    );
}

#[test]
fn png_and_svg_are_formats() {
    for path in ["r.png", "R.PNG", "r.Png"] {
        assert!(
            matches!(
                OutputFormat::from_path(Path::new(path)),
                Some(OutputFormat::Png)
            ),
            "{path}"
        );
    }
    for path in ["e.svg", "E.SVG", "e.Svg"] {
        assert!(
            matches!(
                OutputFormat::from_path(Path::new(path)),
                Some(OutputFormat::Svg)
            ),
            "{path}"
        );
    }
}

/// RG-102 and FR-104 apply to the static snapshots, and a message names
/// the format as an author writes it.
#[test]
fn png_and_svg_are_snapshots_named_in_capitals() {
    let cases = [
        (OutputFormat::Html, false, "HTML"),
        (OutputFormat::Png, true, "PNG"),
        (OutputFormat::Svg, true, "SVG"),
        (OutputFormat::Pdf, true, "PDF"),
    ];
    for (format, snapshot, name) in cases {
        assert_eq!(format.is_snapshot(), snapshot, "{name}");
        assert_eq!(format.to_string(), name);
    }
}

#[test]
fn an_unrecognised_or_absent_extension_is_a_different_error() {
    for path in ["e.jpg", "e", "e."] {
        let cli = parse(&["in.csv", "-o", path]).expect("parses");
        assert_eq!(
            OutputFormat::from_path(cli.output.as_deref().expect("-o")),
            None,
            "{path}"
        );
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

// ---- FR-100: --legend-columns accepts 1..=4, default 1

#[test]
fn legend_columns_accepts_one_to_four() {
    assert_eq!(
        parse(&["in.csv", "-o", "e.png"])
            .expect("parses")
            .legend_columns,
        None
    );
    for n in ["1", "4"] {
        let cli = parse(&["in.csv", "-o", "e.png", "--legend-columns", n]).expect(n);
        assert_eq!(
            cli.legend_columns.map(|n| n.to_string()).as_deref(),
            Some(n)
        );
    }
}

#[test]
fn legend_columns_outside_the_range_is_a_usage_error() {
    for n in ["0", "5", "-1", "two"] {
        assert!(
            parse(&["in.csv", "-o", "e.png", "--legend-columns", n]).is_err(),
            "{n}"
        );
    }
}

// ---- FR-56: --theme, case-insensitive, default light

#[test]
fn theme_ignores_case() {
    // Omitted, it is None: `an_omitted_flag_is_none_so_a_config_key_can_fill_it`.
    let cli = parse(&["in.csv", "-o", "e.png", "--theme", "Dark"]).expect("parses");
    assert_eq!(cli.theme, Some(Theme::Dark));
}

#[test]
fn an_unknown_theme_is_a_usage_error() {
    assert!(parse(&["in.csv", "-o", "e.png", "--theme", "sepia"]).is_err());
}

// ---- --legend: where the HTML page starts; show and hide as hidden aliases

#[test]
fn legend_defaults_to_shown_and_takes_aliases_in_any_case() {
    let cli = parse(&["in.csv", "-o", "e.html"]).expect("parses");
    assert_eq!(cli.legend, None);
    for (given, want) in [
        ("hidden", Legend::Hidden),
        ("Hidden", Legend::Hidden),
        ("hide", Legend::Hidden),
        ("HIDE", Legend::Hidden),
        ("shown", Legend::Shown),
        ("show", Legend::Shown),
    ] {
        let cli = parse(&["in.csv", "-o", "e.html", "--legend", given]).expect(given);
        assert_eq!(cli.legend, Some(want), "{given}");
    }
    assert!(parse(&["in.csv", "-o", "e.html", "--legend", "off"]).is_err());
    // No short form: FR-100's --legend-columns may want one.
    assert!(parse(&["in.csv", "-o", "e.html", "-l", "hidden"]).is_err());
}

#[test]
fn the_legend_help_lists_shown_and_hidden_only() {
    use clap::CommandFactory;
    let short = Cli::command().render_help().to_string();
    let line = short
        .lines()
        .find(|l| l.contains("--legend"))
        .expect("--legend in -h");
    assert!(line.contains("[possible values: shown, hidden]"), "{line}");
    let long = Cli::command().render_long_help().to_string();
    assert!(
        long.contains("- shown:") && long.contains("- hidden:"),
        "{long}"
    );
    assert!(
        !long.contains("- show:") && !long.contains("- hide:"),
        "{long}"
    );
}

// ---- --table: what the HTML page's table publishes

#[test]
fn table_defaults_to_radar_in_any_case() {
    let cli = parse(&["in.csv", "-o", "e.html"]).expect("parses");
    assert_eq!(cli.table, None, "not given");
    for (given, want) in [
        ("radar", Table::Radar),
        ("Radar", Table::Radar),
        ("all", Table::All),
        ("ALL", Table::All),
        ("All", Table::All),
    ] {
        let cli = parse(&["in.csv", "-o", "e.html", "--table", given]).expect(given);
        assert_eq!(cli.table, Some(want), "{given}");
    }
    assert!(parse(&["in.csv", "-o", "e.html", "--table", "every"]).is_err());
    assert!(parse(&["in.csv", "-o", "e.html", "--table"]).is_err());
}

#[test]
fn the_table_help_lists_radar_and_all() {
    use clap::CommandFactory;
    let short = Cli::command().render_help().to_string();
    let line = short
        .lines()
        .find(|l| l.contains("--table"))
        .expect("--table in -h");
    assert!(line.contains("the default is radar"), "{line}");
    assert!(line.contains("[possible values: radar, all]"), "{line}");
}

/// The long help says what each value publishes, and that `all` publishes
/// it to anyone with the page.
#[test]
fn the_table_long_help_says_what_each_value_publishes() {
    use clap::CommandFactory;
    let long = Cli::command().render_long_help().to_string();
    let start = long.find("--table <TABLE>").expect("--table in --help");
    let end = long
        .get(start..)
        .and_then(|rest| rest.find("\n  -w, --width"))
        .expect("the next flag")
        + start;
    let block: Vec<&str> = long
        .get(start..end)
        .unwrap_or_default()
        .lines()
        .map(radargen::trim)
        .filter(|l| !l.is_empty())
        .collect();
    assert_eq!(
        block,
        [
            "--table <TABLE>",
            "What an HTML page's table publishes; the default is radar.",
            "It also decides what the detail panels show. Accepted in any letter case. PNG, SVG and PDF have no table.",
            "Possible values:",
            "- radar: Only the rows the radar draws and the columns radargen reads",
            "- all:   Every row, including those not on the radar, and every column with a header name. Anyone with the page can read them",
        ],
        "{long}"
    );
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
    assert_eq!(cli.theme, Some(Theme::Dark));
    assert_eq!(cli.width, Some(1920));
}

#[test]
fn each_flag_names_itself_in_the_one_character_warning() {
    for (flag, name) in [
        (Setting::Title, "--title"),
        (Setting::Subtitle, "--subtitle"),
        (Setting::Rings, "--rings"),
    ] {
        let mut warnings = Warnings::default();
        assert_eq!(
            clean_setting(&Origin::flag(flag), "a\u{1}b", &mut warnings),
            "ab"
        );
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
        (Setting::Title, "--title"),
        (Setting::Subtitle, "--subtitle"),
        (Setting::Rings, "--rings"),
    ] {
        let mut warnings = Warnings::default();
        assert_eq!(
            clean_setting(&Origin::flag(flag), "\u{1}a\u{7f}b\u{85}", &mut warnings),
            "ab"
        );
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
    assert_eq!(
        clean_setting(&Origin::flag(Setting::Title), value, &mut warnings),
        value
    );
    assert!(warnings.is_empty());
}

#[test]
fn a_title_of_only_undisplayable_characters_cleans_to_empty() {
    let mut warnings = Warnings::default();
    assert_eq!(
        clean_setting(&Origin::flag(Setting::Title), "\u{1}", &mut warnings),
        ""
    );
    assert_eq!(warnings.as_slice().len(), 1);
}

#[test]
fn rings_are_cleaned_before_fr_63_checks_them() {
    let mut warnings = Warnings::default();
    let cleaned = clean_setting(
        &Origin::flag(Setting::Rings),
        "Adopt,\u{1},Hold",
        &mut warnings,
    );
    assert_eq!(
        parse_ring_list(&cleaned, Source::Flag),
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
    clean_setting(&Origin::flag(Setting::Subtitle), "x\u{1}", &mut warnings);
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
        clean_setting(
            &Origin::flag(Setting::Title),
            "Radar\u{202e}2026",
            &mut warnings
        ),
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
        about.contains("PNG") && about.contains("SVG") && about.contains("PDF"),
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
    assert!(matches!(cli.task(), Ok(Task::Create(Starter::Template, _))));
}

// ---- FR-101: --create-palette, given alone

#[test]
fn create_palette_stands_alone() {
    let cli = parse(&["--create-palette", "brand.css"]).expect("parses");
    assert_eq!(
        cli.create_palette.as_deref().and_then(|p| p.to_str()),
        Some("brand.css")
    );
    assert!(cli.csv_path.is_none() && cli.output.is_none());
    assert!(matches!(cli.task(), Ok(Task::Create(Starter::Palette, _))));
}

// ---- FR-109: --create-config, given alone

#[test]
fn create_config_stands_alone() {
    let cli = parse(&["--create-config", "radar.toml"]).expect("parses");
    assert_eq!(
        cli.create_config.as_deref().and_then(|p| p.to_str()),
        Some("radar.toml")
    );
    assert!(cli.csv_path.is_none() && cli.output.is_none());
    assert!(matches!(cli.task(), Ok(Task::Create(Starter::Config, _))));
}

/// FR-109: the help names the flag, with no short form.
#[test]
fn create_config_has_a_help_line() {
    use clap::CommandFactory;
    let help = Cli::command().render_help().to_string();
    assert!(
        help.lines()
            .any(|l| radargen::trim(l).starts_with("--create-config <PATH>")
                && l.contains("Write a starter config file for --config, and render nothing")),
        "{help}"
    );
    let command = Cli::command();
    let arg = command
        .get_arguments()
        .find(|a| a.get_id() == "create_config")
        .expect("the flag");
    assert_eq!(arg.get_short(), None);
}

#[test]
fn a_starter_flag_with_anything_else_is_a_usage_error() {
    let flags = Starter::ALL.map(Starter::flag);
    for flag in flags {
        let other = flags.into_iter().find(|&f| f != flag).expect("another");
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
            &["-p", "b.css"],
            &["-c", "radar.toml"],
            &["-e", "today"],
            &[other, "x"],
        ] {
            let args: Vec<&str> = [flag, "start"]
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
}

#[test]
fn a_render_still_needs_both_paths() {
    assert!(parse(&["-o", "e.html"]).is_err());
    assert!(parse(&["in.csv"]).is_err());
}

/// FR-91 (maintainer, 2026-10-22): with -c, the CSV may come from the
/// file's input key, so -c alone is a render; it is still FR-60's error
/// without -c.
#[test]
fn config_alone_is_a_render() {
    let cli = parse(&["-c", "radar.toml"]).expect("parses");
    assert!(matches!(cli.task(), Ok(Task::Render { input: None })));
    let cli = parse(&["in.csv", "-c", "radar.toml"]).expect("parses");
    assert!(matches!(cli.task(), Ok(Task::Render { input: Some(_) })));
}

/// The usage names every form; clap's own would show the CSV as optional
/// and leave out -o.
#[test]
fn usage_names_every_form() {
    use clap::CommandFactory;
    let help = Cli::command().render_help().to_string();
    assert!(help.contains("radargen <CSV_PATH> -o <OUTPUT>"), "{help}");
    assert!(
        help.contains("radargen [CSV_PATH] -c <FILE> [OPTIONS]"),
        "the CSV is optional with -c, whose input key can name it: {help}"
    );
    assert!(help.contains("radargen --create-template <PATH>"), "{help}");
    assert!(help.contains("radargen --create-palette <PATH>"), "{help}");
    assert!(help.contains("radargen --create-config <PATH>"), "{help}");
}

/// A `Cli` built by hand, which clap would have refused, asking for no
/// task or for several.
#[test]
fn a_cli_asking_for_no_task_or_several_is_an_error() {
    let mut cli = parse(&["in.csv", "-o", "e.html"]).expect("parses");
    cli.create_template = Some("radar.csv".into());
    assert!(matches!(cli.task(), Err(Fatal::NoTask)));
    cli.csv_path = None;
    cli.output = None;
    cli.create_palette = Some("brand.css".into());
    assert!(matches!(cli.task(), Err(Fatal::NoTask)));
    cli.create_config = Some("radar.toml".into());
    assert!(matches!(cli.task(), Err(Fatal::NoTask)));
    cli.create_template = None;
    cli.create_palette = None;
    assert!(matches!(cli.task(), Ok(Task::Create(Starter::Config, _))));
    cli.create_config = None;
    cli.csv_path = Some("in.csv".into());
    assert!(matches!(cli.task(), Err(Fatal::NoTask)));
}

// ---- FR-59: --palette, and its conflict with --theme

#[test]
fn palette_takes_a_path_and_a_short_form() {
    for flag in ["--palette", "-p"] {
        let cli = parse(&["in.csv", "-o", "e.png", flag, "brand.css"]).expect("parses");
        assert_eq!(
            cli.palette.as_deref().and_then(|p| p.to_str()),
            Some("brand.css")
        );
    }
}

#[test]
fn palette_with_theme_is_a_usage_error() {
    // An explicit `light` is still given, though it is the default (FR-59).
    for theme in ["light", "dark"] {
        for args in [
            [
                "in.csv",
                "-o",
                "e.png",
                "--palette",
                "b.css",
                "--theme",
                theme,
            ],
            [
                "in.csv",
                "-o",
                "e.png",
                "--theme",
                theme,
                "--palette",
                "b.css",
            ],
        ] {
            let kind = parse(&args).map(|_| ()).map_err(|e| e.kind());
            assert_eq!(
                kind,
                Err(clap::error::ErrorKind::ArgumentConflict),
                "{args:?}"
            );
        }
    }
}

/// The defaulted theme is not "given": --palette alone parses.
#[test]
fn palette_alone_is_not_a_conflict() {
    assert!(parse(&["in.csv", "-o", "e.png", "-p", "b.css"]).is_ok());
}

// ---- FR-107: --unplotted-rings

/// A comma list kept as written for `rings::RingSet::with_unplotted`, long
/// form only (OQ-2), and absent unless given.
#[test]
fn unplotted_rings_take_a_list_and_no_short_form() {
    let cli = parse(&[
        "in.csv",
        "-o",
        "e.html",
        "--unplotted-rings",
        "In Use, Rejected",
    ])
    .expect("parses");
    assert_eq!(cli.unplotted_rings.as_deref(), Some("In Use, Rejected"));
    let cli = parse(&["in.csv", "-o", "e.html"]).expect("parses");
    assert_eq!(cli.unplotted_rings, None);
    assert!(parse(&["in.csv", "-o", "e.html", "-u", "In Use"]).is_err());
}

#[test]
fn unplotted_rings_name_themselves_in_the_cleaning_warning() {
    let mut warnings = Warnings::default();
    assert_eq!(
        clean_setting(
            &Origin::flag(Setting::UnplottedRings),
            "In\u{1} Use",
            &mut warnings
        ),
        "In Use"
    );
    assert_eq!(
        warnings.render(),
        ["warning: --unplotted-rings: removed U+0001, which the output cannot display"]
    );
}

/// The help says what the flag does, that a ring in neither list still
/// warns, and the overlap rule; `--rings`' help points at it.
#[test]
fn the_unplotted_rings_help_says_what_still_warns() {
    use clap::CommandFactory;
    let long = Cli::command().render_long_help().to_string();
    let start = long
        .find("--unplotted-rings <RINGS>")
        .expect("--unplotted-rings in --help");
    let end = long
        .get(start..)
        .and_then(|rest| rest.find("--title <TITLE>"))
        .expect("the next flag")
        + start;
    let block: Vec<&str> = long
        .get(start..end)
        .unwrap_or_default()
        .lines()
        .map(radargen::trim)
        .filter(|l| !l.is_empty() && !l.starts_with("-t,"))
        .collect();
    assert_eq!(
        block,
        [
            "--unplotted-rings <RINGS>",
            "Rings to leave off the radar without a warning, separated by commas.",
            "For example \"In Use,Rejected\". Their rows are treated like any row whose ring is not drawn, minus the warning; a ring in neither list still warns, so typos show. Case-insensitive. A ring cannot be in both lists, nor, without --rings, be a default ring.",
        ],
        "{long}"
    );
    let rings = long.find("-r, --rings <RINGS>").expect("--rings");
    assert!(
        long.get(rings..start)
            .is_some_and(|h| h.contains("with a warning unless --unplotted-rings names it")),
        "{long}"
    );
}

/// FR-90 names a value from the file by its key and file (NFR-10).
#[test]
fn a_value_from_the_file_names_its_key_when_cleaned() {
    let mut warnings = Warnings::default();
    let origin = Origin::new(Setting::Title, source::config("radar.toml"));
    assert_eq!(clean_setting(&origin, "a\u{1}b", &mut warnings), "ab");
    assert_eq!(
        warnings.render(),
        ["warning: the title key in radar.toml: removed U+0001, which the output cannot display"]
    );
}

#[test]
fn an_omitted_flag_is_none_so_a_config_key_can_fill_it() {
    // rg-ahj: Some means given on the command line, for every flag.
    let cli = parse(&["in.csv", "-o", "e.png"]).expect("parses");
    assert_eq!(
        (cli.theme, cli.legend, cli.table, cli.legend_columns),
        (None, None, None, None)
    );
    let cli = parse(&["in.csv", "-o", "e.png", "--theme", "light"]).expect("parses");
    assert_eq!(
        cli.theme,
        Some(Theme::Light),
        "an explicit default is still given"
    );
}

// ---- --paper: a PDF on landscape paper (FR-111)

#[test]
fn paper_parses_in_any_case() {
    use radargen::options::Paper;
    let cli = parse(&["in.csv", "-o", "e.pdf"]).expect("parses");
    assert_eq!(cli.paper, None, "not given");
    for (given, want) in [
        ("letter", Paper::Letter),
        ("A4", Paper::A4),
        ("a4", Paper::A4),
        ("None", Paper::None),
    ] {
        let cli = parse(&["in.csv", "-o", "e.pdf", "--paper", given]).expect(given);
        assert_eq!(cli.paper, Some(want), "{given}");
    }
    let err = parse(&["in.csv", "-o", "e.pdf", "--paper", "legal"])
        .expect_err("not a paper")
        .to_string();
    assert!(
        err.contains("letter") && err.contains("a4") && err.contains("none"),
        "{err}"
    );
}

#[test]
fn help_describes_paper() {
    use clap::CommandFactory;
    let long = Cli::command().render_long_help().to_string();
    assert!(
        long.contains("Letter or A4, for printing: the radar page first"),
        "{long}"
    );
    // Accepted and ignored, not refused: "Not with" is a clap conflict.
    assert_eq!(
        long.matches("Ignored on paper (--paper letter or a4).")
            .count(),
        2,
        "--width and --legend-columns: {long}"
    );
    assert!(!long.contains("Not with --paper"), "{long}");
}

// ---- --pdf: the radar PDF or the full one (FR-112)

#[test]
fn pdf_parses_in_any_case() {
    use radargen::options::Pdf;
    let cli = parse(&["in.csv", "-o", "e.pdf"]).expect("parses");
    assert_eq!(cli.pdf, None, "not given");
    for (given, want) in [
        ("radar", Pdf::Radar),
        ("FULL", Pdf::Full),
        ("Full", Pdf::Full),
    ] {
        let cli = parse(&["in.csv", "-o", "e.pdf", "--pdf", given]).expect(given);
        assert_eq!(cli.pdf, Some(want), "{given}");
    }
    let err = parse(&["in.csv", "-o", "e.pdf", "--pdf", "heavy"])
        .expect_err("not a PDF kind")
        .to_string();
    assert!(err.contains("radar") && err.contains("full"), "{err}");
}

#[test]
fn help_describes_pdf() {
    use clap::CommandFactory;
    // clap wraps the long help, so it is read with its whitespace folded.
    let long = Cli::command().render_long_help().to_string();
    let long = long.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(
        long.contains("\"full\" adds the table of items")
            && long.contains("always on paper: Letter unless --paper a4"),
        "{long}"
    );
    let short = Cli::command().render_help().to_string();
    assert!(
        short.contains("With full, a PDF adds the table; the default is radar"),
        "{short}"
    );
}

/// The defaults are written in the help as prose (plan RG-019A, Ruling 1);
/// this keeps them the ones `settings` applies.
#[test]
fn help_names_each_default() {
    use clap::CommandFactory;
    use radargen::settings::{DEFAULT_LEGEND_COLUMNS, DEFAULT_TABLE, DEFAULT_THEME};
    let short = Cli::command().render_help().to_string();
    let line = |flag: &str| {
        short
            .lines()
            .find(|l| l.contains(flag))
            .unwrap_or_default()
            .to_owned()
    };
    assert!(
        line("--theme").contains(&format!("the default is {DEFAULT_THEME:?}").to_lowercase()),
        "{short}"
    );
    assert!(
        line("--legend <").contains("the default is shown"),
        "{short}"
    );
    assert!(
        line("--table").contains(&format!("the default is {DEFAULT_TABLE:?}").to_lowercase()),
        "{short}"
    );
    assert!(
        line("--legend-columns").contains(&format!("the default is {DEFAULT_LEGEND_COLUMNS}")),
        "{short}"
    );
}

fn file(text: &str) -> radargen::settings::file::Loaded {
    load(text, Path::new("conf/radar.toml")).expect("parses")
}

fn conf() -> Source {
    source::config("conf/radar.toml")
}

/// FR-91: the command line wins key by key; the file fills the rest.
#[test]
fn a_flag_overrides_its_key_and_only_its_key() {
    let cli = parse(&["in.csv", "-c", "conf/radar.toml", "--title", "Flag"]).expect("parses");
    let loaded = file("output = \"r.html\"\ntitle = \"File\"\nsubtitle = \"Q3\"\nwidth = 1920\n");
    let s = cli.settings(Some(&loaded));
    assert_eq!(
        s.title,
        Effective::Given(Sourced {
            value: "Flag".into(),
            origin: Origin::flag(Setting::Title)
        })
    );
    assert_eq!(
        s.subtitle,
        Effective::Given(Sourced {
            value: "Q3".into(),
            origin: Origin::new(Setting::Subtitle, conf())
        })
    );
    assert_eq!(s.width.as_given().map(|w| w.value), Some(1920));
    assert_eq!(s.theme, Effective::Absent);
}

/// FR-65, FR-91: an empty --title is given, so it overrides with nothing.
#[test]
fn an_empty_title_flag_overrides_the_key() {
    let cli = parse(&["in.csv", "-c", "r.toml", "--title", ""]).expect("parses");
    let s = cli.settings(Some(&file("output = \"r.html\"\ntitle = \"File\"\n")));
    assert_eq!(s.title.as_given().map(|t| t.value.as_str()), Some(""));
}

/// Plan RG-019A, Ruling 3: a broken key is Broken, unless a flag overrides it.
#[test]
fn a_broken_key_is_broken_unless_a_flag_overrides_it() {
    let loaded = file("output = \"r.html\"\nwidth = 5\nrings = \"Adopt\"\n");
    let cli = parse(&["in.csv", "-c", "r.toml", "--width", "800"]).expect("parses");
    let s = cli.settings(Some(&loaded));
    assert_eq!(s.width.as_given().map(|w| w.value), Some(800));
    assert_eq!(s.rings, Effective::Broken);
}

/// FR-59 across sources (rg-ahj), as the maintainer ruled on 2026-10-18:
/// the command line wins over the opposing key, so `-p` drops the file's
/// `theme` and `--theme` drops its `palette`; both in the file conflict,
/// and both on the command line are clap's error.
#[test]
fn a_flag_drops_the_opposing_key_and_both_keys_conflict() {
    let args = |flags: &[&'static str]| {
        let mut args = vec!["in.csv", "-c", "conf/radar.toml"];
        args.extend_from_slice(flags);
        parse(&args).expect("parses")
    };
    let s = args(&["-p", "b.css"]).settings(Some(&file("output = \"r.png\"\ntheme = \"dark\"\n")));
    assert_eq!(s.theme, Effective::Absent, "the file's theme is dropped");
    assert_eq!(
        s.palette.as_given().map(|p| p.origin.source.clone()),
        Some(Source::Flag)
    );
    let s = args(&["--theme", "dark"])
        .settings(Some(&file("output = \"r.png\"\npalette = \"b.css\"\n")));
    assert_eq!(
        s.palette,
        Effective::Absent,
        "the file's palette is dropped"
    );
    assert_eq!(s.theme.as_given().map(|t| t.value), Some(Theme::Dark));
    let s = args(&[]).settings(Some(&file(
        "output = \"r.png\"\ntheme = \"dark\"\npalette = \"b.css\"\n",
    )));
    assert!(
        s.theme.as_given().is_some() && s.palette.as_given().is_some(),
        "both from the file stay, for the pipeline to refuse (FR-59)"
    );
    assert_eq!(
        parse(&["in.csv", "-c", "c.toml", "-T", "dark", "-p", "b.css"])
            .map_err(|e| e.kind())
            .err(),
        Some(clap::error::ErrorKind::ArgumentConflict),
        "both flags stay clap's error"
    );
}

/// FR-91: -o from one source or the other; with neither it is absent, and
/// a broken key is broken, not also absent (the pipeline reports each).
#[test]
fn an_output_is_absent_from_both_sources_or_broken() {
    let cli = parse(&["in.csv", "-c", "conf/radar.toml"]).expect("parses without -o");
    assert_eq!(
        cli.settings(Some(&file("title = \"T\"\n"))).output,
        Effective::Absent
    );
    assert_eq!(
        cli.settings(Some(&file("output = 3\n"))).output,
        Effective::Broken
    );
}

/// FR-91, Review Focus 2: relative from the file's directory, absolute as
/// written, `~` literal; a flag's path as given.
#[test]
fn paths_resolve_against_their_source() {
    let dir = Some(Path::new("conf"));
    let from_file = |p: &str| Sourced {
        value: PathBuf::from(p),
        origin: Origin::new(Setting::Output, conf()),
    };
    assert_eq!(
        resolve(dir, &from_file("out/r.html")).0,
        Path::new("conf/out/r.html")
    );
    assert_eq!(
        resolve(dir, &from_file("~/r.html")).0,
        Path::new("conf/~/r.html")
    );
    let absolute = std::env::temp_dir().join("r.html");
    let (opened, named) = resolve(dir, &from_file(absolute.to_str().expect("utf-8")));
    assert_eq!(opened, absolute);
    assert_eq!(
        named.to_string(),
        format!("the output key in conf/radar.toml ({})", absolute.display())
    );
    let flag = Sourced {
        value: PathBuf::from("out/r.html"),
        origin: Origin::flag(Setting::Output),
    };
    assert_eq!(resolve(dir, &flag).0, Path::new("out/r.html"));
}

/// No file: every flag given is Given from the command line, the rest Absent.
#[test]
fn without_a_file_the_flags_are_the_settings() {
    let cli = parse(&["in.csv", "-o", "r.html", "-r", "A,B"]).expect("parses");
    let s = cli.settings(None);
    assert_eq!(
        s.rings.as_given().map(|r| r.origin.source.clone()),
        Some(Source::Flag)
    );
    assert_eq!((s.legend, s.dir), (Effective::Absent, None));
}

#[test]
fn config_has_a_short_form_and_stands_in_for_output() {
    let cli = parse(&["in.csv", "-c", "r.toml"]).expect("-o is not needed with -c");
    assert_eq!(cli.config.as_deref(), Some(Path::new("r.toml")));
    assert!(parse(&["in.csv"]).is_err(), "still needed without it");
    assert!(
        parse(&["--create-template", "t.csv", "-c", "r.toml"]).is_err(),
        "given alone"
    );
}

/// QA review M-1: `--config`'s help lists every key FR-91 does, so a new
/// key cannot be left out of it.
#[test]
fn config_help_lists_every_key() {
    use clap::CommandFactory;
    let long = Cli::command().render_long_help().to_string();
    let start = long.find("--config <FILE>").expect("--config in --help");
    let paragraph = long
        .get(start..)
        .and_then(|rest| rest.split("\n\n  ").next())
        .unwrap_or_default()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    for setting in Setting::ALL {
        assert!(
            paragraph.contains(&format!("{},", setting.key()))
                || paragraph.contains(&format!("{} ", setting.key())),
            "{} missing: {paragraph}",
            setting.key()
        );
    }
    assert!(paragraph.contains("integers"), "{paragraph}");
    assert!(
        paragraph.contains("arrays of strings, such as"),
        "{paragraph}"
    );
}

/// A setting's value, if given.
fn val<T: Clone>(setting: &Effective<T>) -> Option<T> {
    setting.as_given().map(|given| given.value.clone())
}

/// QA review I-3, I-4 and the precedence matrix: for every setting, a flag
/// beats its key, and without the flag the key is read.
#[test]
fn every_flag_beats_its_key_and_every_key_is_read() {
    let text = "output = \"f.html\"\ntitle = \"F\"\nsubtitle = \"F\"\nrings = [\"F\"]\n\
                unplotted_rings = [\"G\"]\ntheme = \"dark\"\nwidth = 2000\nlegend_columns = 3\n\
                legend = \"hidden\"\ntable = \"all\"\n";
    let loaded = file(text);
    let flags = [
        "in.csv",
        "-c",
        "r.toml",
        "-o",
        "x.html",
        "-t",
        "X",
        "-s",
        "X",
        "-r",
        "X",
        "--unplotted-rings",
        "Y",
        "-T",
        "light",
        "-w",
        "1000",
        "--legend-columns",
        "2",
        "--legend",
        "shown",
        "--table",
        "radar",
    ];
    let s = parse(&flags).expect("parses").settings(Some(&loaded));
    assert_eq!(
        s.output.as_given().map(|v| v.origin.source.clone()),
        Some(Source::Flag)
    );
    assert_eq!(val(&s.title).as_deref(), Some("X"));
    assert_eq!(val(&s.subtitle).as_deref(), Some("X"));
    assert_eq!(val(&s.rings).as_deref(), Some("X"));
    assert_eq!(val(&s.unplotted_rings).as_deref(), Some("Y"));
    assert_eq!(val(&s.theme), Some(Theme::Light));
    assert_eq!(val(&s.width), Some(1000));
    assert_eq!(val(&s.legend_columns), Some(2));
    assert_eq!(val(&s.legend), Some(Legend::Shown));
    assert_eq!(val(&s.table), Some(Table::Radar));
    let s = parse(&["in.csv", "-c", "r.toml"])
        .expect("parses")
        .settings(Some(&loaded));
    assert_eq!(val(&s.output), Some(PathBuf::from("f.html")));
    assert_eq!(val(&s.title).as_deref(), Some("F"));
    assert_eq!(val(&s.subtitle).as_deref(), Some("F"));
    assert_eq!(val(&s.rings).as_deref(), Some("F"));
    assert_eq!(val(&s.unplotted_rings).as_deref(), Some("G"));
    assert_eq!(val(&s.theme), Some(Theme::Dark));
    assert_eq!(val(&s.width), Some(2000));
    assert_eq!(val(&s.legend_columns), Some(3));
    assert_eq!(val(&s.legend), Some(Legend::Hidden));
    assert_eq!(val(&s.table), Some(Table::All));
    assert_eq!(
        s.title.as_given().map(|v| v.origin.source.clone()),
        Some(source::config("conf/radar.toml"))
    );
}
