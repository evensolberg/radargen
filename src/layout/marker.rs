//! One way to draw a marker glyph, shared by the key and the radar
//! (FR-20, FR-40).

use crate::layout::geometry::polar;
use crate::model::{Qualifier, Status};
use crate::palette::{Neutral, Role};
use crate::style::{MARKER_HOLLOW_STROKE, MARKER_STROKE, STATUS_RING_GAP, STATUS_RING_STROKE};
use crate::svg::{Node, Paint, PathCommand, Point};

/// How a marker shows its status (FR-40); `NoChange` shows nothing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StatusMark {
    /// A full ring.
    New,
    /// A half-ring on the side facing the centre.
    MovedIn {
        /// Degrees (SVG frame) from the marker toward the radar's centre.
        centreward: f64,
    },
    /// A half-ring on the side facing away from the centre.
    MovedOut {
        /// Degrees (SVG frame) from the marker toward the radar's centre.
        centreward: f64,
    },
}

impl StatusMark {
    /// The mark for `status`, given the direction toward the centre;
    /// `None` for `NoChange`.
    #[must_use]
    pub const fn of(status: Status, centreward: f64) -> Option<Self> {
        match status {
            Status::New => Some(Self::New),
            Status::MovedIn => Some(Self::MovedIn { centreward }),
            Status::MovedOut => Some(Self::MovedOut { centreward }),
            Status::NoChange => None,
        }
    }
}

/// Half a turn, in degrees: the opposite direction.
const HALF_TURN: f64 = 180.0;

/// The nodes for one marker, status ring first, in paint order.
///
/// `centre` and `radius` are in scale-1 units; the whole glyph (centre,
/// radii, ring gap and stroke widths) is multiplied by `scale`, so it stays
/// inside a block drawn at that scale (FR-70).
#[must_use]
pub fn draw(
    centre: Point,
    radius: f64,
    qualifier: Option<Qualifier>,
    mark: Option<StatusMark>,
    ink: Role,
    scale: f64,
) -> Vec<Node> {
    let centre = centre.scaled(scale);
    let radius = radius * scale;
    let ring_radius = STATUS_RING_GAP.mul_add(scale, radius);
    let ring_stroke = STATUS_RING_STROKE * scale;
    let ring = match mark {
        None => None,
        Some(StatusMark::New) => Some(Node::Circle {
            centre,
            radius: ring_radius,
            fill: Paint::None,
            stroke: Paint::Role(ink),
            stroke_width: ring_stroke,
        }),
        Some(StatusMark::MovedIn { centreward }) => {
            Some(half_ring(centre, ring_radius, centreward, ink, ring_stroke))
        }
        Some(StatusMark::MovedOut { centreward }) => Some(half_ring(
            centre,
            ring_radius,
            centreward + HALF_TURN,
            ink,
            ring_stroke,
        )),
    };
    let marker = match qualifier {
        Some(Qualifier::Conditional) => Node::Circle {
            centre,
            radius,
            fill: Paint::Role(Role::Neutral(Neutral::MarkerHollow)),
            stroke: Paint::Role(ink),
            stroke_width: MARKER_HOLLOW_STROKE * scale,
        },
        Some(Qualifier::Default) | None => Node::Circle {
            centre,
            radius,
            fill: Paint::Role(ink),
            stroke: Paint::Role(Role::Neutral(Neutral::Surface1)),
            stroke_width: MARKER_STROKE * scale,
        },
    };
    ring.into_iter().chain(Some(marker)).collect()
}

