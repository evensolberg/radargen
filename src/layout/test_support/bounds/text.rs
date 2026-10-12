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
    } = node
    else {
        panic!("expected text, got {node:?}");
    };
    let halo = match ink {
        TextPaint::Fill(_) => 0.0,
        TextPaint::Halo { width, .. } => f64::midpoint(0.0, *width),
    };
    let (ascent, descent) = (TEXT_ASCENT * style.size, TEXT_DESCENT * style.size);
    lines
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
            };
            Rect {
                left,
                top: baseline - ascent,
                right: left + width,
                bottom: baseline + descent,
            }
            .grown(halo)
        })
        .chain(tail_bounds(node, tail.as_ref(), fonts))
        .reduce(Rect::union)
}

/// A text's tail, on its last line's baseline from where it starts.
#[expect(
    clippy::suboptimal_flops,
    reason = "NFR-4: plain * and + are bit-identical on every target; mul_add falls back to a libm fma where the CPU lacks one"
)]
fn tail_bounds(node: &Node, tail: Option<&Tail>, fonts: &Fonts) -> Option<Rect> {
    let (
        Node::Text {
            origin,
            lines,
            leading,
            ..
        },
        Some(tail),
    ) = (node, tail)
    else {
        return None;
    };
    let left = node.tail_x(fonts)?;
    #[expect(
        clippy::cast_precision_loss,
        clippy::as_conversions,
        reason = "a line count is far below 2^53, so the conversion is exact"
    )]
    let baseline = origin.y + lines.len().saturating_sub(1) as f64 * leading;
    Some(Rect {
        left,
        top: baseline - TEXT_ASCENT * tail.style.size,
        right: left + tail.style.measure(&tail.text, fonts),
        bottom: baseline + TEXT_DESCENT * tail.style.size,
    })
}
