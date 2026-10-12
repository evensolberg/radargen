//! The HTML page (FR-52, FR-80). In PR 14a, a minimal page around the wide
//! radar, drawn static; PR 14b writes both arrangements, the palettes and the
//! font.

use crate::emit::OUTPUT_VERSION;
use crate::escape::xml_text;
use crate::palette::{Palette, Theme};
use crate::svg::{Document, Target, serialise};

/// The tab title and hidden heading of a page with no title (FR-65, D-75).
pub const UNTITLED: &str = "Technology Radar";

/// Hides a heading from sight but not from assistive technology:
/// `display: none` and `visibility: hidden` would drop it from the
/// accessibility tree, and a zero size from `VoiceOver`.
const VISUALLY_HIDDEN: &str = ".visually-hidden{position:absolute;width:1px;height:1px;margin:-1px;padding:0;border:0;overflow:hidden;clip:rect(0 0 0 0);clip-path:inset(50%);white-space:nowrap}";

/// The page for the wide document.
///
/// `title` is FR-65's resolved title, `None` when absent or blank: the
/// page is then named [`UNTITLED`], with a visually hidden level-1 heading,
/// since the drawn document has none (D-75). With a title, the SVG's own
/// title is the level-1 heading.
#[must_use]
pub fn page(wide: &Document, title: Option<&str>) -> String {
    let svg = serialise(wide, Target::Static(&Palette::built_in(Theme::Light)));
    let heading = if title.is_some() {
        String::new()
    } else {
        ["<h1 class=\"visually-hidden\">", UNTITLED, "</h1>\n"].concat()
    };
    [
        "<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n",
        "<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n",
        "<meta name=\"generator\" content=\"",
        OUTPUT_VERSION,
        "\">\n<title>",
        &xml_text(title.unwrap_or(UNTITLED)),
        "</title>\n<style>",
        VISUALLY_HIDDEN,
        "</style>\n</head>\n<body>\n<main>\n",
        &heading,
        &svg,
        "\n</main>\n</body>\n</html>\n",
    ]
    .concat()
}
