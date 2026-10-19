//! The HTML page (PR 14a, PR 14b): its head, its title fallback (D-75), both
//! arrangements with one shown at a time (FR-69), the focus rules matching
//! the tree they style (FR-48), and nothing it would fetch (NFR-5).

use std::collections::BTreeSet;

use radargen::emit::OUTPUT_VERSION;
use radargen::layout::arrange::{NarrowParts, WideParts, narrow_parts, wide_parts};
use radargen::num::fmt_coord;
use radargen::options::{Legend, Table};

use super::golden::golden;
use super::html::{assert_requests_nothing, outside, style, svgs};
use super::vocabulary::vocabulary;
use super::{build, parse, prd};

const VIEWPORT: &str = r#"<meta name="viewport" content="width=device-width, initial-scale=1">"#;
const VISUALLY_HIDDEN: &str = ".visually-hidden{position:absolute;width:1px;height:1px;margin:-1px;padding:0;border:0;overflow:hidden;clip:rect(0 0 0 0);clip-path:inset(50%);white-space:nowrap}";
const WIDE_OPEN: &str = "<div class=\"arrangement wide\">\n";
const NARROW_OPEN: &str = "<div class=\"arrangement narrow\">\n";
/// The wide arrangement's parts with their SVGs taken out (D-85): title,
/// radar, key, then the legend in its scrolling box.
const WIDE_PARTS: &str = "<div class=\"wide-title\">\n\n</div>\n<div class=\"wide-radar\">\n\n</div>\n<div class=\"wide-key\">\n\n</div>\n<div class=\"wide-legend\" tabindex=\"-1\">\n\n</div>\n";
/// The narrow arrangement's parts with their SVGs taken out (RG-102): the
/// body, then the legend.
const NARROW_PARTS: &str =
    "<div class=\"narrow-body\">\n\n</div>\n<div class=\"narrow-legend\">\n\n</div>\n";
/// The two inks a number on a solid marker takes, as literals in every
/// format, since the series do not change with theme (architecture §5).
const NUMBER_INKS: [&str; 2] = ["#FFFFFF", "#16191F"];

/// The PRD example's wide and narrow parts.
fn documents() -> (WideParts, NarrowParts) {
    let built = prd();
    (wide_parts(&built.blocks), narrow_parts(&built.blocks))
}

fn prd_page(title: Option<&str>) -> String {
    prd().page(title)
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
        around.contains(&format!(
            "{WIDE_OPEN}{WIDE_PARTS}\n</div>\n{NARROW_OPEN}{NARROW_PARTS}\n</div>\n<section id=\"detail\""
        )),
        "the panel, then the table view, follow both arrangements: {around}"
    );
    assert!(
        around.contains("</head>\n<body>\n<main data-narrow-query="),
        "{around}"
    );
    assert!(
        around.ends_with("</details>\n</main>\n<script></script>\n</body>\n</html>\n"),
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
            "</div>\n<h1 class=\"visually-hidden\">Technology Radar</h1>\n{WIDE_OPEN}"
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
    assert_eq!(
        html.matches("<script").count(),
        1,
        "the title opens no script; only the page's own is there"
    );
}

