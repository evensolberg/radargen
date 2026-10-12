//! Every element and attribute from an allow-list; every number NFR-17's
//! shape (NFR-6, FR-47, NFR-17).

use radargen::layout::arrange::arrange;
use radargen::palette::{Palette, Role, Theme};
use radargen::style::TITLE;
use radargen::style::TextStyle;
use radargen::svg::serialise::{
    DATA_HIT, DATA_LEGEND_ROW, DATA_MARKER, DATA_MARKER_INDICATOR, DATA_ROW, DATA_ROW_INDICATOR,
};
use radargen::svg::{
    Anchor, Document, Group, ItemKey, Node, Paint, PathCommand, Point, Target, TextPaint, serialise,
};

use super::html::SVG_ELEMENTS;
use super::{BOTH, parse, prd, targets};

const EITHER: [&str; 29] = [
    "xmlns",
    "viewBox",
    "transform",
    "role",
    "aria-level",
    "aria-label",
    "aria-hidden",
    "cx",
    "cy",
    "r",
    "x",
    "y",
    "width",
    "height",
    "rx",
    "x1",
    "y1",
    "x2",
    "y2",
    "d",
    "fill",
    "stroke",
    "stroke-width",
    "stroke-linejoin",
    "font-family",
    "font-size",
    "font-weight",
    "letter-spacing",
    "text-anchor",
];
const INTERACTIVE_ONLY: [&str; 9] = [
    "tabindex",
    "pointer-events",
    "visibility",
    DATA_ROW,
    DATA_MARKER,
    DATA_LEGEND_ROW,
    DATA_HIT,
    DATA_MARKER_INDICATOR,
    DATA_ROW_INDICATOR,
];
const NUMERIC: [&str; 16] = [
    "cx",
    "cy",
    "r",
    "x",
    "y",
    "width",
    "height",
    "rx",
    "x1",
    "y1",
    "x2",
    "y2",
    "font-size",
    "letter-spacing",
    "stroke-width",
    "viewBox",
];

/// `^-?\d+(\.\d{1,3})?$`, and not `-0`.
pub fn well_formed(token: &str) -> bool {
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

/// One attribute: allowed for the target, no external reference, and its
/// numbers well formed.
fn attribute(name: &str, value: &str, interactive: bool) {
    let allowed = EITHER.contains(&name);
    let interactive_only = INTERACTIVE_ONLY.contains(&name);
    assert!(allowed || (interactive && interactive_only), "{name}");
    if name == "role" && !interactive {
        assert!(
            matches!(value, "heading" | "img" | "group"),
            "static role {value}"
        );
    }
    assert!(!value.contains("url("), "{name}={value}");
    if NUMERIC.contains(&name) {
        assert!(value.split(' ').all(well_formed), "{name}={value}");
    }
    if matches!(name, "d" | "transform") {
        let numbers = value.split([' ', '(', ')']).filter(|token| {
            let digits = token.strip_prefix('-').unwrap_or(token);
            digits.starts_with(|c: char| c.is_ascii_digit())
        });
        assert!(numbers.into_iter().all(well_formed), "{name}={value}");
    }
}

/// Elements and attributes from the allow-list (no id, handler, link or
/// external paint), numbers NFR-17-shaped.
pub fn vocabulary(markup: &str, interactive: bool) {
    let doc = parse(markup);
    for node in doc.descendants().filter(roxmltree::Node::is_element) {
        let tag = node.tag_name().name();
        assert!(SVG_ELEMENTS.contains(&tag), "<{tag}>");
        for a in node.attributes() {
            attribute(a.name(), a.value(), interactive);
        }
    }
}

#[test]
fn output_uses_only_its_vocabulary() {
    let built = prd();
    for theme in [Theme::Light, Theme::Dark] {
        let palette = Palette::built_in(theme);
        for arrangement in BOTH {
            let document = arrange(arrangement, &built.blocks);
            for (target, interactive) in targets(&palette) {
                vocabulary(&serialise(&document, target), interactive);
            }
        }
    }
}

/// One node of every kind, every float field `v`.
fn every_kind(v: f64) -> Vec<Node> {
    let paint = Paint::Role(Role::Series(0));
    let at = Point { x: v, y: v };
    vec![
        Node::Circle {
            centre: at,
            radius: v,
            fill: paint,
            stroke: paint,
            stroke_width: v,
        },
        Node::Rect {
            origin: at,
            width: v,
            height: v,
            radius: v,
            fill: paint,
            stroke: paint,
            stroke_width: v,
        },
        Node::Line {
            from: at,
            to: at,
            stroke: paint,
            stroke_width: v,
        },
        Node::Path {
            commands: vec![
                PathCommand::MoveTo(at),
                PathCommand::Arc {
                    radius: v,
                    large_arc: false,
                    sweep: true,
                    end: at,
                },
            ],
            fill: Paint::None,
            stroke: paint,
            stroke_width: v,
        },
        Node::Text {
            origin: at,
            style: TextStyle {
                size: v,
                letter_spacing: v,
                ..TITLE
            },
            ink: TextPaint::Halo {
                stroke: paint,
                width: v,
            },
            anchor: Anchor::Start,
            lines: vec!["a".to_owned(), "b".to_owned()],
            leading: v,
        },
        Node::HitArea {
            centre: at,
            radius: v,
            key: ItemKey(3),
        },
    ]
}

/// NFR-17: every float through `fmt_coord`, including sums like a tspan's
/// baseline and the group offset.
#[test]
fn every_float_goes_through_fmt_coord() {
    let palette = Palette::built_in(Theme::Light);
    let v = 1.234_567;
    let document = Document {
        width: v,
        height: v,
        root: Node::Group(Group::new(Point { x: v, y: v }, every_kind(v))),
    };
    let markup = serialise(&document, Target::Interactive(&palette));
    vocabulary(&markup, true);
    let transform = parse(&markup)
        .descendants()
        .find_map(|n| n.attribute("transform").map(str::to_owned))
        .expect("the group moves");
    assert_eq!(transform, "translate(1.235 1.235)");
}

#[test]
fn negative_zero_is_zero() {
    let palette = Palette::built_in(Theme::Light);
    let dot = Node::Circle {
        centre: Point::ORIGIN,
        radius: 1.0,
        fill: Paint::None,
        stroke: Paint::None,
        stroke_width: 0.0,
    };
    let document = Document {
        width: 1.0,
        height: 1.0,
        root: Node::Group(Group::new(Point { x: -0.0, y: -3.5 }, vec![dot])),
    };
    let markup = serialise(&document, Target::Static(&palette));
    assert!(markup.contains("translate(0 -3.5)"), "{markup}");
}
