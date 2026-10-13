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

use crate::num::fmt_coord;
use crate::palette::{Neutral, Palette, Theme};
use crate::style::{FOCUS_RING_STROKE, breakpoint_px};
use crate::svg::serialise::{
    DATA_FOCUS_RING, DATA_HIT, DATA_LEGEND_ROW, DATA_MARKER, DATA_MARKER_INDICATOR,
    DATA_ROW_INDICATOR, DATA_SELECTION,
};

use super::chrome::{DATA_FOCUSED, DATA_HOVERED, DATA_SELECTED, DATA_THEME, JS_CLASS};
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

const INSET_WIDTH: &str = "max-width:";
const INSET_REST: &str = "px;margin-inline:auto;padding-inline:";
const INSET_CLOSE: &str = "px;box-sizing:border-box}\n";

/// `open`, a selector and its rule's opening declarations, capped at the
/// wide document's width and inset by its margin, so page chrome lines up
/// with the radar's content edge.
pub(super) fn capped(open: &str, wide_width: f64) -> String {
    [
        open,
        INSET_WIDTH,
        &fmt_coord(wide_width),
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
const WIDE_CAP: &str = ".wide{max-width:";
const NARROW_CAP: &str = ".narrow{max-width:";
const CAP_CLOSE: &str = "px;margin:0 auto}\n";
/// The default: wide shown, narrow hidden. With one query swapping them,
/// there is no width at which both or neither show (FR-69).
const NARROW_HIDDEN: &str = ".narrow{display:none}\n@media ";
const SWAP: &str = "{.wide{display:none}.narrow{display:block}}\n";

/// The outline an indicator replaces: transparent rather than removed, so
/// forced-colours mode paints it in a system colour.
const OUTLINE: &str = "]:focus-visible{outline:2px solid transparent}\n";
const ROW_SHOWN: &str = "]:focus-visible>[";
const VISIBLE: &str = "]{visibility:visible}\n[";
const ROW_RING: &str = "]>rect{stroke:var(";
const ROW_RING_WIDTH: &str = ");stroke-width:";
/// Focus's ring ends solid, whatever hover set before it.
const RING_CLOSE: &str = "px;stroke-dasharray:none}\n";
const LBRACE: &str = "{";
const RBRACE: &str = "}\n";
const SHOW: &str = "visibility:visible";
/// Hover's dashes, a pixel wide at any scale: the narrow arrangement shrinks
/// the SVG, which would thin a scaled stroke below a pixel.
const DASHED: &str = ";stroke-dasharray:3 2;vector-effect:non-scaling-stroke";
const SOLID: &str = ";stroke-dasharray:none";
const SCRIPTED: &str = " [";
const HIDDEN: &str = "]{visibility:hidden}\n[";
const BOTH: &str = "][";
const INSIDE: &str = "] [";
const SHOWN_LAST: &str = "]{visibility:visible}\n";

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

/// Both palettes (FR-56, architecture §5): light by default, dark when the
/// viewer prefers it unless the page is set light, and dark when the page is
/// set dark. The theme toggle (FR-56) sets `data-theme` on `<html>`.
///
/// Values are written as `Palette` holds them: they are validated when a
/// palette is built (crumb `rg-cdo`), which guards this context and the SVG
/// attributes alike (NFR-6).
pub(super) fn palettes() -> String {
    let light = declarations(&Palette::built_in(Theme::Light));
    let dark = declarations(&Palette::built_in(Theme::Dark));
    [
        LIGHT_OPEN,
        &light,
        CLOSE,
        DARK_PREFERRED_OPEN,
        DATA_THEME,
        NOT_LIGHT,
        &dark,
        CLOSE_TWICE,
        DARK_CHOSEN_OPEN,
        DATA_THEME,
        DARK_CHOSEN,
        &dark,
        CLOSE,
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

/// The page around the two arrangements (FR-51, FR-69).
///
/// Each SVG fills its container, and each container is capped at its own
/// document's width, so a large screen draws one CSS pixel per unit rather
/// than enlarging it. Below that width the narrow one spans the viewport,
/// which is where FR-94's `NARROW_HORIZONTAL_INSET_PX` of zero is measured.
pub(super) fn layout(wide_width: f64, narrow_width: f64) -> String {
    [
        BODY_OPEN,
        FAMILY,
        BODY_GROUND,
        Neutral::Surface1.as_str(),
        BODY_INK,
        Neutral::TextPrimary.as_str(),
        SVG_RULE,
        WIDE_CAP,
        &fmt_coord(wide_width),
        CAP_CLOSE,
        NARROW_CAP,
        &fmt_coord(narrow_width),
        CAP_CLOSE,
        NARROW_HIDDEN,
        &narrow_query(wide_width),
        SWAP,
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
/// legend draws a little under `MIN_LEGIBLE_PX`; accepted.
pub(super) fn narrow_query(wide_width: f64) -> String {
    [
        "(max-width: ",
        &breakpoint_px(wide_width).saturating_sub(1).to_string(),
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
    [
        "[",
        DATA_LEGEND_ROW,
        OUTLINE,
        "[",
        DATA_LEGEND_ROW,
        ROW_SHOWN,
        DATA_ROW_INDICATOR,
        VISIBLE,
        DATA_LEGEND_ROW,
        ROW_SHOWN,
        DATA_ROW_INDICATOR,
        ROW_RING,
        Neutral::Focus.as_str(),
        ROW_RING_WIDTH,
        &fmt_coord(FOCUS_RING_STROKE),
        RING_CLOSE,
    ]
    .concat()
}

fn marker_focus() -> String {
    [
        ".",
        JS_CLASS,
        SCRIPTED,
        DATA_MARKER,
        OUTLINE,
        "[",
        DATA_FOCUS_RING,
        HIDDEN,
        DATA_MARKER_INDICATOR,
        BOTH,
        DATA_FOCUSED,
        VISIBLE,
        DATA_MARKER_INDICATOR,
        BOTH,
        DATA_FOCUSED,
        INSIDE,
        DATA_FOCUS_RING,
        SHOWN_LAST,
    ]
    .concat()
}

#[cfg(test)]
mod tests;
