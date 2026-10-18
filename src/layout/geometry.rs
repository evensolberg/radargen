//! The radar's geometry (architecture §3): FR-77's dividers, FR-32's bands,
//! FR-33's marker placement and FR-89's capacity, as pure functions.
//!
//! Coordinates are relative to the radar's centre in SVG's frame: 0° is
//! 3 o'clock, angles grow clockwise on screen, and y grows downward. The
//! formulas are the prototype's, ratified rather than re-derived (D-36).

use crate::diagnostics::{RenderKind, Warning, Warnings};
use crate::model::Catalogue;
use crate::order::Plotted;
use crate::style::{BAND_OUTER_RADIUS, GROWTH_STEP, GROWTH_STEPS, MARKER_RADIUS, MAX_GROWTH};

pub use crate::svg::Point;

/// FR-30: a full turn, which N equal sectors divide.
const FULL_TURN: f64 = 360.0;
/// FR-77: divider 0 sits at 12 o'clock.
const FIRST_DIVIDER: f64 = -90.0;
/// FR-33: markers keep this proportion of the sector clear of each divider.
const INSET: f64 = 0.08;
/// FR-33: the proportion of the sector markers spread across. FR-33 states
/// 0.84 literally; it equals `1 − 2 × INSET`, and the literal keeps
/// bit-parity with the prototype.
const SPREAD: f64 = 0.84;
/// FR-33: the inner and outer tracks, as proportions of the band's width;
/// even-indexed items take the inner one.
const INNER_TRACK: f64 = 0.38;
const OUTER_TRACK: f64 = 0.68;
/// FR-89: how far below one marker diameter two centres may be and still
/// count as clear. `sin` and `cos` differ across platforms by about 1e-13,
/// so a distance exactly at the threshold could otherwise classify
/// differently on two machines and change the warning (NFR-4).
const CAPACITY_TOLERANCE: f64 = 1e-9;
/// FR-88, A-5: above this many quadrants, sectors narrow and labels crowd.
const COMFORTABLE_QUADRANTS: usize = 6;
/// FR-89: the least distance between two clear marker centres.
const CLEARANCE: f64 = 2.0 * MARKER_RADIUS - CAPACITY_TOLERANCE;

/// One ring's band: the radii it spans (FR-32).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Band {
    /// The radius of the band's edge nearer the centre.
    pub inner: f64,
    /// The radius of the band's edge nearer the rim.
    pub outer: f64,
}

/// The two counts every formula needs: quadrants (FR-30) and declared rings
/// (FR-32).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Geometry {
    /// How many distinct quadrants the catalogue has (FR-30).
    pub quadrants: usize,
    /// How many rings are declared (FR-32).
    pub rings: usize,
}

impl Geometry {
    /// The counts for `catalogue`: FR-8's distinct quadrants and every
    /// declared ring, empty ones included (FR-19).
    #[must_use]
    pub const fn of(catalogue: &Catalogue) -> Self {
        Self {
            quadrants: catalogue.quadrants.len(),
            rings: catalogue.rings.len(),
        }
    }

    /// FR-30: the degrees each quadrant's sector spans. With zero quadrants
    /// (a catalogue with no rows, FR-42) there is no sector: this is
    /// infinite, so a caller draws only the ring circles then.
    #[must_use]
    pub fn sector(self) -> f64 {
        FULL_TURN / real(self.quadrants)
    }

