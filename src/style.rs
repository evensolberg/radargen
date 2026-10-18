//! Every layout constant, in one home (architecture §3). Read by two or three
//! of `layout/*` each; scattered, they drift apart under maintenance.
//!
//! Values are user units at scale 1 (px when rendered 1:1), taken from
//! `docs/product/mockup.html` with its 700 weights mapped to `SemiBold` 600 —
//! FR-85 ships two weights. No uppercase transform: CSS `text-transform` is
//! invisible to our measurement and to `resvg`, and doing it for real would
//! edit a label, which FR-76 rules out.

use crate::text::{Fonts, Weight, wrap_by};

/// FR-85: a measurement is (face, size, letter-spacing), never fewer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextStyle {
    /// The face: one of FR-85's two weights.
    pub weight: Weight,
    /// The font size in user units.
    pub size: f64,
    /// Extra advance between clusters, none after the last (FR-84), in user
    /// units; zero for none.
    pub letter_spacing: f64,
}

impl TextStyle {
    /// Size and letter-spacing multiplied by `k`; the face is unchanged.
    #[must_use]
    pub fn scaled(self, k: f64) -> Self {
        Self {
            size: self.size * k,
            letter_spacing: self.letter_spacing * k,
            ..self
        }
    }

    /// `text`'s advance in this style as the shaper lays it out (FR-84):
    /// where drawn text ends, for text that continues it.
    #[must_use]
    pub fn advance(self, text: &str, fonts: &Fonts) -> f64 {
        fonts
            .face(self.weight)
            .measure(text, self.size, self.letter_spacing)
    }

    /// `text`'s layout width in this style: its advance (FR-84) plus the
    /// room a reader's text spacing needs (D-78). Every block sizes and
    /// wraps by this, so spaced text fits where the layout put it.
    #[must_use]
    pub fn measure(self, text: &str, fonts: &Fonts) -> f64 {
        self.advance(text, fonts) + spacing_allowance(text, self.size)
    }

    /// `text` wrapped at `column` in this style, each line's layout width
    /// within it (FR-79, D-78).
    #[must_use]
    pub fn wrap(self, text: &str, column: f64, fonts: &Fonts) -> Vec<String> {
        wrap_by(text, column, &|line| self.measure(line, fonts))
    }
}

/// WCAG 1.4.12's letter spacing, in ems, that layout leaves room for.
const SPACED_LETTERS_EM: f64 = 0.12;
/// WCAG 1.4.12's word spacing, in ems.
const SPACED_WORDS_EM: f64 = 0.16;

/// The room `text` at `size` needs beyond its advance under WCAG 1.4.12.
///
/// A reader may set letter spacing to 0.12em and word spacing to 0.16em
/// (D-78): one allowance per `char`, a slight over-estimate for a combining
/// mark, and one per word separator (U+0020, U+00A0). Line height and
/// paragraph spacing have no effect on SVG text placed at absolute
/// baselines.
#[must_use]
#[expect(
    clippy::suboptimal_flops,
    reason = "NFR-4: plain * and + are bit-identical on every target; mul_add falls back to a libm fma where the CPU lacks one"
)]
pub(crate) fn spacing_allowance(text: &str, size: f64) -> f64 {
    #[expect(
        clippy::cast_precision_loss,
        clippy::as_conversions,
        reason = "a character count is far below 2^53, so the conversion is exact"
    )]
    let (chars, separators) = (
        text.chars().count() as f64,
        text.chars().filter(|c| matches!(c, ' ' | '\u{a0}')).count() as f64,
    );
    SPACED_LETTERS_EM * size * chars + SPACED_WORDS_EM * size * separators
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
/// Ink: `TextMuted`, over a `LABEL_HALO` (it sits on the 12 o'clock divider,
/// FR-31).
pub const RING_LABEL: TextStyle = style(Weight::Regular, 10.0, 0.0);
/// Ink: `TextPrimary` on a hollow marker; on a solid one, `Palette::number_ink`
/// for that entry (FR-82, D-54).
pub const MARKER_NUMBER: TextStyle = style(Weight::SemiBold, 9.0, 0.0);
/// The "Legend" heading, the same style as the key's. Ink: `TextPrimary`.
pub const LEGEND_HEADING: TextStyle = KEY_HEADING;
/// FR-35 level one. 0.03em at 12px. Ink: `TextPrimary`.
///
/// Two units above `LEGEND_RING`, not one: with no uppercase, both headings
/// `SemiBold`, and the two inks only 1.86:1 apart in the dark theme, one unit
/// (0.8 px at the breakpoint) did not read as a separate level.
pub const LEGEND_QUADRANT: TextStyle = style(Weight::SemiBold, 12.0, 0.36);
/// FR-35 level two. 0.03em at 10px. Ink: `TextMuted`.
pub const LEGEND_RING: TextStyle = style(Weight::SemiBold, 10.0, 0.3);
/// FR-35 level three, and the size FR-69's breakpoint protects. Ink:
/// `TextPrimary`.
pub const LEGEND_NAME: TextStyle = style(Weight::Regular, 12.5, 0.0);
/// FR-74's trailing qualifier/status text. Ink: `TextMuted`.
pub const LEGEND_META: TextStyle = style(Weight::Regular, 10.5, 0.0);
/// The badge carries the marker's number, so it is the marker's style.
pub const LEGEND_BADGE_NUMBER: TextStyle = MARKER_NUMBER;
/// Ink: `TextPrimary`.
pub const KEY_HEADING: TextStyle = style(Weight::SemiBold, 13.0, 0.0);
/// Ink: `TextMuted`.
pub const KEY_LABEL: TextStyle = style(Weight::Regular, 11.0, 0.0);
/// The key heading's baseline.
pub const KEY_HEADING_BASELINE: f64 = 20.0;
/// The first key row's sample centre.
pub const KEY_FIRST_ROW: f64 = 40.0;
/// Distance between key rows.
pub const KEY_ROW: f64 = 24.0;
/// The key samples' centre, from the key's left edge.
pub const KEY_SAMPLE_X: f64 = 10.0;
/// Where the key labels start.
pub const KEY_LABEL_X: f64 = 28.0;
/// Between one key column's widest label and the next column (rg-zb9).
/// The labels' layout widths already leave D-78's room for spaced text.
pub const KEY_COLUMN_GAP: f64 = 20.0;
/// A label's baseline sits this far below its sample's centre.
pub const KEY_LABEL_DROP: f64 = 4.0;
/// The key's qualifier samples.
pub const KEY_MARKER_RADIUS: f64 = 7.0;
/// The key's status samples.
pub const KEY_STATUS_MARKER_RADIUS: f64 = 5.0;
/// Below the last key row: the status ring's outer edge, so it cannot drift
/// from the samples.
pub const KEY_BOTTOM: f64 = KEY_STATUS_MARKER_RADIUS + STATUS_RING_GAP + STATUS_RING_STROKE / 2.0;
/// Legend text line height.
pub const LINE_HEIGHT: f64 = 15.0;

