//! The interactive layer's markup (PR 15b-1): one constant script, the
//! breakpoint query it reads, and the page controls only it reveals
//! (FR-56, FR-98, NFR-4).

use super::html::{outside, tags};
use super::{build, prd};

const SCRIPT: &str = include_str!("../../src/emit/html/script.js");

/// The text between the page's `<script>` and `</script>`.
fn script_of(html: &str) -> &str {
    let (_, rest) = html.split_once("<script>").expect("a script");
    rest.split_once("</script>").expect("the script closes").0
}

#[test]
fn one_script_with_no_attributes_and_the_same_bytes_everywhere() {
    let html = prd().page(Some("T"));
    assert_eq!(html.matches("<script").count(), 1, "one script");
    assert_eq!(
        script_of(&html),
        ["\n", SCRIPT].concat(),
        "script.js, verbatim"
    );
    let other = build(
        "name,quadrant,ring,description\nKafka,Tool,Adopt,d\n",
        None,
        None,
    )
    .page(None);
    assert_eq!(script_of(&other), script_of(&html), "NFR-4: no data in it");
    assert!(
        html.contains("</main>\n<script>\n") && html.ends_with("</script>\n</body>\n</html>\n"),
        "last in <body>, after <main>"
    );
}

#[test]
fn main_carries_the_breakpoint_query() {
    let html = prd().page(Some("T"));
    let main = tags(&outside(&html))
        .into_iter()
        .find(|t| t.name == "main")
        .expect("a main");
    let query = main
        .attributes
        .iter()
        .find(|(k, _)| k == "data-narrow-query")
        .map(|(_, v)| v.clone())
        .expect("the query");
    let css = super::html::style(&html);
    assert!(
        css.contains(&format!("@media {query}{{")),
        "the query the stylesheet swaps on: {query}"
    );
}

#[test]
fn the_page_controls_are_hidden_until_the_script_runs() {
    let html = prd().page(Some("T"));
    let main_at = html.find("<main").expect("a main");
    let controls = html
        .find("<div class=\"page-controls\">")
        .expect("the controls");
    let wide = html
        .find("<div class=\"arrangement wide\">")
        .expect("the wide arrangement");
    assert!(main_at < controls && controls < wide, "first in <main>");
    for button in [
        "<button type=\"button\" class=\"skip\" hidden=\"\">Skip to the legend</button>",
        "<button type=\"button\" class=\"theme-toggle\" aria-pressed=\"false\" hidden=\"\">Dark theme</button>",
    ] {
        assert!(html.contains(button), "{button}");
    }
    let css = super::html::style(&html);
    assert!(
        css.contains("[hidden]{display:none!important}"),
        "no rule reveals them"
    );
}

/// FR-38's panel: one, written once outside both arrangements and before
/// the table view, hidden until the script fills it (D-80).
#[test]
fn one_hidden_panel_between_the_radar_and_the_table() {
    let html = prd().page(Some("T"));
    let open = "<section id=\"detail\" class=\"detail\" aria-label=\"Item details\" hidden=\"\">";
    assert_eq!(html.matches("id=\"detail\"").count(), 1, "one panel id");
    let panel = html.find(open).expect("the panel, hidden");
    let narrow = html.find("<div class=\"arrangement narrow\">").expect("narrow");
    let table = html.find("<h2 class=\"table-heading\"").expect("the table view");
    assert!(narrow < panel && panel < table, "after the radar, before the table");
    assert!(html.contains(
        "<div class=\"detail-head\"><h2 class=\"detail-heading\" tabindex=\"-1\"></h2><button type=\"button\" class=\"detail-close\">Close</button></div><dl></dl></section>"
    ));
}
