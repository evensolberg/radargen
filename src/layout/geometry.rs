//! The radar's geometry (architecture §3): FR-77's dividers, FR-32's bands,
//! FR-33's marker placement and FR-89's capacity, as pure functions.
//!
//! Coordinates are relative to the radar's centre in SVG's frame: 0° is
//! 3 o'clock, angles grow clockwise on screen, and y grows downward. The
//! formulas are the prototype's, ratified rather than re-derived (D-36).

use crate::style::BAND_OUTER_RADIUS;

/// FR-30: a full turn, which N equal sectors divide.
const FULL_TURN: f64 = 360.0;
/// FR-77: divider 0 sits at 12 o'clock.
const FIRST_DIVIDER: f64 = -90.0;
/// FR-33: markers keep this proportion of the sector clear of each divider.
const INSET: f64 = 0.08;
/// FR-33: the proportion of the sector markers spread across, `1 − 2 × INSET`.
const SPREAD: f64 = 0.84;
/// FR-33: the inner and outer tracks, as proportions of the band's width;
/// even-indexed items take the inner one.
const INNER_TRACK: f64 = 0.38;
const OUTER_TRACK: f64 = 0.68;

/// A point relative to the radar's centre, in SVG user units.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

/// One ring's band: the radii it spans (FR-32).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Band {
    pub inner: f64,
    pub outer: f64,
}

/// The two counts every formula needs: quadrants (FR-30) and declared rings
/// (FR-32).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Geometry {
    pub quadrants: usize,
    pub rings: usize,
}

impl Geometry {
    /// FR-30: the degrees each quadrant's sector spans.
    #[must_use]
    pub fn sector(self) -> f64 {
        FULL_TURN / real(self.quadrants)
    }

    /// FR-77: the angle of the divider that opens the sector at FR-10
    /// `position`, in degrees. Divider 0 is at 12 o'clock and the rest
    /// follow clockwise, so FR-31 holds at every count.
    #[must_use]
    #[expect(
        clippy::suboptimal_flops,
        reason = "NFR-4: plain * and + are bit-identical on every target; mul_add falls back to a libm fma where the CPU lacks one"
    )]
    pub fn divider(self, position: usize) -> f64 {
        FIRST_DIVIDER + real(position) * self.sector()
    }

    /// FR-32: declared ring `ring`'s band. Widths are equal and depend on
    /// the ring count alone (D-47); the first declared ring is outermost
    /// (D-60).
    ///
    /// # Panics
    ///
    /// In debug builds, if `ring` exceeds `rings`, which overflows the
    /// subtraction. Callers pass a declared ring index, which is below it.
    #[must_use]
    pub fn band(self, ring: usize) -> Band {
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

/// A count as `f64`.
#[expect(
    clippy::cast_precision_loss,
    reason = "f64 holds every integer up to 2^53 exactly, and a count is far below that"
)]
const fn real(n: usize) -> f64 {
    n as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    fn geometry(quadrants: usize, rings: usize) -> Geometry {
        Geometry { quadrants, rings }
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
    fn markers_follow_fr33() {
        let g = geometry(4, 4);
        // Position 1's sector opens at 0°; one item sits at its centre.
        assert!(close(g.angle(1, 0, 1), 45.0));
        assert!(close(g.radius(0, 0), 195.0 + 65.0 * 0.38));
        assert!(close(g.radius(0, 1), 195.0 + 65.0 * 0.68));
        assert!(close(g.radius(0, 2), g.radius(0, 0)));
        let p = g.marker(1, 0, 0, 1);
        let r = 195.0 + 65.0 * 0.38;
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
    fn markers_stay_inset_from_both_dividers() {
        for quadrants in 1..=8 {
            let g = geometry(quadrants, 4);
            for n in 1..=40 {
                for j in 0..n {
                    let a = g.angle(0, j, n);
                    assert!(
                        a > g.divider(0) + 0.08 * g.sector() - 1e-9,
                        "N={quadrants} n={n} j={j}"
                    );
                    assert!(
                        a < g.divider(0) + 0.92 * g.sector() + 1e-9,
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
    fn a_single_quadrant_leaves_a_gap_at_twelve() {
        let g = geometry(1, 4);
        assert!(close(g.angle(0, 0, 1), 90.0));
        let first = g.angle(0, 0, 3);
        let last = g.angle(0, 2, 3);
        assert!(close(first, -90.0 + 360.0 * (0.08 + 0.25 * 0.84)));
        assert!(close(last, -90.0 + 360.0 * (0.08 + 0.75 * 0.84)));
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
}