// --- Title block (FR-65), from the mockup's `buildTitle` ---------------------------
/// Space above the title's text box.
pub const TITLE_TOP: f64 = 4.0;
/// From `TITLE_TOP` to the title's first baseline.
pub const TITLE_FIRST_BASELINE: f64 = 18.0;
/// Baseline-to-baseline distance within a wrapped title.
pub const TITLE_LEADING: f64 = 21.0;
/// From the title's last baseline to the subtitle's first.
pub const SUBTITLE_AFTER_TITLE: f64 = 17.0;
/// From `TITLE_TOP` to the first baseline of a subtitle with no title.
pub const SUBTITLE_ALONE: f64 = 15.0;
/// Baseline-to-baseline distance within a wrapped subtitle.
pub const SUBTITLE_LEADING: f64 = 15.0;
/// Space below the last baseline.
pub const TITLE_BOTTOM: f64 = 14.0;

// --- Legend (FR-35, FR-74, FR-79) -----------------------------------------------
/// Left indent, in user units, of a quadrant heading: the legend's top level.
pub const INDENT_QUADRANT: f64 = 16.0;
/// Left indent, in user units, of a ring subheading: the legend's middle
/// level, inside its quadrant.
pub const INDENT_RING: f64 = 24.0;
/// Left indent, in user units, of an item row: the legend's deepest level.
pub const INDENT_ITEM: f64 = 44.0;
/// Between an item name and its trailing meta (FR-74).
pub const META_GAP: f64 = 8.0;
/// The "Legend" heading's baseline, from the content's top.
pub const LEGEND_HEADING_BASELINE: f64 = 14.0;
/// From the "Legend" heading's baseline to the first quadrant's.
pub const LEGEND_HEADING_TO_GROUP: f64 = 22.0;
/// The first quadrant heading's baseline, from the content's top.
pub const LEGEND_FIRST_GROUP: f64 = LEGEND_HEADING_BASELINE + LEGEND_HEADING_TO_GROUP;
/// From a quadrant heading's last baseline to its first ring heading's.
pub const LEGEND_QUADRANT_TO_RING: f64 = 18.0;
/// From a ring heading's last baseline to the top of its first row.
pub const LEGEND_RING_TO_ROW: f64 = 4.0;
/// From a ring's last row's bottom to the next ring heading's baseline.
pub const LEGEND_ROW_TO_RING: f64 = 12.0;
/// The extra space between one quadrant's group and the next, beyond the
/// step that would separate two rings.
pub const LEGEND_GROUP_GAP: f64 = 8.0;
/// From a quadrant's last row's bottom to the next quadrant's baseline.
pub const LEGEND_ROW_TO_QUADRANT: f64 = LEGEND_ROW_TO_RING + LEGEND_GROUP_GAP;
/// From a quadrant with no rows to the next one: the heading-to-ring step plus
/// the gap between groups.
pub const LEGEND_EMPTY_GROUP_GAP: f64 = LEGEND_QUADRANT_TO_RING + LEGEND_GROUP_GAP;
/// A row's least height, at scale 1.
///
/// FR-94: it renders at least `MIN_TARGET_PX` tall at `MIN_PX_PER_UNIT`
/// (asserted below, in every build). One line of name needs less; the rest
/// is the target.
pub const LEGEND_ROW_MIN_HEIGHT: f64 = 36.0;
/// A row's height beyond its text lines: `n · LINE_HEIGHT + LEGEND_ROW_PAD`.
pub const LEGEND_ROW_PAD: f64 = 7.0;
/// The least height of a row's text block, so a one-line row's text is centred
/// on its badge.
pub const LEGEND_TEXT_MIN_HEIGHT: f64 = 17.0;
/// From a row's text block's top to its first name baseline.
pub const LEGEND_FIRST_BASELINE: f64 = 9.0;
/// Between a row's badge and its name.
pub const LEGEND_BADGE_TO_NAME: f64 = 8.0;
/// A badge's centre, from the content's left edge.
pub const LEGEND_BADGE_X: f64 = INDENT_ITEM + BADGE_RADIUS;
/// Where a row's name starts: after the item indent, the badge and a gap.
pub const LEGEND_NAME_X: f64 = INDENT_ITEM + 2.0 * BADGE_RADIUS + LEGEND_BADGE_TO_NAME;
/// A row's target starts this far left of `INDENT_ITEM`.
pub const LEGEND_ROW_LEFT_OUTSET: f64 = 6.0;
/// A row's target ends this far right of the content width.
pub const LEGEND_ROW_RIGHT_OUTSET: f64 = 4.0;
/// A row target's corner radius.
pub const LEGEND_ROW_RADIUS: f64 = 6.0;
/// How far a row's focus and selection indicator sits inside its target, so
/// the stroke is not painted over by the next row (rows abut).
pub const LEGEND_ROW_INDICATOR_INSET: f64 = 1.0;
/// A selected legend row's bar, at its start inside the indicator's edge
/// (FR-72, FR-73): selection reads as a bar, focus as an outline, so a row
/// that is both shows two marks rather than one thicker edge.
pub const SELECTION_BAR_WIDTH: f64 = 3.0;
// The bar sits between the target's edge and the badge with room to spare,
// so it covers neither the badge nor the name and does not crowd them.
const _: () = assert!(
    LEGEND_ROW_INDICATOR_INSET + SELECTION_BAR_WIDTH + 2.0 <= LEGEND_ROW_LEFT_OUTSET,
    "a legend row's selection bar must clear the badge by two units"
);
/// The legend panel's padding left and right of the content.
pub const LEGEND_PANEL_INSET_X: f64 = 12.0;
/// The legend panel's padding above and below the content.
pub const LEGEND_PANEL_INSET_Y: f64 = 8.0;
/// The legend panel's width beyond its content: an inset each side.
pub const LEGEND_PANEL_INSETS_X: f64 = 2.0 * LEGEND_PANEL_INSET_X;
/// The legend panel's corner radius.
pub const LEGEND_PANEL_RADIUS: f64 = 8.0;
/// FR-79's fixed maximum for the legend block; its rendered width is the lesser
/// of this and the longest measured line. One value for both arrangements, so
/// the narrow document always holds it (D-56).
pub const LEGEND_WIDTH_MAX: f64 = NARROW_WIDTH - 2.0 * MARGIN;
/// The column a row's name and meta wrap in: `LEGEND_WIDTH_MAX` less the
/// name's indent.
pub const LEGEND_NAME_COLUMN: f64 = LEGEND_WIDTH_MAX - LEGEND_NAME_X;

