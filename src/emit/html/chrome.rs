//! The page's own controls, which only its script reveals, and the script
//! itself (FR-48, FR-56, FR-98).
//!
//! The script is a constant, inlined as written: it holds nothing from the
//! CSV (NFR-4, NFR-6). Every name it uses is a constant here or in
//! `svg::serialise`, and a test holds the script's names block to them.
//! Rule fragments are module constants, as in `css.rs`.

use super::css::{HIDDEN_BOX, OUTLINE_CLOSE, Wide, capped, narrow_query};
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
/// The toggle's label says what a press does (FR-56, rg-nuy): the script
/// sets the one for the theme not showing, and the page is written with
/// the light theme's. Only the script writes the other, so it is named
/// here for the test that holds the script's names block.
pub(super) const TO_DARK: &str = "Switch to dark theme";
#[cfg(test)]
pub(super) const TO_LIGHT: &str = "Switch to light theme";
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
/// The toggle's icons (rg-1ru): each shows the theme a press switches to,
/// so the moon shows while the light theme does. The script swaps which is
/// `hidden`; the page is written with the light theme's, as its label is.
/// The icons below spell them out; they are named here for the test that
/// holds the script's names block and the icons to them.
#[cfg(test)]
pub(super) const MOON_CLASS: &str = "theme-moon";
#[cfg(test)]
pub(super) const SUN_CLASS: &str = "theme-sun";
/// Decoration only: the toggle's name is its visually hidden label. The
/// strokes take the button's text colour from the stylesheet.
pub(super) const MOON_ICON: &str = "<svg class=\"theme-moon\" viewBox=\"0 0 24 24\" aria-hidden=\"true\"><path d=\"M20 14.5A8.5 8.5 0 1 1 9.5 4a7.5 7.5 0 0 0 10.5 10.5z\"/></svg>";
pub(super) const SUN_ICON: &str = "<svg class=\"theme-sun\" viewBox=\"0 0 24 24\" aria-hidden=\"true\" hidden=\"\"><circle cx=\"12\" cy=\"12\" r=\"4\"/><path d=\"M12 2v2M12 20v2M2 12h2M20 12h2M4.9 4.9l1.4 1.4M17.7 17.7l1.4 1.4M4.9 19.1l1.4-1.4M17.7 6.3l1.4-1.4\"/></svg>";
pub(super) const SKIP_CLASS: &str = "skip";
/// Each arrangement's container (FR-69).
pub(super) const ARRANGEMENT_CLASS: &str = "arrangement";

const CONTROLS_OPEN: &str = "<div class=\"page-controls\">";
const BUTTON_TAG: &str = "<button type=\"button\" class=\"";
const SKIP_REST: &str = "\" hidden=\"\">Skip to the legend</button>";
const TOGGLE_REST: &str = "\" hidden=\"\">";
const LABEL_OPEN: &str = "<span class=\"visually-hidden\">";
const TOGGLE_END: &str = "</span></button>";
const CONTROLS_CLOSE: &str = "</div>\n";

/// `<main>`'s start tag, carrying the narrow query.
pub(super) fn main_open(query: &str) -> String {
    ["<main ", DATA_NARROW_QUERY, "=", &html_attr(query), ">\n"].concat()
}

/// The skip link and, when the page has both built-in themes to switch
/// between, the theme toggle (FR-56, §6.6): first in `<main>`, both
/// `hidden` until the script runs (FR-98's rule).
pub(super) fn controls(toggle: bool) -> String {
    let mut out = [CONTROLS_OPEN, BUTTON_TAG, SKIP_CLASS, SKIP_REST].concat();
    if toggle {
        out.extend([
            BUTTON_TAG,
            TOGGLE_CLASS,
            TOGGLE_REST,
            MOON_ICON,
            SUN_ICON,
            LABEL_OPEN,
            TO_DARK,
            TOGGLE_END,
        ]);
    }
    out + CONTROLS_CLOSE
}

/// The panel's `id`, the one every control's `aria-controls` names; written
/// once, outside both arrangements (architecture §5).
pub(super) const PANEL_ID: &str = "detail";
pub(super) const HEADING_CLASS: &str = "detail-heading";
pub(super) const CLOSE_CLASS: &str = "detail-close";
pub(super) const EMPTY_CLASS: &str = "detail-empty";
/// A control's expanded state and the panel it opens (D-66). Only the
/// script writes them, so no static markup claims a panel; they are named
/// here for the test that holds the script's names block.
#[cfg(test)]
pub(super) const ARIA_EXPANDED: &str = "aria-expanded";
#[cfg(test)]
pub(super) const ARIA_CONTROLS: &str = "aria-controls";

