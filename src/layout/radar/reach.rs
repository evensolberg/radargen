//! How far the radar's labels reach from its centre, and so how large the
//! block is at a scale (FR-70, FR-94, D-71).

use super::{HALO_REACH, MIN_RADAR_HALF, RadarScale, SectorLabel};
use crate::style::{
    BAND_OUTER_RADIUS, HIT_RADIUS, INDICATOR_REACH, RADAR_FLOOR, RADAR_LABEL_PAD, RING_LABEL,
    SECTOR_LABEL,
};
use crate::text::{TEXT_ASCENT, TEXT_DESCENT};

/// How far a sector label reaches left or right of the centre: its pushed
/// centre's distance, then half its middle-anchored width.
#[expect(
    clippy::suboptimal_flops,
    reason = "NFR-4: plain * and + are bit-identical on every target; mul_add falls back to a libm fma where the CPU lacks one"
)]
pub(super) fn sector_reach_x(sector: &SectorLabel, r: RadarScale) -> f64 {
    sector.centre(r).x.abs() + f64::midpoint(0.0, sector.label.width) * r.labels
}

/// How far a sector label reaches above or below the centre: the descent
/// below its pushed baseline, or the ascent above it.
#[expect(
    clippy::suboptimal_flops,
    reason = "NFR-4: plain * and + are bit-identical on every target; mul_add falls back to a libm fma where the CPU lacks one"
)]
pub(super) fn sector_reach_y(sector: &SectorLabel, r: RadarScale) -> f64 {
    let baseline = sector.centre(r).y;
    let size = SECTOR_LABEL.size * r.labels;
    (baseline + TEXT_DESCENT * size).max(TEXT_ASCENT * size - baseline)
}

/// How far a ring name of `width`, centred on the vertical axis, reaches
/// either side, its halo included.
pub(super) fn ring_reach_x(width: f64, r: RadarScale) -> f64 {
    (f64::midpoint(0.0, width) + HALO_REACH) * r.labels
}

/// How far a ring name whose baseline is `mid` above the centre reaches
/// upward, its ascent and halo included. It never reaches further down
/// than up.
#[expect(
    clippy::suboptimal_flops,
    reason = "NFR-4: plain * and + are bit-identical on every target; mul_add falls back to a libm fma where the CPU lacks one"
)]
pub(super) fn ring_reach_y(mid: f64, r: RadarScale) -> f64 {
    mid * r.geometry + (TEXT_ASCENT * RING_LABEL.size + HALO_REACH) * r.labels
}

/// Half the block on one axis: half the minimum radar width at this scale,
/// the labels' reach plus the pad, or the hit areas' reach, whichever is
/// largest. Hit areas are invisible, so they take no pad.
///
/// Below, `s` stands for `geometry`, the radar scale times D-86's growth
/// `g`; `labels` and `marks` stay functions of the radar scale alone. With
/// `g >= 1`, `marks <= geometry` wherever `marks` is not floored, so each
/// bound that held at `g = 1` holds grown.
///
/// The `MIN_RADAR_HALF * s` floor is also what keeps the ring circles (at
/// most `260 * s` plus half a stroke) and the dividers (`262 * s` plus half
/// a stroke) inside the block: the label terms never count them. That holds
/// for `s` of about 0.0056 and up, where `70 * s` covers half a floored
/// stroke.
///
/// Markers and their indicators need no term of their own: an indicator
/// reaches furthest of everything a marker draws (`INDICATOR_REACH`, 19, at
/// `marks`, past the status ring's 15 and any number under six digits), and
/// it never passes `max(floor, hit reach)`. For a marker `|p|` from the
/// centre on an axis: at a radar scale below `RADAR_FLOOR` it reaches `|p| s + 19 · 7/9`,
/// under the hit area's `|p| s + HIT_RADIUS` because `INDICATOR_REACH * RADAR_FLOOR
/// <= HIT_RADIUS`; from `RADAR_FLOOR` up it reaches `(|p| + 19) s`, under
/// the floor's `330 s` because `|p| < BAND_OUTER_RADIUS` and
/// `BAND_OUTER_RADIUS + INDICATOR_REACH <= MIN_RADAR_HALF`. Both are
/// asserted at compile time, below `half_extent`. The hit term itself binds only
/// for `s < HIT_RADIUS / (330 - |p|)`, about 0.26 at most.
///
/// The block's size is non-decreasing in the radar scale from
/// `RADAR_MONOTONE_FROM` (0.0034) over the growth up, since every term
/// depends on the scale only through `geometry`, `labels` and `marks`, each
/// non-decreasing in it. A sector label's pushed centre is `300 s` plus a push at
/// `max(scale, 1)` along the same direction, so its distance from the centre on
/// each axis never falls. The only decreasing term is a below-centre sector label's
/// ascent, at most 11.625, since its push only lowers the baseline; the
/// outermost ring name's reach, at least `130 * s + 11.19`,
/// outgrows it from there. That relies on at least one ring, which
/// `rings.rs` guarantees by rejecting an empty list. The hit term, `|p| s +
/// HIT_RADIUS · max(scale, 1)`, is increasing.
#[expect(
    clippy::suboptimal_flops,
    reason = "NFR-4: plain * and + are bit-identical on every target; mul_add falls back to a libm fma where the CPU lacks one"
)]
pub(super) fn half_extent(label_reach: f64, hit_reach: f64, r: RadarScale) -> f64 {
    (MIN_RADAR_HALF * r.geometry)
        .max(label_reach + RADAR_LABEL_PAD * r.labels)
        .max(hit_reach)
}

// `half_extent`'s indicator bounds, checked in every build.
const _: () = assert!(
    INDICATOR_REACH * RADAR_FLOOR <= HIT_RADIUS,
    "below the floor scale an indicator must stay inside the hit area"
);
const _: () = assert!(
    BAND_OUTER_RADIUS + INDICATOR_REACH <= MIN_RADAR_HALF,
    "above the floor scale an indicator must stay inside the floor"
);
