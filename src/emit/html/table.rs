//! FR-41's table view: every CSV row and every named column (FR-78, D-80),
//! under a visible heading and a native `<details>` toggle, so it needs no
//! script. PR 15b builds the detail panel from these rows, so each item's
//! prose is in the page once.
//!
//! Every value is escaped where it is written (NFR-6): text with
//! [`xml_text`], an `href` with [`html_attr`]. Rule fragments are module
//! constants, as in `css.rs`.

use crate::escape::{html_attr, xml_text};
use crate::model::{Catalogue, ColumnKind, Item, Link, NOT_ON_THE_RADAR};
use crate::num::fmt_coord;
use crate::order::Plotted;
use crate::palette::Neutral;
use crate::style::MARGIN;
use crate::svg::serialise::DATA_ROW;
use crate::trim;

/// The heading's `id`, written once outside both arrangements (architecture
/// §5's exception): it names the table and its scroll region.
const HEADING_ID: &str = "table-heading";
/// The heading's words and the toggle's.
const HEADING: &str = "Every item";
const SUMMARY: &str = "Show the table";

const OPEN_HEADING: &str = "<h2 class=\"table-heading\" id=\"";
const OPEN_TOGGLE: &str = "</h2>\n<details class=\"table-view\"><summary>";
const OPEN_REGION: &str =
    "</summary>\n<div class=\"table-scroll\" role=\"region\" aria-labelledby=\"";
const OPEN_TABLE: &str = "\" tabindex=\"0\">\n<table aria-labelledby=\"";
const OPEN_HEAD: &str = "\">\n<thead><tr>";
/// The number column: `#` to the eye, "Number" to a screen reader, which
/// may read a bare `#` as "pound" or skip it.
const NUMBER_HEADING: &str = "<th scope=\"col\"><span aria-hidden=\"true\">#</span><span class=\"visually-hidden\">Number</span></th>";
const OPEN_BODY: &str = "</tr></thead>\n<tbody>\n";
const CLOSE: &str = "</tbody></table></div></details>\n";
const NEW_TAB: &str = "<span class=\"visually-hidden\"> (opens in a new tab)</span>";

/// The table view, for `<main>` after both arrangements.
pub(super) fn table(catalogue: &Catalogue, plotted: &Plotted) -> String {
    let columns = order(catalogue);
    let head: String = columns
        .iter()
        .map(|&c| column_heading(catalogue, c))
        .collect();
    let body: String = plotted
        .table_rows()
        .into_iter()
        .filter_map(|i| {
            let item = catalogue.items.get(i)?;
            let number = plotted.numbers.get(i).copied().flatten();
            Some(row(catalogue, &columns, item, number))
        })
        .collect();
    [
        OPEN_HEADING,
        HEADING_ID,
        "\">",
        HEADING,
        OPEN_TOGGLE,
        SUMMARY,
        OPEN_REGION,
        HEADING_ID,
        OPEN_TABLE,
        HEADING_ID,
        OPEN_HEAD,
        NUMBER_HEADING,
        &head,
        OPEN_BODY,
        &body,
        CLOSE,
    ]
    .concat()
}

/// FR-78's columns, as indices into [`Catalogue::columns`]: `name`,
/// `quadrant`, `ring`, then every other named column in header order,
/// `description` included (D-80).
fn order(catalogue: &Catalogue) -> Vec<usize> {
    let first: Vec<usize> = [ColumnKind::Name, ColumnKind::Quadrant, ColumnKind::Ring]
        .into_iter()
        .filter_map(|kind| catalogue.column(kind))
        .collect();
    let rest = (0..catalogue.columns.len()).filter(|c| !first.contains(c));
    first.iter().copied().chain(rest).collect()
}

/// A column's heading: the header cell as written, trimmed (FR-27).
fn column_heading(catalogue: &Catalogue, column: usize) -> String {
    let heading = catalogue
        .columns
        .get(column)
        .map_or("", |c| trim(&c.heading));
    ["<th scope=\"col\">", &xml_text(heading), "</th>"].concat()
}

/// One item's row, keyed on its CSV line as its marker and legend row are
/// (architecture §5). `number` is FR-18's, `None` for an unplotted row.
fn row(catalogue: &Catalogue, columns: &[usize], item: &Item, number: Option<usize>) -> String {
    let class = if number.is_some() {
        ""
    } else {
        " class=\"unplotted\""
    };
    let cells: String = columns.iter().map(|&c| cell(catalogue, c, item)).collect();
    [
        "<tr ",
        DATA_ROW,
        "=\"",
        &item.line.to_string(),
        "\"",
        class,
        "><td class=\"number\">",
        &number.map(|n| n.to_string()).unwrap_or_default(),
        "</td>",
        &cells,
        "</tr>\n",
    ]
    .concat()
}

