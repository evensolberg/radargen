//! The page's stylesheet (FR-48, FR-56, FR-69, NFR-5), generated rather than
//! templated: every value comes from a constant or from the `Document`, so
//! the CSS cannot drift from the markup it styles.
//!
//! Each section is its own function, and `page` assembles them; the table's
//! and the page chrome's rules live beside their markup, in `table.rs` and
//! `chrome.rs`. Rule fragments
//! are module constants: a `{…}` literal inside a function body reads to
//! clippy as a forgotten `format!` argument.

use base64::Engine;
use base64::engine::general_purpose::STANDARD;

use crate::layout::arrange::WideParts;
use crate::num::fmt_coord;
use crate::palette::{Colours, Neutral, Palette, Theme};
use crate::style::{FOCUS_RING_STROKE, WIDE_MIN_SCALE, breakpoint_px};
use crate::svg::serialise::{
    DATA_FOCUS_RING, DATA_HIT, DATA_LEGEND_ROW, DATA_MARKER, DATA_MARKER_INDICATOR,
    DATA_ROW_INDICATOR, DATA_SELECTION,
};

use super::chrome::{
    DATA_FOCUSED, DATA_HOVERED, DATA_LEGEND, DATA_SELECTED, DATA_THEME, ICONS_DARK, ICONS_LIGHT,
    JS_CLASS, LEGEND_HIDDEN,
};
use crate::text::{FAMILY, REGULAR_WOFF2, SEMIBOLD_WOFF2, Weight};

/// Hides a heading from sight but not from assistive technology:
/// `display: none` and `visibility: hidden` would drop it from the
/// accessibility tree, and a zero size from `VoiceOver`.
pub(super) const VISUALLY_HIDDEN: &str = ".visually-hidden";
/// The declarations that hide a box from sight only, for any selector.
pub(super) const HIDDEN_BOX: &str = "{position:absolute;width:1px;height:1px;margin:-1px;padding:0;border:0;overflow:hidden;clip:rect(0 0 0 0);clip-path:inset(50%);white-space:nowrap}\n";
/// The end of a focus outline rule, after its colour: page chrome outlines
/// sit 2px out, so they never touch the control.
pub(super) const OUTLINE_CLOSE: &str = ");outline-offset:2px}\n";
/// The wide arrangement's parts' containers (D-85); the legend's scrolls.
pub(super) const WIDE_TITLE: &str = "wide-title";
pub(super) const WIDE_RADAR: &str = "wide-radar";
pub(super) const WIDE_KEY: &str = "wide-key";
pub(super) const WIDE_LEGEND: &str = "wide-legend";
/// The containers for the narrow arrangement's parts (RG-102). They add no
/// rule of their own: both SVGs are as wide as the narrow document and span
/// `.narrow`'s capped column (`svg{width:100%}`), so they draw at one scale
/// and stack into the one document.
pub(super) const NARROW_BODY: &str = "narrow-body";
pub(super) const NARROW_LEGEND: &str = "narrow-legend";

const CAP_FLOOR: &str = "max(";
const CAP_HEIGHT: &str = "px,calc((100vh - ";
const CAP_UNDER: &str = "px)*";
const CAP_OVER: &str = "/";
const CAP_END: &str = "))";
const SPAN_OPEN: &str = "calc(";
const SPAN_PLUS: &str = " + ";
const SPAN_CLOSE: &str = "px)";
const INSET_WIDTH: &str = "max-width:";
const INSET_REST: &str = ";margin-inline:auto;padding-inline:";
const INSET_CLOSE: &str = "px;box-sizing:border-box}\n";

/// What the stylesheet reads of HTML's wide arrangement (D-85): the sizes
/// of its parts, in user units, which are CSS pixels at natural size.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Wide {
    /// The title part's height; it spans the whole width.
    pub(super) title_height: f64,
    /// The radar part's width, which the key's part shares.
    pub(super) radar_width: f64,
    /// The radar part's height.
    pub(super) radar_height: f64,
    /// The legend part's width.
    pub(super) legend_width: f64,
}

