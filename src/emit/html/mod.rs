//! The HTML page (FR-52, FR-68, FR-69, FR-80): both arrangements, drawn
//! interactive, with one shown at a time, both palettes, and Inter inlined.

mod chrome;
mod css;
mod table;

use crate::emit::OUTPUT_VERSION;
use crate::emit::UNTITLED;
use crate::escape::xml_text;
use crate::layout::arrange::{NarrowParts, WideParts};
use crate::model::Catalogue;
use crate::order::Plotted;
use crate::palette::Colours;
use crate::svg::{Document, Target, serialise};

const HEAD_OPEN: &str = "<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n<meta name=\"generator\" content=\"";
const BODY_OPEN: &str = "</style>\n</head>\n<body>\n";
const HIDDEN_HEADING: &str = "<h1 class=\"visually-hidden\">";
const DIV_OPEN: &str = "<div class=\"";
const WIDE_REST: &str = " wide\">\n";
const NARROW_REST: &str = " narrow\">\n";
const DIV_CLOSE: &str = "\n</div>\n";
const OPEN_REST: &str = "\">\n";
/// The legend's box is out of the tab order: its rows are focusable and
/// scroll it, and Firefox would otherwise stop on the box itself, unnamed.
const UNTABBED_REST: &str = "\" tabindex=\"-1\">\n";
const MAIN_CLOSE: &str = "</main>\n";
const BODY_CLOSE: &str = "</body>\n</html>\n";

/// Everything the page is written from: the two arrangements of one
/// `Blocks` (FR-68), the wide one in its parts (D-85), and the catalogue
/// and its order, which the table view reads (FR-78).
#[derive(Debug, Clone, Copy)]
pub struct Page<'a> {
    /// The wide arrangement, in the parts HTML lays out (D-85).
    pub wide: &'a WideParts,
    /// The narrow arrangement, in the parts the page hides the legend by
    /// (RG-102).
    pub narrow: &'a NarrowParts,
    /// Every row, as the model read it.
    pub catalogue: &'a Catalogue,
    /// The catalogue's order and numbers.
    pub plotted: &'a Plotted,
    /// FR-65's resolved title; `None` names the page [`UNTITLED`].
    pub title: Option<&'a str>,
    /// FR-56, §6.6: the built-in themes with a toggle, or a `--palette`
    /// file's one set without.
    pub colours: &'a Colours,
}

/// The page for `input` (FR-68).
///
/// Both are serialised interactive, binding colours to custom properties
/// the stylesheet declares (FR-56, architecture §5). The bound palette
/// names them and cycles the series: the built-in light one, whose series
/// both themes share, or a `--palette` file's, with its own length.
///
/// `input.title` is FR-65's resolved title, `None` when absent or blank:
/// the page is then named [`UNTITLED`], with a visually hidden level-1 heading,
/// since the drawn document has none (D-75). With a title, the SVG's own
/// title is the level-1 heading.
#[must_use]
pub fn page(input: &Page<'_>) -> String {
    let Page {
        wide,
        narrow,
        catalogue,
        plotted,
        title,
        colours,
    } = *input;
    let bound = colours.interactive();
    let toggle = colours.has_toggle();
    let target = Target::Interactive(&bound);
    let sizes = css::Wide::of(wide);
    let heading = if title.is_some() {
        String::new()
    } else {
        [HIDDEN_HEADING, UNTITLED, "</h1>\n"].concat()
    };
    [
        HEAD_OPEN,
        OUTPUT_VERSION,
        "\">\n<title>",
        &xml_text(title.unwrap_or(UNTITLED)),
        "</title>\n<style>\n",
        &css::font_faces(),
        &css::palettes(colours),
        &css::layout(&sizes, narrow.body.width),
        &css::indicators(),
        css::VISUALLY_HIDDEN,
        css::HIDDEN_BOX,
        &table::css(&sizes),
        &chrome::css(&sizes, toggle),
        BODY_OPEN,
        &chrome::main_open(&css::narrow_query(&sizes)),
        &chrome::controls(toggle),
        &heading,
        DIV_OPEN,
        chrome::ARRANGEMENT_CLASS,
        WIDE_REST,
        &wide_parts(wide, target),
        DIV_CLOSE,
        DIV_OPEN,
        chrome::ARRANGEMENT_CLASS,
        NARROW_REST,
        &narrow_parts(narrow, target),
        DIV_CLOSE,
        &chrome::panel(),
        &table::table(catalogue, plotted),
        MAIN_CLOSE,
        &chrome::script(),
        BODY_CLOSE,
    ]
    .concat()
}

/// The wide arrangement's parts in reading order (FR-68): title, radar,
/// key, then the legend, its container scrolling. The stylesheet's grid
/// puts the legend beside the radar (D-85).
fn wide_parts(parts: &WideParts, target: Target<'_>) -> String {
    let part = |class: &str, rest: &str, document: &Document| {
        [
            DIV_OPEN,
            class,
            rest,
            &serialise(document, target),
            DIV_CLOSE,
        ]
        .concat()
    };
    [
        part(css::WIDE_TITLE, OPEN_REST, &parts.title),
        part(css::WIDE_RADAR, OPEN_REST, &parts.radar),
        part(css::WIDE_KEY, OPEN_REST, &parts.key),
        part(css::WIDE_LEGEND, UNTABBED_REST, &parts.legend),
    ]
    .concat()
}

/// The narrow arrangement's parts in reading order (FR-68): the body, then
/// the legend.
fn narrow_parts(parts: &NarrowParts, target: Target<'_>) -> String {
    let part = |class: &str, document: &Document| {
        [
            DIV_OPEN,
            class,
            OPEN_REST,
            &serialise(document, target),
            DIV_CLOSE,
        ]
        .concat()
    };
    [
        part(css::NARROW_BODY, &parts.body),
        part(css::NARROW_LEGEND, &parts.legend),
    ]
    .concat()
}