    /// FR-77: the angle of the divider that opens the sector at FR-10
    /// `position`, in degrees. Divider 0 is at 12 o'clock and the rest
    /// follow clockwise, so FR-31 holds at every count. With zero quadrants
    /// (FR-42) there is no sector and this is NaN, so a caller draws only
    /// the ring circles then.
    #[must_use]
    #[expect(
        clippy::suboptimal_flops,
        reason = "NFR-4: plain * and + are bit-identical on every target; mul_add falls back to a libm fma where the CPU lacks one"
    )]
    pub fn divider(self, position: usize) -> f64 {
        FIRST_DIVIDER + real(position) * self.sector()
    }

    /// The angle halfway across `position`'s sector, in degrees. NaN with
    /// no quadrants, like [`divider`](Self::divider).
    #[must_use]
    pub fn middle(self, position: usize) -> f64 {
        f64::midpoint(self.divider(position), self.divider(position + 1))
    }

    /// FR-32: declared ring `ring`'s band. Widths are equal and depend on
    /// the ring count alone (D-47); the first declared ring is outermost
    /// (D-60).
    ///
    /// # Panics
    ///
    /// In debug builds, if `ring` is not below `rings`. Callers pass a
    /// declared ring index, which is.
    #[must_use]
    pub fn band(self, ring: usize) -> Band {
        debug_assert!(ring < self.rings, "ring {ring} of {}", self.rings);
        let width = BAND_OUTER_RADIUS / real(self.rings);
        let from_centre = real(self.rings - ring);
        Band {
            inner: width * (from_centre - 1.0),
            outer: width * from_centre,
        }
    }

    /// FR-33: the angle of item `j` of `n` in the sector at `position`,
    /// spread between insets of 8% of the sector at each end.
    #[must_use]
    #[expect(
        clippy::suboptimal_flops,
        reason = "NFR-4: plain * and + are bit-identical on every target; mul_add falls back to a libm fma where the CPU lacks one"
    )]
    pub fn angle(self, position: usize, j: usize, n: usize) -> f64 {
        let along = real(j + 1) / real(n + 1);
        self.divider(position) + self.sector() * (INSET + along * SPREAD)
    }

    /// FR-33: item `j`'s distance from the centre in ring `ring`'s band,
    /// alternating between two tracks by index parity, never by a PRNG
    /// (NFR-4).
    #[must_use]
    #[expect(
        clippy::suboptimal_flops,
        reason = "NFR-4: plain * and + are bit-identical on every target; mul_add falls back to a libm fma where the CPU lacks one"
    )]
    pub fn radius(self, ring: usize, j: usize) -> f64 {
        let band = self.band(ring);
        let track = if j.is_multiple_of(2) {
            INNER_TRACK
        } else {
            OUTER_TRACK
        };
        band.inner + (band.outer - band.inner) * track
    }

    /// FR-33: item `j` of `n`'s marker centre, in the sector at FR-10
    /// `position` and declared ring `ring`'s band.
    #[must_use]
    pub fn marker(self, position: usize, ring: usize, j: usize, n: usize) -> Point {
        polar(self.angle(position, j, n), self.radius(ring, j))
    }

    /// FR-89: whether `n` plain markers in ring `ring`'s band, placed by
    /// FR-33 on a radar grown by `growth` (D-86), keep every two centres at
    /// least one marker diameter apart. Placement is the same in every
    /// sector, so sector 0 stands for all.
    #[must_use]
    pub fn fits(self, ring: usize, n: usize, growth: f64) -> bool {
        let points: Vec<Point> = (0..n).map(|j| self.marker(0, ring, j, n)).collect();
        points.iter().enumerate().all(|(i, a)| {
            points
                .iter()
                .skip(i + 1)
                .all(|b| far_enough(distance(*a, *b) * growth))
        })
    }

    /// FR-89: the largest `n`, up to `limit`, for which the band fits at
    /// `growth` —
    /// searched upward and stopping at the first failure, which is the
    /// largest because fitting is monotone in `n`. A band of one always
    /// fits, so this is at least 1 when `limit` is.
    #[must_use]
    pub fn capacity(self, ring: usize, limit: usize, growth: f64) -> usize {
        (1..=limit)
            .take_while(|&n| self.fits(ring, n, growth))
            .count()
    }
}

/// The point at `angle` degrees and `radius` from the centre, in SVG's
/// frame (0° is 3 o'clock, clockwise, y down).
#[must_use]
pub fn polar(angle: f64, radius: f64) -> Point {
    let (sin, cos) = angle.to_radians().sin_cos();
    Point {
        x: radius * cos,
        y: radius * sin,
    }
}

/// FR-33: every plotted item's marker centre, aligned with
/// [`Plotted::items`] for a `Plotted` that `resolve` built. Item `j` of a
/// band is its `j`th in FR-34's order.
///
/// # Panics
///
/// If `plotted` was not built from `catalogue`, so a run's quadrant is
/// missing from `plotted.rank`.
#[must_use]
// qual:allow(complexity, error_handling) reason: "the documented # Panics: a Plotted from another catalogue is a caller bug"
pub fn place(catalogue: &Catalogue, plotted: &Plotted) -> Vec<Point> {
    let geometry = Geometry::of(catalogue);
    let points: Vec<Point> = plotted
        .runs(catalogue)
        .iter()
        .flat_map(|run| {
            let n = run.items.len();
            let position = *plotted
                .rank
                .get(run.quadrant)
                .expect("rank covers every quadrant of the catalogue");
            (0..n).map(move |j| geometry.marker(position, run.ring, j, n))
        })
        .collect();
    debug_assert_eq!(
        points.len(),
        plotted.items.len(),
        "every plotted item has a ring and FR-34 keeps bands contiguous"
    );
    points
}

