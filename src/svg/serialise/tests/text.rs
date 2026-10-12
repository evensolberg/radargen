//! Text, its lines, its style and its halo.

use super::*;
use crate::style::{QUADRANT_LABEL, TITLE, TextStyle};

fn fill() -> TextPaint {
    TextPaint::Fill(MUTED)
}

#[test]
fn one_line_is_the_content() {
    let palette = light();
    assert_eq!(
        one(
            &text(&["Key"], Anchor::Start, fill()),
            Target::Static(&palette)
        ),
        "<text x=\"10\" y=\"20\" font-family=\"Inter\" font-size=\"11\" font-weight=\"400\" fill=\"#585F6B\">Key</text>\n"
    );
}

/// Absolute baselines, so no rounding accumulates down the lines; and no
/// whitespace between the tspans, which a browser would render as a space
/// at the end of each chunk, shifting a middle-anchored line left.
#[test]
fn several_lines_are_tspans() {
    let palette = light();
    assert_eq!(
        one(
            &text(&["one", "two"], Anchor::Middle, fill()),
            Target::Static(&palette)
        ),
        "<text x=\"10\" y=\"20\" font-family=\"Inter\" font-size=\"11\" font-weight=\"400\" text-anchor=\"middle\" fill=\"#585F6B\"><tspan x=\"10\" y=\"20\">one</tspan><tspan x=\"10\" y=\"35\">two</tspan></text>\n"
    );
}

fn styled(style: TextStyle) -> Node {
    Node::Text {
        origin: Point::ORIGIN,
        style,
        ink: fill(),
        anchor: Anchor::Start,
        lines: vec!["Tool".to_owned()],
        leading: 0.0,
    }
}

#[test]
fn style_attributes() {
    let palette = light();
    let target = Target::Static(&palette);
    let quadrant = one(&styled(QUADRANT_LABEL), target);
    assert_eq!(value_of(&quadrant, "letter-spacing"), "0.48");
    assert_eq!(value_of(&quadrant, "font-weight"), "600");
    let tiny = TextStyle {
        letter_spacing: 1e-5,
        ..TITLE
    };
    assert!(!one(&styled(tiny), target).contains("letter-spacing"));
}

#[test]
fn empty_and_unfilled_text() {
    let palette = light();
    let target = Target::Static(&palette);
    let unfilled = one(
        &text(&["x"], Anchor::Start, TextPaint::Fill(Paint::None)),
        target,
    );
    assert_eq!(value_of(&unfilled, "fill"), "none");
    assert_eq!(one(&text(&[], Anchor::Start, fill()), target), "");
    let blank_first = one(&text(&["", "a"], Anchor::Start, fill()), target);
    assert!(
        blank_first.contains("<tspan x=\"10\" y=\"20\"></tspan>"),
        "{blank_first}"
    );
}

#[test]
fn a_halo_is_an_outline_hidden_from_assistive_technology() {
    let palette = light();
    let halo = TextPaint::Halo {
        stroke: Paint::Role(Role::Neutral(Neutral::Surface1)),
        width: 3.0,
    };
    let markup = one(
        &text(&["Adopt"], Anchor::Middle, halo),
        Target::Static(&palette),
    );
    assert!(
        markup.contains(
            " fill=\"none\" stroke=\"#FFFFFF\" stroke-width=\"3\" stroke-linejoin=\"round\" aria-hidden=\"true\">"
        ),
        "{markup}"
    );
}