const SECTION_OPEN: &str = "<section id=\"";
const PANEL_HEAD: &str = "\" class=\"detail\" aria-label=\"Item details\" hidden=\"\"><div class=\"detail-head\"><h2 class=\"";
const PANEL_CLOSE_OPEN: &str = "\" tabindex=\"-1\"></h2><button type=\"button\" class=\"";
/// The term and value the script clones for each field, and the empty
/// state, so the script writes no markup of its own (NFR-6).
const PANEL_BODY: &str = "\">Close</button></div><dl><dt></dt><dd></dd></dl><p class=\"";
const PANEL_END: &str = "\" hidden=\"\">No further details.</p></section>\n";

/// FR-38's panel, hidden and empty until the script fills it from the
/// item's table row (D-80).
pub(super) fn panel() -> String {
    [
        SECTION_OPEN,
        PANEL_ID,
        PANEL_HEAD,
        HEADING_CLASS,
        PANEL_CLOSE_OPEN,
        CLOSE_CLASS,
        PANEL_BODY,
        EMPTY_CLASS,
        PANEL_END,
    ]
    .concat()
}

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
/// The icon toggle is square, its padding all round (rg-1ru): at the
/// page's 16 px text it is about 35 px a side, past WCAG 2.5.8's 24.
const TOGGLE_ICON_RULE: &str = ".page-controls .theme-toggle{padding:.4em}\n.theme-toggle svg{display:block;width:1.25em;height:1.25em;fill:none;stroke:currentColor;stroke-width:2;stroke-linecap:round;stroke-linejoin:round}\n";
const FOCUS_OPEN: &str = ".page-controls button:focus-visible,.detail-close:focus-visible,.detail-heading:focus-visible{outline:2px solid var(";
/// FR-38's panel, sticky at the viewport's bottom while open so the radar
/// stays in view, and scrolling inside itself past 45% of the screen.
const PANEL_CAP: &str = ".detail{";
const PANEL_OPEN: &str = ".detail{position:sticky;bottom:.75rem;max-height:min(45vh,28em);overflow:auto;margin-block:1em;padding-block:1em;border-radius:8px;box-shadow:0 2px 12px rgb(0 0 0 / .14);background:var(";
const PANEL_EDGE: &str = ");border:1px solid var(";
const PANEL_REST: &str = ")}\n.detail-head{display:flex;justify-content:space-between;align-items:center;gap:1em}\n.detail-heading{margin:0;font-size:1.0625rem;font-weight:600}\n.detail dl{margin:.5em 0 0}\n.detail dd{margin:0 0 .6em;overflow-wrap:anywhere}\n.detail dd.prose{white-space:pre-line;max-width:70ch}\n.detail dt{font-size:.75rem;font-weight:600;color:var(";
const PANEL_EMPTY: &str = ")}\n.detail-empty{color:var(";
const PANEL_CLOSE: &str = ")}\n";
/// Below the breakpoint the panel is a bottom sheet, edge to edge.
const SHEET_OPEN: &str = "@media ";
const SHEET_RULE: &str =
    "{.detail{bottom:0;margin-block-end:0;border-inline:0;border-radius:8px 8px 0 0}}\n";
/// Where there is room, terms and values sit side by side.
const GRID_OPEN: &str = "@media (min-width: ";
const GRID_RULE: &str = "px){.detail dl{display:grid;grid-template-columns:max-content 1fr;column-gap:1.5em;align-items:baseline}.detail dt{grid-column:1}.detail dd{grid-column:2}}\n";

/// The controls' rules, capped and inset like the table view.
/// Plan RG-018A R11: without the toggle, which holds the controls bar
/// open, a focused skip link would push the page down and let it jump
/// back; drawn over the page instead.
pub(super) const SKIP_OVER_PAGE: &str =
    ".page-controls{position:relative}\n.skip:focus{position:absolute;z-index:1}\n";

/// The controls' and panel's rules; `toggle` as for [`controls`].
pub(super) fn css(wide: &Wide, toggle: bool) -> String {
    let skip = if toggle { "" } else { SKIP_OVER_PAGE };
    [
        HIDDEN_RULE,
        &capped(BAR_OPEN, wide),
        BUTTON_OPEN,
        Neutral::StrokeDivider.as_str(),
        BUTTON_GROUND,
        Neutral::Surface2.as_str(),
        BUTTON_INK,
        Neutral::TextPrimary.as_str(),
        BUTTON_CLOSE,
        SKIP_RULE,
        HIDDEN_BOX,
        TOGGLE_ICON_RULE,
        FOCUS_OPEN,
        Neutral::Focus.as_str(),
        OUTLINE_CLOSE,
        &capped(PANEL_CAP, wide),
        PANEL_OPEN,
        Neutral::Surface2.as_str(),
        PANEL_EDGE,
        Neutral::StrokeDivider.as_str(),
        PANEL_REST,
        Neutral::TextMuted.as_str(),
        PANEL_EMPTY,
        Neutral::TextMuted.as_str(),
        PANEL_CLOSE,
        SHEET_OPEN,
        &narrow_query(wide),
        SHEET_RULE,
        GRID_OPEN,
        &wide.breakpoint().to_string(),
        GRID_RULE,
        skip,
    ]
    .concat()
}

#[cfg(test)]
mod tests;
