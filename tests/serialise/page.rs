//! The HTML page (PR 14a, PR 14b): its head, its title fallback (D-75), both
//! arrangements with one shown at a time (FR-69), the focus rules matching
//! the tree they style (FR-48), and nothing it would fetch (NFR-5).

use std::collections::BTreeSet;

use radargen::emit::OUTPUT_VERSION;
use radargen::emit::html::page;
use radargen::layout::arrange::{Arrangement, arrange};
use radargen::num::fmt_coord;
use radargen::style::breakpoint_px;
use radargen::svg::{Document, marker_keys};

use super::golden::golden;
use super::html::{assert_requests_nothing, outside, style, svgs, tags};
use super::identity::interactive;
use super::vocabulary::vocabulary;
use super::{parse, prd};

const VIEWPORT: &str = r#"<meta name="viewport" content="width=device-width, initial-scale=1">"#;
const VISUALLY_HIDDEN: &str = ".visually-hidden{position:absolute;width:1px;height:1px;margin:-1px;padding:0;border:0;overflow:hidden;clip:rect(0 0 0 0);clip-path:inset(50%);white-space:nowrap}";
const WIDE_OPEN: &str = "<div class=\"arrangement wide\">\n";
const NARROW_OPEN: &str = "<div class=\"arrangement narrow\">\n";
/// The two inks a number on a solid marker takes, as literals in every
/// format, since the series do not change with theme (architecture §5).
const NUMBER_INKS: [&str; 2] = ["#FFFFFF", "#16191F"];

/// The PRD example's two documents.
fn documents() -> (Document, Document) {
    let built = prd();
    (
        arrange(Arrangement::Wide, &built.blocks),
        arrange(Arrangement::Narrow, &built.blocks),
    )
}

fn prd_page(title: Option<&str>) -> String {
    let (wide, narrow) = documents();
    page(&wide, &narrow, title)
}

/// The values of `attribute` on every element of `svg` carrying it.
fn values(svg: &str, attribute: &str) -> Vec<String> {
    parse(svg)
        .descendants()
        .filter_map(|n| n.attribute(attribute).map(str::to_owned))
        .collect()
}

#[test]
fn the_page_is_minimal_and_valid() {
    let html = prd_page(Some("Q3 Radar"));
    assert!(
        html.starts_with("<!DOCTYPE html>\n<html lang=\"en\">\n"),
        "{html}"
    );
    assert!(html.ends_with("</html>\n"), "{html}");
    assert!(!html.contains('\r') && !html.starts_with('\u{feff}'));
    assert!(html.contains("<meta charset=\"utf-8\">"));
    assert!(html.contains(VIEWPORT));
    for limit in ["maximum-scale", "minimum-scale", "user-scalable"] {
        assert!(!html.contains(limit), "{limit}: zoom is never limited");
    }
    assert!(html.contains(&format!(
        "<meta name=\"generator\" content=\"{OUTPUT_VERSION}\">"
    )));
    assert!(style(&html).contains(VISUALLY_HIDDEN));
    let around = outside(&html);
    assert!(
        around.ends_with(&format!(
            "<body>\n<main>\n{WIDE_OPEN}\n</div>\n{NARROW_OPEN}\n</div>\n</main>\n</body>\n</html>\n"
        )),
        "{around}"
    );
}

#[test]
fn the_page_requests_nothing() {
    let html = prd_page(None);
    assert_requests_nothing(&html);
    for svg in svgs(&html) {
        vocabulary(svg, true);
    }
}

#[test]
fn the_title_falls_back() {
    let untitled = prd_page(None);
    assert!(untitled.contains("<title>Technology Radar</title>"));
    let around = outside(&untitled);
    assert!(
        around.contains(&format!(
            "<main>\n<h1 class=\"visually-hidden\">Technology Radar</h1>\n{WIDE_OPEN}"
        )),
        "{around}"
    );
    let titled = prd_page(Some("Q3 Radar"));
    assert!(titled.contains("<title>Q3 Radar</title>"));
    assert!(
        !titled.contains("<h1"),
        "the SVG's title is the level-1 heading"
    );
}

#[test]
fn a_hostile_title_is_escaped() {
    let html = prd_page(Some("</title><script>&amp;\""));
    assert!(
        html.contains("<title>&lt;/title&gt;&lt;script&gt;&amp;amp;\"</title>"),
        "{html}"
    );
    assert_requests_nothing(&html);
    assert!(tags(&outside(&html)).iter().all(|tag| tag.name != "script"));
}

