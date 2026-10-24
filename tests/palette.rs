//! FR-81, FR-101: `docs/product/palette.sample.css` is a valid `--palette`
//! file holding the built-in light palette, and each neutral's line names
//! its dark value, so an author copying it gets exactly what the tool draws.

use radargen::palette::file::parse;
use radargen::palette::{Neutral, Palette, Role, SAMPLE, Theme};

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

/// The example palette the guide offers (rg-new), beside the sample.
const SLATE: &str = include_str!("../docs/examples/palette.slate.css");

/// FR-101's order: the series first, then FR-82's roles in order. The
/// slate example keeps it, so the two files read alike.
#[test]
fn declarations_are_in_fr_101_order() {
    for file in [SAMPLE, SLATE] {
        let names: Vec<&str> = file
            .lines()
            .map(unindented)
            .filter(|l| l.starts_with("--"))
            .filter_map(|l| l.split_once(':').map(|(name, _)| name))
            .collect();
        let mut want: Vec<String> = (1..=8).map(|i| format!("--series-{i}")).collect();
        want.extend(Neutral::ALL.iter().map(ToString::to_string));
        assert_eq!(names, want);
    }
}

/// The header names no requirement and no flag a release lacks (plan
/// RG-018A, Ruling 9), says where the guide is, and the file is LF only with
/// no byte-order mark (NFR-17).
#[test]
fn the_sample_reads_as_a_users_file() {
    assert!(!SAMPLE.contains("--create-palette"));
    for file in [SAMPLE, SLATE] {
        assert!(!file.contains("FR-"));
        assert!(!file.contains('\r') && file.ends_with("}\n"));
        assert!(!file.starts_with('\u{feff}'), "no byte-order mark");
        assert!(file.contains("https://github.com/evensolberg/radargen/blob/main/docs/palette.md"));
    }
}

/// The guide's tables give each value the code draws.
#[test]
fn the_guide_lists_every_value() {
    let guide = include_str!("../docs/palette.md");
    let light = Palette::built_in(Theme::Light);
    let dark = Palette::built_in(Theme::Dark);
    for role in Neutral::ALL {
        let row = guide
            .lines()
            .find(|l| l.starts_with(&format!("| `{role}` |")))
            .unwrap_or_else(|| panic!("no row for {role}"));
        let cell = |p: &Palette| format!("| `{}` |", p.value(Role::Neutral(role)));
        assert!(
            row.contains(&cell(&light)) && row.contains(&cell(&dark)),
            "{role}: {row}"
        );
    }
    for i in 0..8 {
        let row = format!(
            "| `--series-{}` | `{}`",
            i + 1,
            light.value(Role::Series(i))
        );
        assert!(guide.contains(&row), "{row}");
    }
}

/// The illustration is well-formed XML: an SVG with `--` in a comment
/// shows as a broken image (plan RG-018A, R23).
#[test]
fn the_illustration_parses() {
    let svg = include_str!("../docs/palette-roles.svg");
    roxmltree::Document::parse(svg).expect("well-formed");
}
