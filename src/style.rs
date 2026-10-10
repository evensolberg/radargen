//! Every layout constant, in one home (architecture §3). Read by two or three
//! of `layout/*` each; scattered, they drift apart under maintenance.
//!
//! Values are user units at scale 1 (px when rendered 1:1), taken from
//! `docs/product/mockup.html` with its 700 weights mapped to `SemiBold` 600 —
//! FR-85 ships two weights. No uppercase transform: CSS `text-transform` is
//! invisible to our measurement and to `resvg`, and doing it for real would
//! edit a label, which FR-76 rules out.

use crate::text::Weight;

/// FR-85: a measurement is (face, size, letter-spacing), never fewer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextStyle {
    pub weight: Weight,
    pub size: f64,
    pub letter_spacing: f64,
}

const fn style(weight: Weight, size: f64, letter_spacing: f64) -> TextStyle {
    TextStyle {
        weight,
        size,
        letter_spacing,
    }
}

// --- Type scale ------------------------------------------------------------
// Each doc comment names the FR-82 role the text is painted with. The mockup's
// --text-secondary is not an FR-82 role; its uses map to primary or muted.
/// Ink: `TextPrimary`.
pub const TITLE: TextStyle = style(Weight::SemiBold, 17.0, 0.0);
/// Ink: `TextMuted`.
pub const SUBTITLE: TextStyle = style(Weight::Regular, 12.0, 0.0);
/// 0.04em at 12px. Ink: `TextPrimary` — FR-39's identity channel.
pub const QUADRANT_LABEL: TextStyle = style(Weight::SemiBold, 12.0, 0.48);
/// Ink: `TextMuted`, over a `LABEL_HALO` (it sits on the 12 o'clock divider, FR-31).
pub const RING_LABEL: TextStyle = style(Weight::Regular, 10.0, 0.0);
/// Ink: `TextPrimary` on a hollow marker; on a solid one,
/// `Palette::number_ink` for that entry (FR-82, D-54).
pub const MARKER_NUMBER: TextStyle = style(Weight::SemiBold, 9.0, 0.0);
/// The "Legend" heading, the same style as the key's. Ink: `TextPrimary`.
pub const LEGEND_HEADING: TextStyle = KEY_HEADING;
/// FR-35 level one. 0.03em at 12px. Ink: `TextPrimary`. Two units above
/// `LEGEND_RING`, not one: with no uppercase, both headings `SemiBold`, and
/// the two inks only 1.86:1 apart in the dark theme, one unit (0.8 px at
/// the breakpoint) did not read as a separate level.
pub const LEGEND_QUADRANT: TextStyle = style(Weight::SemiBold, 12.0, 0.36);
/// FR-35 level two. 0.03em at 10px. Ink: `TextMuted`.
pub const LEGEND_RING: TextStyle = style(Weight::SemiBold, 10.0, 0.3);
/// FR-35 level three, and the size FR-69's breakpoint protects. Ink: `TextPrimary`.
pub const LEGEND_NAME: TextStyle = style(Weight::Regular, 12.5, 0.0);
/// FR-74's trailing qualifier/status text. Ink: `TextMuted`.
pub const LEGEND_META: TextStyle = style(Weight::Regular, 10.5, 0.0);
/// The badge carries the marker's number, so it is the marker's style.
pub const LEGEND_BADGE_NUMBER: TextStyle = MARKER_NUMBER;
/// Ink: `TextPrimary`.
pub const KEY_HEADING: TextStyle = style(Weight::SemiBold, 13.0, 0.0);
/// Ink: `TextMuted`.
pub const KEY_LABEL: TextStyle = style(Weight::Regular, 11.0, 0.0);
/// Legend text line height.
pub const LINE_HEIGHT: f64 = 15.0;

// --- Legend (FR-35, FR-74, FR-79) -----------------------------------------------
pub const INDENT_QUADRANT: f64 = 16.0;
pub const INDENT_RING: f64 = 24.0;
pub const INDENT_ITEM: f64 = 44.0;
/// Between an item name and its trailing meta (FR-74).
pub const META_GAP: f64 = 8.0;
/// FR-79's fixed maximum for the legend block; its rendered width is the
/// lesser of this and the longest measured line. One value for both
/// arrangements, so the narrow document always holds it (D-56).
pub const LEGEND_WIDTH_MAX: f64 = NARROW_WIDTH - 2.0 * MARGIN;

