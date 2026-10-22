//! FR-41 and FR-78's table view (PR 15a): under `--table all`, every CSV
//! row and every named column, plotted rows first in FR-34's order,
//! unplotted ones flagged, and FR-96's links; under the default `--table
//! radar`, only the rows the radar draws and the columns radargen reads
//! (FR-106).
//!
//! The fixture is ordered so a wrong implementation cannot pass by luck:
//! FR-34's order is not CSV order, the unplotted row sits mid-file, and a
//! multi-line record early on makes a row's line differ from its index.

use radargen::model::{Link, NOT_ON_THE_RADAR};
use radargen::options::Table;
use roxmltree::Node;

use super::{Built, build, parse};

/// CSV order differs from FR-34's (`Alpha` in `Platforms` numbers first),
/// `Old` is unplotted mid-file, `Zed`'s name and description span two lines,
/// `Alpha`'s ring is written in lower case and `Beta`'s link is padded and
/// upper-case.
const CSV: &str = "name,sector,ring,description,link,note\n\
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
    let got = keys(&b.page(Some("T")));
    let want = lines(&b, &b.plotted.table_rows());
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
        format!("Old, {NOT_ON_THE_RADAR}"),
        "the name, a comma for the ear, a space, the flag: never run together"
    );
}

#[test]
fn columns_are_the_number_the_three_labels_then_the_header() {
    let b = built();
    let doc_html = b.page(Some("T"));
    let table = table_of(&doc_html);
    let doc = parse(&table);
    assert_eq!(
        headings(&doc_html),
        [
            "#Number",
            "name",
            "Sector",
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

/// 15b: the name cell's control is a native button keyed on the row, its
/// text exactly 15a's cell text, so the button's name holds the flag.
#[test]
fn each_name_cell_holds_one_button() {
    let b = built();
    let html = b.page(Some("T"));
    let table = table_of(&html);
    let doc = parse(&table);
    for row in rows(&doc) {
        let header = row
            .children()
            .find(|n| n.has_tag_name("th"))
            .expect("a row header");
        let buttons: Vec<Node> = header
            .children()
            .filter(|n| n.has_tag_name("button"))
            .collect();
        let [button] = buttons.as_slice() else {
            panic!("one button: {}", text(header));
        };
        assert_eq!(button.attribute("type"), Some("button"));
        assert_eq!(button.attribute("data-row"), row.attribute("data-row"));
        assert_eq!(
            text(*button),
            text(header),
            "all the cell's text is the button's"
        );
    }
}

/// FR-72 and FR-94 in the table: a selected row's start border survives
/// forced colours, and the name button keeps a 24px target and its outline.
#[test]
fn the_table_marks_selection_and_keeps_its_button_usable() {
    let html = built().page(Some("T"));
    let css = super::html::style(&html);
    for rule in [
        "tr[data-selected]>:first-child{border-inline-start:3px solid var(--text-primary)}",
        "th button{all:unset;display:block;padding-block:3px;margin-block:-3px;cursor:pointer;",
        "tr[data-selected]>*{background:var(--surface-1)}",
        "th button:focus-visible{outline:2px solid var(--focus);outline-offset:2px}",
    ] {
        assert!(css.contains(rule), "{rule}");
    }
    assert!(
        !css.contains("\n.prose{"),
        "prose is scoped to the table's cells"
    );
}

/// A table wider than its region shows that it scrolls: shadows pinned to
/// the region's edges, covered at whichever end the reader has reached. The
/// table has no background of its own, which would hide them.
#[test]
fn the_table_shows_when_it_scrolls() {
    let html = built().page(Some("T"));
    let css = super::html::style(&html);
    assert!(
        css.contains(concat!(
            ".table-scroll{position:relative;overflow-x:auto;border-radius:6px;",
            "border:1px solid var(--stroke-divider);background:linear-gradient(to right,"
        )),
        "the scroll cue follows the region's border, its rule intact"
    );
    assert!(
        css.contains(
            "background-attachment:local,local,scroll,scroll;background-color:var(--surface-2)}"
        ),
        "the scroll cue"
    );
    assert!(
        css.contains("table{min-width:100%;border-collapse:collapse;font-size:.8125rem}"),
        "no table background over the cue"
    );
}

/// The column headings of `html`'s table, the number column first.
fn headings(html: &str) -> Vec<String> {
    let table = table_of(html);
    let doc = parse(&table);
    doc.descendants()
        .filter(|n| n.has_tag_name("th") && n.attribute("scope") == Some("col"))
        .map(text)
        .collect()
}

/// The CSV lines of `b`'s items at `indices`, as `data-row` keys.
fn lines(b: &Built, indices: &[usize]) -> Vec<String> {
    indices
        .iter()
        .map(|&i| b.catalogue.items.get(i).expect("an item").line.to_string())
        .collect()
}

/// The `data-row` keys of `html`'s table rows, in order.
fn keys(html: &str) -> Vec<String> {
    let table = table_of(html);
    let doc = parse(&table);
    rows(&doc)
        .iter()
        .filter_map(|r| r.attribute("data-row").map(str::to_owned))
        .collect()
}

/// FR-106: the default table holds the plotted rows only, in FR-34's order,
/// so the unplotted `Old` has no row and so no detail panel; `all` keeps it.
#[test]
fn the_radar_table_holds_only_plotted_rows() {
    let b = built();
    let plotted = lines(&b, &b.plotted.items);
    let radar = b.page_table(Some("T"), Table::Radar);
    assert_eq!(keys(&radar), plotted, "plotted rows, in FR-34's order");
    assert!(
        !table_of(&radar).contains("Old"),
        "the unplotted row is absent"
    );
    assert!(!radar.contains(NOT_ON_THE_RADAR), "so nothing is flagged");
    let all = b.page_table(Some("T"), Table::All);
    assert!(
        keys(&all).len() > plotted.len() && table_of(&all).contains("Old"),
        "--table all keeps the unplotted row"
    );
    assert_eq!(
        all,
        b.page(Some("T")),
        "the harness's page is --table all's"
    );
}

/// FR-106: the default table keeps the columns radargen reads, in FR-78's
/// order, and drops an extra one, from every row as from the header; `all`
/// keeps it.
#[test]
fn the_radar_table_drops_columns_radargen_does_not_read() {
    let b = built();
    let radar = b.page_table(Some("T"), Table::Radar);
    assert_eq!(
        headings(&radar),
        ["#Number", "name", "Sector", "ring", "description", "link"],
        "the extra note column is gone"
    );
    let table = table_of(&radar);
    let doc = parse(&table);
    for row in rows(&doc) {
        assert_eq!(cells(row).len(), 6, "one cell per heading: {}", text(row));
    }
    assert!(
        !table.contains("n1") && !table.contains("n2"),
        "no note value"
    );
    assert!(
        headings(&b.page_table(Some("T"), Table::All)).contains(&"note".to_owned()),
        "--table all keeps it"
    );
}

/// FR-106: every column radargen reads survives `radar`, whatever it is
/// called in the header, so the filter is by meaning and not by position.
#[test]
fn the_radar_table_keeps_every_column_radargen_reads() {
    let csv = "Ticket,name,sector,ring,description,qualifier,domain,rationale,last_affirmed,exit_date,status,link,Owner\n\
T-1,Kafka,Tools,Adopt,d,default,Streaming,why,2026-06-01,,New,https://k.example,Ana\n";
    let page = build(csv, None, None).page_table(None, Table::Radar);
    assert_eq!(
        headings(&page),
        [
            "#Number",
            "name",
            "Sector",
            "ring",
            "description",
            "qualifier",
            "domain",
            "rationale",
            "last_affirmed",
            "exit_date",
            "status",
            "link"
        ]
    );
    assert!(!page.contains("T-1") && !page.contains("Ana"), "{page}");
}

/// FR-106 with nothing plotted: the default table keeps its heading and
/// column headings and says so in one row spanning them all, so the region
/// and its skip target stay. A headers-only CSV reads the same. Under
/// `all`, the unplotted rows are the table, and a headers-only CSV keeps its
/// empty body, as before FR-106.
#[test]
fn the_radar_table_with_nothing_plotted_says_so() {
    let nothing = build(
        "name,sector,ring,description\nOld,Tools,Gone,d\n",
        None,
        None,
    );
    let page = nothing.page_table(None, Table::Radar);
    assert!(keys(&page).is_empty(), "no item row");
    assert!(!page.contains("Old"), "the unplotted row is not published");
    assert_eq!(
        headings(&page),
        ["#Number", "name", "Sector", "ring", "description"]
    );
    assert!(
        page.contains(
            "<tbody>\n<tr><td colspan=\"5\">Nothing is on the radar.</td></tr>\n</tbody>"
        ),
        "one row spanning the five columns"
    );
    assert!(
        page.contains("class=\"table-scroll\" role=\"region\""),
        "the region stays"
    );
    let all = nothing.page_table(None, Table::All);
    assert_eq!(keys(&all).len(), 1, "all lists the unplotted row");
    assert!(!all.contains("Nothing is on the radar."));
    let empty = build("name,sector,ring,description\n", None, None);
    assert!(
        empty
            .page_table(None, Table::Radar)
            .contains("<td colspan=\"5\">Nothing is on the radar.</td>")
    );
    assert!(
        empty
            .page_table(None, Table::All)
            .contains("<tbody>\n</tbody>"),
        "all's empty body is unchanged"
    );
}

/// FR-106: the heading, which names the region, says what the table holds:
/// the items on the radar by default, every item under `all`.
#[test]
fn the_table_heading_names_what_it_holds() {
    let b = built();
    let heading = |html: &str| {
        let at = html
            .find("<h2 class=\"table-heading\" id=\"table-heading\">")
            .expect("the heading");
        let rest = html.get(at..).unwrap_or_default();
        let open = rest.find('>').expect("opens") + 1;
        let close = rest.find("</h2>").expect("closes");
        rest.get(open..close).unwrap_or_default().to_owned()
    };
    assert_eq!(
        heading(&b.page_table(Some("T"), Table::Radar)),
        "Items on the radar"
    );
    assert_eq!(heading(&b.page_table(Some("T"), Table::All)), "Every item");
    assert!(
        b.page_table(Some("T"), Table::Radar)
            .contains("role=\"region\" aria-labelledby=\"table-heading\""),
        "the heading names the region"
    );
}
