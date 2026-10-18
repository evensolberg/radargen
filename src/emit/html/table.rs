//! FR-41's table view, under a visible heading and a native `<details>`
//! toggle, so it needs no script. By default it publishes what the radar
//! draws, the plotted rows and the columns radargen reads; `--table all`
//! publishes every CSV row and every named column (FR-78, FR-106, D-80,
//! D-95). PR 15b builds the detail panel from these rows, so each item's
//! prose is in the page once, and a row or column left out of the table is
//! left out of the panel too.
//!
//! Every value is escaped where it is written (NFR-6): text with
//! [`xml_text`], an `href` with [`html_attr`]. Rule fragments are module
//! constants, as in `css.rs`.

use super::Table;
use super::chrome::DATA_SELECTED;
use super::css::{OUTLINE_CLOSE, Wide, capped};
use crate::escape::{html_attr, xml_text};
use crate::model::{Catalogue, ColumnKind, Item, Link, NOT_ON_THE_RADAR};
use crate::order::Plotted;
use crate::palette::Neutral;
use crate::svg::serialise::DATA_ROW;
use crate::trim;

/// The heading's `id`, written once outside both arrangements (architecture
/// §5's exception): it names the table and its scroll region.
const HEADING_ID: &str = "table-heading";
/// The heading's words and the toggle's.
const HEADING: &str = "Every item";
/// Neutral, since a screen reader adds "collapsed" or "expanded".
const SUMMARY: &str = "The table";

const OPEN_HEADING: &str = "<h2 class=\"table-heading\" id=\"";
/// The table view's `<details>` (FR-41). The script finds it by this class:
/// while it is closed, its `summary` takes focus back when the panel closes.
pub(super) const TABLE_VIEW_CLASS: &str = "table-view";
/// A prose cell's class: its line breaks show, here and in the panel built
/// from it (FR-97, D-80).
pub(super) const PROSE_CLASS: &str = "prose";
const OPEN_TOGGLE: &str = "</h2>\n<details class=\"";
const TOGGLE_REST: &str = "\"><summary>";
const OPEN_REGION: &str =
    "</summary>\n<div class=\"table-scroll\" role=\"region\" aria-labelledby=\"";
/// Only the region is named by the heading: naming the table too would
/// announce "Every item" a third time in a row.
const OPEN_TABLE: &str = "\" tabindex=\"0\">\n<table>\n<thead><tr>";
/// The number column: `#` to the eye, "Number" to a screen reader, which
/// may read a bare `#` as "pound" or skip it.
const NUMBER_HEADING: &str = "<th scope=\"col\"><span aria-hidden=\"true\">#</span><span class=\"visually-hidden\">Number</span></th>";
const OPEN_BODY: &str = "</tr></thead>\n<tbody>\n";
const CLOSE: &str = "</tbody></table></div></details>\n";
const CLASS_OPEN: &str = " class=\"";
const CLASS_CLOSE: &str = "\"";
const NEW_TAB: &str = "<span class=\"visually-hidden\"> (opens in a new tab)</span>";

