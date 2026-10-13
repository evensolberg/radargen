//! The page's own controls, which only its script reveals, and the script
//! itself (FR-48, FR-56, FR-98).
//!
//! The script is a constant, inlined as written: it holds nothing from the
//! CSV (NFR-4, NFR-6). Every name it uses is a constant here or in
//! `svg::serialise`, and a test holds the script's names block to them.
//! Rule fragments are module constants, as in `css.rs`.

use super::css::{HIDDEN_BOX, OUTLINE_CLOSE, capped};
use crate::escape::html_attr;
use crate::palette::Neutral;

/// The interactive layer, inlined (NFR-5).
pub(super) const SCRIPT: &str = include_str!("script.js");

/// `<main>`'s attribute holding FR-69's narrow query for the script's
/// `matchMedia`, so `css::narrow_query` stays its one source.
pub(super) const DATA_NARROW_QUERY: &str = "data-narrow-query";
/// The root attribute the theme toggle sets (FR-56); `css::palettes`
/// declares its overrides.
pub(super) const DATA_THEME: &str = "data-theme";
/// The toggle's state (FR-56), set by the script.
pub(super) const ARIA_PRESSED: &str = "aria-pressed";
/// Set on a marker's indicator while the marker has visible focus (rg-jri).
pub(super) const DATA_FOCUSED: &str = "data-focused";
/// Set by the script on an item's elements while the pointer is over one
/// of them (FR-37).
pub(super) const DATA_HOVERED: &str = "data-hovered";
/// Set by the script on an item's elements while its panel is open
/// (FR-72).
pub(super) const DATA_SELECTED: &str = "data-selected";
/// The class the script puts on `<html>` once it runs: rules that need the
/// script to stand in for the browser apply only under it.
pub(super) const JS_CLASS: &str = "js";
pub(super) const TOGGLE_CLASS: &str = "theme-toggle";
pub(super) const SKIP_CLASS: &str = "skip";
/// Each arrangement's container (FR-69).
pub(super) const ARRANGEMENT_CLASS: &str = "arrangement";

const CONTROLS_OPEN: &str = "<div class=\"page-controls\"><button type=\"button\" class=\"";
const SKIP_REST: &str =
    "\" hidden=\"\">Skip to the legend</button><button type=\"button\" class=\"";
const TOGGLE_REST: &str = "\" ";
const TOGGLE_END: &str = "=\"false\" hidden=\"\">Dark theme</button></div>\n";

/// `<main>`'s start tag, carrying the narrow query.
pub(super) fn main_open(query: &str) -> String {
    ["<main ", DATA_NARROW_QUERY, "=", &html_attr(query), ">\n"].concat()
}

/// The skip link and the theme toggle, first in `<main>`, both `hidden`
/// until the script runs (FR-98's rule).
pub(super) fn controls() -> String {
    [
        CONTROLS_OPEN,
        SKIP_CLASS,
        SKIP_REST,
        TOGGLE_CLASS,
        TOGGLE_REST,
        ARIA_PRESSED,
        TOGGLE_END,
    ]
    .concat()
}

/// FR-38's panel, written once outside both arrangements, the one `id`
/// every control's `aria-controls` names (architecture §5). It is empty and
/// hidden: the script fills it from the item's table row (D-80).
pub(super) const PANEL: &str = "<section id=\"detail\" class=\"detail\" aria-label=\"Item details\" hidden=\"\"><div class=\"detail-head\"><h2 class=\"detail-heading\" tabindex=\"-1\"></h2><button type=\"button\" class=\"detail-close\">Close</button></div><dl></dl></section>\n";

/// The script element, last in `<body>`.
pub(super) fn script() -> String {
    ["<script>\n", SCRIPT, "</script>\n"].concat()
}

const HIDDEN_RULE: &str = "[hidden]{display:none!important}\n";
const BAR_OPEN: &str = ".page-controls{display:flex;";
const BUTTON_OPEN: &str = ".page-controls button,.detail-close{margin-block:.5em;font:inherit;padding:.4em .8em;border-radius:6px;cursor:pointer;border:1px solid var(";
const BUTTON_GROUND: &str = ");background:var(";
const BUTTON_INK: &str = ");color:var(";
const BUTTON_CLOSE: &str = ")}\n";
/// The skip link sits at the start when focused, and is visually hidden
/// otherwise; the toggle keeps the end.
const SKIP_RULE: &str =
    ".skip{margin-inline-end:auto}\n.theme-toggle{margin-inline-start:auto}\n.skip:not(:focus)";
/// The toggle's state is visible as well as announced: a dot, filled when
/// pressed. Forced colours drop backgrounds, so there the dot keeps the
/// button's system ink (WCAG 1.4.1).
const DOT_RULE: &str = ".theme-toggle::before{content:\"\";display:inline-block;width:.75em;height:.75em;border:1.5px solid currentColor;border-radius:50%;margin-inline-end:.4em;vertical-align:-.05em}\n.theme-toggle[aria-pressed=true]::before{background:currentColor}\n@media (forced-colors: active){.theme-toggle[aria-pressed=true]::before{forced-color-adjust:none;background:ButtonText}}\n";
const FOCUS_OPEN: &str = ".page-controls button:focus-visible,.detail-close:focus-visible,.detail-heading:focus-visible{outline:2px solid var(";
/// FR-38's panel, sticky at the viewport's bottom while open so the radar
/// stays in view, and scrolling inside itself past 45% of the screen.
const PANEL_CAP: &str = ".detail{";
const PANEL_OPEN: &str = ".detail{position:sticky;bottom:0;max-height:min(45vh,28em);overflow:auto;margin-block:1em;padding-block:1em;border-radius:8px;background:var(";
const PANEL_EDGE: &str = ");border:1px solid var(";
const PANEL_REST: &str = ")}\n.detail-head{display:flex;justify-content:space-between;align-items:baseline;gap:1em}\n.detail-heading{margin:0;font-size:1.0625rem;font-weight:600}\n.detail dl{margin:.5em 0 0}\n.detail dd{margin:0 0 .6em}\n.detail dd.prose{white-space:pre-line;max-width:70ch}\n.detail dt{font-size:.75rem;font-weight:600;color:var(";
const PANEL_EMPTY: &str = ")}\n.detail-empty{color:var(";
const PANEL_CLOSE: &str = ")}\n";

/// The controls' rules, capped and inset like the table view.
pub(super) fn css(wide_width: f64) -> String {
    [
        HIDDEN_RULE,
        &capped(BAR_OPEN, wide_width),
        BUTTON_OPEN,
        Neutral::StrokeDivider.as_str(),
        BUTTON_GROUND,
        Neutral::Surface2.as_str(),
        BUTTON_INK,
        Neutral::TextPrimary.as_str(),
        BUTTON_CLOSE,
        SKIP_RULE,
        HIDDEN_BOX,
        DOT_RULE,
        FOCUS_OPEN,
        Neutral::Focus.as_str(),
        OUTLINE_CLOSE,
        &capped(PANEL_CAP, wide_width),
        PANEL_OPEN,
        Neutral::Surface2.as_str(),
        PANEL_EDGE,
        Neutral::StrokeDivider.as_str(),
        PANEL_REST,
        Neutral::TextMuted.as_str(),
        PANEL_EMPTY,
        Neutral::TextMuted.as_str(),
        PANEL_CLOSE,
    ]
    .concat()
}

#[cfg(test)]
mod tests;