// FR-94: a legend row is at least `MIN_TARGET_PX` tall wherever the document
// renders smallest, and no taller than it needs to be (D-66 counts every unit
// against the legend's length).
const _: () = assert!(
    LEGEND_ROW_MIN_HEIGHT * MIN_PX_PER_UNIT >= MIN_TARGET_PX
        && (LEGEND_ROW_MIN_HEIGHT - 1.0) * MIN_PX_PER_UNIT < MIN_TARGET_PX,
    "FR-94: a legend row must be the least whole height at least MIN_TARGET_PX tall"
);
// D-56: the content wraps at `LEGEND_WIDTH_MAX` and the panel reaches
// `LEGEND_PANEL_INSET_X` past it each side, so the block is placed at
// `MARGIN - LEGEND_PANEL_INSET_X` and must not reach past the document's edge.
const _: () = assert!(
    LEGEND_PANEL_INSET_X <= MARGIN,
    "D-56: the legend panel must not reach past the document's edge"
);
// The indicator's corners are concentric with the target's, radius
// `LEGEND_ROW_RADIUS - LEGEND_ROW_INDICATOR_INSET`, which must stay positive.
const _: () = assert!(
    LEGEND_ROW_INDICATOR_INSET < LEGEND_ROW_RADIUS,
    "a row's indicator must sit inside its target's rounded corners"
);

