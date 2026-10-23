//! A halo (FR-110) as its glyphs' stroked outlines rather than text, so
//! only the filled copy over it extracts, copies and is searched: each
//! glyph's contours from the face the SVG outliner reads (D-88), placed
//! where `place()` puts the glyph.

use pdf_writer::Content;
use pdf_writer::types::LineJoinStyle;

use super::content::{Canvas, colour, trace};
use super::geometry::Step;
use super::real;
use crate::style::TextStyle;
use crate::svg::{Paint, Point};
use crate::text::{FontError, Segment};

/// How far along a quadratic's arm a cubic's control point sits: degree
/// elevation puts each at two thirds of the way to the quadratic's one.
const ELEVATE: f64 = 2.0 / 3.0;

/// A halo: every glyph of every line as its outline, one stroked path in
/// the halo's colour, width and round join, in its own graphics state so
/// the join does not reach the strokes drawn after it (the legend's square
/// markers). Nothing when its colour is none or no glyph inks.
pub(super) fn halo(
    out: &mut Content,
    starts: &[(&str, Point)],
    style: TextStyle,
    (stroke, width): (Paint, f64),
    canvas: &Canvas<'_>,
) -> Result<(), FontError> {
    let Some([r, g, b]) = colour(stroke, canvas.palette) else {
        return Ok(());
    };
    let mut steps = Vec::new();
    for &(line, at) in starts {
        steps.extend(outlines(line, style, at, canvas)?);
    }
    if steps.is_empty() {
        return Ok(());
    }
    out.save_state();
    out.set_stroke_rgb(r, g, b)
        .set_line_width(real(width))
        .set_line_join(LineJoinStyle::RoundJoin);
    trace(out, &steps);
    out.stroke();
    out.restore_state();
    Ok(())
}

/// One line's glyph outlines from `at`, each glyph where `place()` puts
/// it, as the SVG outliner places its `<use>`s (D-88).
fn outlines(
    text: &str,
    style: TextStyle,
    at: Point,
    canvas: &Canvas<'_>,
) -> Result<Vec<Step>, FontError> {
    let face = canvas.fonts.face(style.weight);
    let scale = style.size / face.units_per_em();
    let mut steps = Vec::new();
    for placed in face.place(text, style.size, style.letter_spacing) {
        let origin = Point {
            x: at.x + placed.x,
            y: at.y + placed.y,
        };
        steps.extend(contours(&face.outline(placed.glyph)?, origin, scale));
    }
    Ok(steps)
}

/// A glyph's contours, in font units and y up, as steps in document units
/// and y down: scaled by `scale`, flipped upright and moved to `origin`.
#[expect(
    clippy::suboptimal_flops,
    reason = "NFR-4: plain * and + are bit-identical on every target; mul_add falls back to a libm fma where the CPU lacks one"
)]
pub(super) fn contours(segments: &[Segment], origin: Point, scale: f64) -> Vec<Step> {
    let at = |x: f32, y: f32| Point {
        x: origin.x + f64::from(x) * scale,
        y: origin.y - f64::from(y) * scale,
    };
    let mut pen = origin;
    segments
        .iter()
        .map(|&segment| {
            let step = match segment {
                Segment::MoveTo(x, y) => Step::Move(at(x, y)),
                Segment::LineTo(x, y) => Step::Line(at(x, y)),
                Segment::QuadTo(cx, cy, x, y) => elevated(pen, at(cx, cy), at(x, y)),
                Segment::CurveTo(c1x, c1y, c2x, c2y, x, y) => Step::Cubic {
                    c1: at(c1x, c1y),
                    c2: at(c2x, c2y),
                    end: at(x, y),
                },
                Segment::Close => Step::Close,
            };
            if let Step::Move(p) | Step::Line(p) | Step::Cubic { end: p, .. } = step {
                pen = p;
            }
            step
        })
        .collect()
}

/// The quadratic from `from` through `control` to `end` as the cubic that
/// draws it: PDF has no quadratic curve.
fn elevated(from: Point, control: Point, end: Point) -> Step {
    Step::Cubic {
        c1: toward(from, control),
        c2: toward(end, control),
        end,
    }
}

/// [`ELEVATE`] of the way from `from` to `to`.
#[expect(
    clippy::suboptimal_flops,
    reason = "NFR-4: plain * and + are bit-identical on every target; mul_add falls back to a libm fma where the CPU lacks one"
)]
fn toward(from: Point, to: Point) -> Point {
    Point {
        x: from.x + (to.x - from.x) * ELEVATE,
        y: from.y + (to.y - from.y) * ELEVATE,
    }
}
