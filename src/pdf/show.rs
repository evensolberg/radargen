//! A text node as PDF text (FR-110): each line in its face's subset, each
//! glyph where the shaper places it (FR-84), so the PDF's layout is the
//! PNG's and the SVG's; the tail after the last line at `Node::tail_x`
//! (D-78). A halo is drawn as its glyphs' stroked outlines, not as text,
//! so a label extracts, copies and is searched once.

use pdf_writer::Content;
use pdf_writer::Str;
use pdf_writer::types::TextRenderingMode;

use super::content::{Canvas, colour};
use super::font::Faces;
use super::{EM, real};
use crate::style::TextStyle;
use crate::svg::{Anchor, Node, Paint, Point, TextPaint};
use crate::text::{FontError, Placed};

/// How much of its advance a middle-anchored line starts before its
/// origin, as the SVG outliner and resvg place it.
const CENTRED: f64 = 0.5;
/// The smallest TJ adjustment written, in thousandths of an em. Anything
/// smaller is float residue between the widths array's rounded advance
/// and the shaper's, about 1e-5 em per glyph: a hundredth of a pixel
/// across a long line, and written it would only bloat the stream.
const RESIDUE: f32 = 0.01;
/// The text matrix's y scale: it flips glyph space, y up, back against
/// the page's flipped transform, so the glyphs stand upright.
const UPRIGHT: f32 = -1.0;

/// `node`'s lines, then its tail; or, for a halo, its outlines. Anything
/// but a text writes nothing.
///
/// # Errors
///
/// [`FontError`] when a halo's glyph cannot be outlined (NFR-15).
pub(super) fn text(out: &mut Content, node: &Node, canvas: &Canvas<'_>) -> Result<(), FontError> {
    let Node::Text {
        style, ink, tail, ..
    } = node
    else {
        return Ok(());
    };
    let starts = starts(node, canvas);
    // As the SVG outliner: a text with no lines draws nothing, its tail
    // included.
    let Some(&(_, last)) = starts.last() else {
        return Ok(());
    };
    match *ink {
        TextPaint::Halo { stroke, width } => {
            super::halo::halo(out, &starts, *style, (stroke, width), canvas)
        }
        TextPaint::Fill(fill) => {
            // Its own graphics state, as a halo's is.
            out.save_state();
            if !fill_with(out, fill, canvas) {
                out.restore_state();
                return Ok(());
            }
            for &(line, at) in &starts {
                run(out, line, *style, at, canvas);
            }
            if let (Some(tail), Some(x)) = (tail, node.tail_x(canvas.fonts))
                && fill_with(out, tail.ink, canvas)
            {
                let at = Point { x, y: last.y };
                run(out, &tail.text, tail.style, at, canvas);
            }
            out.restore_state();
            Ok(())
        }
    }
}

/// Each line of a text node with where it starts: the origin, one leading
/// lower per line, a middle-anchored line moved back half its advance.
#[expect(
    clippy::suboptimal_flops,
    reason = "NFR-4: plain * and + are bit-identical on every target; mul_add falls back to a libm fma where the CPU lacks one"
)]
fn starts<'a>(node: &'a Node, canvas: &Canvas<'_>) -> Vec<(&'a str, Point)> {
    let Node::Text {
        origin,
        style,
        anchor,
        lines,
        leading,
        ..
    } = node
    else {
        return Vec::new();
    };
    (0_u32..)
        .zip(lines)
        .map(|(i, line)| {
            let x = match anchor {
                Anchor::Start => origin.x,
                Anchor::Middle => origin.x - style.advance(line, canvas.fonts) * CENTRED,
            };
            let y = origin.y + f64::from(i) * leading;
            (line.as_str(), Point { x, y })
        })
        .collect()
}

/// Sets `fill` as the fill colour, returning whether anything is painted.
fn fill_with(out: &mut Content, fill: Paint, canvas: &Canvas<'_>) -> bool {
    colour(fill, canvas.palette).is_some_and(|[r, g, b]| {
        out.set_fill_rgb(r, g, b);
        true
    })
}

