//! The HTML page (FR-52, FR-68, FR-69, FR-80): both arrangements, drawn
//! interactive, with one shown at a time, both palettes, and Inter inlined.

mod chrome;
mod css;
mod table;

use crate::emit::OUTPUT_VERSION;
use crate::escape::xml_text;
use crate::model::Catalogue;
use crate::order::Plotted;
use crate::palette::{Palette, Theme};
use crate::svg::{Document, Target, serialise};

/// The tab title and hidden heading of a page with no title (FR-65, D-75).
pub const UNTITLED: &str = "Technology Radar";

const HEAD_OPEN: &str = "<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n<meta name=\"generator\" content=\"";
const BODY_OPEN: &str = "</style>\n</head>\n<body>\n";
const HIDDEN_HEADING: &str = "<h1 class=\"visually-hidden\">";
const DIV_OPEN: &str = "<div class=\"";
const WIDE_REST: &str = " wide\">\n";
const NARROW_REST: &str = " narrow\">\n";
const DIV_CLOSE: &str = "\n</div>\n";
const MAIN_CLOSE: &str = "</main>\n";
const BODY_CLOSE: &str = "</body>\n</html>\n";

/// Everything the page is written from: the two arrangements of one
/// `Blocks` (FR-68), and the catalogue and its order, which the table view
/// reads (FR-78).
#[derive(Debug, Clone, Copy)]
pub struct Page<'a> {
    /// The wide arrangement.
    pub wide: &'a Document,
    /// The narrow arrangement.
    pub narrow: &'a Document,
    /// Every row, as the model read it.
    pub catalogue: &'a Catalogue,
    /// The catalogue's order and numbers.
    pub plotted: &'a Plotted,
    /// FR-65's resolved title; `None` names the page [`UNTITLED`].
    pub title: Option<&'a str>,
}

/// The page for `input` (FR-68).
///
/// Both are serialised interactive, binding colours to custom properties
/// the stylesheet declares for both themes (FR-56, architecture §5). The
/// palette passed only names them and cycles the series, which the two
/// built-in themes share.
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
    } = *input;
    let target = Target::Interactive(&Palette::built_in(Theme::Light));
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
        &css::palettes(),
        &css::layout(wide.width, narrow.width),
        &css::indicators(),
        css::VISUALLY_HIDDEN,
        css::HIDDEN_BOX,
        &table::css(wide.width),
        &chrome::css(wide.width),
        BODY_OPEN,
        &chrome::main_open(&css::narrow_query(wide.width)),
        &chrome::controls(),
        &heading,
        DIV_OPEN,
        chrome::ARRANGEMENT_CLASS,
        WIDE_REST,
        &serialise(wide, target),
        DIV_CLOSE,
        DIV_OPEN,
        chrome::ARRANGEMENT_CLASS,
        NARROW_REST,
        &serialise(narrow, target),
        DIV_CLOSE,
        &chrome::panel(),
        &table::table(catalogue, plotted),
        MAIN_CLOSE,
        &chrome::script(),
        BODY_CLOSE,
    ]
    .concat()
}
