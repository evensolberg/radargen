//! A page drawn with a `--palette` file (§6.6, FR-67, FR-87): one set of
//! colours, no theme toggle, and the file's own series length.

use radargen::palette::file::parse;
use radargen::palette::{Colours, Palette, Role, Theme};

use super::html::style;
use super::palette::{SAMPLE, quadrants, with_series};
use super::{build, prd};

fn sample() -> Colours {
    parse(SAMPLE).expect("the sample parses").into()
}

/// The sample with `series` as its entries.
fn series(series: &[&str]) -> Colours {
    parse(&with_series(series)).expect("parses").into()
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
    let html = build(&quadrants(6), None, None).page_with(&series(&five));
    assert!(!html.contains("--series-6"));
    assert!(style(&html).contains("--series-5:#500A0A;"));
    let first = html.matches("var(--series-1)").count();
    let second = html.matches("var(--series-2)").count();
    assert!(second > 0 && first == 2 * second, "{first} vs {second}");
    // The built-in eight are not used for any quadrant.
    let built_in = Palette::built_in(Theme::Light);
    for i in 0..built_in.series_len() {
        let value = built_in.value(Role::Series(i));
        assert!(!html.contains(value), "{value}");
    }
}