/// One line of `text` from `at`, its glyphs split wherever the vertical
/// offset changes, each piece one text matrix and one TJ.
fn run(out: &mut Content, text: &str, style: TextStyle, at: Point, canvas: &Canvas<'_>) {
    let placed = canvas
        .fonts
        .face(style.weight)
        .place(text, style.size, style.letter_spacing);
    if placed.is_empty() {
        return;
    }
    out.begin_text();
    out.set_font(Faces::resource(style.weight), real(style.size));
    out.set_text_rendering_mode(TextRenderingMode::Fill);
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
        if adjust.abs() >= RESIDUE {
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
            page: crate::pdf::Sheet {
                unit: 1.0,
                scale: 1.0,
                width: 200.0,
                height: 100.0,
            },
        };
        let mut out = Content::new();
        text(&mut out, node, &canvas).expect("outlines");
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

    /// A glyph the shaper raises starts a new text matrix in the same
    /// BT … ET, at the baseline plus its `placed.y` (the string is
    /// `text.rs`'s `place_raises_a_glyph_the_shaper_raises`).
    #[test]
    fn a_raised_glyph_splits_the_line_into_two_text_matrices() {
        let text = "A\u{300}\u{301}";
        let placed = fonts().face(Weight::Regular).place(text, 10.0, 0.0);
        let raised = placed
            .iter()
            .find(|p| p.y.to_bits() != placed.first().expect("glyphs").y.to_bits())
            .expect("a glyph the shaper raises");
        let (s, _) = written(&label(&[text], fill(), Anchor::Start, 0.0, None));
        let ops = lopdf::content::Content::decode(s.as_bytes())
            .expect("decodes")
            .operations;
        assert_eq!(
            ops.iter().filter(|op| op.operator == "BT").count(),
            1,
            "{s}"
        );
        assert_eq!(
            ops.iter().filter(|op| op.operator == "ET").count(),
            1,
            "{s}"
        );
        let ys: Vec<f64> = ops
            .iter()
            .filter(|op| op.operator == "Tm")
            .map(|op| {
                f64::from(
                    op.operands
                        .last()
                        .and_then(|y| y.as_float().ok())
                        .expect("a y"),
                )
            })
            .collect();
        let [base, up] = ys[..] else {
            panic!("two Tm lines: {s}");
        };
        assert!((base - 20.0).abs() < 0.002, "baseline: {s}");
        assert!((up - (20.0 + raised.y)).abs() < 0.002, "raised: {s}");
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

    /// A halo is stroked glyph outlines, not text (so the label extracts
    /// once): no BT or Tr, paths stroked in the halo's colour, width and
    /// round join.
    #[test]
    fn a_halo_strokes_outlines() {
        let stroke = Paint::Role(Role::Neutral(Neutral::Surface1));
        let halo = TextPaint::Halo { stroke, width: 3.0 };
        let (s, _) = written(&label(&["Adopt"], halo, Anchor::Start, 0.0, None));
        let ops = lopdf::content::Content::decode(s.as_bytes())
            .expect("decodes")
            .operations;
        let count = |name: &str| ops.iter().filter(|op| op.operator == name).count();
        assert_eq!(count("BT") + count("Tr") + count("TJ"), 0, "not text: {s}");
        assert_eq!(count("S"), 1, "one stroke for the whole label: {s}");
        assert!(count("m") >= 5, "a contour per glyph at least: {s}");
        assert!(count("c") > 0, "curves: {s}");
        let palette = Palette::built_in(Theme::Light);
        let [r, g, b] = super::colour(stroke, &palette).expect("painted");
        assert!(
            s.contains(&format!("{r} {g} {b} RG\n")) && s.contains("3 w\n") && s.contains("1 j\n"),
            "{s}"
        );
        // Review Focus 6: the join is the halo's alone, so a stroke drawn
        // after it keeps SVG's miter join.
        assert!(
            s.starts_with("q\n") && s.ends_with('Q'),
            "a halo is its own graphics state: {s}"
        );
    }

    /// The halo's outline of a glyph sits where the glyph does: the first
    /// glyph's first point is the outline's, scaled, flipped and moved to
    /// the line's start.
    #[test]
    #[expect(
        clippy::suboptimal_flops,
        reason = "NFR-4: the expected point is computed as `contours` computes it, with plain * and +, never mul_add"
    )]
    fn a_halo_outline_sits_on_its_glyph() {
        let halo = TextPaint::Halo {
            stroke: Paint::Role(Role::Neutral(Neutral::Surface1)),
            width: 3.0,
        };
        let (s, _) = written(&label(&["Adopt"], halo, Anchor::Start, 0.0, None));
        let face = fonts().face(Weight::Regular);
        let first = face.place("Adopt", 10.0, 0.0);
        let first = first.first().expect("a glyph");
        let Some(crate::text::Segment::MoveTo(x, y)) = face
            .outline(first.glyph)
            .expect("outlines")
            .first()
            .copied()
        else {
            panic!("an outline starts with a move");
        };
        let scale = 10.0 / face.units_per_em();
        let want = format!(
            "{} {} m\n",
            real(50.0 + first.x + f64::from(x) * scale),
            real(20.0 + first.y - f64::from(y) * scale)
        );
        assert!(s.contains(&want), "{want} in {s}");
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
