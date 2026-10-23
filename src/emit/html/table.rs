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
use crate::model::{Catalogue, Link, NOT_ON_THE_RADAR};
use crate::order::Plotted;
use crate::palette::Neutral;
use crate::published::{
    Cell, NOTHING_PLOTTED, NUMBER_HEADING, NUMBER_SPOKEN, PublishedRow, published,
};
use crate::svg::serialise::DATA_ROW;

/// The heading's `id`, written once outside both arrangements (architecture
/// §5's exception): it names the table and its scroll region.
const HEADING_ID: &str = "table-heading";
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
/// announce the heading's words a third time in a row.
const OPEN_TABLE: &str = "\" tabindex=\"0\">\n<table>\n<thead><tr>";
/// The number column's heading around [`NUMBER_HEADING`] and
/// [`NUMBER_SPOKEN`]: the mark to the eye, the word to a screen reader.
const NUMBER_OPEN: &str = "<th scope=\"col\"><span aria-hidden=\"true\">";
const NUMBER_SPOKEN_OPEN: &str = "</span><span class=\"visually-hidden\">";
const NUMBER_CLOSE: &str = "</span></th>";
const OPEN_BODY: &str = "</tr></thead>\n<tbody>\n";
const CLOSE: &str = "</tbody></table></div></details>\n";
const CLASS_OPEN: &str = " class=\"";
const CLASS_CLOSE: &str = "\"";
const NEW_TAB: &str = "<span class=\"visually-hidden\"> (opens in a new tab)</span>";
/// The default table's one row when nothing is plotted (FR-106), spanning
/// every column so the region and its skip target stay.
const EMPTY_OPEN: &str = "<tr><td colspan=\"";
const EMPTY_REST: &str = "\">";
const EMPTY_CLOSE: &str = "</td></tr>\n";

/// The table view publishing `publish`, for `<main>` after both
/// arrangements. What it publishes is [`published`]'s; this writes it.
pub(super) fn table(catalogue: &Catalogue, plotted: &Plotted, publish: Table) -> String {
    let table = published(catalogue, plotted, publish);
    let head: String = table
        .columns
        .iter()
        .map(|(_, heading)| ["<th scope=\"col\">", &xml_text(heading), "</th>"].concat())
        .collect();
    // FR-106: `radar` says when nothing is plotted; `all`'s empty body,
    // from a headers-only CSV, is left as it was, so its page is unchanged
    // byte for byte.
    let body: String = if table.says_empty {
        [
            EMPTY_OPEN,
            &(table.columns.len() + 1).to_string(),
            EMPTY_REST,
            NOTHING_PLOTTED,
            EMPTY_CLOSE,
        ]
        .concat()
    } else {
        table.rows.iter().map(row).collect()
    };
    [
        OPEN_HEADING,
        HEADING_ID,
        "\">",
        table.heading,
        OPEN_TOGGLE,
        TABLE_VIEW_CLASS,
        TOGGLE_REST,
        SUMMARY,
        OPEN_REGION,
        HEADING_ID,
        OPEN_TABLE,
        NUMBER_OPEN,
        NUMBER_HEADING,
        NUMBER_SPOKEN_OPEN,
        NUMBER_SPOKEN,
        NUMBER_CLOSE,
        &head,
        OPEN_BODY,
        &body,
        CLOSE,
    ]
    .concat()
}

/// One item's row, keyed on its CSV line as its marker and legend row are
/// (architecture §5).
// The unplotted branches here and in `name_cell`, and the `.unplotted` and
// `.flag` rules, serve `--table all` alone; they stay in every page so
// `all`'s output is unchanged byte for byte (FR-106).
fn row(row: &PublishedRow<'_>) -> String {
    let class = if row.number.is_some() {
        ""
    } else {
        " class=\"unplotted\""
    };
    let cells: String = row.cells.iter().map(|c| cell(c, row.line)).collect();
    [
        "<tr ",
        DATA_ROW,
        "=\"",
        &row.line.to_string(),
        "\"",
        class,
        "><td class=\"number\">",
        &row.number.map(|n| n.to_string()).unwrap_or_default(),
        "</td>",
        &cells,
        "</tr>\n",
    ]
    .concat()
}

/// One published cell of the row on `line`: the name as the row header,
/// prose in a cell that keeps its line breaks (FR-97), a link as FR-96
/// writes it, a label as text.
fn cell(cell: &Cell<'_>, line: usize) -> String {
    match *cell {
        Cell::Name { name, unplotted } => name_cell(name, line, unplotted),
        Cell::Label(text) => td(None, text),
        Cell::Prose(text) => td(Some(PROSE_CLASS), text),
        Cell::Link(address) => link_cell(address),
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
fn name_cell(name: &str, line: usize, unplotted: bool) -> String {
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
        &line.to_string(),
        "\">",
        &xml_text(name),
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
