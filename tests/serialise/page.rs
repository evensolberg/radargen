//! The minimal HTML page around the wide radar (PR 14a): its head, its
//! title fallback (D-75), and nothing it would fetch (NFR-5).

use radargen::emit::OUTPUT_VERSION;
use radargen::emit::html::page;
use radargen::layout::arrange::{Arrangement, arrange};

use super::html::{assert_requests_nothing, split, tags};
use super::prd;
use super::vocabulary::vocabulary;

const VIEWPORT: &str = r#"<meta name="viewport" content="width=device-width, initial-scale=1">"#;
const VISUALLY_HIDDEN: &str = ".visually-hidden{position:absolute;width:1px;height:1px;margin:-1px;padding:0;border:0;overflow:hidden;clip:rect(0 0 0 0);clip-path:inset(50%);white-space:nowrap}";

fn prd_page(title: Option<&str>) -> String {
    page(&arrange(Arrangement::Wide, &prd().blocks), title)
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
    assert!(html.contains(&format!("<style>{VISUALLY_HIDDEN}</style>")));
    let (before, _, after) = split(&html);
    assert!(before.ends_with("<body>\n<main>\n"), "{before}");
    assert!(after.starts_with("\n</main>\n</body>\n"), "{after}");
}

#[test]
fn the_page_requests_nothing() {
    let html = prd_page(None);
    assert_requests_nothing(&html);
    let (_, svg, _) = split(&html);
    vocabulary(svg, false);
}

#[test]
fn the_title_falls_back() {
    let untitled = prd_page(None);
    assert!(untitled.contains("<title>Technology Radar</title>"));
    let (before, _, _) = split(&untitled);
    assert!(
        before.ends_with("<main>\n<h1 class=\"visually-hidden\">Technology Radar</h1>\n"),
        "{before}"
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
    let (before, _, _) = split(&html);
    assert!(tags(before).iter().all(|tag| tag.name != "script"));
}
