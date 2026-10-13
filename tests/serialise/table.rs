//! FR-41 and FR-78's table view (PR 15a): every CSV row and every named
//! column, plotted rows first in FR-34's order, unplotted ones flagged, and
//! FR-96's links.
//!
//! The fixture is ordered so a wrong implementation cannot pass by luck:
//! FR-34's order is not CSV order, the unplotted row sits mid-file, and a
//! multi-line record early on makes a row's line differ from its index.

use radargen::model::{Link, NOT_ON_THE_RADAR};
use roxmltree::Node;

use super::{Built, build, parse};

/// CSV order differs from FR-34's (`Alpha` in `Platforms` numbers first),
/// `Old` is unplotted mid-file, `Zed`'s name and description span two lines,
/// `Alpha`'s ring is written in lower case and `Beta`'s link is padded and
/// upper-case.
const CSV: &str = "name,quadrant,ring,description,link,note\n\
\"Zed\r\nTwo\",Tools,Trial,\"first\r\nsecond\",https://z.example/?a=1&b=2,n1\n\
Old,Tools, Rejected ,gone,javascript:x,\n\
Alpha,Platforms,adopt,d,,\n\
Beta,Tools,Adopt,\"\nIntro\n\n\",  HTTPS://B.example  ,  n2  \n";

fn built() -> Built {
    build(CSV, None, Some("T"))
}

/// The page's `<table>…</table>`, which is well-formed XML by construction.
fn table_of(html: &str) -> String {
    let start = html.find("<table").expect("a table");
    let end = html.find("</table>").expect("the table closes") + "</table>".len();
    html.get(start..end).expect("in the page").to_owned()
}

fn rows<'a>(doc: &'a roxmltree::Document<'a>) -> Vec<Node<'a, 'a>> {
    doc.descendants()
        .filter(|n| n.has_tag_name("tr") && n.attribute("data-row").is_some())
        .collect()
}

fn cells<'a>(row: Node<'a, 'a>) -> Vec<Node<'a, 'a>> {
    row.children()
        .filter(|n| n.has_tag_name("td") || n.has_tag_name("th"))
        .collect()
}

/// All the text under `node`.
fn text(node: Node<'_, '_>) -> String {
    node.descendants()
        .filter(Node::is_text)
        .filter_map(|n| n.text())
        .collect()
}

#[test]
fn rows_come_in_fr78s_order_keyed_on_the_csv_line() {
    let b = built();
    let html = b.page(Some("T"));
    let table = table_of(&html);
    let doc = parse(&table);
    let got: Vec<String> = rows(&doc)
        .iter()
        .filter_map(|r| r.attribute("data-row").map(str::to_owned))
        .collect();
    let want: Vec<String> = b
        .plotted
        .table_rows()
        .into_iter()
        .map(|i| b.catalogue.items.get(i).expect("an item").line.to_string())
        .collect();
    assert_eq!(got, want, "plotted in FR-34's order, then unplotted");
    let csv_order: Vec<String> = b
        .catalogue
        .items
        .iter()
        .map(|i| i.line.to_string())
        .collect();
    assert_ne!(
        got, csv_order,
        "the fixture must not be in FR-78's order already"
    );
}

#[test]
fn numbers_are_fr18s_and_match_the_legend() {
    let b = built();
    let html = b.page(Some("T"));
    let table = table_of(&html);
    let doc = parse(&table);
    for row in rows(&doc) {
        let line: usize = row
            .attribute("data-row")
            .expect("keyed")
            .parse()
            .expect("a line");
        let index = b
            .catalogue
            .items
            .iter()
            .position(|i| i.line == line)
            .expect("an item on that line");
        let number = cells(row).first().map(|c| text(*c)).expect("a number cell");
        let want = b.plotted.numbers.get(index).copied().flatten();
        assert_eq!(
            number,
            want.map(|n| n.to_string()).unwrap_or_default(),
            "line {line}"
        );
        if let Some(n) = want {
            let legend = format!("data-row=\"{line}\" data-legend-row=\"{line}\"");
            let label = html
                .split("aria-label=\"")
                .find(|chunk| chunk.contains(&legend))
                .expect("the legend row");
            assert!(label.starts_with(&format!("{n}. ")), "line {line}: {label}");
        }
    }
}

#[test]
fn unplotted_rows_are_flagged_and_plotted_ones_are_not() {
    let b = built();
    let doc_html = b.page(Some("T"));
    let table = table_of(&doc_html);
    let doc = parse(&table);
    for row in rows(&doc) {
        let header = row
            .children()
            .find(|n| n.has_tag_name("th"))
            .expect("a row header");
        assert_eq!(header.attribute("scope"), Some("row"), "FR-78's name cell");
        let flagged = text(header).ends_with(&format!(" {NOT_ON_THE_RADAR}"));
        let unplotted = row.attribute("class") == Some("unplotted");
        assert_eq!(flagged, unplotted, "{}", text(header));
    }
    let old = rows(&doc)
        .into_iter()
        .find(|r| text(*r).contains("Old"))
        .expect("the unplotted row");
    assert_eq!(
        text(cells(old).get(1).copied().expect("name")),
        format!("Old {NOT_ON_THE_RADAR}")
    );
}