impl Wide {
    /// The sizes `parts` were cut to.
    pub(super) const fn of(parts: &WideParts) -> Self {
        Self {
            title_height: parts.title.height,
            radar_width: parts.radar.width,
            radar_height: parts.radar.height,
            legend_width: parts.legend.width,
        }
    }

    /// The whole wide document's width.
    pub(super) fn width(self) -> f64 {
        self.radar_width + self.legend_width
    }

    /// FR-69's breakpoint, where the radar draws at the legibility floor
    /// beside a natural-size legend.
    pub(super) fn breakpoint(self) -> u32 {
        breakpoint_px(self.width(), self.radar_width)
    }

    /// The radar column's cap: the width at which the radar is as tall as
    /// the viewport below the title, but never less than `WIDE_MIN_SCALE`
    /// of its own width. The floor matters on a short window: FR-69's
    /// breakpoint reads only the width, so without it the height term could
    /// shrink the radar's text past what the breakpoint allows.
    pub(super) fn radar_cap(self) -> String {
        [
            CAP_FLOOR,
            &fmt_coord(self.radar_width * WIDE_MIN_SCALE),
            CAP_HEIGHT,
            &fmt_coord(self.title_height),
            CAP_UNDER,
            &fmt_coord(self.radar_width),
            CAP_OVER,
            &fmt_coord(self.radar_height),
            CAP_END,
        ]
        .concat()
    }

    /// The whole arrangement's cap: the radar column and the legend.
    fn cap(self) -> String {
        [
            SPAN_OPEN,
            &self.radar_cap(),
            SPAN_PLUS,
            &fmt_coord(self.legend_width),
            SPAN_CLOSE,
        ]
        .concat()
    }
}

/// `open`, a selector and its rule's opening declarations, capped as the
/// wide arrangement is and inset by its margin, so page chrome lines up
/// with the radar's content edge at one pixel per unit. Drawn larger, the
/// radar's margin grows and the inset does not, so the chrome's edges sit a
/// few pixels outside the radar's; accepted.
pub(super) fn capped(open: &str, wide: &Wide) -> String {
    [
        open,
        INSET_WIDTH,
        &wide.cap(),
        INSET_REST,
        &fmt_coord(crate::style::MARGIN),
        INSET_CLOSE,
    ]
    .concat()
}

const FACE_OPEN: &str = "@font-face{font-family:\"";
const FACE_WEIGHT: &str = "\";font-style:normal;font-weight:";
const FACE_SRC: &str = ";font-display:block;src:url(data:font/woff2;base64,";
const FACE_CLOSE: &str = ") format(\"woff2\")}\n";

const LIGHT_OPEN: &str = ":root{color-scheme:light;";
const DARK_PREFERRED_OPEN: &str = "@media (prefers-color-scheme: dark){:root:not([";
const NOT_LIGHT: &str = "=\"light\"]){color-scheme:dark;";
const DARK_CHOSEN_OPEN: &str = ":root[";
const DARK_CHOSEN: &str = "=\"dark\"]{color-scheme:dark;";
const CLOSE: &str = "}\n";
const FORCED_OPEN: &str = "@media (forced-colors: active){:root,:root[";
const FORCED_REST: &str = "]{";
const CLOSE_TWICE: &str = "}}\n";