/// Review Focus 1 (FR-69): two arrangements, the wide first, each capped at
/// its own width, and one query swapping them below the breakpoint.
#[test]
fn one_arrangement_at_a_time() {
    let (wide, narrow) = documents();
    let html = page(&wide, &narrow, None);
    assert_eq!(svgs(&html).len(), 2);
    let around = outside(&html);
    let (w, n) = (
        around.find(WIDE_OPEN).expect("a wide container"),
        around.find(NARROW_OPEN).expect("a narrow container"),
    );
    assert!(w < n, "the wide arrangement comes first");
    let css = style(&html);
    let query = breakpoint_px(wide.width) - 1;
    for rule in [
        format!(
            ".wide{{max-width:{}px;margin:0 auto}}",
            fmt_coord(wide.width)
        ),
        format!(
            ".narrow{{max-width:{}px;margin:0 auto}}",
            fmt_coord(narrow.width)
        ),
        format!(
            ".narrow{{display:none}}\n@media (max-width: {query}px){{.wide{{display:none}}.narrow{{display:block}}}}"
        ),
    ] {
        assert!(css.contains(&rule), "{rule}");
    }
}

/// Both arrangements are drawn for the viewer to choose a theme: every paint
/// is a custom property, `none`, or a number ink (FR-56).
#[test]
fn both_are_interactive() {
    let html = prd_page(None);
    let keys: Vec<BTreeSet<String>> = svgs(&html)
        .into_iter()
        .map(|svg| {
            for paint in values(svg, "fill").into_iter().chain(values(svg, "stroke")) {
                assert!(
                    paint.starts_with("var(--")
                        || paint == "none"
                        || NUMBER_INKS.contains(&paint.as_str()),
                    "{paint}"
                );
            }
            values(svg, "data-marker").into_iter().collect()
        })
        .collect();
    let [wide, narrow] = keys.as_slice() else {
        panic!("two arrangements");
    };
    assert!(!wide.is_empty());
    assert_eq!(wide, narrow, "the same items in both");
}

/// The walk finds the marker controls in document order, and nothing else.
#[test]
fn marker_keys_walk_controls_only() {
    let (wide, _) = documents();
    let svg = interactive(&prd(), Arrangement::Wide);
    let walked: Vec<String> = marker_keys(&wide)
        .into_iter()
        .map(|key| key.0.to_string())
        .collect();
    assert_eq!(walked, values(&svg, "data-marker"));
}

/// The generated focus rules match the tree they style: a selector that
/// matches nothing would show no ring and fail no string check (FR-48).
#[test]
fn the_indicator_selectors_match_the_tree() {
    let html = prd_page(None);
    let css = style(&html);
    for svg in svgs(&html) {
        let doc = parse(svg);
        let markers: BTreeSet<&str> = doc
            .descendants()
            .filter_map(|n| n.attribute("data-marker"))
            .collect();
        for indicator in doc
            .descendants()
            .filter(|n| n.attribute("data-row-indicator").is_some())
        {
            let row = indicator.parent_element().expect("a parent");
            assert_eq!(
                row.attribute("data-legend-row"),
                indicator.attribute("data-row-indicator"),
                "a row indicator is its own row's direct child"
            );
            assert!(
                indicator.children().any(|c| c.has_tag_name("rect")),
                "a row indicator holds its rect directly"
            );
        }
        for key in doc
            .descendants()
            .filter_map(|n| n.attribute("data-marker-indicator"))
        {
            assert!(markers.contains(key), "indicator {key} has its marker");
        }
        for key in &markers {
            assert!(
                css.contains(&format!(
                    "svg:has([data-marker=\"{key}\"]:focus-visible) [data-marker-indicator=\"{key}\"]{{visibility:visible}}"
                )),
                "marker {key} has its rule"
            );
        }
        assert_eq!(
            css.matches("svg:has(").count(),
            markers.len(),
            "no rule for a key the page lacks"
        );
    }
}

/// `html` with each inlined font's base64 replaced by `…`, and how many were
/// replaced: the golden would otherwise diff 220 kB on any font change, and
/// the payloads are checked byte for byte elsewhere (Task 1).
fn elided(html: &str) -> (String, usize) {
    let mut parts = html.split("base64,");
    let head = parts.next().unwrap_or_default().to_owned();
    parts.fold((head, 0), |(text, count), part| {
        let (_, after) = part.split_once(')').expect("a payload closes");
        ([text.as_str(), "base64,…)", after].concat(), count + 1)
    })
}

/// NFR-4: the PRD example's page, byte for byte (architecture §10).
#[test]
fn prd_page_matches_its_golden() {
    let (html, count) = elided(&prd_page(Some("Technology Radar")));
    assert_eq!(count, 2, "both font faces were inlined");
    golden("html", "prd.html", &html);
}
