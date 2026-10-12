//! Circles, rectangles, lines and paths.

use super::*;
use crate::svg::PathCommand;

fn static_target(palette: &Palette) -> Target<'_> {
    Target::Static(palette)
}

#[test]
fn a_circle() {
    let palette = light();
    let circle = Node::Circle {
        centre: at(1.0, 2.0),
        radius: 3.0,
        fill: Paint::None,
        stroke: RING,
        stroke_width: 1.5,
    };
    assert_eq!(
        one(&circle, static_target(&palette)),
        "<circle cx=\"1\" cy=\"2\" r=\"3\" fill=\"none\" stroke=\"#7E858F\" stroke-width=\"1.5\"/>\n"
    );
}

fn square(radius: f64) -> Node {
    Node::Rect {
        origin: at(0.0, 0.0),
        width: 10.0,
        height: 5.0,
        radius,
        fill: MUTED,
        stroke: Paint::None,
        stroke_width: 2.0,
    }
}

/// No `stroke-width` without a stroke; `rx` only when it formats non-zero.
#[test]
fn a_rect() {
    let palette = light();
    let target = static_target(&palette);
    let plain =
        "<rect x=\"0\" y=\"0\" width=\"10\" height=\"5\" fill=\"#585F6B\" stroke=\"none\"/>\n";
    assert_eq!(one(&square(0.0), target), plain);
    assert_eq!(one(&square(0.0004), target), plain, "rx formats as 0");
    assert_eq!(
        one(&square(6.0), target),
        "<rect x=\"0\" y=\"0\" width=\"10\" height=\"5\" rx=\"6\" fill=\"#585F6B\" stroke=\"none\"/>\n"
    );
}

#[test]
fn a_line() {
    let palette = light();
    let line = Node::Line {
        from: at(20.0, 7.0),
        to: at(400.0, 7.0),
        stroke: RING,
        stroke_width: 1.0,
    };
    assert_eq!(
        one(&line, static_target(&palette)),
        "<line x1=\"20\" y1=\"7\" x2=\"400\" y2=\"7\" stroke=\"#7E858F\" stroke-width=\"1\"/>\n"
    );
}

fn arc(large_arc: bool, sweep: bool) -> Node {
    Node::Path {
        commands: vec![
            PathCommand::MoveTo(at(0.0, 9.0)),
            PathCommand::Arc {
                radius: 9.0,
                large_arc,
                sweep,
                end: at(0.0, -9.0),
            },
        ],
        fill: Paint::None,
        stroke: RING,
        stroke_width: 2.0,
    }
}

/// The flags in their own slots: swapping them would pass a symmetric test.
#[test]
fn a_path_with_an_arc() {
    let palette = light();
    let target = static_target(&palette);
    assert_eq!(
        one(&arc(true, false), target),
        "<path d=\"M 0 9 A 9 9 0 1 0 0 -9\" fill=\"none\" stroke=\"#7E858F\" stroke-width=\"2\"/>\n"
    );
    assert!(one(&arc(false, true), target).contains("A 9 9 0 0 1 0 -9"));
}
