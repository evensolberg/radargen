//! The radar block's frame (FR-30–FR-32, FR-42, FR-43, FR-70): ring
//! circles, quadrant dividers, quadrant labels and ring names. Markers come
//! in PR 10b.
//!
//! FR-70 splits the radar's scale three ways: positions shrink with it,
//! labels never drop below their natural size, and strokes stop at
//! [`RADAR_FLOOR`]. So the block does not scale linearly, and its size is
//! computed per scale from what it draws.

use crate::layout::block::{Block, Placement, Size};
use crate::layout::geometry::{Geometry, polar};
use crate::model::Catalogue;
use crate::order::Plotted;
use crate::palette::{Neutral, Role};
use crate::style::{
    DIVIDER_RADIUS, GRID_STROKE, LABEL_HALO, MIN_RADAR_WIDTH, QUADRANT_LABEL,
    QUADRANT_LABEL_RADIUS, RADAR_FLOOR, RADAR_LABEL_PAD, RING_LABEL, TextStyle,
};
use crate::svg::{Anchor, Group, Node, Paint, Point, Scale, TextRun};
use crate::text::{Fonts, TEXT_ASCENT, TEXT_DESCENT};

/// Half the minimum radar width: the floor on each half of the block.
const MIN_RADAR_HALF: f64 = MIN_RADAR_WIDTH / 2.0;
/// How far a ring name's halo reaches past its glyphs.
const HALO_REACH: f64 = LABEL_HALO / 2.0;

/// FR-70's three scales for one radar scale `s`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RadarScale {
    /// For positions and radii: `s`.
    pub geometry: f64,
    /// For label text: `max(s, 1)`, so labels never shrink below natural
    /// size.
    pub labels: f64,
    /// For strokes (and, from PR 10b, markers): `max(s, RADAR_FLOOR)`.
    pub marks: f64,
}

impl RadarScale {
    /// `geometry` = s, `labels` = max(s, 1), `marks` = max(s, `RADAR_FLOOR`).
    ///
    /// # Panics
    ///
    /// In debug builds, if `s` is not finite and positive: a NaN would
    /// otherwise vanish into `max` and leave only the floors.
    #[must_use]
    pub fn of(s: f64) -> Self {
        debug_assert!(s.is_finite() && s > 0.0, "radar scale {s}");
        Self {
            geometry: s,
            labels: s.max(1.0),
            marks: s.max(RADAR_FLOOR),
        }
    }

    /// Where labels are placed and how large they are drawn.
    const fn text(self) -> Scale {
        Scale {
            position: self.geometry,
            size: self.labels,
        }
    }
}

/// A label's text and its width at natural size.
#[derive(Debug, Clone, PartialEq)]
struct Label {
    text: String,
    width: f64,
}

impl Label {
    fn measured(text: &str, style: TextStyle, fonts: &Fonts) -> Self {
        let width = fonts
            .face(style.weight)
            .measure(text, style.size, style.letter_spacing);
        Self {
            text: text.to_owned(),
            width,
        }
    }
}

/// The radar (FR-30–FR-32): its rings, dividers and labels.
#[derive(Debug, Clone, PartialEq)]
pub struct RadarBlock {
    geometry: Geometry,
    /// In FR-10 order: label `p` belongs to the sector at position `p`.
    quadrants: Vec<Label>,
    /// In declared order, outermost first (D-60).
    rings: Vec<Label>,
}

impl RadarBlock {
    /// The frame for `catalogue`, its quadrants in `plotted`'s FR-10 order
    /// and every declared ring drawn, empty or not (FR-19). Labels are
    /// measured once here, at natural size.
    ///
    /// # Panics
    ///
    /// If `plotted` names a quadrant `catalogue` lacks (`order::resolve`
    /// guarantees it does not).
    #[must_use]
    // qual:allow(complexity, error_handling) reason: "the documented # Panics: a Plotted from another catalogue is a caller bug"
    pub fn new(catalogue: &Catalogue, plotted: &Plotted, fonts: &Fonts) -> Self {
        let quadrants = plotted
            .quadrants
            .iter()
            .map(|&q| {
                let quadrant = catalogue
                    .quadrants
                    .get(q)
                    .expect("FR-10's order indexes the catalogue's quadrants");
                Label::measured(&quadrant.label, QUADRANT_LABEL, fonts)
            })
            .collect();
        let rings = catalogue
            .rings
            .iter()
            .map(|ring| Label::measured(ring, RING_LABEL, fonts))
            .collect();
        Self {
            geometry: Geometry::of(catalogue),
            quadrants,
            rings,
        }
    }