// --- Markers (FR-20, FR-40) -----------------------------------------------------
/// OQ-16's home: whether this is user units or output pixels is a change to
/// this constant's meaning, not an excavation (architecture §8).
pub const MARKER_RADIUS: f64 = 10.0;
/// Solid marker: a Surface1 stroke that separates overlapping markers.
pub const MARKER_STROKE: f64 = 2.0;
/// Hollow marker (FR-20): series-coloured stroke around a `MarkerHollow` fill.
pub const MARKER_HOLLOW_STROKE: f64 = 3.0;
/// The legend badge — a list bullet, deliberately smaller than the marker. 8,
/// not the mockup's 7: Inter `SemiBold` 9px "44" is ~12px, which a hollow r=7
/// badge (12px clear interior) cannot hold.
pub const BADGE_RADIUS: f64 = 8.0;
/// Outline of a solid legend badge.
pub const BADGE_STROKE: f64 = 1.5;
/// Thinner than the marker's, so the hollow fill gap survives the smaller size.
pub const BADGE_HOLLOW_STROKE: f64 = 2.0;
/// FR-35's quadrant-colour swatch beside each legend group heading.
pub const SWATCH_RADIUS: f64 = 5.0;
/// The swatch's centre sits this far above its heading's first baseline.
pub const SWATCH_RAISE: f64 = 4.0;
/// FR-40's status ring sits this far outside the marker.
pub const STATUS_RING_GAP: f64 = 4.0;
/// Stroke width of the status ring (FR-40).
pub const STATUS_RING_STROKE: f64 = 2.0;
/// The keyboard focus ring (`Focus`), from the marker's edge to the ring's
/// centre line.
///
/// Outside the status ring with a clear Surface1 gap between, so WCAG 1.4.11
/// measures it against the surface (6.0:1 and up) rather than a series fill,
/// where the focus blue manages only 1.3–2.5:1. That holds while nothing is
/// drawn under the ring: markers overlap (`MARKER_STROKE`), so the layout must
/// raise the focused marker over a Surface1 backing.
pub const FOCUS_RING_GAP: f64 = 8.0;
/// Stroke width of the keyboard focus ring.
pub const FOCUS_RING_STROKE: f64 = 2.0;
// The page strokes a legend row's indicator rect with the focus ring's width
// (PR 14b), centred on a rect inset `LEGEND_ROW_INDICATOR_INSET` from the
// row, so half the stroke must fit inside the inset or the next row, painted
// later, covers it.
const _: () = assert!(
    FOCUS_RING_STROKE <= 2.0 * LEGEND_ROW_INDICATOR_INSET,
    "a legend row's focus ring must fit inside its inset"
);

// --- Radar (FR-32, FR-33) -------------------------------------------------------
/// Outer edge of the outermost band; bands divide it equally (FR-32).
pub const BAND_OUTER_RADIUS: f64 = 260.0;
/// Reach of the quadrant dividers, just past the outermost band (FR-30).
pub const DIVIDER_RADIUS: f64 = 262.0;
/// Distance from the centre at which quadrant labels are anchored.
pub const QUADRANT_LABEL_RADIUS: f64 = 300.0;
/// Ring circles and quadrant dividers (`StrokeRing` / `StrokeDivider`).
pub const GRID_STROKE: f64 = 1.0;
/// Width of the Surface1 halo under ring labels, so the 12 o'clock divider
/// (FR-31) doesn't strike through them.
///
/// Emit it as its own stroke-only `<text>` painted before the label, not as
/// `paint-order: stroke`: an SVG consumer that ignores `paint-order` would
/// paint the halo over the glyphs and erase them.
pub const LABEL_HALO: f64 = 3.0;
/// The fraction of their natural size below which markers, their numbers and
/// the radar's strokes do not shrink (FR-70, D-69).
pub const RADAR_FLOOR: f64 = 7.0 / 9.0;
/// Clear space between the radar's outermost label and the block's edge.
pub const RADAR_LABEL_PAD: f64 = 10.0;
/// The least radar scale from which the radar block's size is proved
/// non-decreasing in the scale (the proof is on the radar's `half_extent`).
pub const RADAR_MONOTONE_FROM: f64 = 0.0034;

