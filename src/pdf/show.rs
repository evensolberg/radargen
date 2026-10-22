//! A text node as PDF text (FR-110): each line in its face's subset, each
//! glyph where the shaper places it (FR-84), so the PDF's layout is the
//! PNG's and the SVG's; a halo as the stroked outline only; the tail after
//! the last line at `Node::tail_x` (D-78).

use pdf_writer::Content;
use pdf_writer::Str;
use pdf_writer::types::{LineJoinStyle, TextRenderingMode};

use super::content::{Canvas, colour};
use super::font::Faces;
use super::real;
use crate::style::TextStyle;
use crate::svg::{Anchor, Node, Point, TextPaint};
use crate::text::Placed;

/// How much of its advance a middle-anchored line starts before its
/// origin, as the SVG outliner and resvg place it.
const CENTRED: f64 = 0.5;
/// Glyph space units per em in a TJ adjustment.
const EM: f64 = 1000.0;
/// The text matrix's y scale: it flips glyph space, y up, back against
/// the page's flipped transform, so the glyphs stand upright.
const UPRIGHT: f32 = -1.0;

/// `node`'s lines, then its tail. Anything but a text writes nothing.
#[expect(
    clippy::suboptimal_flops,
    reason = "NFR-4: plain * and + are bit-identical on every target; mul_add falls back to a libm fma where the CPU lacks one"
)]
pub(super) fn text(out: &mut Content, node: &Node, canvas: &Canvas<'_>) {
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
        return;
    };
    // As the SVG outliner: a text with no lines draws nothing, its tail
    // included.
    if lines.is_empty() {
        return;
    }
    // Its own graphics state: a halo's width and round join must not
    // reach the strokes drawn after it (the legend's square markers).
    out.save_state();
    let Some(mode) = paint(out, *ink, canvas) else {
        out.restore_state();
        return;
    };
    let mut baseline = origin.y;
    for (i, line) in (0_u32..).zip(lines) {
        baseline = origin.y + f64::from(i) * leading;
        let x = match anchor {
            Anchor::Start => origin.x,
            Anchor::Middle => origin.x - style.advance(line, canvas.fonts) * CENTRED,
        };
        run(out, line, *style, Point { x, y: baseline }, mode, canvas);
    }
    if let (Some(tail), Some(x)) = (tail, node.tail_x(canvas.fonts)) {
        debug_assert!(
            matches!(ink, TextPaint::Fill(_)),
            "only a filled text carries a tail (D-78)"
        );
        if paint(out, TextPaint::Fill(tail.ink), canvas).is_some() {
            run(
                out,
                &tail.text,
                tail.style,
                Point { x, y: baseline },
                TextRenderingMode::Fill,
                canvas,
            );
        }
    }
    out.restore_state();
}

/// Sets `ink`'s colour (and a halo's width and join), returning the
/// render mode, or `None` when nothing is painted.
fn paint(out: &mut Content, ink: TextPaint, canvas: &Canvas<'_>) -> Option<TextRenderingMode> {
    match ink {
        TextPaint::Fill(fill) => {
            let [r, g, b] = colour(fill, canvas.palette)?;
            out.set_fill_rgb(r, g, b);
            Some(TextRenderingMode::Fill)
        }
        TextPaint::Halo { stroke, width } => {
            let [r, g, b] = colour(stroke, canvas.palette)?;
            out.set_stroke_rgb(r, g, b)
                .set_line_width(real(width))
                .set_line_join(LineJoinStyle::RoundJoin);
            Some(TextRenderingMode::Stroke)
        }
    }
}

/// One line of `text` from `at`, its glyphs split wherever the vertical
/// offset changes, each piece one text matrix and one TJ.
fn run(
    out: &mut Content,
    text: &str,
    style: TextStyle,
    at: Point,
    mode: TextRenderingMode,
    canvas: &Canvas<'_>,
) {
    let placed = canvas
        .fonts
        .face(style.weight)
        .place(text, style.size, style.letter_spacing);
    if placed.is_empty() {
        return;
    }
    out.begin_text();
    out.set_font(Faces::resource(style.weight), real(style.size));
    out.set_text_rendering_mode(mode);
    // Exact equality is meant: one shaper offset, not two near ones
    // (`to_bits`, so clippy's float_cmp has nothing to flag).
    for piece in placed.chunk_by(|a, b| a.y.to_bits() == b.y.to_bits()) {
        let Some(first) = piece.first() else { continue };
        out.set_text_matrix([
            1.0,
            0.0,
            0.0,
            UPRIGHT,
            real(at.x + first.x),
            real(at.y + first.y),
        ]);
        show(out, piece, style, canvas);
    }
    out.end_text();
}

