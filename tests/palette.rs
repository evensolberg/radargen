//! FR-81, FR-101: `docs/product/palette.sample.css` is a valid `--palette`
//! file holding the built-in light palette, and each neutral's line names
//! its dark value, so an author copying it gets exactly what the tool draws.

use radargen::palette::file::parse;
use radargen::palette::{Neutral, Palette, Role, Theme};

const SAMPLE: &str = include_str!("../docs/product/palette.sample.css");

/// A line's text after its indentation.
fn unindented(line: &str) -> &str {
    line.trim_start_matches(' ')
}

#[test]
fn the_sample_is_the_built_in_light_palette() {
    assert_eq!(parse(SAMPLE), Ok(Palette::built_in(Theme::Light)));
}

#[test]
fn each_neutral_line_names_its_dark_value() {
    let dark = Palette::built_in(Theme::Dark);
    for role in Neutral::ALL {
        let opener = format!("{role}:");
        let line = SAMPLE
            .lines()
            .find(|l| unindented(l).starts_with(&opener))
            .unwrap_or_else(|| panic!("no line declares {role}"));
        let want = format!("dark: {}", dark.value(Role::Neutral(role)));
        assert!(line.contains(&want), "{role}: {line}");
    }
}

/// FR-101's order: the series first, then FR-82's roles in order.
#[test]
fn declarations_are_in_fr_101_order() {
    let names: Vec<&str> = SAMPLE
        .lines()
        .map(unindented)
        .filter(|l| l.starts_with("--"))
        .filter_map(|l| l.split_once(':').map(|(name, _)| name))
        .collect();
    let mut want: Vec<String> = (1..=8).map(|i| format!("--series-{i}")).collect();
    want.extend(Neutral::ALL.iter().map(ToString::to_string));
    assert_eq!(names, want);
}

/// The header names no requirement and no flag a release lacks (plan
/// RG-018A, Ruling 9), says where the guide is, and the file is LF only
/// (NFR-17).
#[test]
fn the_sample_reads_as_a_users_file() {
    assert!(!SAMPLE.contains("FR-") && !SAMPLE.contains("--create-palette"));
    assert!(!SAMPLE.contains('\r') && SAMPLE.ends_with("}\n"));
    assert!(SAMPLE.contains("https://github.com/evensolberg/radargen/blob/main/docs/palette.md"));
}