// --- Arrangements (FR-68–FR-71) -------------------------------------------------
/// Blank border around the whole document.
pub const MARGIN: f64 = 20.0;
/// Between radar and legend, wide arrangement only.
pub const COLUMN_GAP: f64 = 28.0;
/// The narrow document is authored at this width and the radar scaled to fit
/// its column, so text keeps its natural size (FR-70, D-31). D-71 widens it
/// when the radar cannot fit at its least scale.
pub const NARROW_WIDTH: f64 = 420.0;
/// The widest narrow document FR-94's targets are sized for (D-71); a wider
/// one renders them below `MIN_TARGET_PX`, and FR-95 warns.
pub const NARROW_MAX_WIDTH: f64 = 480.0;
/// The least scale the narrow radar is drawn at: below about 0.3 a quadrant
/// label at 6 o'clock touches the outer ring's stroke (D-70, D-71).
pub const NARROW_MIN_SCALE: f64 = 0.3;
// D-71: the narrow scale is searched where the radar's size is
// non-decreasing, and never above natural size.
const _: () = assert!(
    NARROW_MIN_SCALE > RADAR_MONOTONE_FROM && NARROW_MIN_SCALE <= 1.0,
    "D-71: the narrow scale's floor must lie in the radar's monotone range"
);
/// rg-06b, D-86: the radar's geometry grows in steps of this much, so an
/// edition redraws every marker only when it crosses a step.
pub const GROWTH_STEP: f64 = 0.25;
/// How many steps past natural size the radar may grow.
pub const GROWTH_STEPS: usize = 4;
/// The most the radar's geometry grows: 2×.
pub const MAX_GROWTH: f64 = 2.0;
const _: () = assert!(
    1.0 + 4.0 * GROWTH_STEP == MAX_GROWTH && GROWTH_STEPS == 4,
    "D-86: the steps end at MAX_GROWTH"
);
// D-86: a grown radar's narrow scale goes down to NARROW_MIN_SCALE /
// growth, which must stay in the radar's monotone range too.
const _: () = assert!(
    NARROW_MIN_SCALE / MAX_GROWTH > RADAR_MONOTONE_FROM,
    "D-86: the grown narrow floor must lie in the radar's monotone range"
);
/// D-86: the narrowest viewport that must still show the wide arrangement.
///
/// Growth widens the wide document, and FR-69's breakpoint with it, so the
/// radar grows no further than keeps the breakpoint at or below this.
pub const WIDE_MIN_VIEWPORT_PX: u32 = 1440;
/// Where the radar block sits from the document's left edge.
///
/// Its widest label's ink then lands on the margin, as the legend's content
/// does.
pub const RADAR_INSET: f64 = MARGIN - RADAR_LABEL_PAD;
/// The radar block's inset on both sides together.
pub const RADAR_INSETS: f64 = 2.0 * RADAR_INSET;
/// The width the narrow radar is scaled to fit: the document less the
/// radar's insets (D-71).
pub const NARROW_RADAR_COLUMN: f64 = NARROW_WIDTH - RADAR_INSETS;
/// Blank space above the document's first block.
///
/// With `TITLE_TOP` and the title's first baseline it puts the title's
/// capitals about `MARGIN` from the top edge: 20 − 4 − (18 − 0.727 · 17)
/// ≈ 10.4, rounded (Inter's cap height is 0.727 em). The exports have no
/// page frame to supply it.
pub const DOC_TOP: f64 = 10.0;
/// From the title block's bottom to the wide legend panel's top.
///
/// About 20 below the subtitle's baseline. Deliberately not aligned to the
/// radar's ring, which would add height to every document with a tall
/// legend.
pub const LEGEND_WIDE_DROP: f64 = 6.0;
/// From the key's bottom to the narrow legend panel's top.
///
/// The same distance as from the radar's ink to the legend panel in the wide
/// arrangement.
pub const KEY_TO_LEGEND: f64 = COLUMN_GAP - LEGEND_PANEL_INSET_X;
/// The rule above the key, the arrangement's (FR-66).
pub const KEY_RULE_STROKE: f64 = 1.0;
/// A floor on the radar block's width at scale 1, not its width.
///
/// The block's size is measured (architecture §4): quadrant labels never wrap
/// (FR-79), so a long one widens the block — `mockup.html` computes
/// `max(MIN_RADAR_W, 2 × (QLABEL_R + widest label / 2 + 10))`.
pub const MIN_RADAR_WIDTH: f64 = 660.0;

/// The smallest rendered size legend item names (`LEGEND_NAME`) may reach
/// before the HTML switches to the narrow arrangement (FR-69, architecture §4).
///
/// Smaller legend styles go proportionally lower; this is not a floor for them.
pub const MIN_LEGIBLE_PX: f64 = 10.0;

/// FR-64: the narrowest `--width`, in pixels.
pub const MIN_EXPORT_WIDTH: u32 = 320;
/// FR-64: the widest `--width`, and the clamp for a default past it (D-87).
pub const MAX_EXPORT_WIDTH: u32 = 10_000;
/// D-87: the largest image drawn, `MAX_EXPORT_WIDTH` squared.
pub const MAX_PNG_PIXELS: u32 = MAX_EXPORT_WIDTH * MAX_EXPORT_WIDTH;