/// Inter for the page's own text too, not only the SVG's: without it the
/// heading, toggle and table fall back to the browser's serif.
const BODY_OPEN: &str = "body{margin:0;line-height:1.4;font-family:\"";
const BODY_GROUND: &str = "\";background:var(";
const BODY_INK: &str = ");color:var(";
const SVG_RULE: &str = ")}\nsvg{display:block;width:100%;height:auto}\n";
const GRID_OPEN: &str = ".wide{display:grid;grid-template-columns:minmax(0,";
const GRID_CLOSE: &str = ") max-content;justify-content:center}\n";
/// The title spans both columns, its size contained so its width never
/// widens the legend's column, which is sized to its content.
const TITLE_RULE: &str = "{grid-row:1;grid-column:1/-1;contain:inline-size}\n";
const RADAR_RULE: &str = "{grid-row:2;grid-column:1}\n";
const KEY_RULE: &str = "{grid-row:3;grid-column:1}\n";
/// The legend scrolls in a box as tall as the radar beside it (D-85): a
/// zero height takes it out of the row's sizing, so the radar alone sets
/// the row, and the minimum stretches it to fill the row. Its column is
/// as wide as the legend and a reserved scrollbar gutter, so a classic
/// scrollbar never narrows the legend; where `scrollbar-gutter` is not
/// supported, the scrollbar clips the legend's right edge rather than
/// adding a horizontal scrollbar.
const LEGEND_RULE: &str = "{grid-row:2;grid-column:2;height:0;min-height:100%;box-sizing:border-box;overflow-x:hidden;overflow-y:auto;scrollbar-gutter:stable}\n";
const SVG_WIDTH_OPEN: &str = " svg{width:";
/// The legend's width is fixed: its column is sized to it.
const FIXED_CLOSE: &str = "px}\n";
/// The title's and key's shrink with the column when it is narrower.
const NATURAL_CLOSE: &str = "px;max-width:100%}\n";
const NARROW_CAP: &str = ".narrow{max-width:";
const PX: &str = "px";
const CAP_CLOSE: &str = ";margin:0 auto}\n";
/// The default: wide shown, narrow hidden. With one query swapping them,
/// there is no width at which both or neither show (FR-69).
const NARROW_HIDDEN: &str = ".narrow{display:none}\n@media ";
const SWAP: &str = "{.wide{display:none}.narrow{display:block}}\n";

/// The outline an indicator replaces: transparent rather than removed, so
/// forced-colours mode paints it in a system colour.
const NO_OUTLINE: &str = "outline:2px solid transparent";
const FOCUS_VISIBLE: &str = ":focus-visible";
const LBRACE: &str = "{";
const RBRACE: &str = "}\n";
const SHOW: &str = "visibility:visible";
/// Hover's dashes, a pixel wide at any scale: the narrow arrangement shrinks
/// the SVG, which would thin a scaled stroke below a pixel.
const DASHED: &str = ";stroke-dasharray:3 2;vector-effect:non-scaling-stroke";
/// The selected square's stroke, and how focus's ring ends: solid, whatever
/// hover set before it.
const SOLID: &str = ";stroke-dasharray:none";

/// Both vendored faces, inlined (NFR-5) under the family the SVG text names
/// (FR-85). `block` because the data is inline: there is no load to wait
/// for, so no fallback ever flashes.
pub(super) fn font_faces() -> String {
    [
        face(REGULAR_WOFF2, Weight::Regular),
        face(SEMIBOLD_WOFF2, Weight::SemiBold),
    ]
    .concat()
}

fn face(woff2: &[u8], weight: Weight) -> String {
    [
        FACE_OPEN,
        FAMILY,
        FACE_WEIGHT,
        &weight.css().to_string(),
        FACE_SRC,
        &STANDARD.encode(woff2),
        FACE_CLOSE,
    ]
    .concat()
}

/// A `--palette` page's one set (§6.6): `color-scheme` from the palette
/// (plan RG-018A, Ruling 4), then its declarations.
const FILE_OPEN: &str = ":root{color-scheme:";

