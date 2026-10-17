//! Text, its lines and its halo.

use super::{Context, attr, coord, paint, shows};
use crate::escape::xml_text;
use crate::style::TextStyle;
use crate::svg::{Anchor, Point, Tail, TextPaint};
use crate::text::FAMILY;

/// A text node's fields.
pub(super) struct Run<'a> {
    pub(super) origin: Point,
    pub(super) style: TextStyle,
    pub(super) ink: TextPaint,
    pub(super) anchor: Anchor,
    pub(super) lines: &'a [String],
    pub(super) words: &'a str,
    pub(super) leading: f64,
    pub(super) tail: Option<&'a Tail>,
}

/// One line as the content, several as `<tspan>`s at absolute baselines;
/// no lines, nothing.
pub(super) fn text(run: &Run<'_>, context: Context<'_>) -> String {
    debug_assert!(
        run.lines
            .iter()
            .all(|line| line.is_empty() || !line.chars().all(char::is_whitespace)),
        "a line is never only whitespace: wrap normalises it"
    );
    if run.lines.is_empty() {
        return String::new();
    }
    let mut out = String::from("<text");
    coord(&mut out, "x", run.origin.x);
    coord(&mut out, "y", run.origin.y);
    style(&mut out, run.style, run.anchor);
    ink(&mut out, run.ink, context);
    if context.target.interactive() && context.control.is_some() {
        // A click on a number or name reaches its control (FR-37).
        attr(&mut out, "pointer-events", "none");
    }
    out.push('>');
    out.push_str(&content(run));
    if let Some(tail) = run.tail {
        out.push_str(&super::tail::tspan(tail, run.style, context));
    }
    out.push_str("</text>\n");
    out
}

fn style(out: &mut String, style: TextStyle, anchor: Anchor) {
    attr(out, "font-family", FAMILY);
    coord(out, "font-size", style.size);
    attr(out, "font-weight", &style.weight.css().to_string());
    if shows(style.letter_spacing) {
        coord(out, "letter-spacing", style.letter_spacing);
    }
    if anchor == Anchor::Middle {
        attr(out, "text-anchor", "middle");
    }
}

/// A fill, or a halo: an outline only, hidden from assistive technology so
/// the filled copy is read once.
pub(super) fn ink(out: &mut String, ink: TextPaint, context: Context<'_>) {
    match ink {
        TextPaint::Fill(fill) => attr(out, "fill", &paint(fill, context.target)),
        TextPaint::Halo { stroke, width } => {
            attr(out, "fill", "none");
            attr(out, "stroke", &paint(stroke, context.target));
            coord(out, "stroke-width", width);
            attr(out, "stroke-linejoin", "round");
            attr(out, "aria-hidden", "true");
        }
    }
}

fn content(run: &Run<'_>) -> String {
    match run.lines {
        [line] => xml_text(line),
        // No whitespace between tspans: a browser renders it as a space at
        // the end of each chunk, shifting a middle-anchored line left.
        lines => (0_u32..)
            .zip(lines)
            .map(|(i, line)| tspan(run, i, line))
            .collect(),
    }
}

#[expect(
    clippy::suboptimal_flops,
    reason = "NFR-4: plain * and + are bit-identical on every target; mul_add falls back to a libm fma where the CPU lacks one"
)]
fn tspan(run: &Run<'_>, index: u32, line: &str) -> String {
    let mut out = String::from("<tspan");
    coord(&mut out, "x", run.origin.x);
    coord(&mut out, "y", run.origin.y + f64::from(index) * run.leading);
    out.push('>');
    out.push_str(&xml_text(line));
    out.push_str("</tspan>");
    out
}
