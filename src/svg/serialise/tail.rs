//! A text's tail: the closing tspan that continues its last line (FR-74,
//! D-78).

use super::{Context, attr, coord, paint, shows};
use crate::escape::xml_text;
use crate::style::TextStyle;
use crate::svg::Tail;

/// A tail as the text's closing tspan: `dx` from the last line's end, and
/// its own size, weight and paint. A tspan inherits its text's
/// letter-spacing, so the tail states its own whenever the two differ.
pub(super) fn tspan(tail: &Tail, parent: TextStyle, context: Context<'_>) -> String {
    debug_assert!(
        !tail.text.chars().all(char::is_whitespace),
        "a tail is never only whitespace"
    );
    let mut out = String::from("<tspan");
    coord(&mut out, "dx", tail.gap);
    coord(&mut out, "font-size", tail.style.size);
    attr(
        &mut out,
        "font-weight",
        &tail.style.weight.css().to_string(),
    );
    if shows(tail.style.letter_spacing - parent.letter_spacing) {
        coord(&mut out, "letter-spacing", tail.style.letter_spacing);
    }
    attr(&mut out, "fill", &paint(tail.ink, context.target));
    out.push('>');
    out.push_str(&xml_text(&tail.text));
    out.push_str("</tspan>");
    out
}
