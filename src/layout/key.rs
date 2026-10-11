//! The encoding key (FR-66): what the marker shapes and status rings mean.
//! Its content is fixed whatever the dataset uses (D-25).

use std::num::NonZeroU8;

use crate::layout::block::{Block, Placement, Size};
use crate::layout::marker::{Glyph, StatusMark};
use crate::model::Qualifier;
use crate::palette::{Neutral, Role};
use crate::style::{
    KEY_BOTTOM, KEY_FIRST_ROW, KEY_HEADING, KEY_HEADING_BASELINE, KEY_LABEL, KEY_LABEL_DROP,
    KEY_LABEL_X, KEY_MARKER_RADIUS, KEY_ROW, KEY_SAMPLE_X, KEY_STATUS_MARKER_RADIUS, TextStyle,
};
use crate::svg::{Anchor, Group, Node, Paint, Point, Semantics};
use crate::text::Fonts;

/// The heading's text.
const HEADING: &str = "Key";

/// What a row's sample shows.
#[derive(Clone, Copy)]
struct Row {
    label: &'static str,
    radius: f64,
    qualifier: Option<Qualifier>,
    mark: Option<StatusMark>,
}

/// The five rows, top to bottom. `centreward` is arbitrary for the samples;
/// 180 puts the half-rings on the left (in) and right (out).
const ROWS: [Row; 5] = [
    Row {
        label: "Default, or no qualifier \u{2014} solid marker",
        radius: KEY_MARKER_RADIUS,
        qualifier: Some(Qualifier::Default),
        mark: None,
    },
    Row {
        label: "Conditional \u{2014} hollow marker",
        radius: KEY_MARKER_RADIUS,
        qualifier: Some(Qualifier::Conditional),
        mark: None,
    },
    Row {
        label: "New this edition \u{2014} full ring",
        radius: KEY_STATUS_MARKER_RADIUS,
        qualifier: None,
        mark: Some(StatusMark::New),
    },
    Row {
        label: "Moved in \u{2014} half-ring on the side toward the radar's centre",
        radius: KEY_STATUS_MARKER_RADIUS,
        qualifier: None,
        mark: Some(StatusMark::MovedIn { centreward: 180.0 }),
    },
    Row {
        label: "Moved out \u{2014} half-ring on the side toward the radar's outer edge",
        radius: KEY_STATUS_MARKER_RADIUS,
        qualifier: None,
        mark: Some(StatusMark::MovedOut { centreward: 180.0 }),
    },
];

/// The heading's level: below the document title's level one.
const HEADING_LEVEL: NonZeroU8 = match NonZeroU8::new(2) {
    Some(level) => level,
    None => panic!("2 is not zero"),
};

const SAMPLE_INK: Role = Role::Neutral(Neutral::TextMuted);

/// The centre of row `index`'s sample, at scale 1.
const fn sample_centre(index: usize) -> Point {
    #[expect(
        clippy::cast_precision_loss,
        clippy::as_conversions,
        reason = "a row index is at most four, so the conversion is exact"
    )]
    let y = (index as f64).mul_add(KEY_ROW, KEY_FIRST_ROW);
    Point { x: KEY_SAMPLE_X, y }
}

/// A text node of one line, muted or primary.
fn text(label: &str, origin: Point, style: TextStyle, ink: Neutral, scale: f64) -> Node {
    Node::Text {
        origin: origin.scaled(scale),
        style: style.scaled(scale),
        ink: Paint::Role(Role::Neutral(ink)),
        anchor: Anchor::Start,
        lines: vec![label.to_owned()],
        leading: 0.0,
    }
}

/// One row: its decorative sample, then its label.
fn row_nodes(index: usize, row: Row, scale: f64) -> [Node; 2] {
    let centre = sample_centre(index);
    let glyph = Glyph {
        centre,
        radius: row.radius,
        qualifier: row.qualifier,
        mark: row.mark,
        ink: SAMPLE_INK,
    }
    .draw(scale);
    let origin = Point {
        x: KEY_LABEL_X,
        y: centre.y + KEY_LABEL_DROP,
    };
    [
        Node::Group(Group::new(Point { x: 0.0, y: 0.0 }, glyph).with(Semantics::Decorative)),
        text(row.label, origin, KEY_LABEL, Neutral::TextMuted, scale),
    ]
}