    /// Where quadrant label `position` is anchored, at scale 1.
    fn anchor(&self, position: usize) -> Point {
        polar(self.geometry.middle(position), QUADRANT_LABEL_RADIUS)
    }

    /// The middle of ring `ring`'s band, where its name sits, at scale 1.
    fn mid(&self, ring: usize) -> f64 {
        let band = self.geometry.band(ring);
        f64::midpoint(band.inner, band.outer)
    }

    /// The furthest any label reaches left or right of the centre.
    fn label_half_width(&self, r: RadarScale) -> f64 {
        let quadrants = self
            .quadrants
            .iter()
            .enumerate()
            .map(|(p, label)| quadrant_reach_x(self.anchor(p), label.width, r));
        let rings = self.rings.iter().map(|label| ring_reach_x(label.width, r));
        quadrants.chain(rings).fold(0.0, f64::max)
    }

    /// The furthest any label reaches above or below the centre.
    fn label_half_height(&self, r: RadarScale) -> f64 {
        let quadrants = (0..self.quadrants.len()).map(|p| quadrant_reach_y(self.anchor(p), r));
        let rings = (0..self.rings.len()).map(|i| ring_reach_y(self.mid(i), r));
        quadrants.chain(rings).fold(0.0, f64::max)
    }

    /// The radar's centre in the block: half its size on each axis.
    fn centre(&self, r: RadarScale) -> Point {
        Point {
            x: half_extent(self.label_half_width(r), r),
            y: half_extent(self.label_half_height(r), r),
        }
    }

    /// The block's size at `r`. PR 10b adds the markers' and hit areas'
    /// reach as one more term each.
    // qual:allow(complexity, magic_numbers) reason: "a box centred on its centre spans twice the centre offset"
    fn extent(&self, r: RadarScale) -> Size {
        let centre = self.centre(r);
        Size {
            width: 2.0 * centre.x,
            height: 2.0 * centre.y,
        }
    }

    /// One circle per declared ring, at its band's outer edge (FR-32).
    fn ring_circles(&self, r: RadarScale) -> impl Iterator<Item = Node> + '_ {
        (0..self.rings.len()).map(move |i| Node::Circle {
            centre: Point::ORIGIN,
            radius: self.geometry.band(i).outer * r.geometry,
            fill: Paint::None,
            stroke: Paint::Role(Role::Neutral(Neutral::StrokeRing)),
            stroke_width: GRID_STROKE * r.marks,
        })
    }

    /// One divider per quadrant (FR-77); none with no quadrants (FR-42).
    fn dividers(&self, r: RadarScale) -> impl Iterator<Item = Node> + '_ {
        (0..self.quadrants.len()).map(move |p| Node::Line {
            from: Point::ORIGIN,
            to: polar(self.geometry.divider(p), DIVIDER_RADIUS * r.geometry),
            stroke: Paint::Role(Role::Neutral(Neutral::StrokeDivider)),
            stroke_width: GRID_STROKE * r.marks,
        })
    }

    /// Each quadrant's name, mid-sector beyond the rim.
    fn quadrant_labels(&self, r: RadarScale) -> impl Iterator<Item = Node> + '_ {
        self.quadrants.iter().enumerate().map(move |(p, label)| {
            TextRun {
                origin: self.anchor(p),
                style: QUADRANT_LABEL,
                ink: Neutral::TextPrimary,
                anchor: Anchor::Middle,
                lines: vec![label.text.clone()],
                leading: 0.0,
            }
            .node(r.text())
        })
    }

    /// Each ring's name at 12 o'clock, over a halo so the divider there
    /// (FR-31) stops short of it.
    fn ring_labels(&self, r: RadarScale) -> impl Iterator<Item = Node> + '_ {
        self.rings.iter().enumerate().flat_map(move |(i, label)| {
            let run = TextRun {
                origin: Point {
                    x: 0.0,
                    y: -self.mid(i),
                },
                style: RING_LABEL,
                ink: Neutral::TextMuted,
                anchor: Anchor::Middle,
                lines: vec![label.text.clone()],
                leading: 0.0,
            };
            [run.halo(LABEL_HALO, r.text()), run.node(r.text())]
        })
    }
}