/// D-86's growth factors, smallest first: 1, then a `GROWTH_STEP` more
/// each, up to `MAX_GROWTH`.
#[expect(
    clippy::suboptimal_flops,
    reason = "NFR-4: plain * and + are bit-identical on every target; mul_add falls back to a libm fma where the CPU lacks one"
)]
pub fn growth_steps() -> impl DoubleEndedIterator<Item = f64> {
    (0..=GROWTH_STEPS).map(|step| 1.0 + real(step) * GROWTH_STEP)
}

/// D-86: the least growth step at which every two of `points` are a marker
/// diameter apart.
///
/// `points` are marker centres at natural size; past the last step this is
/// `MAX_GROWTH`. Every FR-33 position is proportional to the radius, so
/// growing by `g` moves every pair `g` times as far apart.
///
/// Any two markers, not only two in one band: across a divider, two
/// sectors' markers can be closer than any pair inside a band.
#[must_use]
pub fn growth(points: &[Point]) -> f64 {
    let closest = points
        .iter()
        .enumerate()
        .flat_map(|(i, &a)| points.iter().skip(i + 1).map(move |&b| distance(a, b)))
        .fold(f64::INFINITY, f64::min);
    growth_steps()
        .find(|&g| far_enough(closest * g))
        .unwrap_or(MAX_GROWTH)
}

/// How far apart two points are.
fn distance(a: Point, b: Point) -> f64 {
    (a.x - b.x).hypot(a.y - b.y)
}

/// FR-88 and FR-89: one warning when the quadrant count passes A-5's six,
/// then one per band still overfull, in FR-34's order.
///
/// Bands are judged on a radar grown by `growth` (D-86). Both warnings
/// concern the render as a whole, so NFR-11 prints them before every row
/// warning.
///
/// Capacity stops at the first failure, so a band of any size costs at
/// most its capacity's checks; this relies on the monotonicity that
/// `fitting_is_monotone_in_n` verifies.
///
/// # Panics
///
/// If `plotted` was not built from `catalogue`, so a run names a quadrant
/// or ring the catalogue lacks.
// qual:allow(complexity, error_handling) reason: "the documented # Panics: a Plotted from another catalogue is a caller bug"
pub fn warn_crowding(
    catalogue: &Catalogue,
    plotted: &Plotted,
    growth: f64,
    warnings: &mut Warnings,
) {
    let geometry = Geometry::of(catalogue);
    if geometry.quadrants > COMFORTABLE_QUADRANTS {
        warnings.push(Warning::Render(RenderKind::QuadrantCount {
            count: geometry.quadrants,
        }));
    }
    for run in plotted.runs(catalogue) {
        let count = run.items.len();
        let fits = geometry.capacity(run.ring, count, growth);
        if fits < count {
            warnings.push(Warning::Render(RenderKind::OverfullBand {
                quadrant: catalogue
                    .quadrants
                    .get(run.quadrant)
                    .expect("a run's quadrant is one of the catalogue's")
                    .label
                    .clone(),
                ring: catalogue
                    .rings
                    .get(run.ring)
                    .expect("a run's ring is one of the catalogue's")
                    .clone(),
                count,
                fits,
            }));
        }
    }
}

/// FR-89: whether two marker centres `distance` apart do not overlap.
fn far_enough(distance: f64) -> bool {
    distance >= CLEARANCE
}

