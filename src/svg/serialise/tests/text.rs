//! Text, its lines, its style and its halo.

use super::*;
use crate::style::{LEGEND_META, SECTOR_LABEL, TITLE, TextStyle};
use crate::svg::Tail;

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
        words: "Tool".to_owned(),
        leading: 0.0,
        tail: None,
    }
}

#[test]
fn style_attributes() {
    let palette = light();
    let target = Target::Static(&palette);
    let sector = one(&styled(SECTOR_LABEL), target);
    assert_eq!(value_of(&sector, "letter-spacing"), "0.48");
    assert_eq!(value_of(&sector, "font-weight"), "600");
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

/// A text of `lines` whose last line continues in a tail (FR-74, D-78).
fn tailed(lines: &[&str], parent: TextStyle, tail: TextStyle, words: &str) -> Node {
    Node::Text {
        origin: at(10.0, 20.0),
        style: parent,
        ink: fill(),
        anchor: Anchor::Start,
        lines: lines.iter().map(|&l| l.to_owned()).collect(),
        words: lines.join(" "),
        leading: 15.0,
        tail: Some(Tail {
            gap: 5.0,
            style: tail,
            ink: MUTED,
            text: words.to_owned(),
        }),
    }
}

/// The tail closes the text's content as a `dx` tspan, with nothing
/// between: a newline would collapse to an extra space.
#[test]
fn a_tail_follows_the_last_line() {
    let palette = light();
    let one_line = tailed(&["Kafka"], KEY_LABEL, LEGEND_META, " Default");
    assert_eq!(
        one(&one_line, Target::Static(&palette)),
        "<text x=\"10\" y=\"20\" font-family=\"Inter\" font-size=\"11\" font-weight=\"400\" fill=\"#585F6B\">Kafka<tspan dx=\"5\" font-size=\"10.5\" font-weight=\"400\" fill=\"#585F6B\"> Default</tspan></text>\n"
    );
    assert!(
        one(&one_line, Target::Interactive(&palette))
            .contains("<tspan dx=\"5\" font-size=\"10.5\" font-weight=\"400\" fill=\"var(--text-muted)\"> Default</tspan></text>")
    );
    let two_lines = tailed(&["one", "two"], KEY_LABEL, LEGEND_META, " New");
    assert!(
        one(&two_lines, Target::Static(&palette)).ends_with(
            "<tspan x=\"10\" y=\"35\">two</tspan><tspan dx=\"5\" font-size=\"10.5\" font-weight=\"400\" fill=\"#585F6B\"> New</tspan></text>\n"
        )
    );
}

#[test]
fn a_tail_is_escaped() {
    let palette = light();
    let markup = one(
        &tailed(&["Kafka"], KEY_LABEL, LEGEND_META, " <&>"),
        Target::Static(&palette),
    );
    assert!(markup.contains("> &lt;&amp;&gt;</tspan>"), "{markup}");
}

/// A tspan inherits its text's letter-spacing, so the tail states its own
/// whenever it differs, zero included.
#[test]
fn a_tail_states_its_letter_spacing_when_it_differs() {
    let palette = light();
    let target = Target::Static(&palette);
    let spaced_parent = one(&tailed(&["Tool"], SECTOR_LABEL, LEGEND_META, " x"), target);
    assert!(
        spaced_parent.contains(
            "<tspan dx=\"5\" font-size=\"10.5\" font-weight=\"400\" letter-spacing=\"0\""
        ),
        "{spaced_parent}"
    );
    let spaced_tail = one(&tailed(&["Tool"], KEY_LABEL, SECTOR_LABEL, " x"), target);
    assert!(
        spaced_tail.contains("font-weight=\"600\" letter-spacing=\"0.48\" fill"),
        "{spaced_tail}"
    );
    let same = one(&tailed(&["Tool"], KEY_LABEL, LEGEND_META, " x"), target);
    assert_eq!(same.matches("letter-spacing").count(), 0, "{same}");
}
