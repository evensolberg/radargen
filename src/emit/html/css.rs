//! The page's stylesheet (FR-48, FR-56, FR-69, NFR-5), generated rather than
//! templated: every value comes from a constant or from the `Document`, so
//! the CSS cannot drift from the markup it styles.
//!
//! Each section is its own function, and `page` assembles them, so PR 15 can
//! add the table's and the panel's rules beside their markup. Rule fragments
//! are module constants: a `{…}` literal inside a function body reads to
//! clippy as a forgotten `format!` argument.

use base64::Engine;
use base64::engine::general_purpose::STANDARD;

use crate::num::fmt_coord;
use crate::palette::{Neutral, Palette, Theme};
use crate::style::{FOCUS_RING_STROKE, breakpoint_px};
use crate::svg::ItemKey;
use crate::svg::serialise::{
    DATA_LEGEND_ROW, DATA_MARKER, DATA_MARKER_INDICATOR, DATA_ROW_INDICATOR,
};
use crate::text::{FAMILY, REGULAR_WOFF2, SEMIBOLD_WOFF2, Weight};

/// Hides a heading from sight but not from assistive technology:
/// `display: none` and `visibility: hidden` would drop it from the
/// accessibility tree, and a zero size from `VoiceOver`.
pub(super) const VISUALLY_HIDDEN: &str = ".visually-hidden{position:absolute;width:1px;height:1px;margin:-1px;padding:0;border:0;overflow:hidden;clip:rect(0 0 0 0);clip-path:inset(50%);white-space:nowrap}\n";

const FACE_OPEN: &str = "@font-face{font-family:\"";
const FACE_WEIGHT: &str = "\";font-style:normal;font-weight:";
const FACE_SRC: &str = ";font-display:block;src:url(data:font/woff2;base64,";
const FACE_CLOSE: &str = ") format(\"woff2\")}\n";

const LIGHT_OPEN: &str = ":root{color-scheme:light;";
const DARK_PREFERRED_OPEN: &str =
    "@media (prefers-color-scheme: dark){:root:not([data-theme=\"light\"]){color-scheme:dark;";
const DARK_CHOSEN_OPEN: &str = ":root[data-theme=\"dark\"]{color-scheme:dark;";
const CLOSE: &str = "}\n";
const CLOSE_TWICE: &str = "}}\n";

const BODY_OPEN: &str = "body{margin:0;background:var(";
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
const OUTLINE: &str = "]:focus-visible{outline:2px solid transparent}\n[";
const ROW_SHOWN: &str = "]:focus-visible>[";
const VISIBLE: &str = "]{visibility:visible}\n[";
const ROW_RING: &str = "]>rect{stroke:var(";
const ROW_RING_WIDTH: &str = ");stroke-width:";
const PX_CLOSE: &str = "px}\n";
const SUPPORTS_HAS: &str = "@supports selector(:has(*)){\n[";
const MARKER_OUTLINE: &str = "]:focus-visible{outline:2px solid transparent}\n";
const HAS_OPEN: &str = "svg:has([";
const KEY_OPEN: &str = "=\"";
const HAS_FOCUS: &str = "\"]:focus-visible) [";
const KEY_VISIBLE: &str = "\"]{visibility:visible}\n";

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
/// set dark. PR 15's toggle sets `data-theme` on `<html>`.
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
        &dark,
        CLOSE_TWICE,
        DARK_CHOSEN_OPEN,
        &dark,
        CLOSE,
    ]
    .concat()
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

/// The focus indicators (FR-48, architecture §5), shown with no script.
///
/// A legend row shows its own inset ring. A marker's ring is in a later
/// layer, so a rule per key reaches it through `:has`, inside
/// `@supports`; an engine without `:has` keeps its own outline. Each rule
/// that hides the outline shares its replacement's selector, so the two
/// stand or fall together (WCAG 2.4.7).
pub(super) fn indicators(markers: &[ItemKey]) -> String {
    [legend_focus(), marker_focus(markers)].concat()
}

fn legend_focus() -> String {
    [
        "[",
        DATA_LEGEND_ROW,
        OUTLINE,
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
        PX_CLOSE,
    ]
    .concat()
}

fn marker_focus(markers: &[ItemKey]) -> String {
    if markers.is_empty() {
        return String::new();
    }
    let rules: String = markers.iter().copied().map(marker_rule).collect();
    [SUPPORTS_HAS, DATA_MARKER, MARKER_OUTLINE, &rules, CLOSE].concat()
}

/// `svg:has([data-marker="L"]:focus-visible) [data-marker-indicator="L"]`:
/// qualified to markers on both sides, so a legend row neither lights a
/// marker's ring nor is lit by it.
fn marker_rule(key: ItemKey) -> String {
    let line = key.0.to_string();
    [
        HAS_OPEN,
        DATA_MARKER,
        KEY_OPEN,
        &line,
        HAS_FOCUS,
        DATA_MARKER_INDICATOR,
        KEY_OPEN,
        &line,
        KEY_VISIBLE,
    ]
    .concat()
}

#[cfg(test)]
mod tests;
