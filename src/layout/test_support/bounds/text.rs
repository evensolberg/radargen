//! Where a text node paints: its lines, and its tail after the last line.

use super::Rect;
use crate::svg::{Anchor, Node, Tail, TextPaint};
use crate::text::{Fonts, TEXT_ASCENT, TEXT_DESCENT};

/// Each line spans its measured width, from the ascent above its baseline to
/// the descent below it; a halo widens that on all four sides.
pub(super) fn text_bounds(node: &Node, fonts: &Fonts) -> Option<Rect> {
    let Node::Text {
        origin,
        style,
        ink,
        anchor,
        lines,
        leading,
        tail,
        ..
    } = node
    else {
        panic!("expected text, got {node:?}");
    };
    let halo = match ink {
        TextPaint::Fill(_) => 0.0,
        TextPaint::Halo { width, .. } => f64::midpoint(0.0, *width),
    };
    let (ascent, descent) = (TEXT_ASCENT * style.size, TEXT_DESCENT * style.size);
    let rects: Vec<Rect> = lines
        .iter()
        .scan(origin.y, |baseline, line| {
            let at = *baseline;
            *baseline += leading;
            Some((at, line))
        })
        .map(|(baseline, line)| {
            let width = style.measure(line, fonts);
            let left = match anchor {
                Anchor::Start => origin.x,
                Anchor::Middle => origin.x - f64::midpoint(0.0, width),
                // The ink ends at the origin; the layout width runs on past
                // it by the room left for text spacing (D-78).
                Anchor::End => origin.x - style.advance(line, fonts),
            };
            Rect {
                left,
                top: baseline - ascent,
                right: left + width,
                bottom: baseline + descent,
            }
            .grown(halo)
        })
        .collect();
    let tail = tail
        .as_ref()
        .zip(rects.last())
        .map(|(tail, last)| tail_bounds(tail, *last, last.bottom - descent, fonts));
    rects.into_iter().chain(tail).reduce(Rect::union)
}

/// A text's tail after its last line's box `last`, on that line's
/// `baseline`. It starts at the line's layout end, where a reader's text
/// spacing puts it (D-78), as `Row::right_edge` holds it; the drawn start,
/// `Node::tail_x`, is further left. Only filled text has a tail, so the box
/// carries no halo.
#[expect(
    clippy::suboptimal_flops,
    reason = "NFR-4: plain * and + are bit-identical on every target; mul_add falls back to a libm fma where the CPU lacks one"
)]
fn tail_bounds(tail: &Tail, last: Rect, baseline: f64, fonts: &Fonts) -> Rect {
    Rect {
        left: last.right + tail.gap,
        top: baseline - TEXT_ASCENT * tail.style.size,
        right: last.right + tail.layout_width(fonts),
        bottom: baseline + TEXT_DESCENT * tail.style.size,
    }
}