/// The page's colours (FR-56, architecture §5). Built in: light by default,
/// dark when the viewer prefers it unless the page is set light, and dark
/// when the page is set dark; the theme toggle sets `data-theme` on
/// `<html>`, and each block says which of its icons shows (rg-1ru). From a
/// `--palette` file: that one set, with no preference and no toggle
/// (§6.6). Forced colours apply to both (rg-min, Ruling 5).
///
/// Values are written as `Palette` holds them, each a `Hex`, which admits
/// only `#RRGGBB` (NFR-6 context 5).
pub(super) fn palettes(colours: &Colours) -> String {
    let block = match colours {
        Colours::BuiltIn(_) => {
            let light = declarations(&Palette::built_in(Theme::Light));
            let dark = declarations(&Palette::built_in(Theme::Dark));
            [
                LIGHT_OPEN,
                &light,
                ICONS_LIGHT,
                CLOSE,
                DARK_PREFERRED_OPEN,
                DATA_THEME,
                NOT_LIGHT,
                &dark,
                ICONS_DARK,
                CLOSE_TWICE,
                DARK_CHOSEN_OPEN,
                DATA_THEME,
                DARK_CHOSEN,
                &dark,
                ICONS_DARK,
                CLOSE,
            ]
            .concat()
        }
        Colours::File(palette) => {
            let scheme = if palette.is_dark() { "dark;" } else { "light;" };
            [FILE_OPEN, scheme, &declarations(palette), CLOSE].concat()
        }
    };
    [
        &block,
        FORCED_OPEN,
        DATA_THEME,
        FORCED_REST,
        &forced(),
        CLOSE_TWICE,
    ]
    .concat()
}

/// Forced colours (Windows high contrast) remap neither SVG paint nor these
/// properties, so a ring could keep the palette's blue against a system
/// background the palette never saw (rg-min). There, every neutral role
/// takes its system colour; the series keep theirs, since colour is never
/// the only cue (FR-39). After the themes, at their specificity, so it wins
/// in every theme.
fn forced() -> String {
    Neutral::ALL
        .into_iter()
        .map(|role| [role.as_str(), ":", system_colour(role), ";"].concat())
        .collect()
}

/// The CSS system colour standing for `role` in forced colours.
const fn system_colour(role: Neutral) -> &'static str {
    match role {
        Neutral::Surface1 | Neutral::Surface2 | Neutral::MarkerHollow => "Canvas",
        Neutral::TextPrimary
        | Neutral::TextMuted
        | Neutral::StrokeRing
        | Neutral::StrokeDivider => "CanvasText",
        Neutral::Focus => "Highlight",
    }
}

/// `palette`'s custom properties, `--name:value;` each.
fn declarations(palette: &Palette) -> String {
    palette
        .declarations()
        .into_iter()
        .map(|(role, value)| [role.to_string().as_str(), ":", value, ";"].concat())
        .collect()
}

/// The page around the two arrangements (FR-51, FR-69, D-85).
///
/// The wide arrangement is a grid of its parts: the title across the top,
/// the radar's column growing with the window (`Wide::radar_cap`), the key
/// under it and the legend beside it, scrolling. Only the radar fills its
/// column; the title, key and legend draw at one pixel per unit. The
/// narrow arrangement's two SVGs are capped together at their document's
/// width, and below that span the viewport, which is where FR-94's
/// `NARROW_HORIZONTAL_INSET_PX` of zero is measured.
pub(super) fn layout(wide: &Wide, narrow_width: f64) -> String {
    let class = |name: &str, rule: &str| [".", name, rule].concat();
    let svg_width = |name: &str, width: f64, close: &str| {
        [".", name, SVG_WIDTH_OPEN, &fmt_coord(width), close].concat()
    };
    [
        BODY_OPEN,
        FAMILY,
        BODY_GROUND,
        Neutral::Surface1.as_str(),
        BODY_INK,
        Neutral::TextPrimary.as_str(),
        SVG_RULE,
        GRID_OPEN,
        &wide.radar_cap(),
        GRID_CLOSE,
        &class(WIDE_TITLE, TITLE_RULE),
        &svg_width(WIDE_TITLE, wide.width(), NATURAL_CLOSE),
        &class(WIDE_RADAR, RADAR_RULE),
        &class(WIDE_KEY, KEY_RULE),
        &svg_width(WIDE_KEY, wide.radar_width, NATURAL_CLOSE),
        &class(WIDE_LEGEND, LEGEND_RULE),
        &svg_width(WIDE_LEGEND, wide.legend_width, FIXED_CLOSE),
        NARROW_CAP,
        &fmt_coord(narrow_width),
        PX,
        CAP_CLOSE,
        NARROW_HIDDEN,
        &narrow_query(wide),
        SWAP,
    ]
    .concat()
}

