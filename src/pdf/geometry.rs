//! Circles, rounded rectangles and SVG circular arcs as cubic Béziers:
//! PDF has lines and cubics but no arcs (FR-110). Every arc is cut into
//! pieces of at most a quarter turn, whose cubic stays within 2.8e-4 of
//! the radius.

use std::f64::consts::{FRAC_PI_2, TAU};

use crate::svg::{PathCommand, Point};

/// One step of a PDF path, in document units (y down).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Step {
    /// Start a sub-path.
    Move(Point),
    /// A straight edge to here.
    Line(Point),
    /// A cubic curve: two control points, then where it ends.
    Cubic {
        /// The first control point.
        c1: Point,
        /// The second control point.
        c2: Point,
        /// Where the curve ends.
        end: Point,
    },
    /// Close the sub-path.
    Close,
}

/// The control-arm length for a quarter turn of a unit circle:
/// 4/3 · tan(π/8).
const QUARTER: f64 = 0.552_284_749_830_793_4;

/// A point at `angle` (radians, y down) on the circle at `centre`.
#[expect(
    clippy::suboptimal_flops,
    reason = "NFR-4: plain * and + are bit-identical on every target; mul_add falls back to a libm fma where the CPU lacks one"
)]
fn on(centre: Point, radius: f64, angle: f64) -> Point {
    Point {
        x: centre.x + radius * angle.cos(),
        y: centre.y + radius * angle.sin(),
    }
}

/// One cubic for the arc from angle `a` to `b` (at most a quarter turn)
/// ending exactly at `end`.
#[expect(
    clippy::suboptimal_flops,
    reason = "NFR-4: plain * and + are bit-identical on every target; mul_add falls back to a libm fma where the CPU lacks one"
)]
fn piece(centre: Point, radius: f64, a: f64, b: f64, end: Point) -> Step {
    let k = 4.0 / 3.0 * ((b - a) / 4.0).tan() * radius;
    let start = on(centre, radius, a);
    Step::Cubic {
        c1: Point {
            x: start.x - k * a.sin(),
            y: start.y + k * a.cos(),
        },
        c2: Point {
            x: end.x + k * b.sin(),
            y: end.y - k * b.cos(),
        },
        end,
    }
}

/// The circle at `centre`: a move to its rightmost point, four quarter
/// cubics clockwise on screen, and a close.
#[must_use]
pub fn circle(centre: Point, radius: f64) -> Vec<Step> {
    let mut steps = vec![Step::Move(on(centre, radius, 0.0))];
    for quarter in 0_u8..4 {
        let a = f64::from(quarter) * FRAC_PI_2;
        let b = a + FRAC_PI_2;
        steps.push(piece(centre, radius, a, b, on(centre, radius, b)));
    }
    steps.push(Step::Close);
    steps
}

/// A rectangle from `origin`, `radius` on every corner as the SVG
/// serialiser writes it: `rx` only, so `ry` is `rx`, and each is clamped
/// to half its own side (SVG 1.1 §9.2), which makes a radius past half
/// the height an elliptical corner. Square corners are plain lines.
#[must_use]
pub fn rect(origin: Point, width: f64, height: f64, radius: f64) -> Vec<Step> {
    let (x0, y0, x1, y1) = (origin.x, origin.y, origin.x + width, origin.y + height);
    let p = |x, y| Point { x, y };
    let rx = radius.min(width / 2.0).max(0.0);
    let ry = radius.min(height / 2.0).max(0.0);
    if crate::num::fmt_coord(rx) == "0" || crate::num::fmt_coord(ry) == "0" {
        return vec![
            Step::Move(p(x0, y0)),
            Step::Line(p(x1, y0)),
            Step::Line(p(x1, y1)),
            Step::Line(p(x0, y1)),
            Step::Close,
        ];
    }
    let (kx, ky) = (QUARTER * rx, QUARTER * ry);
    vec![
        Step::Move(p(x0 + rx, y0)),
        Step::Line(p(x1 - rx, y0)),
        Step::Cubic {
            c1: p(x1 - rx + kx, y0),
            c2: p(x1, y0 + ry - ky),
            end: p(x1, y0 + ry),
        },
        Step::Line(p(x1, y1 - ry)),
        Step::Cubic {
            c1: p(x1, y1 - ry + ky),
            c2: p(x1 - rx + kx, y1),
            end: p(x1 - rx, y1),
        },
        Step::Line(p(x0 + rx, y1)),
        Step::Cubic {
            c1: p(x0 + rx - kx, y1),
            c2: p(x0, y1 - ry + ky),
            end: p(x0, y1 - ry),
        },
        Step::Line(p(x0, y0 + ry)),
        Step::Cubic {
            c1: p(x0, y0 + ry - ky),
            c2: p(x0 + rx - kx, y0),
            end: p(x0 + rx, y0),
        },
        Step::Close,
    ]
}

