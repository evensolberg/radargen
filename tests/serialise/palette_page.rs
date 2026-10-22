//! A page drawn with a `--palette` file (§6.6, FR-67, FR-87): one set of
//! colours, no theme toggle, and the file's own series length.

use radargen::palette::file::parse;
use radargen::palette::{Colours, Palette, Role, Theme};

use super::html::{style, svgs};
use super::palette::{SAMPLE, sectors, with_series};
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
    assert!(
        !html.contains("theme-moon") && !html.contains("theme-sun"),
        "and none of its icons"
    );
    assert_eq!(
        html.matches("<svg").count(),
        svgs(&html).len() + 2,
        "every other SVG is a drawing, but for the legend button's two icons"
    );
    assert!(
        html.contains("class=\"legend-toggle\""),
        "the legend button is there"
    );
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

/// FR-67 in HTML: five entries over six sectors. Sector 6 names
/// `--series-1`, so with one item per sector `--series-1` is named twice
/// as often as `--series-2`; no sixth entry is named or declared.
#[test]
fn a_palette_page_cycles_over_the_files_series() {
    let five = ["#100A0A", "#200A0A", "#300A0A", "#400A0A", "#500A0A"];
    let html = build(&sectors(6), None, None).page_with(&series(&five));
    assert!(!html.contains("--series-6"));
    assert!(style(&html).contains("--series-5:#500A0A;"));
    let first = html.matches("var(--series-1)").count();
    let second = html.matches("var(--series-2)").count();
    assert!(second > 0 && first == 2 * second, "{first} vs {second}");
    // The built-in eight are not used for any sector.
    let built_in = Palette::built_in(Theme::Light);
    for i in 0..built_in.series_len() {
        let value = built_in.value(Role::Series(i));
        assert!(!html.contains(value), "{value}");
    }
}

/// rg-ft5: the radar's cap allows for the controls bar only where it
/// shows: with the script, on screen, on a page with a control to show. A
/// `--palette` page with nothing plotted has none, as the script leaves
/// its legend button and skip link hidden and it has no theme toggle.
#[test]
fn the_cap_allows_for_the_controls_only_where_they_show() {
    const ALLOWANCE: &str = "@media not print{:root.js{--controls-height:calc(3.05rem + 2px)}}\n";
    let nothing = build("name,sector,ring,description\n", None, None);
    for (html, shows) in [
        (prd().page_with(&sample()), true),
        (prd().page_with(&Colours::BuiltIn(Theme::Light)), true),
        (nothing.page_with(&Colours::BuiltIn(Theme::Light)), true),
        (nothing.page_with(&sample()), false),
    ] {
        let css = style(&html);
        assert_eq!(css.contains(ALLOWANCE), shows, "{css}");
        assert_eq!(
            css.matches("--controls-height:").count(),
            usize::from(shows)
        );
        assert!(css.contains(" - var(--controls-height,0px))*"), "{css}");
        assert!(!css.contains("3.05rem - 2px"), "{css}");
    }
}
