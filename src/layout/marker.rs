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

/// The nodes for one marker at `centre`, status ring first, in paint order.
#[must_use]
pub fn draw(
    centre: Point,
    radius: f64,
    qualifier: Option<Qualifier>,
    mark: Option<StatusMark>,
    ink: Role,
) -> Vec<Node> {
    let ring_radius = radius + STATUS_RING_GAP;
    let ring = match mark {
        None => None,
        Some(StatusMark::New) => Some(Node::Circle {
            centre,
            radius: ring_radius,
            fill: Paint::None,
            stroke: Paint::Role(ink),
            stroke_width: STATUS_RING_STROKE,
        }),
        Some(StatusMark::MovedIn { centreward }) => {
            Some(half_ring(centre, ring_radius, centreward, ink))
        }
        Some(StatusMark::MovedOut { centreward }) => {
            Some(half_ring(centre, ring_radius, centreward + 180.0, ink))
        }
    };
    let marker = match qualifier {
        Some(Qualifier::Conditional) => Node::Circle {
            centre,
            radius,
            fill: Paint::Role(Role::Neutral(Neutral::MarkerHollow)),
            stroke: Paint::Role(ink),
            stroke_width: MARKER_HOLLOW_STROKE,
        },
        Some(Qualifier::Default) | None => Node::Circle {
            centre,
            radius,
            fill: Paint::Role(ink),
            stroke: Paint::Role(Role::Neutral(Neutral::Surface1)),
            stroke_width: MARKER_STROKE,
        },
    };
    ring.into_iter().chain(Some(marker)).collect()
}

/// The half-circle of `radius` bulging toward `direction` degrees.
fn half_ring(centre: Point, radius: f64, direction: f64, ink: Role) -> Node {
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
        stroke_width: STATUS_RING_STROKE,
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
        let nodes = draw(CENTRE, 7.0, Some(Qualifier::Conditional), None, INK);
        assert_eq!(nodes.len(), 1);
        assert_eq!(
            circle_fill(first(&nodes)),
            Paint::Role(Role::Neutral(Neutral::MarkerHollow))
        );
    }

    #[test]
    fn a_solid_marker_fills_with_its_ink() {
        for q in [Some(Qualifier::Default), None] {
            let nodes = draw(CENTRE, 7.0, q, None, INK);
            assert_eq!(circle_fill(first(&nodes)), Paint::Role(INK));
        }
    }

    #[test]
    fn new_draws_a_full_ring_under_the_marker() {
        let nodes = draw(CENTRE, 7.0, None, Some(StatusMark::New), INK);
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
            );
            let (s, e) = ends(first(&moved_in));
            assert!(near(s, from_centre(c - 90.0)) && near(e, from_centre(c + 90.0)));
            let moved_out = draw(
                CENTRE,
                7.0,
                None,
                Some(StatusMark::MovedOut { centreward: c }),
                INK,
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
        );
        let moved_out = draw(
            CENTRE,
            7.0,
            None,
            Some(StatusMark::MovedOut { centreward: c }),
            INK,
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
        assert_eq!(draw(CENTRE, 7.0, None, None, INK).len(), 1);
    }
}
