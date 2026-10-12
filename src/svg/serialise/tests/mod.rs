//! The serialiser, element by element: exact strings, since attribute order
//! is part of NFR-4's byte identity.

mod group;
mod numbers;
mod paint;
mod semantics;
mod shape;
mod text;

use super::*;
use crate::palette::{Neutral, Role, Theme};
use crate::style::KEY_LABEL;
use crate::svg::{Anchor, Group, Point, TextPaint};

fn light() -> Palette {
    Palette::built_in(Theme::Light)
}

/// `node` written at the top level for `target`.
fn one(node: &Node, target: Target<'_>) -> String {
    super::node(
        node,
        Context {
            target,
            control: None,
        },
    )
}

const MUTED: Paint = Paint::Role(Role::Neutral(Neutral::TextMuted));
const RING: Paint = Paint::Role(Role::Neutral(Neutral::StrokeRing));

const fn at(x: f64, y: f64) -> Point {
    Point { x, y }
}

fn swatch(fill: Paint) -> Node {
    Node::Circle {
        centre: Point::ORIGIN,
        radius: 1.0,
        fill,
        stroke: Paint::None,
        stroke_width: 0.0,
    }
}

fn text(lines: &[&str], anchor: Anchor, ink: TextPaint) -> Node {
    Node::Text {
        origin: at(10.0, 20.0),
        style: KEY_LABEL,
        ink,
        anchor,
        lines: lines.iter().map(|&l| l.to_owned()).collect(),
        leading: 15.0,
    }
}

fn group(offset: Point, children: Vec<Node>) -> Node {
    Node::Group(Group::new(offset, children))
}

/// The value of `name` in the first element of `markup` that has it.
fn value_of<'a>(markup: &'a str, name: &str) -> &'a str {
    let key = format!(" {name}=\"");
    let start = markup
        .find(&key)
        .map(|i| i + key.len())
        .expect("the attribute");
    let rest = markup.get(start..).expect("in range");
    rest.get(..rest.find('"').expect("closed"))
        .expect("in range")
}

#[test]
fn the_root_carries_the_view_box() {
    let palette = light();
    let document = Document {
        width: 420.5,
        height: 100.0,
        root: group(Point::ORIGIN, Vec::new()),
    };
    assert_eq!(
        serialise(&document, Target::Static(&palette)),
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 420.5 100\">\n</svg>"
    );
}