/// A count as `f64`.
#[expect(
    clippy::cast_precision_loss,
    clippy::as_conversions,
    reason = "usize has no lossless conversion to f64, and f64 holds every integer up to 2^53 exactly; a count is far below that"
)]
const fn real(n: usize) -> f64 {
    n as f64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::block::close;

    fn geometry(quadrants: usize, rings: usize) -> Geometry {
        Geometry { quadrants, rings }
    }

    #[test]
    fn middle_is_halfway_across_the_sector() {
        let expect = |n: usize, want: &[f64]| {
            let got: Vec<f64> = (0..n).map(|i| geometry(n, 4).middle(i)).collect();
            assert!(
                got.len() == want.len() && got.iter().zip(want).all(|(a, b)| close(*a, *b)),
                "{n}: {got:?} vs {want:?}"
            );
        };
        expect(1, &[90.0]);
        expect(2, &[0.0, 180.0]);
        expect(3, &[-30.0, 90.0, 210.0]);
        expect(4, &[-45.0, 45.0, 135.0, 225.0]);
        let four = geometry(4, 4);
        for p in 0..4 {
            assert!(close(four.middle(p), four.angle(p, 0, 1)));
        }
        assert!(geometry(0, 4).middle(0).is_nan());
    }

    /// FR-77, FR-43: divider 0 at 12 o'clock (-90°), the rest clockwise at
    /// equal steps, for one and two quadrants as for any other count.
    #[test]
    fn dividers_start_at_twelve_and_run_clockwise() {
        let dividers =
            |n: usize| -> Vec<f64> { (0..n).map(|i| geometry(n, 4).divider(i)).collect() };
        assert_eq!(dividers(1), [-90.0]);
        assert_eq!(dividers(2), [-90.0, 90.0]);
        // N = 2's second sector opens at 6 o'clock; one item sits at 9.
        assert!(close(geometry(2, 4).angle(1, 0, 1), 180.0));
        assert_eq!(dividers(3), [-90.0, 30.0, 150.0]);
        assert_eq!(dividers(4), [-90.0, 0.0, 90.0, 180.0]);
        assert!(close(geometry(4, 4).sector(), 90.0));
    }

    /// FR-32, D-60: equal widths, the first declared ring outermost.
    #[test]
    fn the_first_declared_ring_is_outermost() {
        let g = geometry(4, 4);
        assert_eq!(
            g.band(0),
            Band {
                inner: 195.0,
                outer: 260.0
            }
        );
        assert_eq!(
            g.band(3),
            Band {
                inner: 0.0,
                outer: 65.0
            }
        );
        let g = geometry(4, 5);
        assert_eq!(
            g.band(0),
            Band {
                inner: 208.0,
                outer: 260.0
            }
        );
        assert_eq!(
            g.band(4),
            Band {
                inner: 0.0,
                outer: 52.0
            }
        );
    }

    /// FR-33: one item sits mid-sector, on the inner track; the next
    /// alternates to the outer track (parity, never a PRNG).
    #[test]
    #[expect(
        clippy::suboptimal_flops,
        reason = "NFR-4: plain * and + are bit-identical on every target; mul_add falls back to a libm fma where the CPU lacks one"
    )]
    fn markers_follow_fr33() {
        let g = geometry(4, 4);
        // Position 1's sector opens at 0°; one item sits at its centre.
        assert!(close(g.angle(1, 0, 1), 45.0));
        assert!(close(g.radius(0, 0), 65.0 * 0.38 + 195.0));
        assert!(close(g.radius(0, 1), 65.0 * 0.68 + 195.0));
        assert!(close(g.radius(0, 2), g.radius(0, 0)));
        let p = g.marker(1, 0, 0, 1);
        let r = 65.0 * 0.38 + 195.0;
        let h = r * std::f64::consts::FRAC_1_SQRT_2;
        assert!(close(p.x, h) && close(p.y, h), "{p:?}");
    }

    /// SVG's frame: -90° is straight up, 0° is right, 90° is down.
    #[test]
    fn polar_uses_svgs_frame() {
        let up = polar(-90.0, 10.0);
        assert!(close(up.x, 0.0) && close(up.y, -10.0), "{up:?}");
        let right = polar(0.0, 10.0);
        assert!(close(right.x, 10.0) && close(right.y, 0.0), "{right:?}");
        let down = polar(90.0, 10.0);
        assert!(close(down.x, 0.0) && close(down.y, 10.0), "{down:?}");
    }

    /// FR-33's 8% inset at both ends: every marker stays strictly inside
    /// its sector, clear of both dividers, for any count.
    #[test]
    #[expect(
        clippy::suboptimal_flops,
        reason = "NFR-4: plain * and + are bit-identical on every target; mul_add falls back to a libm fma where the CPU lacks one"
    )]
    fn markers_stay_inset_from_both_dividers() {
        for quadrants in 1..=8 {
            let g = geometry(quadrants, 4);
            for n in 1..=40 {
                for j in 0..n {
                    let a = g.angle(0, j, n);
                    assert!(
                        a > 0.08 * g.sector() + g.divider(0) - 1e-9,
                        "N={quadrants} n={n} j={j}"
                    );
                    assert!(
                        a < 0.92 * g.sector() + g.divider(0) + 1e-9,
                        "N={quadrants} n={n} j={j}"
                    );
                }
            }
        }
    }

    /// FR-43: with one quadrant the single divider is both edges, so the
    /// 8% inset leaves a gap either side of 12 o'clock; one item sits at
    /// 6 o'clock.
    #[test]
    #[expect(
        clippy::suboptimal_flops,
        reason = "NFR-4: plain * and + are bit-identical on every target; mul_add falls back to a libm fma where the CPU lacks one"
    )]
    fn a_single_quadrant_leaves_a_gap_at_twelve() {
        let g = geometry(1, 4);
        assert!(close(g.angle(0, 0, 1), 90.0));
        let first = g.angle(0, 0, 3);
        let last = g.angle(0, 2, 3);
        assert!(close(first, 360.0 * (0.25 * 0.84 + 0.08) - 90.0));
        assert!(close(last, 360.0 * (0.75 * 0.84 + 0.08) - 90.0));
        assert!(first > -90.0 + 28.0 && last < 270.0 - 28.0);
    }

    /// Every marker lies inside its band, between its inner and outer
    /// radius (FR-19's empty bands draw regardless; this is the full ones).
    #[test]
    fn markers_lie_inside_their_band() {
        let g = geometry(3, 5);
        for ring in 0..5 {
            let band = g.band(ring);
            for j in 0..4 {
                let r = g.radius(ring, j);
                assert!(band.inner < r && r < band.outer, "ring {ring} j {j}");
            }
        }
    }

    /// The two points `d` apart along the x axis.
    fn pair(d: f64) -> [Point; 2] {
        [Point::ORIGIN, Point { x: d, y: 0.0 }]
    }

    /// rg-06b: a radar whose markers already clear, or that has fewer than
    /// two, does not grow.
    #[test]
    fn growth_is_one_when_every_pair_clears() {
        let apart = pair(2.0 * MARKER_RADIUS);
        assert!(close(growth(&apart), 1.0));
        assert!(close(growth(apart.get(..1).expect("one point")), 1.0));
        assert!(close(growth(&[]), 1.0));
    }

    /// The least step at which the closest pair clears: 20/13 is 1.54, so
    /// 1.75; 16 clears at exactly 1.25; past the cap, the cap.
    #[test]
    fn growth_is_the_least_step_that_clears() {
        assert!(close(growth(&pair(13.0)), 1.75));
        assert!(close(growth(&pair(16.0)), 1.25));
        assert!(close(growth(&pair(19.0)), 1.25));
        assert!(close(growth(&pair(1.0)), MAX_GROWTH));
        let steps: Vec<f64> = growth_steps().collect();
        assert_eq!(steps, [1.0, 1.25, 1.5, 1.75, 2.0]);
    }

    /// The closest pair is any two markers, not two in one band: at eight
    /// quadrants, one item in each innermost band fits its band, but
    /// neighbours across a divider are 18.9 apart.
    #[test]
    fn growth_counts_pairs_across_a_divider() {
        let g = geometry(8, 4);
        let points: Vec<Point> = (0..8).map(|p| g.marker(p, 3, 0, 1)).collect();
        assert!(g.fits(3, 1, 1.0));
        assert!(close(growth(&points), 1.25));
        let far = pair(30.0);
        let near = [far[0], far[1], Point { x: 0.0, y: 15.0 }];
        assert!(close(growth(&near), 1.5), "the closest of three pairs");
    }

    /// FR-89 at a growth factor: a band that overflows at 1 fits once grown
    /// far enough, and capacity rises with it.
    #[test]
    fn a_grown_band_holds_more() {
        let g = geometry(4, 4);
        assert_eq!(g.capacity(3, 100, 1.0), 2);
        assert!(!g.fits(3, 3, 1.0));
        assert!(g.fits(3, 3, 1.25));
        assert!(g.capacity(3, 100, 2.0) > g.capacity(3, 100, 1.0));
    }

    /// Each band's capacity, outermost first, with no practical limit.
    fn capacities(quadrants: usize, rings: usize) -> Vec<usize> {
        let g = geometry(quadrants, rings);
        (0..rings).map(|ring| g.capacity(ring, 1000, 1.0)).collect()
    }

    /// FR-89 against every figure the PRD and crumb `rg-cbx` quote, for
    /// plain markers of radius 10 on a 260-unit radar:
    /// - `rg-cbx`'s table, 3/4/6/8 quadrants × 4 rings, innermost last;
    /// - OQ-16: 28 per quadrant at eight quadrants;
    /// - FR-14: at four quadrants the innermost band holds 2, the next 10;
    /// - D-50: five rings at four quadrants, `In Use` 7 and `Retire` 1;
    /// - `rg-cbx`: three quadrants' innermost band holds 4, 3 and 2 at
    ///   three, four and five rings.
    #[test]
    fn capacity_matches_the_prd_figures() {
        assert_eq!(capacities(3, 4), [37, 26, 14, 3]);
        assert_eq!(capacities(4, 4), [27, 19, 10, 2]);
        assert_eq!(capacities(6, 4), [18, 12, 6, 2]);
        assert_eq!(capacities(8, 4), [13, 9, 4, 2]);
        assert_eq!(capacities(8, 4).iter().sum::<usize>(), 28);
        assert_eq!(capacities(4, 5), [23, 18, 12, 7, 1]);
        assert_eq!(capacities(3, 3).last(), Some(&4));
        assert_eq!(capacities(3, 5).last(), Some(&2));
    }

    /// FR-43: one and two quadrants have capacities like any other count.
    #[test]
    fn capacity_at_one_and_two_quadrants() {
        assert_eq!(capacities(1, 4), [114, 80, 46, 11]);
        assert_eq!(capacities(2, 4), [56, 39, 22, 5]);
    }

    /// A band of one always fits, and the search stops at `limit`.
    #[test]
    fn capacity_is_at_least_one_and_at_most_the_limit() {
        let g = geometry(8, 6);
        assert!(g.fits(5, 1, 1.0));
        assert_eq!(g.capacity(5, 1, 1.0), 1);
        assert_eq!(g.capacity(0, 3, 1.0), 3);
        assert_eq!(g.capacity(0, 0, 1.0), 0);
    }

    /// FR-89's "largest n" is the first failure's predecessor only because
    /// fitting is monotone: once a band overflows, adding items never makes
    /// it fit again.
    #[test]
    fn fitting_is_monotone_in_n() {
        for quadrants in 1..=8 {
            for rings in 2..=6 {
                let g = geometry(quadrants, rings);
                for ring in 0..rings {
                    let cap = g.capacity(ring, 200, 1.0);
                    assert!(
                        cap < 200,
                        "N={quadrants} rings={rings} ring={ring}: capacity reached the search limit"
                    );
                    for n in cap + 1..=cap + 20 {
                        assert!(
                            !g.fits(ring, n, 1.0),
                            "N={quadrants} rings={rings} ring={ring} n={n}"
                        );
                    }
                }
            }
        }
    }

    /// NFR-4: a distance a hair under one diameter, as another platform's
    /// `sin` might produce, still fits; a visibly short one does not.
    #[test]
    fn capacity_tolerates_rounding_at_the_threshold() {
        let diameter = 2.0 * MARKER_RADIUS;
        assert!(far_enough(diameter - 1e-12));
        assert!(far_enough(diameter));
        assert!(!far_enough(diameter - 1e-3));
    }

    /// The quadrant and ring counts come from the catalogue: FR-8's
    /// distinct quadrants and the declared rings, including empty ones
    /// (FR-19).
    #[test]
    fn geometry_counts_quadrants_and_declared_rings() {
        let cat = crate::model::Catalogue {
            columns: vec![],
            quadrants: vec![],
            rings: vec!["A".into(), "B".into(), "C".into()],
            ring_source: crate::settings::Source::Default,
            items: vec![],
        };
        assert_eq!(Geometry::of(&cat), geometry(0, 3));
        let band = Geometry::of(&cat).band(0);
        assert!(close(band.inner, 520.0 / 3.0) && close(band.outer, 260.0));
    }
}
