//! The serialiser, element by element: exact strings, since attribute order
//! is part of NFR-4's byte identity.

mod group;
mod outline;
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
    super::node(node, Context::top(target))
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
        words: lines.join(" "),
        leading: 15.0,
        tail: None,
    }
}

/// A text whose name is `words`, as wrap gives a token broken mid-word.
fn worded(lines: &[&str], words: &str, ink: TextPaint) -> Node {
    let mut node = text(lines, Anchor::Start, ink);
    if let Node::Text { words: w, .. } = &mut node {
        words.clone_into(w);
    }
    node
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
    // Interactive: the static target adds its ground (D-87), tested below.
    assert_eq!(
        serialise(&document, Target::Interactive(&palette)),
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 420.5 100\">\n</svg>"
    );
}

/// D-87: a static document opens with a Surface1 rect over the whole
/// viewBox; an interactive one has none (the page paints its own).
#[test]
fn the_static_ground_is_the_themes_surface() {
    let document = Document {
        width: 120.0,
        height: 80.5,
        root: group(Point::ORIGIN, Vec::new()),
    };
    for theme in [Theme::Light, Theme::Dark] {
        let palette = Palette::built_in(theme);
        let svg = serialise(&document, Target::Static(&palette));
        let first = svg.lines().nth(1).expect("a first child");
        let fill = palette.value(Role::Neutral(Neutral::Surface1));
        assert_eq!(
            first,
            format!(
                "<rect x=\"0\" y=\"0\" width=\"120\" height=\"80.5\" fill=\"{fill}\" stroke=\"none\"/>"
            ),
            "{theme:?}"
        );
        let page = serialise(&document, Target::Interactive(&palette));
        assert!(!page.contains("<rect"), "{page}");
    }
}