/// The heading, as a level-two heading group.
fn heading_node(scale: f64) -> Node {
    let origin = Point {
        x: 0.0,
        y: KEY_HEADING_BASELINE,
    };
    let node = text(HEADING, origin, KEY_HEADING, Neutral::TextPrimary, scale);
    Node::Group(
        Group::new(Point { x: 0.0, y: 0.0 }, vec![node]).with(Semantics::Heading(HEADING_LEVEL)),
    )
}

/// The scale-1 size: the widest label (after its indent) or the heading.
fn measure(fonts: &Fonts) -> Size {
    let width_of = |style: TextStyle, label: &str| {
        fonts
            .face(style.weight)
            .measure(label, style.size, style.letter_spacing)
    };
    let labels = ROWS
        .iter()
        .map(|row| KEY_LABEL_X + width_of(KEY_LABEL, row.label));
    let width = labels.fold(width_of(KEY_HEADING, HEADING), f64::max);
    Size {
        width,
        height: sample_centre(ROWS.len() - 1).y + KEY_BOTTOM,
    }
}

/// The key to the markers (FR-66): a heading and five fixed rows.
#[derive(Debug, Clone, PartialEq)]
pub struct KeyBlock {
    size: Size,
}

impl KeyBlock {
    /// The key, its width measured with `fonts`.
    #[must_use]
    pub fn new(fonts: &Fonts) -> Self {
        Self {
            size: measure(fonts),
        }
    }
}

impl Block for KeyBlock {
    fn size(&self, scale: f64) -> Size {
        self.size.scaled(scale)
    }