/// The half-circle of `radius` bulging toward `direction` degrees.
fn half_ring(centre: Point, radius: f64, direction: f64, ink: Role, stroke_width: f64) -> Node {
    let at = |angle: f64| {
        let offset = polar(angle, radius);
        Point {
            x: centre.x + offset.x,
            y: centre.y + offset.y,
        }
    };
    Node::Path {
        commands: vec![
            PathCommand::MoveTo(at(direction - 90.0)),
            PathCommand::Arc {
                radius,
                large_arc: false,
                sweep: true,
                end: at(direction + 90.0),
            },
        ],
        fill: Paint::None,
        stroke: Paint::Role(ink),
        stroke_width,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const INK: Role = Role::Series(2);
    const CENTRE: Point = Point { x: 100.0, y: 50.0 };

    fn circle_fill(node: &Node) -> Paint {
        match node {
            Node::Circle { fill, .. } => *fill,
            other => panic!("expected a circle, got {other:?}"),
        }
    }

    fn stroke_of(node: &Node) -> (Paint, f64) {
        match node {
            Node::Circle {
                stroke,
                stroke_width,
                ..
            }
            | Node::Path {
                stroke,
                stroke_width,
                ..
            } => (*stroke, *stroke_width),
            other => panic!("expected a shape, got {other:?}"),
        }
    }

    fn ends(node: &Node) -> (Point, Point) {
        let Node::Path { commands, .. } = node else {
            panic!("expected a path, got {node:?}");
        };
        match commands.as_slice() {
            [PathCommand::MoveTo(start), PathCommand::Arc { end, .. }] => (*start, *end),
            other => panic!("unexpected commands {other:?}"),
        }
    }

    fn first(nodes: &[Node]) -> &Node {
        let Some(node) = nodes.first() else {
            panic!("no nodes drawn");
        };
        node
    }

    fn near(a: Point, b: Point) -> bool {
        (a.x - b.x).abs() < 1e-9 && (a.y - b.y).abs() < 1e-9
    }

    fn from_centre(angle: f64) -> Point {
        let p = polar(angle, 7.0 + STATUS_RING_GAP);
        Point {
            x: CENTRE.x + p.x,
            y: CENTRE.y + p.y,
        }
    }

    #[test]
    fn the_hollow_marker_fills_with_marker_hollow() {
        let nodes = draw(CENTRE, 7.0, Some(Qualifier::Conditional), None, INK, 1.0);
        assert_eq!(nodes.len(), 1);
        assert_eq!(
            circle_fill(first(&nodes)),
            Paint::Role(Role::Neutral(Neutral::MarkerHollow))
        );
    }

    #[test]
    fn a_marker_strokes_as_its_kind_does() {
        let hollow = draw(CENTRE, 7.0, Some(Qualifier::Conditional), None, INK, 1.0);
        assert_eq!(
            stroke_of(first(&hollow)),
            (Paint::Role(INK), MARKER_HOLLOW_STROKE)
        );
        let solid = draw(CENTRE, 7.0, None, None, INK, 1.0);
        assert_eq!(
            stroke_of(first(&solid)),
            (Paint::Role(Role::Neutral(Neutral::Surface1)), MARKER_STROKE)
        );
    }

    #[test]
    fn the_rings_stroke_in_the_ink() {
        for mark in [
            StatusMark::New,
            StatusMark::MovedIn { centreward: 10.0 },
            StatusMark::MovedOut { centreward: 10.0 },
        ] {
            let nodes = draw(CENTRE, 7.0, None, Some(mark), INK, 1.0);
            assert_eq!(
                stroke_of(first(&nodes)),
                (Paint::Role(INK), STATUS_RING_STROKE)
            );
        }
    }

    #[test]
    fn scale_multiplies_the_whole_glyph() {
        let nodes = draw(CENTRE, 7.0, None, Some(StatusMark::New), INK, 0.5);
        let [ring, marker] = nodes.as_slice() else {
            panic!("a ring and a marker");
        };
        let Node::Circle {
            centre,
            radius,
            stroke_width,
            ..
        } = ring
        else {
            panic!("expected a ring");
        };
        assert_eq!(*centre, Point { x: 50.0, y: 25.0 });
        assert!((radius - f64::midpoint(7.0, STATUS_RING_GAP)).abs() < 1e-9);
        assert!((stroke_width - STATUS_RING_STROKE / 2.0).abs() < 1e-9);
        assert!((stroke_of(marker).1 - MARKER_STROKE / 2.0).abs() < 1e-9);
    }

    #[test]
    fn a_solid_marker_fills_with_its_ink() {
        for q in [Some(Qualifier::Default), None] {
            let nodes = draw(CENTRE, 7.0, q, None, INK, 1.0);
            assert_eq!(circle_fill(first(&nodes)), Paint::Role(INK));
        }
    }

    #[test]
    fn new_draws_a_full_ring_under_the_marker() {
        let nodes = draw(CENTRE, 7.0, None, Some(StatusMark::New), INK, 1.0);
        assert_eq!(nodes.len(), 2);
        match first(&nodes) {
            Node::Circle { radius, fill, .. } => {
                assert!((radius - (7.0 + STATUS_RING_GAP)).abs() < 1e-9);
                assert_eq!(*fill, Paint::None);
            }
            other => panic!("expected a ring, got {other:?}"),
        }
        assert_eq!(nodes.get(1).map(circle_fill), Some(Paint::Role(INK)));
    }

    #[test]
    fn half_ring_endpoints_follow_centreward() {
        for c in [0.0, 90.0, 225.0] {
            let moved_in = draw(
                CENTRE,
                7.0,
                None,
                Some(StatusMark::MovedIn { centreward: c }),
                INK,
                1.0,
            );
            let (s, e) = ends(first(&moved_in));
            assert!(near(s, from_centre(c - 90.0)) && near(e, from_centre(c + 90.0)));
            let moved_out = draw(
                CENTRE,
                7.0,
                None,
                Some(StatusMark::MovedOut { centreward: c }),
                INK,
                1.0,
            );
            let (s, e) = ends(first(&moved_out));
            assert!(near(s, from_centre(c + 90.0)) && near(e, from_centre(c + 270.0)));
        }
    }

    #[test]
    fn the_half_rings_are_mirror_images() {
        let c = 180.0;
        let moved_in = draw(
            CENTRE,
            7.0,
            None,
            Some(StatusMark::MovedIn { centreward: c }),
            INK,
            1.0,
        );
        let moved_out = draw(
            CENTRE,
            7.0,
            None,
            Some(StatusMark::MovedOut { centreward: c }),
            INK,
            1.0,
        );
        let (in_s, in_e) = ends(first(&moved_in));
        let (out_s, out_e) = ends(first(&moved_out));
        assert!(near(in_s, out_e) && near(in_e, out_s));
    }

    #[test]
    fn arc_flags() {
        let nodes = draw(
            CENTRE,
            7.0,
            None,
            Some(StatusMark::MovedIn { centreward: 30.0 }),
            INK,
            1.0,
        );
        let Node::Path { commands, .. } = first(&nodes) else {
            panic!("expected a path");
        };
        assert!(matches!(
            commands.get(1),
            Some(PathCommand::Arc {
                large_arc: false,
                sweep: true,
                ..
            })
        ));
    }

    #[test]
    fn status_mark_of_maps_every_status() {
        assert_eq!(StatusMark::of(Status::New, 5.0), Some(StatusMark::New));
        assert_eq!(
            StatusMark::of(Status::MovedIn, 5.0),
            Some(StatusMark::MovedIn { centreward: 5.0 })
        );
        assert_eq!(
            StatusMark::of(Status::MovedOut, 5.0),
            Some(StatusMark::MovedOut { centreward: 5.0 })
        );
        assert_eq!(StatusMark::of(Status::NoChange, 5.0), None);
    }

    #[test]
    fn no_mark_draws_the_marker_alone() {
        assert_eq!(draw(CENTRE, 7.0, None, None, INK, 1.0).len(), 1);
    }
}