/// WCAG 2.5.8's minimum target, in CSS pixels (FR-94).
pub const MIN_TARGET_PX: f64 = 24.0;
/// The narrowest viewport the narrow arrangement is held to (D-66).
pub const NARROW_MIN_VIEWPORT_PX: f64 = 320.0;
/// Horizontal space between the viewport and the narrow SVG. Zero because the
/// narrow SVG spans the viewport; PR 14 must keep that or change this
/// constant, since `MIN_PX_PER_UNIT` and `HIT_RADIUS` rest on it.
pub const NARROW_HORIZONTAL_INSET_PX: f64 = 0.0;
/// D-85: the least scale HTML draws the wide radar at, by width or height.
///
/// Where `LEGEND_NAME`-sized text renders at `MIN_LEGIBLE_PX`: the scale the
/// whole wide document reached at its breakpoint before the radar was drawn
/// apart from the legend.
pub const WIDE_MIN_SCALE: f64 = MIN_LEGIBLE_PX / LEGEND_NAME.size;
/// The fewest rendered pixels per user unit FR-94 is held to.
///
/// A narrow document `NARROW_MAX_WIDTH` wide at `NARROW_MIN_VIEWPORT_PX`, or
/// the wide radar at `WIDE_MIN_SCALE`, its least by width or height (D-85),
/// beside a legend at full size; the narrow term binds, at 2/3.
pub const MIN_PX_PER_UNIT: f64 =
    ((NARROW_MIN_VIEWPORT_PX - NARROW_HORIZONTAL_INSET_PX) / NARROW_MAX_WIDTH).min(WIDE_MIN_SCALE);
/// FR-94's hit-area radius at scale 1, in user units.
///
/// 18 renders at least `MIN_TARGET_PX` across at `MIN_PX_PER_UNIT` (asserted
/// below, in every build).
pub const HIT_RADIUS: f64 = 18.0;
// FR-94: the hit area is at least `MIN_TARGET_PX` across wherever the
// document renders smallest. A compile-time check, so changing a premise
// (the viewport, the inset, the legend size) cannot silently break it. The
// radius is the least whole one that works, so hit areas overlap no more
// than they must.
const _: () = assert!(
    2.0 * HIT_RADIUS * MIN_PX_PER_UNIT >= MIN_TARGET_PX
        && 2.0 * (HIT_RADIUS - 1.0) * MIN_PX_PER_UNIT < MIN_TARGET_PX,
    "FR-94: the hit radius must be the least whole one at least MIN_TARGET_PX across"
);
/// How far an indicator reaches from its marker's centre at scale 1: the
/// marker, the focus ring's gap and half the ring's stroke (D-66).
pub const INDICATOR_REACH: f64 = MARKER_RADIUS + FOCUS_RING_GAP + FOCUS_RING_STROKE / 2.0;

/// FR-69's media-query breakpoint for a wide document `wide_width` units
/// wide whose radar part is `radar_width` of them (D-85).
///
/// HTML draws the radar to fill the window and everything else at one pixel
/// per unit, so as the viewport narrows only the radar shrinks. The
/// breakpoint is where it reaches `WIDE_MIN_SCALE`: the radar's text draws
/// no smaller than it did before the split, and the legend's at full size.
///
/// Per document, not a constant — the widths grow with the legend and the
/// radar (§4). There is no upper bound: quadrant names never wrap (FR-79),
/// so a long enough one makes the wide document wider than any screen, which
/// is valid input whose breakpoint simply sits past every screen (rg-tzz).
/// FR-64's 10,000 bounds `--width` in output pixels, not layout units.
///
/// # Panics
///
/// On a width that is not a finite number of at least 1 unit, or a radar
/// part outside it: a layout bug, which would otherwise come out as a
/// silent `max-width: 0px` media query.
#[must_use]
pub fn breakpoint_px(wide_width: f64, radar_width: f64) -> u32 {
    assert!(
        wide_width.is_finite() && wide_width >= 1.0,
        "breakpoint_px: {wide_width} is not a document width"
    );
    assert!(
        (0.0..=wide_width).contains(&radar_width),
        "breakpoint_px: a {radar_width}-unit radar in a {wide_width}-unit document"
    );
    // How much the radar shrinks by the floor; the rest keeps its size.
    #[expect(
        clippy::suboptimal_flops,
        reason = "NFR-4: plain * and + are bit-identical on every target; mul_add falls back to a libm fma where the CPU lacks one"
    )]
    let shrinks = radar_width - radar_width * WIDE_MIN_SCALE;
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::as_conversions,
        reason = "f64 has no TryFrom to an integer; the asserts above make it positive and finite, and `as` saturates past u32::MAX, a breakpoint no screen reaches either way"
    )]
    let px = (wide_width - shrinks).round() as u32;
    px
}

/// D-86 and FR-105: whether a wide document `wide_width` units wide, its radar part
/// `radar_width`, still shows on a `WIDE_MIN_VIEWPORT_PX` screen.
///
/// A width `breakpoint_px` would refuse (not finite, or under 1) never
/// shows, and is never handed to it.
#[must_use]
pub fn wide_shows_on_laptops(wide_width: f64, radar_width: f64) -> bool {
    wide_width.is_finite()
        && wide_width >= 1.0
        && (0.0..=wide_width).contains(&radar_width)
        && breakpoint_px(wide_width, radar_width) <= WIDE_MIN_VIEWPORT_PX
}