    fn render(&self, at: Placement) -> Node {
        let rows = ROWS
            .iter()
            .enumerate()
            .flat_map(|(index, row)| row_nodes(index, *row, at.scale));
        let children = std::iter::once(heading_node(at.scale))
            .chain(rows)
            .collect();
        Node::Group(Group::new(at.origin, children))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::geometry::polar;
    use crate::layout::title::TitleBlock;
    use crate::style::{STATUS_RING_GAP, STATUS_RING_STROKE};
    use crate::svg::PathCommand;

    fn fonts() -> Fonts {
        Fonts::embedded().expect("the vendored fonts parse")
    }

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    fn at(scale: f64) -> Placement {
        Placement {
            origin: Point { x: 0.0, y: 0.0 },
            scale,
        }
    }

    fn rendered(scale: f64) -> Vec<Node> {
        let Node::Group(group) = KeyBlock::new(&fonts()).render(at(scale)) else {
            panic!("the key renders a group");
        };
        group.children
    }

    fn child(nodes: &[Node], i: usize) -> &Node {
        nodes.get(i).expect("the child exists")
    }

    fn group(node: &Node) -> &Group {
        let Node::Group(group) = node else {
            panic!("expected a group, got {node:?}");
        };
        group
    }

    /// Row `i`'s sample nodes (children 1 + 2i) and label (2 + 2i).
    fn sample(nodes: &[Node], i: usize) -> &[Node] {
        &group(child(nodes, 1 + 2 * i)).children
    }

    fn label(nodes: &[Node], i: usize) -> &Node {
        child(nodes, 2 + 2 * i)
    }

    fn node_at(nodes: &[Node], i: usize) -> &Node {
        nodes.get(i).expect("the node exists")
    }

    fn text_parts(node: &Node) -> (Point, &[String], &TextStyle, Paint) {
        let Node::Text {
            origin,
            lines,
            style,
            ink,
            ..
        } = node
        else {
            panic!("expected text, got {node:?}");
        };
        (*origin, lines, style, *ink)
    }

    fn circle(node: &Node) -> (Point, f64, Paint, Paint) {
        let Node::Circle {
            centre,
            radius,
            fill,
            stroke,
            ..
        } = node
        else {
            panic!("expected a circle, got {node:?}");
        };
        (*centre, *radius, *fill, *stroke)
    }

    fn arc_ends(node: &Node) -> (Point, Point) {
        let Node::Path { commands, .. } = node else {
            panic!("expected a path, got {node:?}");
        };
        match commands.as_slice() {
            [PathCommand::MoveTo(start), PathCommand::Arc { end, .. }] => (*start, *end),
            other => panic!("unexpected commands {other:?}"),
        }
    }

    fn near(p: Point, x: f64, y: f64) -> bool {
        close(p.x, x) && close(p.y, y)
    }

    #[test]
    fn the_key_draws_every_row_always() {
        let nodes = rendered(1.0);
        assert_eq!(nodes.len(), 11);
        let (_, heading, _, _) = text_parts(child(&group(child(&nodes, 0)).children, 0));
        assert_eq!(heading, ["Key"]);
        let expected = [
            "Default, or no qualifier \u{2014} solid marker",
            "Conditional \u{2014} hollow marker",
            "New this edition \u{2014} full ring",
            "Moved in \u{2014} half-ring on the side toward the radar's centre",
            "Moved out \u{2014} half-ring on the side toward the radar's outer edge",
        ];
        for (i, want) in expected.iter().enumerate() {
            let (_, lines, _, _) = text_parts(label(&nodes, i));
            assert_eq!(lines, [(*want).to_owned()]);
        }
    }

    #[test]
    fn the_key_samples_are_the_radar_s_markers() {
        let nodes = rendered(1.0);
        let c = |y| Point { x: 10.0, y };
        let ink = Role::Neutral(Neutral::TextMuted);
        let expected = [
            Glyph {
                centre: c(40.0),
                radius: 7.0,
                qualifier: Some(Qualifier::Default),
                mark: None,
                ink,
            }
            .draw(1.0),
            Glyph {
                centre: c(64.0),
                radius: 7.0,
                qualifier: Some(Qualifier::Conditional),
                mark: None,
                ink,
            }
            .draw(1.0),
            Glyph {
                centre: c(88.0),
                radius: 5.0,
                qualifier: None,
                mark: Some(StatusMark::New),
                ink,
            }
            .draw(1.0),
            Glyph {
                centre: c(112.0),
                radius: 5.0,
                qualifier: None,
                mark: Some(StatusMark::MovedIn { centreward: 180.0 }),
                ink,
            }
            .draw(1.0),
            Glyph {
                centre: c(136.0),
                radius: 5.0,
                qualifier: None,
                mark: Some(StatusMark::MovedOut { centreward: 180.0 }),
                ink,
            }
            .draw(1.0),
        ];
        for (i, want) in expected.iter().enumerate() {
            assert_eq!(sample(&nodes, i), want.as_slice());
        }
    }

    #[test]
    fn key_status_rows_literal_geometry() {
        let nodes = rendered(1.0);
        let (start, end) = arc_ends(node_at(sample(&nodes, 3), 0));
        assert!(near(start, 10.0, 121.0) && near(end, 10.0, 103.0));
        let (start, end) = arc_ends(node_at(sample(&nodes, 4), 0));
        assert!(near(start, 10.0, 127.0) && near(end, 10.0, 145.0));
        let (centre, radius, _, _) = circle(node_at(sample(&nodes, 2), 0));
        assert!(near(centre, 10.0, 88.0) && close(radius, 9.0));
    }

    /// The far point of a clockwise (`sweep`) half-circle: the chord's
    /// midpoint pushed a radius to the left of travel on screen.
    fn apex(start: Point, end: Point, radius: f64) -> Point {
        let (dx, dy) = (end.x - start.x, end.y - start.y);
        let chord = dx.hypot(dy);
        Point {
            x: f64::midpoint(start.x, end.x) + radius * dy / chord,
            y: f64::midpoint(start.y, end.y) - radius * dx / chord,
        }
    }

    #[test]
    fn the_half_rings_bulge_the_right_way() {
        let nodes = rendered(1.0);
        let reach = KEY_STATUS_MARKER_RADIUS + STATUS_RING_GAP;
        // Moved in bulges left, toward the radar's centre (180 degrees);
        // moved out bulges right.
        for (row, y, direction, x) in [(3, 112.0, 180.0, 1.0), (4, 136.0, 0.0, 19.0)] {
            let (start, end) = arc_ends(node_at(sample(&nodes, row), 0));
            let got = apex(start, end, reach);
            let want = polar(direction, reach);
            assert!(near(got, 10.0 + want.x, y + want.y), "row {row}: {got:?}");
            assert!(near(got, x, y), "row {row}: {got:?}");
        }
    }

    #[test]
    fn key_solid_sample_fill_is_text_muted() {
        let nodes = rendered(1.0);
        let muted = Paint::Role(Role::Neutral(Neutral::TextMuted));
        let (_, _, fill, _) = circle(node_at(sample(&nodes, 0), 0));
        assert_eq!(fill, muted);
        let (_, _, fill, stroke) = circle(node_at(sample(&nodes, 1), 0));
        assert_eq!(stroke, muted);
        assert_eq!(fill, Paint::Role(Role::Neutral(Neutral::MarkerHollow)));
    }

    #[test]
    fn the_samples_are_decorative_and_the_heading_is_a_heading() {
        let nodes = rendered(1.0);
        assert_eq!(
            group(child(&nodes, 0)).semantics,
            Semantics::Heading(HEADING_LEVEL)
        );
        for i in 0..5 {
            assert_eq!(
                group(child(&nodes, 1 + 2 * i)).semantics,
                Semantics::Decorative
            );
            assert!(matches!(label(&nodes, i), Node::Text { .. }));
        }
    }

    #[test]
    fn key_geometry() {
        let nodes = rendered(1.0);
        let (origin, _, style, ink) = text_parts(child(&group(child(&nodes, 0)).children, 0));
        assert!(close(origin.y, 20.0) && close(origin.x, 0.0));
        assert_eq!(*style, KEY_HEADING);
        assert_eq!(ink, Paint::Role(Role::Neutral(Neutral::TextPrimary)));
        for (i, y) in [40.0, 64.0, 88.0, 112.0, 136.0].into_iter().enumerate() {
            let (origin, _, style, ink) = text_parts(label(&nodes, i));
            assert!(close(origin.x, 28.0) && close(origin.y, y + 4.0));
            assert_eq!(*style, KEY_LABEL);
            assert_eq!(ink, Paint::Role(Role::Neutral(Neutral::TextMuted)));
        }
        let size = KeyBlock::new(&fonts()).size(1.0);
        assert!(close(size.height, 146.0));
        let face = fonts();
        let widest = ROWS
            .iter()
            .map(|row| face.face(KEY_LABEL.weight).measure(row.label, 11.0, 0.0))
            .fold(0.0, f64::max);
        assert!(close(size.width, 28.0 + widest));
    }

    #[test]
    fn the_samples_scale_with_the_key() {
        let nodes = rendered(0.5);
        let ink = Role::Neutral(Neutral::TextMuted);
        for (i, row) in ROWS.iter().enumerate() {
            let want = Glyph {
                centre: sample_centre(i),
                radius: row.radius,
                qualifier: row.qualifier,
                mark: row.mark,
                ink,
            }
            .draw(0.5);
            assert_eq!(sample(&nodes, i), want.as_slice());
        }
        let Node::Circle {
            radius,
            stroke_width,
            ..
        } = node_at(sample(&nodes, 2), 0)
        else {
            panic!("row 3 starts with its ring");
        };
        assert!(close(*radius, 4.5));
        assert!(close(*stroke_width, STATUS_RING_STROKE / 2.0));
    }

    #[test]
    fn a_placed_key_is_the_origin_key_translated() {
        crate::layout::block::assert_translates(&KeyBlock::new(&fonts()));
    }

    #[test]
    fn size_scales_linearly() {
        let b = KeyBlock::new(&fonts());
        let (one, half) = (b.size(1.0), b.size(0.5));
        assert!(close(half.width, one.width / 2.0));
        assert!(close(half.height, one.height / 2.0));
    }

    /// The block's size and where the node being checked sits within it.
    #[derive(Clone, Copy)]
    struct Frame {
        offset: Point,
        size: Size,
    }

    impl Frame {
        fn contains(self, x: f64, y: f64, what: &str) {
            let (x, y) = (x + self.offset.x, y + self.offset.y);
            let Size { width, height } = self.size;
            assert!(
                x >= -1e-9 && y >= -1e-9 && x <= width + 1e-9 && y <= height + 1e-9,
                "{what} at ({x}, {y}) lies outside {:?}",
                self.size
            );
        }

        /// The four extremes of a disc of `reach` about `centre`.
        fn contains_disc(self, centre: Point, reach: f64, what: &str) {
            for (dx, dy) in [(-reach, 0.0), (reach, 0.0), (0.0, -reach), (0.0, reach)] {
                self.contains(centre.x + dx, centre.y + dy, what);
            }
        }

        fn within(self, group: &Group) -> Self {
            Self {
                offset: Point {
                    x: self.offset.x + group.offset.x,
                    y: self.offset.y + group.offset.y,
                },
                ..self
            }
        }
    }

    /// Every arc here is a half-circle, so its chord is a diameter, its
    /// midpoint is the circle's centre and the circle bounds the arc.
    fn assert_arcs_inside(commands: &[PathCommand], stroke_width: f64, frame: Frame) {
        let mut from = None;
        for command in commands {
            match command {
                PathCommand::MoveTo(p) => from = Some(*p),
                PathCommand::Arc { radius, end, .. } => {
                    let start = from.expect("an arc follows a move");
                    let chord = (end.x - start.x).hypot(end.y - start.y);
                    assert!(
                        close(chord, 2.0 * radius),
                        "an arc of radius {radius} spans {chord}, so it is not a half-circle"
                    );
                    let centre = Point {
                        x: f64::midpoint(start.x, end.x),
                        y: f64::midpoint(start.y, end.y),
                    };
                    frame.contains_disc(centre, radius + stroke_width / 2.0, "arc");
                    from = Some(*end);
                }
            }
        }
    }

    /// Text is bounded by a 0.8 em ascent above the top of the block, a 0.3 em
    /// descent below each baseline and its measured width.
    fn assert_text_inside(node: &Node, frame: Frame, fonts: &Fonts) {
        let Node::Text {
            origin,
            style,
            lines,
            leading,
            ..
        } = node
        else {
            panic!("expected text, got {node:?}");
        };
        for (i, line) in lines.iter().enumerate() {
            #[expect(
                clippy::cast_precision_loss,
                clippy::as_conversions,
                reason = "a line index is tiny, so the conversion is exact"
            )]
            let baseline = (i as f64).mul_add(*leading, origin.y);
            frame.contains(origin.x, 0.8f64.mul_add(-style.size, baseline), "text top");
            frame.contains(
                origin.x,
                0.3f64.mul_add(style.size, baseline),
                "text bottom",
            );
            let face = fonts.face(style.weight);
            let width = face.measure(line, style.size, style.letter_spacing);
            frame.contains(origin.x + width, baseline, "text end");
        }
    }

    /// Everything drawn under `node`, translated through groups, lies within
    /// the frame's size.
    fn assert_inside(node: &Node, frame: Frame, fonts: &Fonts) {
        match node {
            Node::Group(g) => {
                for c in &g.children {
                    assert_inside(c, frame.within(g), fonts);
                }
            }
            Node::Circle {
                centre,
                radius,
                stroke_width,
                ..
            } => frame.contains_disc(*centre, radius + stroke_width / 2.0, "circle"),
            Node::Path {
                commands,
                stroke_width,
                ..
            } => assert_arcs_inside(commands, *stroke_width, frame),
            Node::Text { .. } => assert_text_inside(node, frame, fonts),
        }
    }

    #[test]
    fn everything_drawn_lies_inside_the_size() {
        let fonts = fonts();
        let origin = Point { x: 0.0, y: 0.0 };
        let key = KeyBlock::new(&fonts);
        let title = TitleBlock::new(Some("A title"), Some("A subtitle"), &fonts);
        for scale in [1.0, 0.5] {
            let frame = |size| Frame {
                offset: origin,
                size,
            };
            let rendered = key.render(at(scale));
            assert_inside(&rendered, frame(key.size(scale)), &fonts);
            let rendered = title.render(at(scale));
            assert_inside(&rendered, frame(title.size(scale)), &fonts);
        }
    }

    #[test]
    fn key_bottom_is_the_status_ring_edge() {
        assert!(close(
            KEY_BOTTOM,
            KEY_STATUS_MARKER_RADIUS + STATUS_RING_GAP + STATUS_RING_STROKE / 2.0
        ));
    }
}