/// One TJ for `piece`: runs of CIDs, with an adjustment wherever the
/// widths array's advance does not reach the next glyph's x.
fn show(out: &mut Content, piece: &[Placed], style: TextStyle, canvas: &Canvas<'_>) {
    let mut shown = out.show_positioned();
    let mut items = shown.items();
    let mut bytes: Vec<u8> = Vec::new();
    for (i, glyph) in piece.iter().enumerate() {
        let cid = canvas.faces.cid(style.weight, glyph.glyph);
        bytes.extend(cid.to_be_bytes());
        let Some(next) = piece.get(i + 1) else { break };
        let advance = f64::from(canvas.faces.width(style.weight, cid)) * style.size / EM;
        let adjust = real((glyph.x + advance - next.x) * EM / style.size);
        if adjust != 0.0 {
            items.show(Str(&bytes));
            bytes.clear();
            items.adjust(adjust);
        }
    }
    if !bytes.is_empty() {
        items.show(Str(&bytes));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::block::fonts;
    use crate::palette::{Neutral, Palette, Role, Theme};
    use crate::pdf::font::Faces;
    use crate::style::TextStyle;
    use crate::svg::{Anchor, Document, Paint, Point, Tail, TextPaint};
    use crate::text::Weight;

    fn label(
        lines: &[&str],
        ink: TextPaint,
        anchor: Anchor,
        letter_spacing: f64,
        tail: Option<Tail>,
    ) -> Node {
        Node::Text {
            origin: Point { x: 50.0, y: 20.0 },
            style: TextStyle {
                weight: Weight::Regular,
                size: 10.0,
                letter_spacing,
            },
            ink,
            anchor,
            lines: lines.iter().map(|&l| l.to_owned()).collect(),
            words: lines.join(" "),
            leading: 12.0,
            tail,
        }
    }

    fn written(node: &Node) -> (String, Faces) {
        let document = Document {
            width: 200.0,
            height: 100.0,
            root: node.clone(),
        };
        let faces = Faces::of([&document.root], fonts()).expect("subsets");
        let palette = Palette::built_in(Theme::Light);
        let canvas = crate::pdf::content::Canvas {
            document: &document,
            palette: &palette,
            fonts: fonts(),
            faces: &faces,
            scale: 1.0,
        };
        let mut out = Content::new();
        text(&mut out, node, &canvas);
        (
            String::from_utf8(out.finish().into_vec()).expect("ASCII operators; strings are hex"),
            faces,
        )
    }

    fn fill() -> TextPaint {
        TextPaint::Fill(Paint::Role(Role::Neutral(Neutral::TextPrimary)))
    }

    /// Each line is one BT … ET at its baseline, upright under the page's
    /// flip, in the face's resource at the style's size, filled.
    #[test]
    fn lines_sit_on_their_baselines() {
        let (s, _) = written(&label(
            &["Adopt", "Trial"],
            fill(),
            Anchor::Start,
            0.0,
            None,
        ));
        assert_eq!(s.matches("BT\n").count(), 2, "{s}");
        assert!(s.contains("/F0 10 Tf\n"), "{s}");
        assert!(s.contains("0 Tr\n"), "fill mode: {s}");
        assert!(s.contains("1 0 0 -1 50 20 Tm\n"), "first baseline: {s}");
        assert!(
            s.contains("1 0 0 -1 50 32 Tm\n"),
            "second, one leading below: {s}"
        );
        assert!(s.contains(" TJ\n"), "{s}");
    }

    /// A middle-anchored line starts half its advance left of the origin.
    #[test]
    #[expect(
        clippy::suboptimal_flops,
        reason = "NFR-4: the expected x is computed as `text` computes it, with plain * and -, never mul_add"
    )]
    fn a_middle_line_is_centred() {
        let node = label(&["Adopt"], fill(), Anchor::Middle, 0.0, None);
        let (s, _) = written(&node);
        let style = TextStyle {
            weight: Weight::Regular,
            size: 10.0,
            letter_spacing: 0.0,
        };
        let x = real(50.0 - style.advance("Adopt", fonts()) * 0.5);
        assert!(s.contains(&format!("1 0 0 -1 {x} 20 Tm\n")), "{s}");
    }

    /// A halo is the outline only: stroke mode, its width, a round join.
    #[test]
    fn a_halo_strokes() {
        let halo = TextPaint::Halo {
            stroke: Paint::Role(Role::Neutral(Neutral::Surface1)),
            width: 3.0,
        };
        let (s, _) = written(&label(&["Adopt"], halo, Anchor::Start, 0.0, None));
        assert!(
            s.contains("1 Tr\n") && s.contains("3 w\n") && s.contains("1 j\n"),
            "{s}"
        );
        // Review Focus 6: the join is the halo's alone, so a stroke drawn
        // after it keeps SVG's miter join.
        assert!(
            s.starts_with("q\n") && s.ends_with('Q'),
            "a text is its own graphics state: {s}"
        );
    }

    /// TJ adjustments put each glyph where `place()` puts it: replaying the
    /// widths array's advances and the adjustments reaches every x.
    #[test]
    fn glyphs_land_where_the_shaper_places_them() {
        let node = label(&["AVATAR Office"], fill(), Anchor::Start, 1.5, None);
        let (s, faces) = written(&node);
        let face = fonts().face(Weight::Regular);
        let placed = face.place("AVATAR Office", 10.0, 1.5);
        // Decoded by lopdf, not by hand: pdf-writer writes a CID string as
        // a literal `( … )` with octal escapes whenever every byte is below
        // 0x80, and as hex otherwise.
        let ops = lopdf::content::Content::decode(s.as_bytes())
            .expect("decodes")
            .operations;
        let tj = ops.iter().find(|op| op.operator == "TJ").expect("a TJ");
        let items = tj
            .operands
            .first()
            .and_then(|a| a.as_array().ok())
            .expect("a TJ array");
        let mut pen = 0.0_f64;
        let mut seen = Vec::new();
        for item in items {
            match item {
                lopdf::Object::String(bytes, _) => {
                    let (pairs, rest) = bytes.as_chunks::<2>();
                    assert!(rest.is_empty(), "a CID string is whole two-byte CIDs: {s}");
                    for pair in pairs {
                        let cid = u16::from_be_bytes(*pair);
                        seen.push((cid, pen));
                        pen += f64::from(faces.width(Weight::Regular, cid)) * 10.0 / 1000.0;
                    }
                }
                number => {
                    pen -= f64::from(number.as_float().expect("an adjustment")) * 10.0 / 1000.0;
                }
            }
        }
        assert_eq!(seen.len(), placed.len(), "{s}");
        for ((cid, x), want) in seen.iter().zip(&placed) {
            assert_eq!(*cid, faces.cid(Weight::Regular, want.glyph));
            assert!(
                (x - want.x).abs() < 0.002,
                "glyph at {x}, placed at {}: {s}",
                want.x
            );
        }
    }

    /// A tail is drawn after the last line, at `tail_x`, in its own style
    /// and ink (D-78).
    #[test]
    fn the_tail_follows_the_last_line() {
        let tail = Tail {
            gap: 4.0,
            style: TextStyle {
                weight: Weight::SemiBold,
                size: 8.0,
                letter_spacing: 0.0,
            },
            ink: Paint::Role(Role::Neutral(Neutral::TextMuted)),
            text: "\u{a0}new".to_owned(),
        };
        let node = label(&["Adopt", "Trial"], fill(), Anchor::Start, 0.0, Some(tail));
        let (s, _) = written(&node);
        let x = real(node.tail_x(fonts()).expect("a tail"));
        assert!(s.contains("/F1 8 Tf\n"), "{s}");
        assert!(
            s.contains(&format!("1 0 0 -1 {x} 32 Tm\n")),
            "on the last baseline: {s}"
        );
    }
}