/// The table view publishing `publish`, for `<main>` after both
/// arrangements.
pub(super) fn table(catalogue: &Catalogue, plotted: &Plotted, publish: Table) -> String {
    let columns = order(catalogue, publish);
    let head: String = columns
        .iter()
        .map(|&c| column_heading(catalogue, c))
        .collect();
    // FR-106: `radar` lists FR-34's plotted items alone; `all` follows them
    // with the unplotted rows (FR-78).
    let rows = match publish {
        Table::Radar => plotted.items.clone(),
        Table::All => plotted.table_rows(),
    };
    let body: String = rows
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
        TABLE_VIEW_CLASS,
        TOGGLE_REST,
        SUMMARY,
        OPEN_REGION,
        HEADING_ID,
        OPEN_TABLE,
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
/// `description` included (D-80). Under `radar`, a column radargen does not
/// read ([`ColumnKind::Other`]) is left out (FR-106).
fn order(catalogue: &Catalogue, publish: Table) -> Vec<usize> {
    let first: Vec<usize> = [ColumnKind::Name, ColumnKind::Quadrant, ColumnKind::Ring]
        .into_iter()
        .filter_map(|kind| catalogue.column(kind))
        .collect();
    let published = |c: &usize| {
        publish == Table::All
            || catalogue
                .columns
                .get(*c)
                .is_some_and(|column| column.kind != ColumnKind::Other)
    };
    let rest = (0..catalogue.columns.len()).filter(|c| !first.contains(c) && published(c));
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
    let cells: String = columns
        .iter()
        .map(|&c| cell(catalogue, c, item, number.is_none()))
        .collect();
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
/// The name is the row's header, flagged when `unplotted`. Quadrant and
/// ring show the labels the legend uses (FR-9, FR-17); a link shows what the
/// model decided (FR-96). Prose is verbatim, keeping its line breaks and
/// blank lines (FR-97, D-76: its whitespace may be deliberate); the other
/// labels are shown trimmed (D-76).
fn cell(catalogue: &Catalogue, column: usize, item: &Item, unplotted: bool) -> String {
    let kind = catalogue
        .columns
        .get(column)
        .map_or(ColumnKind::Other, |c| c.kind);
    let value = item.cell(column).unwrap_or_default();
    match kind {
        ColumnKind::Name => name_cell(item, unplotted),
        ColumnKind::Quadrant => td(
            None,
            catalogue
                .quadrants
                .get(item.quadrant)
                .map_or("", |q| q.label.as_str()),
        ),
        ColumnKind::Ring => td(None, catalogue.ring_label(item)),
        ColumnKind::Link => link_cell(item.link.as_ref()),
        ColumnKind::Description
        | ColumnKind::Domain
        | ColumnKind::Rationale
        | ColumnKind::Other => td(Some(PROSE_CLASS), value),
        ColumnKind::Qualifier
        | ColumnKind::Status
        | ColumnKind::LastAffirmed
        | ColumnKind::ExitDate => td(None, trim(value)),
    }
}

/// A data cell holding `text`, escaped, with `class` when given.
fn td(class: Option<&str>, text: &str) -> String {
    let class = class.map_or_else(String::new, |c| [CLASS_OPEN, c, CLASS_CLOSE].concat());
    ["<td", &class, ">", &xml_text(text), "</td>"].concat()
}

/// The row header: a native button keyed on the row, holding the name and,
/// for an unplotted row, FR-41's flag after a space, so the text and the
/// button's name read "Old not on the radar" and never run together (D-66,
/// WCAG 2.5.3). The script adds its `aria-expanded` and `aria-controls`, so
/// without it the button claims no state.
fn name_cell(item: &Item, unplotted: bool) -> String {
    let flag = if unplotted {
        // A comma for the ear only, so the name and the flag are heard apart.
        [
            "<span class=\"visually-hidden\">,</span> <span class=\"flag\">",
            NOT_ON_THE_RADAR,
            "</span>",
        ]
        .concat()
    } else {
        String::new()
    };
    [
        "<th scope=\"row\"><button type=\"button\" ",
        DATA_ROW,
        "=\"",
        &item.line.to_string(),
        "\">",
        &xml_text(&item.name),
        &flag,
        "</button></th>",
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

const CAP_OPEN: &str = ".table-heading,.table-view{";
const HEADING_RULE: &str = ".table-heading{margin-block:1.5em .25em;font-size:1rem;font-weight:600}\n.table-view{margin-block-end:1.5em}\n";
const SUMMARY_RULE: &str =
    "summary{cursor:pointer;width:fit-content;padding-block:.5em;font-weight:600}\n";
const SCROLL_OPEN: &str =
    ".table-scroll{position:relative;overflow-x:auto;border-radius:6px;border:1px solid var(";
/// The scroll cue: a shadow pinned to each edge of the region, and a cover
/// in the surface colour that scrolls with the table and hides the shadow at
/// whichever end the reader has reached. So a table wider than the region
/// shows that it scrolls, even under overlay scrollbars that show nothing.
/// The table itself has no background, or it would cover the shadows.
const CUE_OPEN: &str = ");background:linear-gradient(to right,var(";
const CUE_LEFT_COVER: &str = ") 30%,transparent) left/2.5em 100%,linear-gradient(to left,var(";
const CUE_RIGHT_COVER: &str = ") 30%,transparent) right/2.5em 100%,radial-gradient(farthest-side at 0 50%,color-mix(in srgb,var(";
const CUE_LEFT_SHADOW: &str = ") 45%,transparent),transparent) left/.9em 100%,radial-gradient(farthest-side at 100% 50%,color-mix(in srgb,var(";
const CUE_RIGHT_SHADOW: &str = ") 45%,transparent),transparent) right/.9em 100%;background-repeat:no-repeat;background-attachment:local,local,scroll,scroll;background-color:var(";
const TABLE_OPEN: &str = ")}\ntable{min-width:100%;border-collapse:collapse;font-size:.8125rem";
const CELL_OPEN: &str =
    "}\nth,td{text-align:start;vertical-align:top;padding:.4em .6em;border-bottom:1px solid var(";
const HEAD_OPEN: &str =
    ")}\nth{font-weight:600}\nthead th{font-size:.75rem;border-bottom-width:2px;color:var(";
const NUMBER_RULE: &str = ")}\n.number{text-align:end;font-variant-numeric:tabular-nums}\ntd.prose{white-space:pre-line;min-width:16em}\ntd.link{overflow-wrap:anywhere;min-width:12em}\n";
const FLAG_OPEN: &str = ".flag{display:inline-block;padding:0 .45em;border:1px solid currentColor;border-radius:1em;font-size:.8em;font-weight:600;white-space:nowrap;color:var(";
const BOUNDARY_OPEN: &str = ")}\ntr:not(.unplotted)+tr.unplotted>*{border-top:2px solid var(";
const LINK_OPEN: &str = ")}\na,a:visited{text-decoration:underline;color:var(";
/// A selected row's mark: a start border, which forced colours keep where
/// they drop backgrounds and shadows (FR-72).
const SELECTED_OPEN: &str = ")}\ntr[";
const SELECTED_REST: &str = "]>:first-child{border-inline-start:3px solid var(";
const SELECTED_TINT: &str = ")}\ntr[";
const TINT_REST: &str = "]>*{background:var(";
/// The name cell's button looks like the 15a text it holds, with a dotted
/// underline that says it does something, solid on hover and unlike a
/// link's. Padding keeps a 24px target (FR-94) on the cell's baseline, which
/// a minimum height, centring the text, would not. The focus rule below is more
/// specific, so the reset never removes the outline.
const BUTTON_RESET: &str = ")}\nth button{all:unset;display:block;padding-block:3px;margin-block:-3px;cursor:pointer;text-decoration:underline dotted;text-underline-offset:.2em}\nth button:hover{text-decoration-style:solid}\n";
const FOCUS_OPEN: &str = "summary:focus-visible,.table-scroll:focus-visible,a:focus-visible,th button:focus-visible{outline:2px solid var(";

/// The table view's rules, capped as the wide arrangement is and inset by
/// its margin, so the table's edge lines up with the radar's content.
///
/// Links take the body text colour, underlined: a browser's default blue is
/// 1.70:1 on the dark Surface2, while the text colour is checked at 4.5:1 in
/// both themes, and the underline keeps a link distinct (WCAG 1.4.1). No
/// height is fixed anywhere, so 1.4.12's spacing clips nothing (D-78).
pub(super) fn css(wide: &Wide) -> String {
    [
        &capped(CAP_OPEN, wide),
        HEADING_RULE,
        SUMMARY_RULE,
        SCROLL_OPEN,
        Neutral::StrokeDivider.as_str(),
        CUE_OPEN,
        Neutral::Surface2.as_str(),
        CUE_LEFT_COVER,
        Neutral::Surface2.as_str(),
        CUE_RIGHT_COVER,
        Neutral::TextMuted.as_str(),
        CUE_LEFT_SHADOW,
        Neutral::TextMuted.as_str(),
        CUE_RIGHT_SHADOW,
        Neutral::Surface2.as_str(),
        TABLE_OPEN,
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
        SELECTED_OPEN,
        DATA_SELECTED,
        SELECTED_REST,
        Neutral::TextPrimary.as_str(),
        SELECTED_TINT,
        DATA_SELECTED,
        TINT_REST,
        Neutral::Surface1.as_str(),
        BUTTON_RESET,
        FOCUS_OPEN,
        Neutral::Focus.as_str(),
        OUTLINE_CLOSE,
    ]
    .concat()
}
