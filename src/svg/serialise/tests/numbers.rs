//! NFR-17: every float through `fmt_coord`.

use super::*;
use crate::style::{TITLE, TextStyle};
use crate::svg::{ItemKey, PathCommand};

/// Every number an attribute holds: values split on space, comma and
/// parentheses, keeping tokens that are numbers (not `--text-muted`).
fn numbers(markup: &str) -> Vec<String> {
    markup
        .split('"')
        .skip(1)
        .step_by(2)
        .flat_map(|value| {
            value
                .split([' ', ',', '(', ')'])
                .filter(|t| {
                    let digits = t.strip_prefix('-').unwrap_or(t);
                    digits.starts_with(|c: char| c.is_ascii_digit())
                })
                .map(str::to_owned)
                .collect::<Vec<_>>()
        })
        .collect()
}

/// `^-?\d+(\.\d{1,3})?$`, and not `-0`.
fn well_formed(token: &str) -> bool {
    let unsigned = token.strip_prefix('-').unwrap_or(token);
    let (whole, frac) = unsigned.split_once('.').unwrap_or((unsigned, ""));
    let digits = |s: &str| s.chars().all(|c| c.is_ascii_digit());
    !whole.is_empty()
        && digits(whole)
        && frac.len() <= 3
        && digits(frac)
        && (!unsigned.contains('.') || !frac.is_empty())
        && token != "-0"
}

/// One node of every kind, every float field `v`.
fn every_kind(v: f64) -> Vec<Node> {
    let style = TextStyle {
        size: v,
        letter_spacing: v,
        ..TITLE
    };
    let arc = PathCommand::Arc {
        radius: v,
        large_arc: false,
        sweep: true,
        end: at(v, v),
    };
    vec![
        Node::Circle {
            centre: at(v, v),
            radius: v,
            fill: MUTED,
            stroke: RING,
            stroke_width: v,
        },
        Node::Rect {
            origin: at(v, v),
            width: v,
            height: v,
            radius: v,
            fill: MUTED,
            stroke: RING,
            stroke_width: v,
        },
        Node::Line {
            from: at(v, v),
            to: at(v, v),
            stroke: RING,
            stroke_width: v,
        },
        Node::Path {
            commands: vec![PathCommand::MoveTo(at(v, v)), arc],
            fill: Paint::None,
            stroke: RING,
            stroke_width: v,
        },
        Node::Text {
            origin: at(v, v),
            style,
            ink: TextPaint::Halo {
                stroke: RING,
                width: v,
            },
            anchor: Anchor::Start,
            lines: vec!["a".to_owned(), "b".to_owned()],
            leading: v,
        },
        Node::HitArea {
            centre: at(v, v),
            radius: v,
            key: ItemKey(3),
        },
    ]
}

#[test]
fn every_float_goes_through_fmt_coord() {
    let palette = light();
    let v = 1.234_567;
    let document = Document {
        width: v,
        height: v,
        root: group(at(v, v), every_kind(v)),
    };
    let markup = serialise(&document, Target::Interactive(&palette));
    for token in numbers(&markup) {
        assert!(well_formed(&token), "{token} in {markup}");
    }
}

#[test]
fn negative_zero_is_zero() {
    let palette = light();
    let markup = one(
        &group(at(-0.0, -3.5), vec![swatch(Paint::None)]),
        Target::Static(&palette),
    );
    assert!(markup.contains("translate(0 -3.5)"), "{markup}");
    assert!(numbers(&markup).iter().all(|t| t != "-0"), "{markup}");
}