/// Review Focus 1 (FR-69): two arrangements, the wide first, in its four
/// parts with the radar's column growing to fill the window (D-85), the
/// narrow capped at its own width, and one query swapping them below the
/// breakpoint.
#[test]
fn one_arrangement_at_a_time() {
    let (wide, narrow) = documents();
    let html = prd().page_of(&wide, &narrow, None);
    assert_eq!(svgs(&html).len(), 6, "four wide parts and two narrow");
    let around = outside(&html);
    let (w, n) = (
        around.find(WIDE_OPEN).expect("a wide container"),
        around.find(NARROW_OPEN).expect("a narrow container"),
    );
    assert!(w < n, "the wide arrangement comes first");
    let css = style(&html);
    let query = wide.breakpoint() - 1;
    for rule in [
        format!(
            ".wide{{display:grid;grid-template-columns:minmax(0,max({f}px,calc((100vh - {t}px - var(--controls-height,0px))*{r}/{h}))) max-content;justify-content:center}}",
            f = fmt_coord(wide.radar.width * 0.8),
            r = fmt_coord(wide.radar.width),
            t = fmt_coord(wide.title.height),
            h = fmt_coord(wide.radar.height),
        ),
        // rg-dok: the title's SVG is drawn at its own part's width.
        format!(
            ".wide-title svg{{width:{}px;max-width:100%}}",
            fmt_coord(wide.title.width)
        ),
        format!(
            ".narrow{{max-width:{}px;margin:0 auto}}",
            fmt_coord(narrow.body.width)
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
    // The wide arrangement's markers are all in its radar part (D-85).
    let [title, wide, key, legend, narrow_body, narrow_legend] = keys.as_slice() else {
        panic!("four wide parts and two narrow");
    };
    assert!(title.is_empty() && key.is_empty() && legend.is_empty() && narrow_legend.is_empty());
    assert!(!wide.is_empty());
    assert_eq!(wide, narrow_body, "the same items in both");
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
        // rg-jri: the script shows a ring by its indicator's key, so each
        // marker needs exactly one indicator holding exactly one ring.
        for key in &markers {
            let rings: usize = doc
                .descendants()
                .filter(|n| n.attribute("data-marker-indicator") == Some(key))
                .map(|n| {
                    n.descendants()
                        .filter(|d| d.attribute("data-focus-ring") == Some(key))
                        .count()
                })
                .sum();
            assert_eq!(rings, 1, "marker {key} has one ring to show");
        }
        assert!(!css.contains(":has("), "no rule per key");
    }
}

/// The allow-list catches an attribute on the wrong element: an `href` on a
/// `<div>` is a request or a navigation the page never makes (NFR-5).
#[test]
#[should_panic(expected = "href=\"https://x\" on <div>")]
fn a_stray_href_fails_the_allow_list() {
    let html = prd_page(None).replacen(
        "<div class=\"page-controls\">",
        "<div href=\"https://x\"></div><div class=\"page-controls\">",
        1,
    );
    assert_requests_nothing(&html);
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

/// NFR-4: the PRD example's page under `--table all`, byte for byte
/// (architecture §10). It pins the page as it was before FR-106 gave the
/// table a choice: `all` publishes every row, so the unplotted `Rejected`
/// row is in it.
#[test]
fn prd_page_matches_its_golden() {
    let (html, count) = elided(&prd_page(Some("Technology Radar")));
    assert_eq!(count, 2, "both font faces were inlined");
    golden("html", "prd.html", &html);
}

/// NFR-4: the PRD example's page under the default `--table radar`, byte
/// for byte: as `prd.html` without the unplotted row (FR-106).
#[test]
fn prd_radar_page_matches_its_golden() {
    let (html, count) = elided(&prd().page_table(Some("Technology Radar"), Table::Radar));
    assert_eq!(count, 2, "both font faces were inlined");
    assert!(
        !html.contains("Legacy Ticketing"),
        "the unplotted row is not published"
    );
    golden("html", "prd-radar.html", &html);
}

/// RG-102: the page records where its legend starts on <html>; shown is the
/// default and writes nothing; nothing plotted writes nothing either, since
/// there is no legend and no button to bring one back.
#[test]
fn the_page_records_a_hidden_start() {
    assert!(
        prd()
            .page(None)
            .starts_with("<!DOCTYPE html>\n<html lang=\"en\">\n")
    );
    let hidden = prd().page_legend(Legend::Hidden);
    assert!(hidden.starts_with("<!DOCTYPE html>\n<html lang=\"en\" data-legend=\"hidden\">\n"));
    assert_requests_nothing(&hidden);
    let unplotted = build(
        "name,quadrant,ring,description\nLegacy Bus,Tools,Rejected,x\n",
        None,
        None,
    );
    let unplotted = unplotted.page_legend(Legend::Hidden);
    assert!(unplotted.starts_with("<!DOCTYPE html>\n<html lang=\"en\">\n"));
    // One start, read the same way by the attribute and the button's name.
    assert!(
        unplotted.contains(">Hide legend</span>") && !unplotted.contains(">Show legend</span>")
    );
}