impl Block for RadarBlock {
    fn size(&self, scale: f64) -> Size {
        self.extent(RadarScale::of(scale))
    }

    fn render(&self, at: Placement) -> Node {
        let r = RadarScale::of(at.scale);
        let frame = self
            .ring_circles(r)
            .chain(self.dividers(r))
            .chain(self.quadrant_labels(r))
            .chain(self.ring_labels(r))
            .collect();
        let centred = Node::Group(Group::new(self.centre(r), frame));
        Node::Group(Group::new(at.origin, vec![centred]))
    }
}

/// How far a quadrant label anchored at `at` reaches left or right of the
/// centre: its anchor's distance, then half its middle-anchored width.
#[expect(
    clippy::suboptimal_flops,
    reason = "NFR-4: plain * and + are bit-identical on every target; mul_add falls back to a libm fma where the CPU lacks one"
)]
fn quadrant_reach_x(at: Point, width: f64, r: RadarScale) -> f64 {
    at.x.abs() * r.geometry + f64::midpoint(0.0, width) * r.labels
}

/// How far a quadrant label anchored at `at` reaches above or below the
/// centre: the descent below its baseline, or the ascent above it.
#[expect(
    clippy::suboptimal_flops,
    reason = "NFR-4: plain * and + are bit-identical on every target; mul_add falls back to a libm fma where the CPU lacks one"
)]
fn quadrant_reach_y(at: Point, r: RadarScale) -> f64 {
    let baseline = at.y * r.geometry;
    let size = QUADRANT_LABEL.size * r.labels;
    (baseline + TEXT_DESCENT * size).max(TEXT_ASCENT * size - baseline)
}

/// How far a ring name of `width`, centred on the vertical axis, reaches
/// either side, its halo included.
fn ring_reach_x(width: f64, r: RadarScale) -> f64 {
    f64::midpoint(width, LABEL_HALO) * r.labels
}

/// How far a ring name whose baseline is `mid` above the centre reaches
/// upward, its ascent and halo included. It never reaches further down
/// than up.
#[expect(
    clippy::suboptimal_flops,
    reason = "NFR-4: plain * and + are bit-identical on every target; mul_add falls back to a libm fma where the CPU lacks one"
)]
fn ring_reach_y(mid: f64, r: RadarScale) -> f64 {
    mid * r.geometry + (TEXT_ASCENT * RING_LABEL.size + HALO_REACH) * r.labels
}