#[cfg(test)]
mod tests {
    use super::*;

    /// D-78: the layout width is the shaper's advance plus WCAG 1.4.12's
    /// allowance, 0.12em per character and 0.16em per word separator.
    #[test]
    #[expect(
        clippy::suboptimal_flops,
        reason = "NFR-4: plain * and + are bit-identical on every target; mul_add falls back to a libm fma where the CPU lacks one"
    )]
    fn layout_width_is_advance_plus_allowance() {
        let fonts = crate::text::Fonts::embedded().expect("the vendored fonts parse");
        let text = "Kafka Streams";
        let advance = LEGEND_NAME.advance(text, &fonts);
        let want = advance + 0.12 * 12.5 * 13.0 + 0.16 * 12.5 * 1.0;
        assert!((LEGEND_NAME.measure(text, &fonts) - want).abs() < 1e-9);
        let nbsp = "a\u{a0}b";
        let want = LEGEND_NAME.advance(nbsp, &fonts) + 0.12 * 12.5 * 3.0 + 0.16 * 12.5;
        assert!(
            (LEGEND_NAME.measure(nbsp, &fonts) - want).abs() < 1e-9,
            "NBSP is a word separator"
        );
        assert!(LEGEND_NAME.measure("", &fonts).abs() < 1e-12);
    }

    /// Every wrapped line fits its column at the layout width, so spaced
    /// text stays inside its block (D-78).
    #[test]
    fn wrap_holds_spaced_lines() {
        let fonts = crate::text::Fonts::embedded().expect("the vendored fonts parse");
        let text = "Infrastructure Platforms and Tooling for Managed Estates";
        for column in [80.0, 150.0, 260.0] {
            let lines = LEGEND_NAME.wrap(text, column, &fonts);
            assert!(lines.len() > 1, "{column}: the fixture wraps");
            for line in &lines {
                assert!(
                    LEGEND_NAME.measure(line, &fonts) <= column,
                    "{column}: {line}"
                );
            }
        }
    }

    /// One measure for every block: the style's face, size and spacing.
    #[test]
    fn a_style_measures_with_its_face_size_and_spacing() {
        let fonts = Fonts::embedded().expect("faces");
        for style in [LEGEND_NAME, QUADRANT_LABEL, RING_LABEL, TITLE] {
            let want =
                fonts
                    .face(style.weight)
                    .measure("Kubernetes", style.size, style.letter_spacing);
            let got = style.advance("Kubernetes", &fonts);
            assert!((got - want).abs() < 1e-12, "{style:?}: {got} vs {want}");
        }
    }

    /// And one wrap (FR-79).
    #[test]
    fn a_style_wraps_with_its_face_size_and_spacing() {
        let fonts = Fonts::embedded().expect("faces");
        let text = "Continuous delivery pipelines for managed infrastructure";
        let style = LEGEND_NAME;
        // At the layout width (D-78), not the bare advance.
        let want = crate::text::wrap_by(text, 200.0, &|line| style.measure(line, &fonts));
        assert_eq!(style.wrap(text, 200.0, &fonts), want);
        assert!(want.len() > 1, "the fixture wraps: {want:?}");
    }

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
            for b in levels.iter().skip(i + 1) {
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
    #[expect(
        clippy::suboptimal_flops,
        reason = "NFR-4: plain * and + are bit-identical on every target; mul_add falls back to a libm fma where the CPU lacks one"
    )]
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

    /// OQ-16, as on the markers: three digits do not fit a hollow badge.
    /// Recorded so a change of face or size that makes them fit is noticed.
    #[test]
    fn three_digit_numbers_overflow_the_badge() {
        let fonts = Fonts::embedded().expect("faces");
        let t = LEGEND_BADGE_NUMBER;
        let widest = (100..=999_u32)
            .map(|n| {
                fonts
                    .face(t.weight)
                    .measure(&n.to_string(), t.size, t.letter_spacing)
            })
            .fold(0.0_f64, f64::max);
        // A hollow stroke is centred on r, so the clear interior is 2r − stroke.
        #[expect(
            clippy::suboptimal_flops,
            reason = "NFR-4: plain * and + are bit-identical on every target; mul_add falls back to a libm fma where the CPU lacks one"
        )]
        let clear = 2.0 * BADGE_RADIUS - BADGE_HOLLOW_STROKE;
        assert!(widest > clear, "OQ-16: {widest} now fits in {clear}");
    }

    #[test]
    fn badge_is_smaller_than_the_marker() {
        // mockup.html: the badge is a list bullet, not a second copy of the marker.
        const { assert!(BADGE_RADIUS < MARKER_RADIUS) };
    }

    /// Architecture §4: W × (floor / legend size), rounded. 1148 is the
    /// mockup's wide width (660 + 28 + 440 + 20), whose fixed breakpoint was
    /// 900.
    #[test]
    fn breakpoint_is_computed_from_the_wide_width() {
        // A document that is all radar breaks where it did before the split.
        assert_eq!(breakpoint_px(1148.0, 1148.0), 918);
        assert_eq!(breakpoint_px(1250.0, 1250.0), 1000);
        // 1150.625 × 10 / 12.5 is exactly 920.5, which rounds away from zero.
        assert_eq!(breakpoint_px(1150.625, 1150.625), 921);
        // D-56: a full-width 380 legend beside a 670-unit radar part keeps
        // its 418 units whole, so 1088 breaks at 1088 − 0.2 × 670.
        assert_eq!(breakpoint_px(1088.0, 670.0), 954);
        // With nothing to shrink, the document never fits a narrower window.
        assert_eq!(breakpoint_px(400.0, 0.0), 400);
    }

    #[test]
    #[should_panic(expected = "is not a document width")]
    fn a_zero_width_is_a_bug_not_a_breakpoint() {
        let _ = breakpoint_px(0.0, 0.0);
    }

    #[test]
    fn widths_that_are_not_document_widths_are_bugs() {
        for w in [f64::NAN, f64::INFINITY, -100.0, 0.5] {
            assert!(
                std::panic::catch_unwind(|| breakpoint_px(w, 0.0)).is_err(),
                "{w}"
            );
        }
        assert_eq!(breakpoint_px(1.0, 1.0), 1);
    }

    /// The focus ring clears the status ring by at least 2 units of surface, so
    /// it is judged against the surface, never a series fill (WCAG 1.4.11).
    #[test]
    fn focus_ring_sits_clear_of_the_status_ring() {
        let status_outer = STATUS_RING_GAP + STATUS_RING_STROKE / 2.0;
        let focus_inner = FOCUS_RING_GAP - FOCUS_RING_STROKE / 2.0;
        assert!(
            focus_inner - status_outer >= 2.0,
            "{status_outer} vs {focus_inner}"
        );
    }

    /// D-85: at the breakpoint the radar part draws at the scale the whole
    /// document did before the split, and the rest keeps its natural size.
    #[test]
    fn breakpoint_puts_the_radar_at_the_floor() {
        let (w, r) = (1400.0, 900.0);
        let radar_scale = (f64::from(breakpoint_px(w, r)) - (w - r)) / r;
        let rendered = LEGEND_NAME.size * radar_scale;
        assert!((rendered - MIN_LEGIBLE_PX).abs() < 0.02, "{rendered}");
    }

    /// rg-tzz: a quadrant name long enough to make the wide document wider
    /// than any screen is valid input, not a layout bug. Its breakpoint sits
    /// past every screen, so the narrow arrangement shows, as FR-95 warns.
    #[test]
    fn a_document_wider_than_any_screen_has_a_breakpoint() {
        // A 600-W quadrant name: the long label is in the radar part.
        assert_eq!(breakpoint_px(18_355.334, 17_944.22), 14_766);
        // Past what a u32 holds, the breakpoint saturates rather than wraps.
        assert_eq!(breakpoint_px(5e9, 0.0), u32::MAX);
    }

    #[test]
    #[should_panic(expected = "-unit radar in a")]
    fn a_radar_wider_than_its_document_is_a_bug() {
        let _ = breakpoint_px(1000.0, 1000.5);
    }

    #[test]
    fn min_px_per_unit_is_the_narrow_viewport_over_its_widest() {
        assert!(
            (MIN_PX_PER_UNIT - 320.0 / 480.0).abs() < 1e-12,
            "{MIN_PX_PER_UNIT}"
        );
    }

    #[test]
    fn indicator_reach_is_19() {
        assert!((INDICATOR_REACH - 19.0).abs() < 1e-12, "{INDICATOR_REACH}");
    }

    /// D-86: a document wider than a laptop, or no width at all, never
    /// shows, and a non-width is never handed to `breakpoint_px`, which
    /// would panic on it.
    #[test]
    fn a_document_past_a_laptop_does_not_show_on_laptops() {
        assert!(!wide_shows_on_laptops(18_355.334, 600.0));
        assert!(!wide_shows_on_laptops(f64::NAN, 600.0));
        assert!(!wide_shows_on_laptops(1000.0, f64::NAN));
        assert!(wide_shows_on_laptops(1000.0, 600.0));
    }

    /// D-86 and FR-105: a breakpoint of exactly `WIDE_MIN_VIEWPORT_PX`
    /// still shows on a laptop; one pixel more does not.
    #[test]
    fn a_breakpoint_at_a_laptop_still_shows_on_laptops() {
        assert_eq!(breakpoint_px(1440.0, 0.0), WIDE_MIN_VIEWPORT_PX);
        assert!(wide_shows_on_laptops(1440.0, 0.0));
        assert!(!wide_shows_on_laptops(1441.0, 0.0));
    }
}