// --- Markers (FR-20, FR-40) -----------------------------------------------------
/// OQ-16's home: whether this is user units or output pixels is a change to
/// this constant's meaning, not an excavation (architecture §8).
pub const MARKER_RADIUS: f64 = 10.0;
/// Solid marker: a Surface1 stroke that separates overlapping markers.
pub const MARKER_STROKE: f64 = 2.0;
/// Hollow marker (FR-20): series-coloured stroke around a `MarkerHollow` fill.
pub const MARKER_HOLLOW_STROKE: f64 = 3.0;
/// The legend badge — a list bullet, deliberately smaller than the marker.
/// 8, not the mockup's 7: Inter `SemiBold` 9px "44" is ~12px, which a hollow
/// r=7 badge (12px clear interior) cannot hold.
pub const BADGE_RADIUS: f64 = 8.0;
pub const BADGE_STROKE: f64 = 1.5;
/// Thinner than the marker's, so the hollow fill gap survives the smaller size.
pub const BADGE_HOLLOW_STROKE: f64 = 2.0;
/// FR-35's quadrant-colour swatch beside each legend group heading.
pub const SWATCH_RADIUS: f64 = 5.0;
/// FR-40's status ring sits this far outside the marker.
pub const STATUS_RING_GAP: f64 = 4.0;
pub const STATUS_RING_STROKE: f64 = 2.0;
/// The keyboard focus ring (`Focus`), from the marker's edge to the ring's
/// centre line. Outside the status ring with a clear Surface1 gap between, so
/// WCAG 1.4.11 measures it against the surface (6.0:1 and up) and never
/// against a series fill, where the focus blue manages only 1.3–2.5:1.
pub const FOCUS_RING_GAP: f64 = 8.0;
pub const FOCUS_RING_STROKE: f64 = 2.0;

// --- Radar (FR-32, FR-33) -------------------------------------------------------
/// Outer edge of the outermost band; bands divide it equally (FR-32).
pub const BAND_OUTER_RADIUS: f64 = 260.0;
pub const DIVIDER_RADIUS: f64 = 262.0;
pub const QUADRANT_LABEL_RADIUS: f64 = 300.0;
/// Ring circles and quadrant dividers (`StrokeRing` / `StrokeDivider`).
pub const GRID_STROKE: f64 = 1.0;
/// Width of the Surface1 halo under ring labels, so the 12 o'clock divider
/// (FR-31) doesn't strike through them. Emit it as its own stroke-only
/// `<text>` painted before the label, not as `paint-order: stroke`: an SVG
/// consumer that ignores `paint-order` would paint the halo over the glyphs
/// and erase them.
pub const LABEL_HALO: f64 = 3.0;

// --- Arrangements (FR-68–FR-71) -------------------------------------------------
pub const MARGIN: f64 = 20.0;
/// Between radar and legend, wide arrangement only.
pub const COLUMN_GAP: f64 = 28.0;
/// The narrow document is authored at this width and the radar scaled to fit
/// inside it, so text keeps its natural size (FR-70, D-31).
pub const NARROW_WIDTH: f64 = 420.0;
/// A floor on the radar block's width at scale 1, not its width. The block's
/// size is measured (architecture §4): quadrant labels never wrap (FR-79), so
/// a long one widens the block — `mockup.html` computes
/// `max(MIN_RADAR_W, 2 × (QLABEL_R + widest label / 2 + 10))`.
pub const MIN_RADAR_WIDTH: f64 = 660.0;

/// The smallest rendered size legend item names (`LEGEND_NAME`) may reach
/// before the HTML switches to the narrow arrangement (FR-69, architecture §4).
/// Smaller legend styles go proportionally lower; this is not a floor for them.
pub const MIN_LEGIBLE_PX: f64 = 10.0;

/// FR-64's widest output, as a bound on any document width layout produces.
const MAX_WIDTH: f64 = 10_000.0;