/// A path's commands as steps: each `Arc` from the current point, by SVG's
/// endpoint-to-centre conversion for a circle (SVG 1.1 F.6.5, F.6.6).
#[must_use]
pub fn path(commands: &[PathCommand]) -> Vec<Step> {
    let mut steps = Vec::new();
    let mut pen = Point::ORIGIN;
    for command in commands {
        match *command {
            PathCommand::MoveTo(p) => {
                steps.push(Step::Move(p));
                pen = p;
            }
            PathCommand::Arc {
                radius,
                large_arc,
                sweep,
                end,
            } => {
                steps.extend(arc(pen, radius, large_arc, sweep, end));
                pen = end;
            }
        }
    }
    steps
}

/// SVG F.6.5 for `rx = ry = radius`, no rotation, in y-down space.
#[expect(
    clippy::suboptimal_flops,
    reason = "NFR-4: plain * and + are bit-identical on every target; mul_add falls back to a libm fma where the CPU lacks one"
)]
fn arc(from: Point, radius: f64, large_arc: bool, sweep: bool, end: Point) -> Vec<Step> {
    if from == end {
        return Vec::new();
    }
    let mut r = radius.abs();
    if r == 0.0 {
        return vec![Step::Line(end)];
    }
    // Half the chord, in a frame centred on its midpoint.
    let (hx, hy) = ((from.x - end.x) / 2.0, (from.y - end.y) / 2.0);
    let d2 = hx * hx + hy * hy;
    // F.6.6: a radius too small for the chord grows to reach it.
    r = r.max(d2.sqrt());
    let sign = if large_arc == sweep { -1.0 } else { 1.0 };
    let coef = sign * ((r * r - d2) / d2).max(0.0).sqrt();
    let centre = Point {
        x: coef * hy + f64::midpoint(from.x, end.x),
        y: -coef * hx + f64::midpoint(from.y, end.y),
    };
    let start = (from.y - centre.y).atan2(from.x - centre.x);
    let mut delta = (end.y - centre.y).atan2(end.x - centre.x) - start;
    if sweep && delta < 0.0 {
        delta += TAU;
    } else if !sweep && delta > 0.0 {
        delta -= TAU;
    }
    // The 1e-9 keeps an exact quarter or half turn (four- and two-sector
    // radars) from tipping into one more piece on a platform whose atan2
    // rounds its last bit up (NFR-4).
    let pieces = (delta.abs() / FRAC_PI_2 - 1e-9).ceil().max(1.0);
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::as_conversions,
        reason = "at most 4, from the ceiling of a quarter-turn count of at most a full turn"
    )]
    let count = pieces as u8;
    let step = delta / pieces;
    (1..=count)
        .map(|i| {
            let a = start + f64::from(i - 1) * step;
            let b = start + f64::from(i) * step;
            let to = if i == count { end } else { on(centre, r, b) };
            piece(centre, r, a, b, to)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The point a cubic from `p0` reaches at `t`.
    #[expect(
        clippy::suboptimal_flops,
        clippy::many_single_char_names,
        reason = "the Bernstein form as written, for a test"
    )]
    fn at(p0: Point, c1: Point, c2: Point, p3: Point, t: f64) -> Point {
        let u = 1.0 - t;
        let (a, b, c, d) = (u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t);
        Point {
            x: a * p0.x + b * c1.x + c * c2.x + d * p3.x,
            y: a * p0.y + b * c1.y + c * c2.y + d * p3.y,
        }
    }

    /// The largest distance from `centre` minus `radius` along `steps`,
    /// sampled 32 times per cubic.
    fn worst(steps: &[Step], centre: Point, radius: f64) -> f64 {
        let mut pen = Point::ORIGIN;
        let mut worst: f64 = 0.0;
        for step in steps {
            match *step {
                Step::Move(p) | Step::Line(p) => pen = p,
                Step::Cubic { c1, c2, end } => {
                    for i in 0..=32 {
                        let p = at(pen, c1, c2, end, f64::from(i) / 32.0);
                        let d = ((p.x - centre.x).hypot(p.y - centre.y) - radius).abs();
                        worst = worst.max(d);
                    }
                    pen = end;
                }
                Step::Close => {}
            }
        }
        worst
    }

    /// A circle is four quarter cubics, closed, within the quarter-cubic
    /// approximation's error of 2.8e-4 of the radius.
    #[test]
    fn a_circle_is_four_quarters() {
        let centre = Point { x: 10.0, y: 20.0 };
        let steps = circle(centre, 100.0);
        assert_eq!(steps.len(), 6, "move, four cubics, close: {steps:?}");
        assert_eq!(
            steps.first(),
            Some(&Step::Move(Point { x: 110.0, y: 20.0 }))
        );
        assert_eq!(steps.last(), Some(&Step::Close));
        assert!(worst(&steps, centre, 100.0) < 100.0 * 2.8e-4);
    }

    /// Square corners are four lines; round ones add a quarter cubic each,
    /// `rx` clamped to half the width and `ry` (which is `rx`) to half the
    /// height, as SVG clamps each (the SVG writes `rx` alone).
    #[test]
    fn rects_square_and_round() {
        let square = rect(Point::ORIGIN, 40.0, 20.0, 0.0);
        assert_eq!(
            square,
            [
                Step::Move(Point::ORIGIN),
                Step::Line(Point { x: 40.0, y: 0.0 }),
                Step::Line(Point { x: 40.0, y: 20.0 }),
                Step::Line(Point { x: 0.0, y: 20.0 }),
                Step::Close,
            ]
        );
        let round = rect(Point::ORIGIN, 40.0, 20.0, 50.0);
        let cubics = round
            .iter()
            .filter(|s| matches!(s, Step::Cubic { .. }))
            .count();
        assert_eq!(cubics, 4, "{round:?}");
        assert!(
            round.contains(&Step::Move(Point { x: 20.0, y: 0.0 })),
            "rx 50 clamps to 20: {round:?}"
        );
        assert!(
            round.contains(&Step::Line(Point { x: 40.0, y: 10.0 })),
            "ry 50 clamps to 10: {round:?}"
        );
    }

    /// SVG's arc flags: from (100, 0) around the origin to (0, 100). In
    /// SVG's y-down space, sweep 1 is clockwise on screen, through
    /// (70.7, 70.7); sweep 0 with the large arc goes the long way round,
    /// through (-100, 0). Both stay on the circle and end exactly at `end`.
    #[test]
    fn arcs_follow_svg_flags() {
        let start = Point { x: 100.0, y: 0.0 };
        let end = Point { x: 0.0, y: 100.0 };
        let short = path(&[
            PathCommand::MoveTo(start),
            PathCommand::Arc {
                radius: 100.0,
                large_arc: false,
                sweep: true,
                end,
            },
        ]);
        let long = path(&[
            PathCommand::MoveTo(start),
            PathCommand::Arc {
                radius: 100.0,
                large_arc: true,
                sweep: false,
                end,
            },
        ]);
        let cubics = |s: &[Step]| s.iter().filter(|s| matches!(s, Step::Cubic { .. })).count();
        assert_eq!(cubics(&short), 1, "90 degrees is one cubic: {short:?}");
        assert_eq!(cubics(&long), 3, "270 degrees is three: {long:?}");
        for steps in [&short, &long] {
            assert!(worst(steps, Point::ORIGIN, 100.0) < 100.0 * 2.8e-4);
            assert!(matches!(steps.last(), Some(Step::Cubic { end: e, .. }) if *e == end));
        }
        // The short arc's midpoint is in the lower right quadrant (y down).
        let Some(Step::Cubic { c1, c2, end: e }) = short.get(1).copied() else {
            panic!("a cubic: {short:?}");
        };
        let mid = at(start, c1, c2, e, 0.5);
        assert!(mid.x > 0.0 && mid.y > 0.0, "{mid:?}");
        // The long arc passes the far side, x near -100.
        let far = long
            .iter()
            .any(|s| matches!(s, Step::Cubic { end: e, .. } if (e.x + 100.0).abs() < 1e-9));
        assert!(far, "{long:?}");
    }

    /// SVG F.6.6: a radius too small for the chord is scaled up until the
    /// arc is a half circle; a zero radius is a straight line; an arc to
    /// its own start draws nothing.
    #[test]
    fn degenerate_arcs() {
        let start = Point { x: 0.0, y: 0.0 };
        let end = Point { x: 10.0, y: 0.0 };
        let small = path(&[
            PathCommand::MoveTo(start),
            PathCommand::Arc {
                radius: 1.0,
                large_arc: false,
                sweep: true,
                end,
            },
        ]);
        assert!(
            worst(&small, Point { x: 5.0, y: 0.0 }, 5.0) < 5.0 * 2.8e-4,
            "{small:?}"
        );
        let zero = path(&[
            PathCommand::MoveTo(start),
            PathCommand::Arc {
                radius: 0.0,
                large_arc: false,
                sweep: true,
                end,
            },
        ]);
        assert_eq!(zero, [Step::Move(start), Step::Line(end)]);
        let none = path(&[
            PathCommand::MoveTo(start),
            PathCommand::Arc {
                radius: 5.0,
                large_arc: false,
                sweep: true,
                end: start,
            },
        ]);
        assert_eq!(none, [Step::Move(start)]);
    }

    /// NFR-17 for PDF: thousandths, half away from zero, then f32.
    #[test]
    fn real_rounds_as_fmt_coord_does() {
        assert_eq!(crate::pdf::real(1.0005), 1.001);
        assert_eq!(crate::pdf::real(-7.654_32), -7.654);
        assert_eq!(crate::pdf::real(1e-13), 0.0);
        assert_eq!(crate::pdf::real(260.0), 260.0);
    }
}