/// One cell of `item` under `column`.
///
/// The name is the row's header, flagged when unplotted. Quadrant and ring
/// show the labels the legend uses (FR-9, FR-17); a link shows what the
/// model decided (FR-96); prose keeps its line breaks (FR-97), and every
/// other value is shown trimmed (D-76).
fn cell(catalogue: &Catalogue, column: usize, item: &Item) -> String {
    let kind = catalogue
        .columns
        .get(column)
        .map_or(ColumnKind::Other, |c| c.kind);
    let value = item.cell(column).map_or("", trim);
    match kind {
        ColumnKind::Name => name_cell(item),
        ColumnKind::Quadrant => {
            let label = catalogue
                .quadrants
                .get(item.quadrant)
                .map_or("", |q| q.label.as_str());
            ["<td>", &xml_text(label), "</td>"].concat()
        }
        ColumnKind::Ring => ["<td>", &xml_text(catalogue.ring_label(item)), "</td>"].concat(),
        ColumnKind::Link => link_cell(item.link.as_ref()),
        ColumnKind::Description
        | ColumnKind::Domain
        | ColumnKind::Rationale
        | ColumnKind::Other => ["<td class=\"prose\">", &xml_text(value), "</td>"].concat(),
        ColumnKind::Qualifier
        | ColumnKind::Status
        | ColumnKind::LastAffirmed
        | ColumnKind::ExitDate => ["<td>", &xml_text(value), "</td>"].concat(),
    }
}

/// The row header: the name, and for an unplotted row FR-41's flag after a
/// space, so the text reads "Old not on the radar" and never runs together.
fn name_cell(item: &Item) -> String {
    let flag = if item.ring.is_some() {
        String::new()
    } else {
        [" <span class=\"flag\">", NOT_ON_THE_RADAR, "</span>"].concat()
    };
    [
        "<th scope=\"row\">",
        &xml_text(trim(&item.name)),
        &flag,
        "</th>",
    ]
    .concat()
}

/// FR-96: an address is a link that opens in a new tab and says so; refused
/// text is text (D-77).
fn link_cell(link: Option<&Link>) -> String {
    let inner = match link {
        Some(Link::Address(address)) => [
            // `html_attr` writes the quotes, so the value is never unquoted.
            "<a href=",
            &html_attr(address),
            " target=\"_blank\" rel=\"noopener noreferrer\">",
            &xml_text(address),
            NEW_TAB,
            "</a>",
        ]
        .concat(),
        Some(Link::Text(text)) => xml_text(text),
        None => String::new(),
    };
    ["<td class=\"link\">", &inner, "</td>"].concat()
}

const CAP_OPEN: &str = ".table-heading,.table-view{max-width:";
const CAP_REST: &str = "px;margin-inline:auto;padding-inline:";
const CAP_CLOSE: &str = "px;box-sizing:border-box}\n";
const HEADING_RULE: &str = ".table-heading{margin-block:1.5em .25em;font-size:1rem;font-weight:600}\n.table-view{margin-block-end:1.5em}\n";
const SUMMARY_RULE: &str =
    "summary{cursor:pointer;width:fit-content;padding-block:.5em;font-weight:600}\n";
const SCROLL_OPEN: &str =
    ".table-scroll{position:relative;overflow-x:auto;border-radius:6px;border:1px solid var(";
const TABLE_OPEN: &str =
    ")}\ntable{min-width:100%;border-collapse:collapse;font-size:.8125rem;background:var(";
const CELL_OPEN: &str =
    ")}\nth,td{text-align:start;vertical-align:top;padding:.4em .6em;border-bottom:1px solid var(";
const HEAD_OPEN: &str =
    ")}\nth{font-weight:600}\nthead th{font-size:.75rem;border-bottom-width:2px;color:var(";
const NUMBER_RULE: &str = ")}\n.number{text-align:end;font-variant-numeric:tabular-nums}\n.prose{white-space:pre-line;min-width:16em}\n.link{overflow-wrap:anywhere;min-width:12em}\n";
const FLAG_OPEN: &str = ".flag{display:inline-block;padding:0 .45em;border:1px solid currentColor;border-radius:1em;font-size:.8em;font-weight:600;white-space:nowrap;color:var(";
const BOUNDARY_OPEN: &str = ")}\ntr:not(.unplotted)+tr.unplotted>*{border-top:2px solid var(";
const LINK_OPEN: &str = ")}\na,a:visited{text-decoration:underline;color:var(";
const FOCUS_OPEN: &str =
    ")}\nsummary:focus-visible,.table-scroll:focus-visible,a:focus-visible{outline:2px solid var(";
const FOCUS_CLOSE: &str = ");outline-offset:2px}\n";

/// The table view's rules, capped at the wide document's width and inset by
/// its margin, so the table's edge lines up with the radar's content.
///
/// Links take the body text colour, underlined: a browser's default blue is
/// 1.70:1 on the dark Surface2, while the text colour is checked at 4.5:1 in
/// both themes, and the underline keeps a link distinct (WCAG 1.4.1). No
/// height is fixed anywhere, so 1.4.12's spacing clips nothing (D-78).
pub(super) fn css(wide_width: f64) -> String {
    [
        CAP_OPEN,
        &fmt_coord(wide_width),
        CAP_REST,
        &fmt_coord(MARGIN),
        CAP_CLOSE,
        HEADING_RULE,
        SUMMARY_RULE,
        SCROLL_OPEN,
        Neutral::StrokeDivider.as_str(),
        TABLE_OPEN,
        Neutral::Surface2.as_str(),
        CELL_OPEN,
        Neutral::StrokeDivider.as_str(),
        HEAD_OPEN,
        Neutral::TextMuted.as_str(),
        NUMBER_RULE,
        FLAG_OPEN,
        Neutral::TextMuted.as_str(),
        BOUNDARY_OPEN,
        Neutral::StrokeDivider.as_str(),
        LINK_OPEN,
        Neutral::TextPrimary.as_str(),
        FOCUS_OPEN,
        Neutral::Focus.as_str(),
        FOCUS_CLOSE,
    ]
    .concat()
}
