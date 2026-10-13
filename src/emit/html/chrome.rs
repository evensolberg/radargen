//! The page's own controls, which only its script reveals, and the script
//! itself (FR-48, FR-56, FR-98).
//!
//! The script is a constant, inlined as written: it holds nothing from the
//! CSV (NFR-4, NFR-6). Every name it uses is a constant here or in
//! `svg::serialise`, and a test holds the script's names block to them.
//! Rule fragments are module constants, as in `css.rs`.

use crate::escape::html_attr;
use crate::num::fmt_coord;
use crate::palette::Neutral;
use crate::style::MARGIN;

/// The interactive layer, inlined (NFR-5).
pub(super) const SCRIPT: &str = include_str!("script.js");

/// `<main>`'s attribute holding FR-69's narrow query for the script's
/// `matchMedia`, so `css::narrow_query` stays its one source.
pub(super) const DATA_NARROW_QUERY: &str = "data-narrow-query";
/// The root attribute the theme toggle sets (FR-56); `css::palettes`
/// declares its overrides.
pub(super) const DATA_THEME: &str = "data-theme";
/// Set on a marker's indicator while the marker has visible focus (rg-jri).
pub(super) const DATA_FOCUSED: &str = "data-focused";
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
const TOGGLE_REST: &str = "\" aria-pressed=\"false\" hidden=\"\">Dark theme</button></div>\n";

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
    ]
    .concat()
}

/// The script element, last in `<body>`.
pub(super) fn script() -> String {
    ["<script>\n", SCRIPT, "</script>\n"].concat()
}

const HIDDEN_RULE: &str = "[hidden]{display:none!important}\n";
const BAR_OPEN: &str = ".page-controls{display:flex;max-width:";
const BAR_REST: &str = "px;margin-inline:auto;padding-inline:";
const BAR_CLOSE: &str = "px;box-sizing:border-box}\n";
const BUTTON_OPEN: &str = ".page-controls button{margin-block:.5em;font:inherit;padding:.4em .8em;border-radius:6px;cursor:pointer;border:1px solid var(";
const BUTTON_GROUND: &str = ");background:var(";
const BUTTON_INK: &str = ");color:var(";
const BUTTON_CLOSE: &str = ")}\n";
/// The skip link sits at the start when focused, and is visually hidden
/// otherwise; the toggle keeps the end.
const SKIP_RULE: &str = ".skip{margin-inline-end:auto}\n.skip:not(:focus){position:absolute;width:1px;height:1px;margin:-1px;padding:0;border:0;overflow:hidden;clip:rect(0 0 0 0);clip-path:inset(50%);white-space:nowrap}\n.theme-toggle{margin-inline-start:auto}\n";
/// The toggle's state is visible as well as announced: a dot, filled when
/// pressed.
const DOT_RULE: &str = ".theme-toggle::before{content:\"\";display:inline-block;width:.75em;height:.75em;border:1.5px solid currentColor;border-radius:50%;margin-inline-end:.4em;vertical-align:-.05em}\n.theme-toggle[aria-pressed=true]::before{background:currentColor}\n";
const FOCUS_OPEN: &str = ".page-controls button:focus-visible{outline:2px solid var(";
const FOCUS_CLOSE: &str = ");outline-offset:2px}\n";

/// The controls' rules, capped and inset like the table view.
pub(super) fn css(wide_width: f64) -> String {
    [
        HIDDEN_RULE,
        BAR_OPEN,
        &fmt_coord(wide_width),
        BAR_REST,
        &fmt_coord(MARGIN),
        BAR_CLOSE,
        BUTTON_OPEN,
        Neutral::StrokeDivider.as_str(),
        BUTTON_GROUND,
        Neutral::Surface2.as_str(),
        BUTTON_INK,
        Neutral::TextPrimary.as_str(),
        BUTTON_CLOSE,
        SKIP_RULE,
        DOT_RULE,
        FOCUS_OPEN,
        Neutral::Focus.as_str(),
        FOCUS_CLOSE,
    ]
    .concat()
}

#[cfg(test)]
mod tests;
