//! The HTML page (FR-52, FR-68, FR-69, FR-80): both arrangements, drawn
//! interactive, with one shown at a time, both palettes, and Inter inlined.

mod css;

use crate::emit::OUTPUT_VERSION;
use crate::escape::xml_text;
use crate::palette::{Palette, Theme};
use crate::svg::{Document, Target, marker_keys, serialise};

/// The tab title and hidden heading of a page with no title (FR-65, D-75).
pub const UNTITLED: &str = "Technology Radar";

const HEAD_OPEN: &str = "<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n<meta name=\"generator\" content=\"";
const BODY_OPEN: &str = "</style>\n</head>\n<body>\n<main>\n";
const HIDDEN_HEADING: &str = "<h1 class=\"visually-hidden\">";
const WIDE_OPEN: &str = "<div class=\"arrangement wide\">\n";
const NARROW_OPEN: &str = "\n</div>\n<div class=\"arrangement narrow\">\n";
const BODY_CLOSE: &str = "\n</div>\n</main>\n</body>\n</html>\n";

/// The page for the two arrangements of one `Blocks` (FR-68).
///
/// Both are serialised interactive, binding colours to custom properties
/// the stylesheet declares for both themes (FR-56, architecture §5). The
/// palette passed only names them and cycles the series, which the two
/// built-in themes share.
///
/// `title` is FR-65's resolved title, `None` when absent or blank: the
/// page is then named [`UNTITLED`], with a visually hidden level-1 heading,
/// since the drawn document has none (D-75). With a title, the SVG's own
/// title is the level-1 heading.
#[must_use]
pub fn page(wide: &Document, narrow: &Document, title: Option<&str>) -> String {
    let markers = marker_keys(wide);
    debug_assert_eq!(
        markers,
        marker_keys(narrow),
        "FR-68: the same items in both"
    );
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
        &css::indicators(&markers),
        css::VISUALLY_HIDDEN,
        BODY_OPEN,
        &heading,
        WIDE_OPEN,
        &serialise(wide, target),
        NARROW_OPEN,
        &serialise(narrow, target),
        BODY_CLOSE,
    ]
    .concat()
}