/// Half the block on one axis: half the minimum radar width at this scale,
/// or the labels' reach plus the pad, whichever is larger.
///
/// The `MIN_RADAR_HALF * s` floor is also what keeps the ring circles (at
/// most `260 * s` plus half a stroke) and the dividers (`262 * s` plus half
/// a stroke) inside the block: the label terms never count them. That holds
/// for `s` of about 0.0056 and up, where `70 * s` covers half a floored
/// stroke. PR 10b must add an explicit term for anything that can reach
/// further than the floor.
///
/// The block's size is non-decreasing in `s` from 0.0034 up. The only
/// decreasing term is a below-centre quadrant label's ascent, at most
/// 11.625; the outermost ring name's reach, at least `130 * s + 11.19`,
/// outgrows it from there. That relies on at least one ring, which
/// `rings.rs` guarantees by rejecting an empty list.
#[expect(
    clippy::suboptimal_flops,
    reason = "NFR-4: plain * and + are bit-identical on every target; mul_add falls back to a libm fma where the CPU lacks one"
)]
fn half_extent(label_reach: f64, r: RadarScale) -> f64 {
    (MIN_RADAR_HALF * r.geometry).max(label_reach + RADAR_LABEL_PAD * r.labels)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::block::{Block, assert_translates, assert_translates_at, at, close, fonts};
    use crate::layout::geometry::{Geometry, polar};
    use crate::layout::test_support::{assert_finite, assert_inside, bounds, plotted};
    use crate::palette::{Neutral, Role};
    use crate::style::{
        GRID_STROKE, LABEL_HALO, QUADRANT_LABEL, RADAR_FLOOR, RADAR_LABEL_PAD, RING_LABEL,
        TextStyle,
    };
    use crate::svg::{Anchor, Node, Paint, Point, Semantics, TextPaint};
    use crate::text::{TEXT_ASCENT, TEXT_DESCENT, Weight};

    const HEADER: &str = "name,quadrant,ring,description\n";
    const PRD_EXAMPLE: &str = include_str!("../../tests/fixtures/three-quadrant.csv");
    /// How near the block's edge the drawn labels must come: float noise.
    const TIGHT: f64 = 1e-6;
    const PRIMARY: TextPaint = TextPaint::Fill(Paint::Role(Role::Neutral(Neutral::TextPrimary)));
    const MUTED: TextPaint = TextPaint::Fill(Paint::Role(Role::Neutral(Neutral::TextMuted)));

    /// One row per quadrant name, all in `ring`.
    fn csv_of(quadrants: &[&str], ring: &str) -> String {
        quadrants
            .iter()
            .enumerate()
            .fold(HEADER.to_owned(), |text, (i, q)| {
                format!("{text}item {i},{q},{ring},d\n")
            })
    }

    fn block(csv: &str, rings: Option<&str>) -> RadarBlock {
        let (catalogue, order) = plotted(csv, rings);
        RadarBlock::new(&catalogue, &order, &fonts())
    }

    /// `n` short quadrants, `Q0`… in FR-10 order, default rings.
    fn quadrants(n: usize) -> RadarBlock {
        let names: Vec<String> = (0..n).map(|i| format!("Q{i}")).collect();
        let names: Vec<&str> = names.iter().map(String::as_str).collect();
        block(&csv_of(&names, "Adopt"), None)
    }

    fn long_quadrant() -> String {
        "W".repeat(40)
    }

    fn long_ring() -> String {
        "W".repeat(80)
    }

    /// N = 4 with one quadrant named 40 × "W".
    fn long_quadrant_block() -> RadarBlock {
        let long = long_quadrant();
        block(&csv_of(&["B", "C", "D", &long], "Adopt"), None)
    }

    /// No quadrants and one ring named 80 × "W".
    fn long_ring_block() -> RadarBlock {
        block(HEADER, Some(&long_ring()))
    }

    /// N = 1 with a long name: its label is at 6 o'clock, so the width is
    /// the label's alone below scale 1.
    fn long_single() -> RadarBlock {
        block(&csv_of(&[&long_quadrant()], "Adopt"), None)
    }

    fn fixtures() -> Vec<(&'static str, RadarBlock)> {
        vec![
            ("prd", block(PRD_EXAMPLE, None)),
            ("n1", quadrants(1)),
            ("n2", quadrants(2)),
            ("n6", quadrants(6)),
            ("n8", quadrants(8)),
            ("zero", block(HEADER, None)),
            ("long quadrant", long_quadrant_block()),
            ("long ring", long_ring_block()),
        ]
    }

    /// The frame's children: inside the centring group, inside the root.
    fn frame(node: &Node) -> Vec<Node> {
        let Node::Group(root) = node else {
            panic!("the radar renders a group, got {node:?}");
        };
        let [Node::Group(centred)] = root.children.as_slice() else {
            panic!("the root holds one centring group: {:?}", root.children);
        };
        centred.children.clone()
    }

    fn frame_at(b: &RadarBlock, s: f64) -> Vec<Node> {
        frame(&b.render(at(s)))
    }

    /// (radius, stroke width) of every circle.
    fn circles(nodes: &[Node]) -> Vec<(f64, f64)> {
        nodes
            .iter()
            .filter_map(|n| match n {
                Node::Circle {
                    centre,
                    radius,
                    fill,
                    stroke,
                    stroke_width,
                } => {
                    assert_eq!(*centre, Point::ORIGIN, "a ring is centred");
                    assert_eq!(*fill, Paint::None, "a ring is unfilled");
                    assert_eq!(*stroke, Paint::Role(Role::Neutral(Neutral::StrokeRing)));
                    Some((*radius, *stroke_width))
                }
                _ => None,
            })
            .collect()
    }

    /// (end, stroke width) of every line.
    fn lines(nodes: &[Node]) -> Vec<(Point, f64)> {
        nodes
            .iter()
            .filter_map(|n| match n {
                Node::Line {
                    from,
                    to,
                    stroke,
                    stroke_width,
                } => {
                    assert_eq!(*from, Point::ORIGIN, "a divider starts at the centre");
                    assert_eq!(*stroke, Paint::Role(Role::Neutral(Neutral::StrokeDivider)));
                    Some((*to, *stroke_width))
                }
                _ => None,
            })
            .collect()
    }

    /// A text node's parts.
    struct Text<'a> {
        origin: Point,
        style: TextStyle,
        ink: TextPaint,
        anchor: Anchor,
        lines: &'a [String],
    }

    fn text(node: &Node) -> Option<Text<'_>> {
        match node {
            Node::Text {
                origin,
                style,
                ink,
                anchor,
                lines,
                ..
            } => Some(Text {
                origin: *origin,
                style: *style,
                ink: *ink,
                anchor: *anchor,
                lines,
            }),
            _ => None,
        }
    }

    /// Quadrant labels are the only `SemiBold` texts in the frame.
    fn quadrant_labels(nodes: &[Node]) -> Vec<Text<'_>> {
        nodes
            .iter()
            .filter_map(text)
            .filter(|t| t.style.weight == Weight::SemiBold)
            .collect()
    }

    /// Ring names and their halos are the only `Regular` texts.
    fn ring_texts(nodes: &[Node]) -> Vec<Text<'_>> {
        nodes
            .iter()
            .filter_map(text)
            .filter(|t| t.style.weight == Weight::Regular)
            .collect()
    }

    fn ring_names(nodes: &[Node]) -> Vec<Text<'_>> {
        ring_texts(nodes)
            .into_iter()
            .filter(|t| t.ink == MUTED)
            .collect()
    }

    fn same(a: Point, b: Point) -> bool {
        close(a.x, b.x) && close(a.y, b.y)
    }

    fn ends_are(got: &[(Point, f64)], angles: &[f64], radius: f64) {
        assert_eq!(got.len(), angles.len(), "{got:?} vs {angles:?}");
        for ((end, _), angle) in got.iter().zip(angles) {
            assert!(same(*end, polar(*angle, radius)), "{end:?} vs {angle}°");
        }
    }

    fn label_texts(nodes: &[Node]) -> Vec<String> {
        quadrant_labels(nodes)
            .iter()
            .map(|t| t.lines.concat())
            .collect()
    }

    #[test]
    fn radar_scale() {
        let is = |s: f64, [geometry, labels, marks]: [f64; 3]| {
            let r = RadarScale::of(s);
            assert!(
                close(r.geometry, geometry) && close(r.labels, labels) && close(r.marks, marks),
                "{s}: ({}, {}, {})",
                r.geometry,
                r.labels,
                r.marks
            );
        };
        is(1.0, [1.0, 1.0, 1.0]);
        is(0.9, [0.9, 1.0, 0.9]);
        is(RADAR_FLOOR, [7.0 / 9.0, 1.0, 7.0 / 9.0]);
        is(0.5, [0.5, 1.0, 7.0 / 9.0]);
        is(1.5, [1.5, 1.5, 1.5]);
    }

    #[test]
    fn zero_quadrants_draws_rings_only() {
        let b = block(HEADER, None);
        let size = b.size(1.0);
        assert!(
            close(size.width, 660.0) && close(size.height, 660.0),
            "{size:?}"
        );
        let rendered = b.render(at(1.0));
        assert_finite(&rendered);
        let nodes = frame(&rendered);
        assert_eq!(circles(&nodes).len(), 4);
        assert_eq!(lines(&nodes).len(), 0);
        let texts: Vec<Text> = nodes.iter().filter_map(text).collect();
        assert_eq!(texts.len(), 8);
        let halos = texts
            .iter()
            .filter(|t| matches!(t.ink, TextPaint::Halo { .. }))
            .count();
        assert_eq!(halos, 4);
        assert_eq!(ring_names(&nodes).len(), 4);
        assert_eq!(nodes.len(), 12, "nothing else: {nodes:?}");
    }

    #[test]
    fn one_quadrant_has_one_divider() {
        let nodes = frame_at(&quadrants(1), 1.0);
        ends_are(&lines(&nodes), &[-90.0], 262.0);
        let labels = quadrant_labels(&nodes);
        let [label] = labels.as_slice() else {
            panic!("one quadrant label");
        };
        assert!(same(label.origin, polar(90.0, 300.0)), "{:?}", label.origin);
        assert!(close(label.origin.x, 0.0) && close(label.origin.y, 300.0));
    }

    #[test]
    #[expect(
        clippy::suboptimal_flops,
        reason = "NFR-4: plain * and + are bit-identical on every target; mul_add falls back to a libm fma where the CPU lacks one"
    )]
    fn dividers_follow_fr77() {
        let eight: Vec<f64> = (0..8_u32).map(|k| 45.0 * f64::from(k) - 90.0).collect();
        let cases: [(usize, &[f64]); 4] = [
            (2, &[-90.0, 90.0]),
            (3, &[-90.0, 30.0, 150.0]),
            (4, &[-90.0, 0.0, 90.0, 180.0]),
            (8, &eight),
        ];
        for (n, angles) in cases {
            ends_are(&lines(&frame_at(&quadrants(n), 1.0)), angles, 262.0);
        }
    }

    #[test]
    fn quadrant_labels_follow_fr10() {
        let check = |csv: &str| {
            let nodes = frame_at(&block(csv, None), 1.0);
            assert_eq!(label_texts(&nodes), ["platform", "Practice", "Tool"]);
            let g = Geometry {
                quadrants: 3,
                rings: 4,
            };
            for (i, label) in quadrant_labels(&nodes).iter().enumerate() {
                assert!(same(label.origin, polar(g.middle(i), 300.0)), "{i}");
            }
            assert_eq!(lines(&nodes).len(), 3);
        };
        check(&csv_of(&["Tool", "platform", "Practice"], "Adopt"));
        // FR-42: every row in an undeclared ring plots nothing, but the
        // quadrants still exist and are still drawn.
        check(&csv_of(&["Tool", "platform", "Practice"], "Nowhere"));
    }

    /// `count` declared rings of equal `band` width: circles at each outer
    /// edge, names at each band's middle, first declared outermost.
    fn rings_are(b: &RadarBlock, names: &[&str], band: f64) {
        let nodes = frame_at(b, 1.0);
        let got = circles(&nodes);
        let count = names.len();
        assert_eq!(got.len(), count);
        let outer = |i: usize| band * f64::from(u32::try_from(count - i).expect("small"));
        for (i, (radius, _)) in got.iter().enumerate() {
            assert!(close(*radius, outer(i)), "ring {i}: {radius}");
        }
        let drawn = ring_names(&nodes);
        assert_eq!(drawn.len(), count);
        for (i, (name, want)) in drawn.iter().zip(names).enumerate() {
            let y = -f64::midpoint(outer(i) - band, outer(i));
            assert!(same(name.origin, Point { x: 0.0, y }), "ring {i}");
            assert_eq!(name.lines, [*want]);
        }
    }

    #[test]
    fn every_declared_ring_draws() {
        let csv = csv_of(&["Tool"], "C");
        rings_are(
            &block(&csv, Some("A,B,C,D,E")),
            &["A", "B", "C", "D", "E"],
            52.0,
        );
        rings_are(
            &block(&csv_of(&["Tool"], "Only"), Some("Only")),
            &["Only"],
            260.0,
        );
        let eight = ["A", "B", "C", "D", "E", "F", "G", "H"];
        rings_are(&block(&csv, Some(&eight.join(","))), &eight, 32.5);
    }

    fn decorative(node: &Node) -> bool {
        match node {
            Node::Group(g) => {
                g.semantics == Semantics::Decorative || g.children.iter().any(decorative)
            }
            _ => false,
        }
    }

    #[test]
    fn ring_labels_sit_on_a_halo() {
        let b = block(PRD_EXAMPLE, None);
        let rendered = b.render(at(1.0));
        assert!(!decorative(&rendered), "no decorative group in the radar");
        let nodes = frame(&rendered);
        let mut names = 0;
        for (i, node) in nodes.iter().enumerate() {
            let Some(name) = text(node).filter(|t| t.ink == MUTED) else {
                continue;
            };
            names += 1;
            let halo = i
                .checked_sub(1)
                .and_then(|j| nodes.get(j))
                .and_then(text)
                .expect("a text precedes every ring name");
            assert!(same(halo.origin, name.origin));
            assert!(close(halo.style.size, name.style.size));
            assert!(close(halo.style.letter_spacing, name.style.letter_spacing));
            assert_eq!(halo.style.weight, name.style.weight);
            assert_eq!(halo.anchor, name.anchor);
            assert_eq!(halo.lines, name.lines);
            let TextPaint::Halo { stroke, width } = halo.ink else {
                panic!("expected a halo, got {:?}", halo.ink);
            };
            assert_eq!(stroke, Paint::Role(Role::Neutral(Neutral::Surface1)));
            assert!(close(width, LABEL_HALO), "{width}");
        }
        assert_eq!(names, 4);
        let last_line = nodes
            .iter()
            .rposition(|n| matches!(n, Node::Line { .. }))
            .expect("three quadrants draw dividers");
        let first_ring_text = nodes
            .iter()
            .position(|n| text(n).is_some_and(|t| t.style.weight == Weight::Regular))
            .expect("ring names are drawn");
        assert!(
            first_ring_text > last_line,
            "ring names paint over dividers"
        );
    }

    #[test]
    fn quadrant_labels_are_primary_and_middle_anchored() {
        let nodes = frame_at(&block(PRD_EXAMPLE, None), 1.0);
        let labels = quadrant_labels(&nodes);
        assert_eq!(labels.len(), 3);
        for label in labels {
            assert_eq!(label.ink, PRIMARY);
            assert_eq!(label.anchor, Anchor::Middle);
            assert_eq!(label.style, QUADRANT_LABEL);
            assert_eq!(label.lines.len(), 1);
        }
        for name in ring_names(&nodes) {
            assert_eq!(name.anchor, Anchor::Middle);
            assert_eq!(name.style, RING_LABEL);
        }
    }

    #[test]
    fn three_scales() {
        let b = quadrants(4);
        let g = Geometry {
            quadrants: 4,
            rings: 4,
        };
        for s in [1.0_f64, 0.9, 7.0 / 9.0, 0.7, 0.5, 0.4] {
            let nodes = frame_at(&b, s);
            let stroke = GRID_STROKE * s.max(7.0 / 9.0);
            let rings = circles(&nodes);
            assert_eq!(rings.len(), 4, "{s}");
            for (i, (radius, width)) in rings.into_iter().enumerate() {
                assert!(close(radius, g.band(i).outer * s), "{s} ring {i}");
                assert!(close(width, stroke), "{s} ring {i}: {width}");
            }
            let ends = lines(&nodes);
            assert_eq!(ends.len(), 4);
            for (p, (end, width)) in ends.into_iter().enumerate() {
                assert!(same(end, polar(g.divider(p), 262.0 * s)), "{s} divider {p}");
                assert!(close(width, stroke), "{s} divider {p}: {width}");
            }
            let labels = quadrant_labels(&nodes);
            assert_eq!(labels.len(), 4);
            for (p, label) in labels.iter().enumerate() {
                assert!(
                    same(label.origin, polar(g.middle(p), 300.0 * s)),
                    "{s} label {p}"
                );
                assert!(close(label.style.size, QUADRANT_LABEL.size));
                assert!(close(
                    label.style.letter_spacing,
                    QUADRANT_LABEL.letter_spacing
                ));
            }
            let texts = ring_texts(&nodes);
            assert_eq!(texts.len(), 8);
            let halos = texts
                .iter()
                .filter(|t| matches!(t.ink, TextPaint::Halo { .. }))
                .count();
            assert_eq!(halos, 4, "{s}: one halo per ring name");
            for (i, t) in texts.iter().enumerate() {
                let band = g.band(i / 2);
                let y = -f64::midpoint(band.inner, band.outer) * s;
                assert!(same(t.origin, Point { x: 0.0, y }), "{s} ring text {i}");
                assert!(close(t.style.size, RING_LABEL.size), "{s} ring text {i}");
                if let TextPaint::Halo { width, .. } = t.ink {
                    assert!(close(width, LABEL_HALO), "{s} halo {i}: {width}");
                }
            }
        }
    }

    #[test]
    #[expect(
        clippy::suboptimal_flops,
        reason = "NFR-4: plain * and + are bit-identical on every target; mul_add falls back to a libm fma where the CPU lacks one"
    )]
    fn size_pins() {
        let prd = block(PRD_EXAMPLE, None).size(1.0);
        assert!(
            close(prd.width, 660.0) && close(prd.height, 660.0),
            "{prd:?}"
        );
        let zero = block(HEADER, None).size(0.5);
        assert!(
            close(zero.width, 330.0) && close(zero.height, 330.0),
            "{zero:?}"
        );

        // N = 8: middle −67.5° puts a label above the centre, so its ascent
        // sets the height once the floor (198 at 0.3) is passed.
        let s = 0.3;
        let above = 300.0 * s * 67.5_f64.to_radians().sin();
        let want = 2.0 * (TEXT_ASCENT * 12.0 + above + 10.0);
        let got = quadrants(8).size(s).height;
        assert!(close(got, want), "{got} vs {want}");
        assert!(want > 660.0 * s && (want - 209.55).abs() < 0.01, "{want}");

        // N = 1: the label hangs below 6 o'clock, so its descent counts.
        let s = 0.4_f64;
        let want = (660.0 * s).max(2.0 * (TEXT_DESCENT * 12.0 + 300.0 * s + 10.0));
        let got = quadrants(1).size(s).height;
        assert!(close(got, want), "{got} vs {want}");
    }

    #[test]
    #[expect(
        clippy::suboptimal_flops,
        reason = "NFR-4: plain * and + are bit-identical on every target; mul_add falls back to a libm fma where the CPU lacks one"
    )]
    fn a_long_quadrant_label_widens_the_block() {
        let w = fonts().face(QUADRANT_LABEL.weight).measure(
            &long_quadrant(),
            QUADRANT_LABEL.size,
            QUADRANT_LABEL.letter_spacing,
        );
        let b = long_quadrant_block();
        for s in [1.0, 0.576] {
            let want = 2.0 * (polar(45.0, 300.0).x.abs() * s + w / 2.0 + 10.0);
            let got = b.size(s).width;
            assert!(close(got, want) && got > 660.0, "{s}: {got} vs {want}");
        }
    }

    #[test]
    fn a_long_ring_name_widens_the_block() {
        let w = fonts().face(RING_LABEL.weight).measure(
            &long_ring(),
            RING_LABEL.size,
            RING_LABEL.letter_spacing,
        );
        let want = 2.0 * (w / 2.0 + 1.5 + 10.0);
        let got = long_ring_block().size(1.0).width;
        assert!(close(got, want) && got > 660.0, "{got} vs {want}");
    }

    #[test]
    #[expect(
        clippy::suboptimal_flops,
        reason = "NFR-4: plain * and + are bit-identical on every target; mul_add falls back to a libm fma where the CPU lacks one"
    )]
    fn size_is_non_decreasing() {
        let steps: Vec<f64> = (0..=14_u32).map(|k| 0.05 * f64::from(k) + 0.30).collect();
        for (name, b) in fixtures() {
            for pair in steps.windows(2) {
                let [lo, hi] = pair else {
                    panic!("windows of two");
                };
                let (a, z) = (b.size(*lo), b.size(*hi));
                assert!(
                    z.width > a.width || close(z.width, a.width),
                    "{name}: width {lo} → {hi}"
                );
                assert!(
                    z.height > a.height || close(z.height, a.height),
                    "{name}: height {lo} → {hi}"
                );
            }
        }
        let b = long_single();
        let (low, high) = (b.size(0.4).width, b.size(0.7).width);
        assert!(close(low, high), "the plateau: {low} vs {high}");
    }

    #[test]
    fn everything_drawn_lies_inside_the_size_and_touches_it() {
        let fonts = fonts();
        let mut dominated = 0;
        for (name, b) in fixtures() {
            for s in [1.5, 1.0, 0.576, 0.4] {
                let size = b.size(s);
                let rendered = b.render(at(s));
                assert_inside(&rendered, size, &fonts);
                let drawn = bounds(&rendered, &fonts).expect("the frame paints");
                let r = RadarScale::of(s);
                let pad = RADAR_LABEL_PAD * r.labels;
                let floor = 330.0 * s;
                if b.label_half_width(r) + pad > floor {
                    dominated += 1;
                    let gap = drawn.left.min(size.width - drawn.right);
                    assert!((gap - pad).abs() < TIGHT, "{name} at {s}: x gap {gap}");
                }
                if b.label_half_height(r) + pad > floor {
                    dominated += 1;
                    let gap = drawn.top.min(size.height - drawn.bottom);
                    assert!((gap - pad).abs() < TIGHT, "{name} at {s}: y gap {gap}");
                }
            }
        }
        assert!(dominated > 0, "some fixture's labels set its size");
    }

    #[test]
    fn names_are_carried_raw() {
        let csv = format!("{HEADER}x,\"A<&\"\">B\",Adopt,d\ny,技术,Adopt,d\n");
        let labels = label_texts(&frame_at(&block(&csv, None), 1.0));
        assert!(labels.iter().any(|l| l == "A<&\">B"), "{labels:?}");
        assert!(labels.iter().any(|l| l == "技术"), "{labels:?}");
    }

    #[test]
    fn a_placed_radar_is_the_origin_radar_translated() {
        let b = block(PRD_EXAMPLE, None);
        assert_translates(&b);
        assert_translates_at(&b, 0.5);
    }
}