const SCREEN_OPEN: &str = "@media screen{\n";
const SCREEN_CLOSE: &str = "}\n";
const STATE_OPEN: &str = ":root.";
const STATE_ATTR: &str = "[";
const STATE_VALUE: &str = "=\"";
const STATE_CLOSE: &str = "\"]";
const DESCENDANT: &str = " .";
const SELECTOR_AND: &str = ",";
const EVERY_CHILD: &str = " *";
/// `!important` over the indicators' own `visibility:visible`, which would
/// otherwise show a hovered or selected row through the hidden legend.
const INVISIBLE_RULE: &str = "{visibility:hidden!important}\n";
const HIDE_RULE: &str = "{display:none}\n";
/// Contained, as the title is, so spanning both tracks does not let the
/// radar's own size widen them.
const SPAN_TRACKS_RULE: &str = " .wide-radar{grid-column:1/-1;contain:inline-size}\n";
const RADAR_KEPT_OPEN: &str = " .wide-radar svg{max-width:";
const RADAR_KEPT_CLOSE: &str = "}\n";

/// RG-102: while the legend is hidden, it is gone from both arrangements.
///
/// The wide legend turns invisible in place, which also takes its rows out
/// of the tab order and the accessibility tree, so the grid's tracks, the
/// legend's scrollbar gutter included, never change and nothing moves
/// sideways. The radar spans both tracks, its SVG keeping its cap, so it
/// grows only into width the legend was holding back. The narrow legend,
/// last in its column, is simply removed. Only under the script's class,
/// since without script nothing can bring the legend back (FR-98's rule),
/// and only on screen, since a printout has no button either.
pub(super) fn legend_hiding(wide: &Wide) -> String {
    let on = [
        STATE_OPEN,
        JS_CLASS,
        STATE_ATTR,
        DATA_LEGEND,
        STATE_VALUE,
        LEGEND_HIDDEN,
        STATE_CLOSE,
    ]
    .concat();
    [
        SCREEN_OPEN,
        on.as_str(),
        DESCENDANT,
        WIDE_LEGEND,
        SELECTOR_AND,
        on.as_str(),
        DESCENDANT,
        WIDE_LEGEND,
        EVERY_CHILD,
        INVISIBLE_RULE,
        on.as_str(),
        DESCENDANT,
        NARROW_LEGEND,
        HIDE_RULE,
        on.as_str(),
        SPAN_TRACKS_RULE,
        on.as_str(),
        RADAR_KEPT_OPEN,
        &wide.radar_cap(),
        RADAR_KEPT_CLOSE,
        SCREEN_CLOSE,
    ]
    .concat()
}

/// The media query under which the narrow arrangement shows: one below
/// FR-69's breakpoint (architecture §4), the one source PR 15's
/// `matchMedia` reads too.
///
/// `max-width` rather than range syntax (`width < B`), which closes the gap
/// between `B - 1` and `B` at fractional zoom but drops the whole rule in
/// Safari before 16.4. The gap shows the wide arrangement, the safe side. A
/// classic scrollbar counts towards the width, so at exactly `B` the wide
/// radar draws a little under the floor; accepted.
pub(super) fn narrow_query(wide: &Wide) -> String {
    [
        "(max-width: ",
        &wide.breakpoint().saturating_sub(1).to_string(),
        "px)",
    ]
    .concat()
}

/// The focus indicators (FR-48, architecture §5).
///
/// A legend row shows its own inset ring, with no script. A marker's ring
/// is in a later layer, so the script marks the indicator `data-focused`
/// while the marker has visible focus, and fixed rules show its ring: no
/// rule per key, so a Tab costs the same at 300 markers as at 3 (rg-jri).
/// The marker's own outline is hidden only under the script's `.js`
/// class; without the script the browser's outline stays (WCAG 2.4.7).
pub(super) fn indicators() -> String {
    [
        legend_states(),
        legend_focus(),
        marker_focus(),
        marker_states(),
        rule(
            &[&attr(DATA_HIT), ",[role=button]"].concat(),
            "cursor:pointer",
        ),
    ]
    .concat()
}