/// FR-69's media-query breakpoint for a wide document `wide_width` units wide:
/// the viewport width at which `LEGEND_NAME` renders at `MIN_LEGIBLE_PX`. Per
/// document, not a constant — the wide width grows with the legend (§4).
///
/// # Panics
///
/// On a width that is not a positive, finite number of at most `MAX_WIDTH`
/// units: a layout bug, which would otherwise come out as a silent
/// `max-width: 0px` media query.
#[must_use]
pub fn breakpoint_px(wide_width: f64) -> u32 {
    assert!(
        wide_width.is_finite() && wide_width > 0.0 && wide_width <= MAX_WIDTH,
        "breakpoint_px: {wide_width} is not a document width"
    );
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "bounded by the assert above, so the result is positive and fits u32"
    )]
    let px = (wide_width * MIN_LEGIBLE_PX / LEGEND_NAME.size).round() as u32;
    px
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::Fonts;

    /// FR-35: three levels, each stepped in under the one above, and each
    /// distinguished by type style as well.
    #[test]
    fn legend_levels_differ_in_indent_and_style() {
        // `const`: an assert over constants alone trips clippy's `assertions_on_constants`.
        const { assert!(INDENT_QUADRANT < INDENT_RING && INDENT_RING < INDENT_ITEM) };
        // The two headings share a weight, so size carries the step.
        const { assert!(LEGEND_QUADRANT.size - LEGEND_RING.size >= 2.0) };
        let levels = [LEGEND_QUADRANT, LEGEND_RING, LEGEND_NAME];
        for (i, a) in levels.iter().enumerate() {
            for b in &levels[i + 1..] {
                // FR-35: letter-spacing alone reads as a flat list, so a level
                // must differ in weight or by a visible size step.
                assert!(
                    a.weight != b.weight || (a.size - b.size).abs() >= 1.0,
                    "two legend levels read as one: {a:?} vs {b:?}"
                );
            }
        }
    }

    /// OQ-16 (d): the marker must hold its number. Two digits is the common
    /// case at realistic item counts; three is OQ-16's problem, not this one's.
    #[test]
    fn two_digit_numbers_fit_inside_the_marker() {
        let fonts = Fonts::embedded().expect("faces");
        let widest = |t: TextStyle| {
            (10..=99_u32)
                .map(|n| {
                    fonts
                        .face(t.weight)
                        .measure(&n.to_string(), t.size, t.letter_spacing)
                })
                .fold(0.0_f64, f64::max)
        };
        // A hollow stroke is centred on r, so the clear interior is 2r − stroke.
        let clear = |r: f64, stroke: f64| 2.0 * r - stroke;
        let marker = widest(MARKER_NUMBER);
        assert!(
            marker < clear(MARKER_RADIUS, MARKER_HOLLOW_STROKE),
            "{marker} does not fit the marker"
        );
        let badge = widest(LEGEND_BADGE_NUMBER);
        assert!(
            badge < clear(BADGE_RADIUS, BADGE_HOLLOW_STROKE),
            "{badge} does not fit the badge"
        );
    }

    #[test]
    fn badge_is_smaller_than_the_marker() {
        // mockup.html: the badge is a list bullet, not a second copy of the marker.
        const { assert!(BADGE_RADIUS < MARKER_RADIUS) };
    }

    /// Architecture §4: W × (floor / legend size), rounded. 1148 is the
    /// mockup's wide width (660 + 28 + 440 + 20), whose fixed breakpoint was 900.
    #[test]
    fn breakpoint_is_computed_from_the_wide_width() {
        assert_eq!(breakpoint_px(1148.0), 918);
        assert_eq!(breakpoint_px(1250.0), 1000);
        // D-56: a full-width 380 legend gives 660 + 28 + 380 + 20.
        assert_eq!(breakpoint_px(1088.0), 870);
        // 1150.625 × 10 / 12.5 is exactly 920.5, which rounds away from zero.
        assert_eq!(breakpoint_px(1150.625), 921);
    }

    #[test]
    #[should_panic(expected = "is not a document width")]
    fn a_zero_width_is_a_bug_not_a_breakpoint() {
        let _ = breakpoint_px(0.0);
    }

    #[test]
    #[should_panic(expected = "is not a document width")]
    fn a_nan_width_is_a_bug_not_a_breakpoint() {
        let _ = breakpoint_px(f64::NAN);
    }

    /// The focus ring clears the status ring by at least 2 units of surface,
    /// so it is judged against the surface, never a series fill (WCAG 1.4.11).
    #[test]
    fn focus_ring_sits_clear_of_the_status_ring() {
        let status_outer = STATUS_RING_GAP + STATUS_RING_STROKE / 2.0;
        let focus_inner = FOCUS_RING_GAP - FOCUS_RING_STROKE / 2.0;
        assert!(
            focus_inner - status_outer >= 2.0,
            "{status_outer} vs {focus_inner}"
        );
    }

    #[test]
    fn breakpoint_puts_legend_text_at_the_floor() {
        let w = 1400.0;
        let rendered = LEGEND_NAME.size * f64::from(breakpoint_px(w)) / w;
        assert!((rendered - MIN_LEGIBLE_PX).abs() < 0.01, "{rendered}");
    }
}