#[test]
fn columns_are_the_number_the_three_labels_then_the_header() {
    let b = built();
    let doc_html = b.page(Some("T"));
    let table = table_of(&doc_html);
    let doc = parse(&table);
    let heads: Vec<String> = doc
        .descendants()
        .filter(|n| n.has_tag_name("th") && n.attribute("scope") == Some("col"))
        .map(text)
        .collect();
    assert_eq!(
        heads,
        [
            "#Number",
            "name",
            "quadrant",
            "ring",
            "description",
            "link",
            "note"
        ],
        "D-80: every named column, description included"
    );
    let hash = doc
        .descendants()
        .find(|n| n.has_tag_name("span") && n.attribute("aria-hidden") == Some("true"))
        .expect("the visible #");
    assert_eq!(text(hash), "#");
}

#[test]
fn cells_show_labels_trimmed_and_prose_with_its_lines() {
    let b = built();
    let doc_html = b.page(Some("T"));
    let table = table_of(&doc_html);
    let doc = parse(&table);
    let row = |name: &str| {
        rows(&doc)
            .into_iter()
            .find(|r| text(cells(*r).get(1).copied().expect("name")).starts_with(name))
            .unwrap_or_else(|| panic!("{name}"))
    };
    let texts = |name: &str| cells(row(name)).into_iter().map(text).collect::<Vec<_>>();
    assert_eq!(
        texts("Alpha"),
        ["1", "Alpha", "Platforms", "Adopt", "d", "", ""],
        "declared ring label; empty cells stay empty (FR-29)"
    );
    let old = texts("Old");
    assert_eq!(
        old.get(3).map(String::as_str),
        Some("Rejected"),
        "an unplotted ring, trimmed"
    );
    let zed = cells(row("Zed"));
    let name = zed.get(1).copied().expect("name");
    assert_eq!(text(name), "Zed Two", "FR-97: a label on one line");
    assert_eq!(name.attribute("class"), None, "a label is not prose");
    let description = zed.get(4).copied().expect("description");
    assert_eq!(
        text(description),
        "first\nsecond",
        "FR-97: prose keeps its line"
    );
    assert_eq!(description.attribute("class"), Some("prose"));
    assert_eq!(
        zed.get(6).and_then(|c| c.attribute("class")),
        Some("prose"),
        "an Other column"
    );
    let beta = cells(row("Beta"));
    assert_eq!(
        beta.get(4).copied().map(text).as_deref(),
        Some("\nIntro\n\n"),
        "D-76: prose stays verbatim, blank lines included"
    );
    assert_eq!(
        beta.get(6).copied().map(text).as_deref(),
        Some("  n2  "),
        "an Other column is prose, verbatim"
    );
}

#[test]
fn links_are_anchors_and_refused_values_are_text() {
    let b = built();
    let doc_html = b.page(Some("T"));
    let table = table_of(&doc_html);
    let doc = parse(&table);
    let anchors: Vec<Node> = doc.descendants().filter(|n| n.has_tag_name("a")).collect();
    let addresses: Vec<&str> = b
        .catalogue
        .items
        .iter()
        .filter_map(|i| match &i.link {
            Some(Link::Address(a)) => Some(a.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(anchors.len(), addresses.len(), "one anchor per address");
    for a in &anchors {
        let href = a.attribute("href").expect("an href");
        assert!(addresses.contains(&href), "{href}");
        assert_eq!(a.attribute("target"), Some("_blank"));
        assert_eq!(a.attribute("rel"), Some("noopener noreferrer"));
        assert_eq!(
            text(*a),
            format!("{href} (opens in a new tab)"),
            "the address as written, trimmed"
        );
        assert_eq!(a.parent().and_then(|p| p.attribute("class")), Some("link"));
    }
    assert!(
        addresses.contains(&"HTTPS://B.example"),
        "scheme case kept, trimmed"
    );
    let refused = rows(&doc)
        .into_iter()
        .find(|r| text(*r).contains("javascript:x"))
        .expect("the refused link's row");
    assert!(
        !refused.descendants().any(|n| n.has_tag_name("a")),
        "D-77: a refused value is never a link"
    );
    assert!(!text(refused).contains("opens in a new tab"));
}

#[test]
fn the_table_is_closed_under_a_visible_heading() {
    let html = built().page(Some("T"));
    let heading = html
        .find("<h2 class=\"table-heading\" id=\"table-heading\">")
        .expect("the heading");
    let details = html
        .find("<details class=\"table-view\">")
        .expect("the toggle");
    assert!(heading < details, "the heading precedes the toggle");
    assert!(
        !html.contains("<details class=\"table-view\" open"),
        "closed by default"
    );
    assert!(
        html.contains("<summary>The table</summary>"),
        "FR-41's toggle, worded for both states"
    );
    let table = table_of(&html);
    assert!(
        table.starts_with("<table>\n"),
        "only the region is named by the heading: {table}"
    );
    assert!(
        html.contains("<div class=\"table-scroll\" role=\"region\" aria-labelledby=\"table-heading\" tabindex=\"0\">"),
        "a named, focusable scroll region"
    );
}
