//! One way to draw a marker glyph, shared by the key and the radar
//! (FR-20, FR-40).

use crate::layout::geometry::polar;
use crate::model::{Qualifier, Status};
use crate::palette::{Neutral, Role};
use crate::style::{MARKER_HOLLOW_STROKE, MARKER_STROKE, STATUS_RING_GAP, STATUS_RING_STROKE};
use crate::svg::{Node, Paint, PathCommand, Point, Scale};

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

/// One marker: where it is, how big, and what it shows.
///
/// `centre` and `radius` are in scale-1 units.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Glyph {
    /// The marker's centre.
    pub centre: Point,
    /// The marker disc's radius.
    pub radius: f64,
    /// Solid or hollow.
    pub qualifier: Option<Qualifier>,
    /// The status ring, if any.
    pub mark: Option<StatusMark>,
    /// The marker's colour role.
    pub ink: Role,
}

impl Glyph {
    /// The glyph's nodes, status ring first, in paint order.
    ///
    /// The centre is multiplied by `scale.position`; the radii, ring gap and
    /// stroke widths by `scale.size` (FR-70).
    #[must_use]
    #[expect(
        clippy::suboptimal_flops,
        reason = "NFR-4: plain * and + are bit-identical on every target; mul_add falls back to a libm fma where the CPU lacks one"
    )]
    pub fn draw(&self, scale: Scale) -> Vec<Node> {
        let Self {
            centre,
            radius,
            qualifier,
            mark,
            ink,
        } = *self;
        let Scale { position, size } = scale;
        let centre = centre.scaled(position);
        let radius = radius * size;
        let ring_radius = radius + STATUS_RING_GAP * size;
        let ring_stroke = STATUS_RING_STROKE * size;
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
        let marker = if qualifier.is_some_and(Qualifier::is_hollow) {
            Node::Circle {
                centre,
                radius,
                fill: Paint::Role(Role::Neutral(Neutral::MarkerHollow)),
                stroke: Paint::Role(ink),
                stroke_width: MARKER_HOLLOW_STROKE * size,
            }
        } else {
            Node::Circle {
                centre,
                radius,
                fill: Paint::Role(ink),
                stroke: Paint::Role(Role::Neutral(Neutral::Surface1)),
                stroke_width: MARKER_STROKE * size,
            }
        };
        ring.into_iter().chain(Some(marker)).collect()
    }
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
    use crate::layout::test_support::{near, plus};

    const UNIT: Scale = Scale::uniform(1.0);
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

    fn from_centre(angle: f64) -> Point {
        plus(CENTRE, polar(angle, 7.0 + STATUS_RING_GAP))
    }

    #[test]
    fn the_hollow_marker_fills_with_marker_hollow() {
        let nodes = Glyph {
            centre: CENTRE,
            radius: 7.0,
            qualifier: Some(Qualifier::Conditional),
            mark: None,
            ink: INK,
        }
        .draw(UNIT);
        assert_eq!(nodes.len(), 1);
        assert_eq!(
            circle_fill(first(&nodes)),
            Paint::Role(Role::Neutral(Neutral::MarkerHollow))
        );
    }

    #[test]
    fn a_marker_strokes_as_its_kind_does() {
        let hollow = Glyph {
            centre: CENTRE,
            radius: 7.0,
            qualifier: Some(Qualifier::Conditional),
            mark: None,
            ink: INK,
        }
        .draw(UNIT);
        assert_eq!(
            stroke_of(first(&hollow)),
            (Paint::Role(INK), MARKER_HOLLOW_STROKE)
        );
        let solid = Glyph {
            centre: CENTRE,
            radius: 7.0,
            qualifier: None,
            mark: None,
            ink: INK,
        }
        .draw(UNIT);
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
            let nodes = Glyph {
                centre: CENTRE,
                radius: 7.0,
                qualifier: None,
                mark: Some(mark),
                ink: INK,
            }
            .draw(UNIT);
            assert_eq!(
                stroke_of(first(&nodes)),
                (Paint::Role(INK), STATUS_RING_STROKE)
            );
        }
    }

    #[test]
    fn scale_multiplies_the_whole_glyph() {
        let nodes = Glyph {
            centre: CENTRE,
            radius: 7.0,
            qualifier: None,
            mark: Some(StatusMark::New),
            ink: INK,
        }
        .draw(Scale::uniform(0.5));
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
    fn glyph_scales_position_and_size_apart() {
        let scale = Scale {
            position: 0.5,
            size: 7.0 / 9.0,
        };
        let nodes = Glyph {
            centre: CENTRE,
            radius: 10.0,
            qualifier: None,
            mark: Some(StatusMark::New),
            ink: INK,
        }
        .draw(scale);
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
        assert!((radius - 10.8889).abs() < 1e-4, "{radius}");
        assert!((stroke_width - STATUS_RING_STROKE * 7.0 / 9.0).abs() < 1e-9);
        let Node::Circle { radius, .. } = marker else {
            panic!("expected a disc");
        };
        assert!((radius - 7.7778).abs() < 1e-4, "{radius}");
        assert!((stroke_of(marker).1 - MARKER_STROKE * 7.0 / 9.0).abs() < 1e-9);
    }

    #[test]
    fn a_solid_marker_fills_with_its_ink() {
        for q in [Some(Qualifier::Default), None] {
            let nodes = Glyph {
                centre: CENTRE,
                radius: 7.0,
                qualifier: q,
                mark: None,
                ink: INK,
            }
            .draw(UNIT);
            assert_eq!(circle_fill(first(&nodes)), Paint::Role(INK));
        }
    }

    #[test]
    fn new_draws_a_full_ring_under_the_marker() {
        let nodes = Glyph {
            centre: CENTRE,
            radius: 7.0,
            qualifier: None,
            mark: Some(StatusMark::New),
            ink: INK,
        }
        .draw(UNIT);
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
            let moved_in = Glyph {
                centre: CENTRE,
                radius: 7.0,
                qualifier: None,
                mark: Some(StatusMark::MovedIn { centreward: c }),
                ink: INK,
            }
            .draw(UNIT);
            let (s, e) = ends(first(&moved_in));
            assert!(near(s, from_centre(c - 90.0)) && near(e, from_centre(c + 90.0)));
            let moved_out = Glyph {
                centre: CENTRE,
                radius: 7.0,
                qualifier: None,
                mark: Some(StatusMark::MovedOut { centreward: c }),
                ink: INK,
            }
            .draw(UNIT);
            let (s, e) = ends(first(&moved_out));
            assert!(near(s, from_centre(c + 90.0)) && near(e, from_centre(c + 270.0)));
        }
    }

    #[test]
    fn the_half_rings_are_mirror_images() {
        let c = 180.0;
        let moved_in = Glyph {
            centre: CENTRE,
            radius: 7.0,
            qualifier: None,
            mark: Some(StatusMark::MovedIn { centreward: c }),
            ink: INK,
        }
        .draw(UNIT);
        let moved_out = Glyph {
            centre: CENTRE,
            radius: 7.0,
            qualifier: None,
            mark: Some(StatusMark::MovedOut { centreward: c }),
            ink: INK,
        }
        .draw(UNIT);
        let (in_s, in_e) = ends(first(&moved_in));
        let (out_s, out_e) = ends(first(&moved_out));
        assert!(near(in_s, out_e) && near(in_e, out_s));
    }

    #[test]
    fn arc_flags() {
        let nodes = Glyph {
            centre: CENTRE,
            radius: 7.0,
            qualifier: None,
            mark: Some(StatusMark::MovedIn { centreward: 30.0 }),
            ink: INK,
        }
        .draw(UNIT);
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
        assert_eq!(
            Glyph {
                centre: CENTRE,
                radius: 7.0,
                qualifier: None,
                mark: None,
                ink: INK
            }
            .draw(UNIT)
            .len(),
            1
        );
    }
}