/// `[name]`.
fn attr(name: &str) -> String {
    ["[", name, "]"].concat()
}

/// `selector{declarations}` on its own line.
fn rule(selector: &str, declarations: &str) -> String {
    [selector, LBRACE, declarations, RBRACE].concat()
}

/// `var(--role)`.
fn var(role: Neutral) -> String {
    ["var(", role.as_str(), ")"].concat()
}

/// A legend row's hover and selection (FR-37, FR-72, FR-73). Hover dashes
/// the inset outline in the muted colour; focus, after it at the same
/// specificity, strokes it solid. Selection is the start bar, so a focused,
/// selected row shows two marks, not one thicker edge.
fn legend_states() -> String {
    let row = attr(DATA_LEGEND_ROW);
    let hovered = [
        row.as_str(),
        &attr(DATA_HOVERED),
        ">",
        &attr(DATA_ROW_INDICATOR),
    ]
    .concat();
    [
        rule(&hovered, SHOW),
        rule(
            &[hovered.as_str(), ">rect"].concat(),
            &[
                "stroke:",
                &var(Neutral::TextMuted),
                ";stroke-width:1px",
                DASHED,
            ]
            .concat(),
        ),
        rule(
            &[
                row.as_str(),
                &attr(DATA_SELECTED),
                ">",
                &attr(DATA_SELECTION),
            ]
            .concat(),
            SHOW,
        ),
    ]
    .concat()
}

/// A marker's hover and selection (FR-37, FR-72, FR-73): its indicator
/// shows, raising the marker; hover adds the square dashed and muted,
/// selection solid in the text colour, later so it wins. Never a ring,
/// which would read as a status (FR-40).
fn marker_states() -> String {
    let indicator = attr(DATA_MARKER_INDICATOR);
    let hovered = [indicator.as_str(), &attr(DATA_HOVERED)].concat();
    let selected = [indicator.as_str(), &attr(DATA_SELECTED)].concat();
    let square = |state: &str| [state, " ", &attr(DATA_SELECTION)].concat();
    [
        rule(&[hovered.as_str(), ",", &selected].concat(), SHOW),
        rule(&square(&hovered), SHOW),
        rule(
            &[square(&hovered).as_str(), ">rect"].concat(),
            &["stroke:", &var(Neutral::TextMuted), DASHED].concat(),
        ),
        rule(&square(&selected), SHOW),
        rule(
            &[square(&selected).as_str(), ">rect"].concat(),
            &["stroke:", &var(Neutral::TextPrimary), SOLID].concat(),
        ),
    ]
    .concat()
}

fn legend_focus() -> String {
    let row = [attr(DATA_LEGEND_ROW).as_str(), FOCUS_VISIBLE].concat();
    let shown = [row.as_str(), ">", &attr(DATA_ROW_INDICATOR)].concat();
    [
        rule(&row, NO_OUTLINE),
        rule(&shown, SHOW),
        rule(
            &[shown.as_str(), ">rect"].concat(),
            &[
                "stroke:",
                &var(Neutral::Focus),
                ";stroke-width:",
                &fmt_coord(FOCUS_RING_STROKE),
                PX,
                SOLID,
            ]
            .concat(),
        ),
    ]
    .concat()
}

fn marker_focus() -> String {
    let ring = attr(DATA_FOCUS_RING);
    let indicator = attr(DATA_MARKER_INDICATOR);
    let focused = [indicator.as_str(), &attr(DATA_FOCUSED)].concat();
    [
        rule(
            &[".", JS_CLASS, " ", &attr(DATA_MARKER), FOCUS_VISIBLE].concat(),
            NO_OUTLINE,
        ),
        rule(&ring, "visibility:hidden"),
        rule(&focused, SHOW),
        rule(&[focused.as_str(), " ", &ring].concat(), SHOW),
    ]
    .concat()
}

#[cfg(test)]
mod tests;
