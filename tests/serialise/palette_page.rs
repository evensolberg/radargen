//! A page drawn with a `--palette` file (§6.6, FR-67, FR-87): one set of
//! colours, no theme toggle, and the file's own series length.

use std::fmt::Write as _;

use radargen::palette::file::parse;
use radargen::palette::{Colours, Palette, Theme};

use super::html::style;
use super::{build, prd};

const SAMPLE: &str = include_str!("../../docs/product/palette.sample.css");

fn sample() -> Colours {
    Colours::File(Box::new(parse(SAMPLE).expect("the sample parses")))
}

/// The sample with `series` in place of its eight entries.
fn with_series(series: &[&str]) -> Colours {
    let mut text = String::from(":root {\n");
    for (i, value) in series.iter().enumerate() {
        writeln!(text, "  --series-{}: {value};", i + 1).expect("writes");
    }
    for line in SAMPLE.lines() {
        let declaration = line.trim_start_matches(' ');
        if declaration.starts_with("--") && !declaration.starts_with("--series-") {
            writeln!(text, "{line}").expect("writes");
        }
    }
    text.push_str("}\n");
    Colours::File(Box::new(parse(&text).expect("parses")))
}

/// A CSV with `count` quadrants, Q01 onward, one item each.
fn quadrants(count: usize) -> String {
    let mut csv = String::from("name,quadrant,ring,description\n");
    for q in 1..=count {
        writeln!(csv, "Item {q},Q{q:02},Adopt,d").expect("writes");
    }
    csv
}

#[test]
fn a_palette_page_has_one_set_and_no_toggle() {
    let html = prd().page_with(&sample());
    let css = style(&html);
    for absent in [
        "@media (prefers-color-scheme",
        ":root:not([data-theme",
        ":root[data-theme=\"dark\"]",
    ] {
        assert!(!css.contains(absent), "{absent}");
    }
    assert!(!html.contains("class=\"theme-toggle\""), "no toggle");
    assert!(html.contains("class=\"skip\""), "the skip link stays");
    assert!(css.contains("@media (forced-colors: active)"));
}

/// The built-in page keeps both themes and the toggle.
#[test]
fn the_built_in_page_keeps_its_toggle() {
    let html = prd().page_with(&Colours::BuiltIn(Theme::Dark));
    assert!(html.contains("class=\"theme-toggle\""));
    assert!(style(&html).contains("@media (prefers-color-scheme: dark)"));
}

/// FR-67 in HTML: five entries over six quadrants. Quadrant 6 names
/// `--series-1`, so with one item per quadrant `--series-1` is named twice
/// as often as `--series-2`; no sixth entry is named or declared.
#[test]
fn a_palette_page_cycles_over_the_files_series() {
    let five = ["#100A0A", "#200A0A", "#300A0A", "#400A0A", "#500A0A"];
    let html = build(&quadrants(6), None, None).page_with(&with_series(&five));
    assert!(!html.contains("--series-6"));
    assert!(style(&html).contains("--series-5:#500A0A;"));
    let first = html.matches("var(--series-1)").count();
    let second = html.matches("var(--series-2)").count();
    assert!(second > 0 && first == 2 * second, "{first} vs {second}");
    // The built-in eight are not used for any quadrant.
    for built_in in (0..8).map(|i| {
        Palette::built_in(Theme::Light)
            .value(radargen::palette::Role::Series(i))
            .to_string()
    }) {
        assert!(!html.contains(&built_in), "{built_in}");
    }
}
