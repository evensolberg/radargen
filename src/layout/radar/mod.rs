//! The radar block (FR-30–FR-36, FR-40, FR-42, FR-43, FR-48, FR-50, FR-70,
//! FR-94): ring circles, quadrant dividers, quadrant labels and ring names,
//! then every plotted item's marker.
//!
//! The radar paints four layers, in order (architecture §5): the **frame**;
//! the **hit layer**, every marker's invisible pointer target, below every
//! marker so a visible marker always takes its own pointer events; one
//! **control** per marker, in FR-34's order, which is reading and tab order;
//! and the **indicator layer**, each marker's focus indicator over a Surface1
//! backing, above every marker so a focused marker is never covered. Without
//! markers there is neither a hit nor an indicator layer.
//!
//! FR-70 splits the radar's scale three ways: positions shrink with it,
//! labels and hit areas never drop below their natural size, and strokes,
//! markers and their numbers stop at [`RADAR_FLOOR`]. So the block does not
//! scale linearly, and its size is computed per scale from what it draws.

mod markers;

use crate::layout::block::{Block, Placement, Size};
use crate::layout::geometry::{Geometry, polar};
use crate::model::Catalogue;
use crate::order::Plotted;
use crate::palette::{Neutral, Role};
use crate::style::{
    BAND_OUTER_RADIUS, DIVIDER_RADIUS, GRID_STROKE, HIT_RADIUS, INDICATOR_REACH, LABEL_HALO,
    MIN_RADAR_WIDTH, QUADRANT_LABEL, QUADRANT_LABEL_RADIUS, RADAR_FLOOR, RADAR_LABEL_PAD,
    RING_LABEL, TextStyle,
};
use crate::svg::{Anchor, Group, Node, Paint, Point, Scale, Semantics, TextRun};
use crate::text::{Fonts, TEXT_ASCENT, TEXT_DESCENT};
use markers::{Marker, hit_node, hit_reach, indicator_node, layer, marker_nodes, markers};

/// Half the minimum radar width: the floor on each half of the block.
const MIN_RADAR_HALF: f64 = MIN_RADAR_WIDTH / 2.0;
/// How far a ring name's halo reaches past its glyphs.
const HALO_REACH: f64 = LABEL_HALO / 2.0;

/// FR-70's three scales for one radar scale `s`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RadarScale {
    /// For positions and radii: `s`.
    pub geometry: f64,
    /// For label text and hit areas: `max(s, 1)`, so neither shrinks below
    /// natural size.
    pub labels: f64,
    /// For strokes, markers, their numbers and focus indicators:
    /// `max(s, RADAR_FLOOR)` (D-69).
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

    /// Where markers are placed and how large they are drawn.
    const fn marker(self) -> Scale {
        Scale {
            position: self.geometry,
            size: self.marks,
        }
    }
}

/// A label's text, its width at natural size and where it is anchored at
/// scale 1.
#[derive(Debug, Clone, PartialEq)]
struct Label {
    text: String,
    width: f64,
    at: Point,
}

impl Label {
    fn measured(text: &str, style: TextStyle, at: Point, fonts: &Fonts) -> Self {
        let width = fonts
            .face(style.weight)
            .measure(text, style.size, style.letter_spacing);
        Self {
            text: text.to_owned(),
            width,
            at,
        }
    }
}

/// A quadrant's name and how far FR-79 pushes it outward (D-70).
#[derive(Debug, Clone, PartialEq)]
struct QuadrantLabel {
    /// Anchored at the label point, on the sector's middle angle.
    label: Label,
    /// Along the middle angle, at natural size: half the name's width times
    /// `|cos|` of the angle, which puts a middle-anchored name's inner end on
    /// the label circle's tangent at the label point: on the circle at 3 and
    /// 9 o'clock, beyond it elsewhere, at `√(R² + h² sin² θ)` for a drawn
    /// half-width `h`. Nothing at 12 and 6 o'clock, half the width at 3 and 9.
    ///
    /// So the name's box, which reaches its ascent (11.625) above the
    /// baseline, comes no closer to the centre than `300 s − 11.625` for
    /// `s <= 1`, which clears the outer ring (`260 s` plus half a stroke)
    /// from `s` of about 0.3 up. Below that a 6 o'clock name touches the
    /// ring's stroke whatever its length, unpushed.
    push: Point,
}

impl QuadrantLabel {
    fn new(label: Label, middle: f64) -> Self {
        let push = polar(
            middle,
            f64::midpoint(0.0, label.width) * middle.to_radians().cos().abs(),
        );
        Self { label, push }
    }

    /// Where the name's middle sits at `r`. The label point moves with the
    /// geometry and the push with the text, so the inner end stays on the
    /// label circle's tangent at the label point at any scale.
    fn centre(&self, r: RadarScale) -> Point {
        let at = self.label.at.scaled(r.geometry);
        let push = self.push.scaled(r.labels);
        Point {
            x: at.x + push.x,
            y: at.y + push.y,
        }
    }
}

/// The radar (FR-30–FR-36): its rings, dividers, labels and markers.
#[derive(Debug, Clone, PartialEq)]
pub struct RadarBlock {
    geometry: Geometry,
    /// In FR-10 order: label `p` belongs to the sector at position `p`.
    quadrants: Vec<QuadrantLabel>,
    /// In declared order, outermost first (D-60).
    rings: Vec<Label>,
    /// In FR-34's order.
    markers: Vec<Marker>,
}

impl RadarBlock {
    /// The radar for `catalogue`, its quadrants in `plotted`'s FR-10 order,
    /// every declared ring drawn, empty or not (FR-19), and a marker for
    /// every plotted item. Labels are measured once here, at natural size.
    ///
    /// # Panics
    ///
    /// If `plotted` names a quadrant or item `catalogue` lacks
    /// (`order::resolve` guarantees it does not).
    #[must_use]
    // qual:allow(complexity, error_handling) reason: "the documented # Panics: a Plotted from another catalogue is a caller bug"
    pub fn new(catalogue: &Catalogue, plotted: &Plotted, fonts: &Fonts) -> Self {
        let geometry = Geometry::of(catalogue);
        let quadrants = plotted
            .quadrants
            .iter()
            .enumerate()
            .map(|(p, &q)| {
                let quadrant = catalogue
                    .quadrants
                    .get(q)
                    .expect("FR-10's order indexes the catalogue's quadrants");
                let middle = geometry.middle(p);
                let at = polar(middle, QUADRANT_LABEL_RADIUS);
                let label = Label::measured(&quadrant.label, QUADRANT_LABEL, at, fonts);
                QuadrantLabel::new(label, middle)
            })
            .collect();
        // A ring's name sits at 12 o'clock, in the middle of its band.
        let rings = catalogue
            .rings
            .iter()
            .enumerate()
            .map(|(i, ring)| {
                let band = geometry.band(i);
                let at = Point {
                    x: 0.0,
                    y: -f64::midpoint(band.inner, band.outer),
                };
                Label::measured(ring, RING_LABEL, at, fonts)
            })
            .collect();
        Self {
            geometry,
            quadrants,
            rings,
            markers: markers(catalogue, plotted),
        }
    }

    /// The furthest any label reaches left or right of the centre.
    fn label_half_width(&self, r: RadarScale) -> f64 {
        let quadrants = self
            .quadrants
            .iter()
            .map(|label| quadrant_reach_x(label, r));
        let rings = self.rings.iter().map(|label| ring_reach_x(label.width, r));
        quadrants.chain(rings).fold(0.0, f64::max)
    }

    /// The furthest any label reaches above or below the centre.
    fn label_half_height(&self, r: RadarScale) -> f64 {
        let quadrants = self
            .quadrants
            .iter()
            .map(|label| quadrant_reach_y(label, r));
        let rings = self.rings.iter().map(|label| ring_reach_y(-label.at.y, r));
        quadrants.chain(rings).fold(0.0, f64::max)
    }

    /// The furthest any hit area reaches left or right of the centre; 0
    /// without markers.
    fn hit_half_width(&self, r: RadarScale) -> f64 {
        self.markers
            .iter()
            .map(|m| hit_reach(m.at.x, r))
            .fold(0.0, f64::max)
    }

    /// The furthest any hit area reaches above or below the centre; 0
    /// without markers.
    fn hit_half_height(&self, r: RadarScale) -> f64 {
        self.markers
            .iter()
            .map(|m| hit_reach(m.at.y, r))
            .fold(0.0, f64::max)
    }

    /// The radar's centre in the block: half its size on each axis.
    fn centre(&self, r: RadarScale) -> Point {
        Point {
            x: half_extent(self.label_half_width(r), self.hit_half_width(r), r),
            y: half_extent(self.label_half_height(r), self.hit_half_height(r), r),
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

    /// Each quadrant's name, mid-sector beyond the rim, pushed out by its
    /// own half-width (D-70). The centre is already at `r`, so only the size
    /// scales.
    fn quadrant_labels(&self, r: RadarScale) -> impl Iterator<Item = Node> + '_ {
        let scale = Scale {
            position: 1.0,
            size: r.labels,
        };
        self.quadrants.iter().map(move |quadrant| {
            TextRun {
                origin: quadrant.centre(r),
                style: QUADRANT_LABEL,
                ink: Neutral::TextPrimary,
                anchor: Anchor::Middle,
                lines: vec![quadrant.label.text.clone()],
                leading: 0.0,
            }
            .node(scale)
        })
    }

    /// Each ring's name at 12 o'clock, outermost first.
    fn ring_runs(&self) -> impl Iterator<Item = TextRun> + '_ {
        self.rings.iter().map(|label| TextRun {
            origin: label.at,
            style: RING_LABEL,
            ink: Neutral::TextMuted,
            anchor: Anchor::Middle,
            lines: vec![label.text.clone()],
            leading: 0.0,
        })
    }

    /// Every ring's name over a halo, so the divider there (FR-31) stops
    /// short of it. Every halo is drawn before any name: at many thin bands
    /// a halo drawn after the previous name would erase its descenders
    /// (D-70).
    fn ring_labels(&self, r: RadarScale) -> impl Iterator<Item = Node> + '_ {
        let halos = self
            .ring_runs()
            .map(move |run| run.halo(LABEL_HALO, r.text()));
        let names = self.ring_runs().map(move |run| run.node(r.text()));
        halos.chain(names)
    }
}

impl Block for RadarBlock {
    // qual:allow(complexity, magic_numbers) reason: "a box centred on its centre spans twice the centre offset"
    fn size(&self, scale: f64) -> Size {
        let centre = self.centre(RadarScale::of(scale));
        Size {
            width: 2.0 * centre.x,
            height: 2.0 * centre.y,
        }
    }

    fn render(&self, at: Placement) -> Node {
        let r = RadarScale::of(at.scale);
        let frame = self
            .ring_circles(r)
            .chain(self.dividers(r))
            .chain(self.quadrant_labels(r))
            .chain(self.ring_labels(r));
        // Each marker's glyph and number, drawn once: the indicator raises
        // a copy and the control takes the originals.
        let drawn: Vec<Vec<Node>> = self.markers.iter().map(|m| marker_nodes(m, r)).collect();
        let hits = layer(self.markers.iter().map(|m| hit_node(m, r)).collect());
        let indicators = layer(
            self.markers
                .iter()
                .zip(drawn.iter().cloned())
                .map(|(m, nodes)| indicator_node(m, nodes, r))
                .collect(),
        );
        let controls = self.markers.iter().zip(drawn).map(|(m, nodes)| {
            Node::Group(Group::new(Point::ORIGIN, nodes).with(Semantics::Marker(m.control.clone())))
        });
        let children = frame
            .chain(hits)
            .chain(controls)
            .chain(indicators)
            .collect();
        let centred = Node::Group(Group::new(self.centre(r), children));
        Node::Group(Group::new(at.origin, vec![centred]))
    }
}

/// How far a quadrant label reaches left or right of the centre: its pushed
/// centre's distance, then half its middle-anchored width.
#[expect(
    clippy::suboptimal_flops,
    reason = "NFR-4: plain * and + are bit-identical on every target; mul_add falls back to a libm fma where the CPU lacks one"
)]
fn quadrant_reach_x(quadrant: &QuadrantLabel, r: RadarScale) -> f64 {
    quadrant.centre(r).x.abs() + f64::midpoint(0.0, quadrant.label.width) * r.labels
}

/// How far a quadrant label reaches above or below the centre: the descent
/// below its pushed baseline, or the ascent above it.
#[expect(
    clippy::suboptimal_flops,
    reason = "NFR-4: plain * and + are bit-identical on every target; mul_add falls back to a libm fma where the CPU lacks one"
)]
fn quadrant_reach_y(quadrant: &QuadrantLabel, r: RadarScale) -> f64 {
    let baseline = quadrant.centre(r).y;
    let size = QUADRANT_LABEL.size * r.labels;
    (baseline + TEXT_DESCENT * size).max(TEXT_ASCENT * size - baseline)
}

/// How far a ring name of `width`, centred on the vertical axis, reaches
/// either side, its halo included.
fn ring_reach_x(width: f64, r: RadarScale) -> f64 {
    (f64::midpoint(0.0, width) + HALO_REACH) * r.labels
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
/// the labels' reach plus the pad, or the hit areas' reach, whichever is
/// largest. Hit areas are invisible, so they take no pad.
///
/// The `MIN_RADAR_HALF * s` floor is also what keeps the ring circles (at
/// most `260 * s` plus half a stroke) and the dividers (`262 * s` plus half
/// a stroke) inside the block: the label terms never count them. That holds
/// for `s` of about 0.0056 and up, where `70 * s` covers half a floored
/// stroke.
///
/// Markers and their indicators need no term of their own: an indicator
/// reaches furthest of everything a marker draws (`INDICATOR_REACH`, 19, at
/// `marks`, past the status ring's 15 and any number under six digits), and
/// it never passes `max(floor, hit reach)`. For a marker `|p|` from the
/// centre on an axis: below `RADAR_FLOOR` it reaches `|p| s + 19 · 7/9`,
/// under the hit area's `|p| s + HIT_RADIUS` because `INDICATOR_REACH * RADAR_FLOOR
/// <= HIT_RADIUS`; from `RADAR_FLOOR` up it reaches `(|p| + 19) s`, under
/// the floor's `330 s` because `|p| < BAND_OUTER_RADIUS` and
/// `BAND_OUTER_RADIUS + INDICATOR_REACH <= MIN_RADAR_HALF`. Both are
/// asserted at compile time, below `half_extent`. The hit term itself binds only
/// for `s < HIT_RADIUS / (330 - |p|)`, about 0.26 at most.
///
/// The block's size is non-decreasing in `s` from 0.0034 up. A quadrant
/// label's pushed centre is `300 s` plus a push at `max(s, 1)` along the
/// same direction, so its distance from the centre on each axis never
/// falls. The only decreasing term is a below-centre quadrant label's
/// ascent, at most 11.625, since its push only lowers the baseline; the
/// outermost ring name's reach, at least `130 * s + 11.19`,
/// outgrows it from there. That relies on at least one ring, which
/// `rings.rs` guarantees by rejecting an empty list. The hit term, `|p| s +
/// 16 · max(s, 1)`, is increasing.
#[expect(
    clippy::suboptimal_flops,
    reason = "NFR-4: plain * and + are bit-identical on every target; mul_add falls back to a libm fma where the CPU lacks one"
)]
fn half_extent(label_reach: f64, hit_reach: f64, r: RadarScale) -> f64 {
    (MIN_RADAR_HALF * r.geometry)
        .max(label_reach + RADAR_LABEL_PAD * r.labels)
        .max(hit_reach)
}

// `half_extent`'s indicator bounds, checked in every build.
const _: () = assert!(
    INDICATOR_REACH * RADAR_FLOOR <= HIT_RADIUS,
    "below the floor scale an indicator must stay inside the hit area"
);
const _: () = assert!(
    BAND_OUTER_RADIUS + INDICATOR_REACH <= MIN_RADAR_HALF,
    "above the floor scale an indicator must stay inside the floor"
);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::block::{assert_translates, at, close, fonts};
    use crate::layout::geometry::place;
    use crate::layout::test_support::{
        PRD_EXAMPLE, Rect, assert_finite, bounds, fill, near, plotted, plus,
    };
    use crate::svg::TextPaint;
    use crate::text::Weight;

    pub(super) const HEADER: &str = "name,quadrant,ring,description\n";
    /// How near the block's edge the drawn labels must come: float noise.
    const TIGHT: f64 = 1e-6;

    /// One row per quadrant name, all in `ring`.
    pub(super) fn csv_of(quadrants: &[&str], ring: &str) -> String {
        quadrants
            .iter()
            .enumerate()
            .fold(HEADER.to_owned(), |text, (i, q)| {
                format!("{text}item {i},{q},{ring},d\n")
            })
    }

    pub(super) fn block(fonts: &Fonts, csv: &str, rings: Option<&str>) -> RadarBlock {
        let (catalogue, order) = plotted(csv, rings);
        RadarBlock::new(&catalogue, &order, fonts)
    }

    /// `n` short quadrants, `Q0`… in FR-10 order, default rings.
    pub(super) fn quadrants(fonts: &Fonts, n: usize) -> RadarBlock {
        let names: Vec<String> = (0..n).map(|i| format!("Q{i}")).collect();
        let names: Vec<&str> = names.iter().map(String::as_str).collect();
        block(fonts, &csv_of(&names, "Adopt"), None)
    }

    /// A name of `n` × "W", the widest glyph.
    fn long(n: usize) -> String {
        "W".repeat(n)
    }

    /// Three `Q0` rows in Assess, the outermost default band (N = 1).
    fn assess(fonts: &Fonts) -> RadarBlock {
        block(fonts, &csv_of(&["Q0", "Q0", "Q0"], "Assess"), None)
    }

    /// 30 rows in one band.
    pub(super) fn crowded(fonts: &Fonts) -> RadarBlock {
        block(fonts, &csv_of(&["Q0"; 30], "Adopt"), None)
    }

    pub(super) const MARKED_HEADER: &str = "name,quadrant,ring,description,qualifier,status\n";
    const QUALIFIERS: [&str; 2] = ["", "conditional"];
    const STATUSES: [&str; 5] = ["", "No Change", "New", "Moved In", "Moved Out"];

    /// N = 4, each quadrant holding every qualifier × status pair in Trial.
    pub(super) fn marked_csv() -> String {
        let rows = ["Q0", "Q1", "Q2", "Q3"].into_iter().flat_map(|q| {
            QUALIFIERS.into_iter().flat_map(move |qualifier| {
                STATUSES.into_iter().map(move |status| {
                    format!("{q}/{qualifier}/{status},{q},Trial,d,{qualifier},{status}\n")
                })
            })
        });
        std::iter::once(MARKED_HEADER.to_owned())
            .chain(rows)
            .collect()
    }

    fn fixtures(fonts: &Fonts) -> Vec<(&'static str, RadarBlock)> {
        // N = 4 with one quadrant named 40 × "W"; no quadrants and one ring
        // named 80 × "W".
        let long_quadrant = csv_of(&["B", "C", "D", &long(40)], "Adopt");
        vec![
            ("prd", block(fonts, PRD_EXAMPLE, None)),
            ("n1", quadrants(fonts, 1)),
            ("n2", quadrants(fonts, 2)),
            ("n6", quadrants(fonts, 6)),
            ("n8", quadrants(fonts, 8)),
            ("zero", block(fonts, HEADER, None)),
            ("long quadrant", block(fonts, &long_quadrant, None)),
            ("long ring", block(fonts, HEADER, Some(&long(80)))),
            ("assess", assess(fonts)),
            ("crowded", crowded(fonts)),
            ("marked", block(fonts, &marked_csv(), None)),
        ]
    }

    /// The centring group's children, inside the root: the frame, then the
    /// hit layer, the marker controls and the indicator layer.
    pub(super) fn frame(node: &Node) -> Vec<Node> {
        let Node::Group(root) = node else {
            panic!("the radar renders a group, got {node:?}");
        };
        let [Node::Group(centred)] = root.children.as_slice() else {
            panic!("the root holds one centring group: {:?}", root.children);
        };
        centred.children.clone()
    }

    pub(super) fn frame_at(b: &RadarBlock, s: f64) -> Vec<Node> {
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
    pub(super) struct Text<'a> {
        pub(super) origin: Point,
        pub(super) style: TextStyle,
        pub(super) ink: TextPaint,
        pub(super) anchor: Anchor,
        pub(super) lines: &'a [String],
    }

    pub(super) fn text(node: &Node) -> Option<Text<'_>> {
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
            .filter(|t| t.ink == fill(Neutral::TextMuted))
            .collect()
    }

    /// How many of `texts` are halos.
    fn halos(texts: &[Text]) -> usize {
        texts
            .iter()
            .filter(|t| matches!(t.ink, TextPaint::Halo { .. }))
            .count()
    }

    fn ends_are(got: &[(Point, f64)], angles: &[f64], radius: f64) {
        assert_eq!(got.len(), angles.len(), "{got:?} vs {angles:?}");
        for ((end, _), angle) in got.iter().zip(angles) {
            assert!(near(*end, polar(*angle, radius)), "{end:?} vs {angle}°");
        }
    }

    /// A quadrant name's width at natural size.
    fn quadrant_width(fonts: &Fonts, name: &str) -> f64 {
        fonts.face(QUADRANT_LABEL.weight).measure(
            name,
            QUADRANT_LABEL.size,
            QUADRANT_LABEL.letter_spacing,
        )
    }

    /// FR-79's centre for a name of `width` on `middle` at scale `s`, from
    /// its parts: the label point at `s`, plus the push of half the width
    /// times `|cos|` along the same angle at `max(s, 1)` (D-70).
    fn pushed(middle: f64, width: f64, s: f64) -> Point {
        plus(
            polar(middle, 300.0 * s),
            polar(
                middle,
                width / 2.0 * middle.to_radians().cos().abs() * s.max(1.0),
            ),
        )
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
        let b = block(fonts(), HEADER, None);
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
        assert_eq!(halos(&texts), 4);
        assert_eq!(ring_names(&nodes).len(), 4);
        assert_eq!(nodes.len(), 12, "nothing else: {nodes:?}");
        assert!(
            !nodes.iter().any(|n| matches!(n, Node::Group(_))),
            "no hit or indicator layer without markers"
        );
    }

    #[test]
    fn one_quadrant_has_one_divider() {
        let nodes = frame_at(&quadrants(fonts(), 1), 1.0);
        assert_eq!(lines(&nodes).len(), 1);
        let labels = quadrant_labels(&nodes);
        let [label] = labels.as_slice() else {
            panic!("one quadrant label");
        };
        assert!(near(label.origin, polar(90.0, 300.0)), "{:?}", label.origin);
        assert!(close(label.origin.x, 0.0) && close(label.origin.y, 300.0));
    }

    #[test]
    #[expect(
        clippy::suboptimal_flops,
        reason = "NFR-4: plain * and + are bit-identical on every target; mul_add falls back to a libm fma where the CPU lacks one"
    )]
    fn dividers_follow_fr77() {
        let eight: Vec<f64> = (0..8_u32).map(|k| 45.0 * f64::from(k) - 90.0).collect();
        let fonts = fonts();
        let cases: [(usize, &[f64]); 5] = [
            (1, &[-90.0]),
            (2, &[-90.0, 90.0]),
            (3, &[-90.0, 30.0, 150.0]),
            (4, &[-90.0, 0.0, 90.0, 180.0]),
            (8, &eight),
        ];
        for (n, angles) in cases {
            ends_are(&lines(&frame_at(&quadrants(fonts, n), 1.0)), angles, 262.0);
        }
    }

    #[test]
    fn quadrant_labels_follow_fr10() {
        let fonts = fonts();
        let check = |csv: &str| {
            let nodes = frame_at(&block(fonts, csv, None), 1.0);
            assert_eq!(label_texts(&nodes), ["platform", "Practice", "Tool"]);
            let g = Geometry {
                quadrants: 3,
                rings: 4,
            };
            for (i, label) in quadrant_labels(&nodes).iter().enumerate() {
                let width = quadrant_width(fonts, &label.lines.concat());
                assert!(near(label.origin, pushed(g.middle(i), width, 1.0)), "{i}");
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
            assert!(near(name.origin, Point { x: 0.0, y }), "ring {i}");
            assert_eq!(name.lines, [*want]);
        }
    }

    #[test]
    fn every_declared_ring_draws() {
        let fonts = fonts();
        let csv = csv_of(&["Tool"], "C");
        rings_are(
            &block(fonts, &csv, Some("A,B,C,D,E")),
            &["A", "B", "C", "D", "E"],
            52.0,
        );
        rings_are(
            &block(fonts, &csv_of(&["Tool"], "Only"), Some("Only")),
            &["Only"],
            260.0,
        );
        let eight = ["A", "B", "C", "D", "E", "F", "G", "H"];
        rings_are(&block(fonts, &csv, Some(&eight.join(","))), &eight, 32.5);
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
        let b = block(fonts(), PRD_EXAMPLE, None);
        let rendered = b.render(at(1.0));
        assert!(!decorative(&rendered), "no decorative group in the radar");
        let nodes = frame(&rendered);
        let texts = ring_texts(&nodes);
        let (halos, names) = texts.split_at(texts.len() / 2);
        assert_eq!(names.len(), 4);
        for (halo, name) in halos.iter().zip(names) {
            assert_eq!(name.ink, fill(Neutral::TextMuted));
            assert!(near(halo.origin, name.origin));
            assert_eq!(halo.anchor, name.anchor);
            assert_eq!(halo.lines, name.lines);
            assert_eq!(halo.style.weight, name.style.weight);
            assert!(close(halo.style.size, name.style.size));
            assert!(close(halo.style.letter_spacing, name.style.letter_spacing));
            let TextPaint::Halo { stroke, width } = halo.ink else {
                panic!("expected a halo, got {:?}", halo.ink);
            };
            assert_eq!(stroke, Paint::Role(Role::Neutral(Neutral::Surface1)));
            assert!(close(width, LABEL_HALO), "{width}");
        }
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
        let nodes = frame_at(&block(fonts(), PRD_EXAMPLE, None), 1.0);
        let labels = quadrant_labels(&nodes);
        assert_eq!(labels.len(), 3);
        for label in labels {
            assert_eq!(label.ink, fill(Neutral::TextPrimary));
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
        let fonts = fonts();
        let b = quadrants(fonts, 4);
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
                assert!(near(end, polar(g.divider(p), 262.0 * s)), "{s} divider {p}");
                assert!(close(width, stroke), "{s} divider {p}: {width}");
            }
            let labels = quadrant_labels(&nodes);
            assert_eq!(labels.len(), 4);
            for (p, label) in labels.iter().enumerate() {
                let width = quadrant_width(fonts, &label.lines.concat());
                assert!(
                    near(label.origin, pushed(g.middle(p), width, s)),
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
            assert_eq!(halos(&texts), 4, "{s}: one halo per ring name");
            // Four halos, then four names, each outermost first.
            for (i, t) in texts.iter().enumerate() {
                let band = g.band(i % 4);
                let y = -f64::midpoint(band.inner, band.outer) * s;
                assert!(near(t.origin, Point { x: 0.0, y }), "{s} ring text {i}");
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
        let fonts = fonts();
        let prd = block(fonts, PRD_EXAMPLE, None).size(1.0);
        assert!(
            close(prd.width, 660.0) && close(prd.height, 660.0),
            "{prd:?}"
        );
        let zero = block(fonts, HEADER, None).size(0.5);
        assert!(
            close(zero.width, 330.0) && close(zero.height, 330.0),
            "{zero:?}"
        );

        // N = 8: middles −67.5° (Q0) and 247.5° (Q7) put labels above the
        // centre, so the higher one's ascent sets the height once the floor
        // (198 at 0.3) is passed. The push raises it by the wider name's
        // half-width × cos 67.5° × sin 67.5°, at natural size.
        let s = 0.3;
        let width = quadrant_width(fonts, "Q0").max(quadrant_width(fonts, "Q7"));
        let above = -pushed(-67.5, width, s).y;
        let want = 2.0 * (TEXT_ASCENT * 12.0 + above + 10.0);
        let got = quadrants(fonts, 8).size(s).height;
        assert!(close(got, want), "{got} vs {want}");
        assert!(want > 660.0 * s && (want - 215.80).abs() < 0.01, "{want}");

        // N = 1: the label hangs below 6 o'clock, so its descent counts.
        let s = 0.4_f64;
        let want = (660.0 * s).max(2.0 * (TEXT_DESCENT * 12.0 + 300.0 * s + 10.0));
        let got = quadrants(fonts, 1).size(s).height;
        assert!(close(got, want), "{got} vs {want}");
    }

    #[test]
    #[expect(
        clippy::suboptimal_flops,
        reason = "NFR-4: plain * and + are bit-identical on every target; mul_add falls back to a libm fma where the CPU lacks one"
    )]
    fn a_long_quadrant_label_widens_the_block() {
        let fonts = fonts();
        let w = quadrant_width(fonts, &long(40));
        let b = block(fonts, &csv_of(&["B", "C", "D", &long(40)], "Adopt"), None);
        // The long name is last, on 225°: its push reaches w/2 · cos² 45°,
        // a quarter of its width, further out horizontally.
        for s in [1.0, 0.576] {
            let want = 2.0 * (polar(45.0, 300.0).x.abs() * s + w / 4.0 + w / 2.0 + 10.0);
            let got = b.size(s).width;
            assert!(close(got, want) && got > 660.0, "{s}: {got} vs {want}");
        }
    }

    #[test]
    fn a_long_ring_name_widens_the_block() {
        let fonts = fonts();
        let w = fonts.face(RING_LABEL.weight).measure(
            &long(80),
            RING_LABEL.size,
            RING_LABEL.letter_spacing,
        );
        let want = 2.0 * (w / 2.0 + 1.5 + 10.0);
        let got = block(fonts, HEADER, Some(&long(80))).size(1.0).width;
        assert!(close(got, want) && got > 660.0, "{got} vs {want}");
    }

    #[test]
    #[expect(
        clippy::suboptimal_flops,
        reason = "NFR-4: plain * and + are bit-identical on every target; mul_add falls back to a libm fma where the CPU lacks one"
    )]
    fn size_is_non_decreasing() {
        let fonts = fonts();
        // 0.02–0.30 in steps of 0.02, where the hit term overtakes the floor
        // (near 0.154 for the Assess fixture), then 0.30–1.00 in 0.05.
        let low = (1..=15_u32).map(|k| 0.02 * f64::from(k));
        let high = (1..=14_u32).map(|k| 0.05 * f64::from(k) + 0.30);
        let steps: Vec<f64> = low.chain(high).collect();
        let ge = |z: f64, a: f64| z > a || close(z, a);
        for (name, b) in fixtures(fonts) {
            for (lo, hi) in steps.iter().zip(steps.iter().skip(1)) {
                let (a, z) = (b.size(*lo), b.size(*hi));
                assert!(ge(z.width, a.width), "{name}: width {lo} → {hi}");
                assert!(ge(z.height, a.height), "{name}: height {lo} → {hi}");
            }
        }
        // N = 1 with a long name: its label is at 6 o'clock, so the width is
        // the label's alone below scale 1.
        let b = block(fonts, &csv_of(&[&long(40)], "Adopt"), None);
        let (low, high) = (b.size(0.4).width, b.size(0.7).width);
        assert!(close(low, high), "the plateau: {low} vs {high}");
    }

    /// How far any indicator reaches along `axis` (0 for x, 1 for y): what
    /// `half_extent`'s proof says never sets the size.
    #[expect(
        clippy::suboptimal_flops,
        reason = "NFR-4: plain * and + are bit-identical on every target; mul_add falls back to a libm fma where the CPU lacks one"
    )]
    fn indicator_reach(b: &RadarBlock, r: RadarScale, axis: fn(Point) -> f64) -> f64 {
        b.markers
            .iter()
            .map(|m| axis(m.at).abs() * r.geometry + INDICATOR_REACH * r.marks)
            .fold(0.0, f64::max)
    }

    #[test]
    #[expect(
        clippy::suboptimal_flops,
        reason = "NFR-4: plain * and + are bit-identical on every target; mul_add falls back to a libm fma where the CPU lacks one"
    )]
    fn everything_drawn_lies_inside_the_size_and_touches_it() {
        let fonts = fonts();
        let (mut labelled, mut hit) = (0, 0);
        for (name, b) in fixtures(fonts) {
            for s in [1.5, 1.0, 0.576, 0.4, 0.1] {
                let size = b.size(s);
                let drawn = bounds(&b.render(at(s)), fonts).expect("the frame paints");
                assert!(
                    drawn.fits(size),
                    "{name} at {s}: {drawn:?} outside {size:?}"
                );
                let r = RadarScale::of(s);
                let pad = RADAR_LABEL_PAD * r.labels;
                let floor = MIN_RADAR_HALF * s;
                // How far the rings and dividers can reach: below about
                // s = 0.15 they can pass the labels without passing the floor.
                let grid = DIVIDER_RADIUS * s + f64::midpoint(0.0, GRID_STROKE * r.marks);
                // Per axis: the labels' reach, the hit areas' and the
                // indicators', and the drawing's nearest gap to the block's
                // edge. Where the labels set the size and reach past the
                // grid and every marker, the gap is the pad; where the hit
                // areas set it, a hit area's edge touches the block's.
                let axes = [
                    (
                        "x",
                        b.label_half_width(r),
                        b.hit_half_width(r),
                        indicator_reach(&b, r, |p| p.x),
                        drawn.left.min(size.width - drawn.right),
                    ),
                    (
                        "y",
                        b.label_half_height(r),
                        b.hit_half_height(r),
                        indicator_reach(&b, r, |p| p.y),
                        drawn.top.min(size.height - drawn.bottom),
                    ),
                ];
                for (axis, label, hits, indicators, gap) in axes {
                    if label + pad > floor && label > grid.max(hits).max(indicators) {
                        labelled += 1;
                        assert!((gap - pad).abs() < TIGHT, "{name} at {s}: {axis} gap {gap}");
                    }
                    if hits > floor && hits > label + pad {
                        hit += 1;
                        assert!(gap.abs() < TIGHT, "{name} at {s}: {axis} gap {gap}");
                    }
                }
            }
        }
        assert!(labelled > 0, "some fixture's labels set its size");
        assert!(hit > 0, "some fixture's hit areas set its size");
    }

    #[test]
    #[expect(
        clippy::suboptimal_flops,
        reason = "NFR-4: plain * and + are bit-identical on every target; mul_add falls back to a libm fma where the CPU lacks one"
    )]
    fn size_counts_markers() {
        let fonts = fonts();
        let (catalogue, order) = plotted(&csv_of(&["Q0", "Q0", "Q0"], "Assess"), None);
        let points = place(&catalogue, &order);
        let b = RadarBlock::new(&catalogue, &order, fonts);
        let s = 0.1;
        let r = RadarScale::of(s);
        let x = points.iter().map(|p| p.x.abs()).fold(0.0, f64::max);
        let size = b.size(s);
        // The labels and the floor alone give 66 wide.
        let labelled = 2.0 * (MIN_RADAR_HALF * s).max(b.label_half_width(r) + RADAR_LABEL_PAD);
        assert!(close(labelled, 66.0), "{labelled}");
        let width = 2.0 * (x * s + HIT_RADIUS);
        assert!(close(size.width, width), "{} vs {width}", size.width);
        assert!((size.width - 78.5595).abs() < 1e-3, "{}", size.width);
        let height = 2.0 * (MIN_RADAR_HALF * s).max(b.label_half_height(r) + RADAR_LABEL_PAD);
        assert!(close(size.height, height), "{} vs {height}", size.height);
        assert!((size.height - 87.875).abs() < 1e-3, "{}", size.height);

        // No markers, no marker term.
        let none = block(fonts, HEADER, None);
        let s = 0.02;
        let r = RadarScale::of(s);
        assert!(close(none.hit_half_width(r), 0.0) && close(none.hit_half_height(r), 0.0));
        let size = none.size(s);
        let width = 2.0 * (MIN_RADAR_HALF * s).max(none.label_half_width(r) + RADAR_LABEL_PAD);
        assert!(close(size.width, width), "{} vs {width}", size.width);
    }

    #[test]
    fn ring_halos_paint_before_every_name() {
        let fonts = fonts();
        let twenty: Vec<String> = (0..20).map(|i| format!("Ring {i} gjpqy")).collect();
        let csv = csv_of(&["Tool"], "Ring 0 gjpqy");
        for b in [
            block(fonts, PRD_EXAMPLE, None),
            block(fonts, &csv, Some(&twenty.join(","))),
        ] {
            for s in [1.0, 0.576] {
                let nodes = frame_at(&b, s);
                let texts = ring_texts(&nodes);
                let first_name = texts
                    .iter()
                    .position(|t| t.ink == fill(Neutral::TextMuted))
                    .expect("ring names are drawn");
                let (halos, names) = texts.split_at(first_name);
                assert!(
                    halos
                        .iter()
                        .all(|t| matches!(t.ink, TextPaint::Halo { .. })),
                    "{s}: only halos before the first name"
                );
                assert!(
                    names.iter().all(|t| t.ink == fill(Neutral::TextMuted)),
                    "{s}: no halo after the first name"
                );
                assert_eq!(halos.len(), names.len(), "{s}");
                for (halo, name) in halos.iter().zip(names) {
                    assert!(near(halo.origin, name.origin), "{s}");
                    assert_eq!(halo.lines, name.lines, "{s}");
                }
            }
        }
    }

    /// The nearest point of `rect` to the radar's centre, as a distance.
    fn nearest(rect: Rect) -> f64 {
        let x = 0.0_f64.clamp(rect.left, rect.right);
        let y = 0.0_f64.clamp(rect.top, rect.bottom);
        x.hypot(y)
    }

    #[test]
    #[expect(
        clippy::suboptimal_flops,
        reason = "NFR-4: plain * and + are bit-identical on every target; mul_add falls back to a libm fma where the CPU lacks one"
    )]
    fn long_names_clear_the_outer_ring() {
        let fonts = fonts();
        // Short widths too: the push shrinks with the name, so a short
        // name's box is the one nearest the ring.
        let stems = ["i".to_owned(), "WW".to_owned(), "WWWW".to_owned(), long(30)];
        for (n, stem) in [2, 3, 4, 8]
            .into_iter()
            .flat_map(|n| stems.iter().map(move |stem| (n, stem)))
        {
            let names: Vec<String> = (0..n).map(|i| format!("{stem}{i}")).collect();
            let names: Vec<&str> = names.iter().map(String::as_str).collect();
            let b = block(fonts, &csv_of(&names, "Adopt"), None);
            for s in [1.0, 0.576, 0.4, 0.31] {
                let ring = BAND_OUTER_RADIUS * s
                    + f64::midpoint(0.0, GRID_STROKE * RadarScale::of(s).marks);
                let nodes = frame_at(&b, s);
                let labels: Vec<&Node> = nodes
                    .iter()
                    .filter(|node| text(node).is_some_and(|t| t.style.weight == Weight::SemiBold))
                    .collect();
                assert_eq!(labels.len(), n, "N = {n}");
                for label in labels {
                    let rect = bounds(label, fonts).expect("a label paints");
                    let gap = nearest(rect);
                    assert!(
                        gap > ring,
                        "N = {n}, {stem} at {s}: {rect:?} reaches {gap} ≤ {ring}"
                    );
                }
            }
        }
    }

    #[test]
    #[expect(
        clippy::suboptimal_flops,
        reason = "NFR-4: plain * and + are bit-identical on every target; mul_add falls back to a libm fma where the CPU lacks one"
    )]
    fn the_inner_end_sits_on_the_tangent() {
        let fonts = fonts();
        for (n, stem) in [2, 3, 4, 8]
            .into_iter()
            .flat_map(|n| ["i", "Tool", "WWWWWWWWWWWW"].map(|stem| (n, stem)))
        {
            let names: Vec<String> = (0..n).map(|i| format!("{stem}{i}")).collect();
            let names: Vec<&str> = names.iter().map(String::as_str).collect();
            let b = block(fonts, &csv_of(&names, "Adopt"), None);
            let g = Geometry {
                quadrants: n,
                rings: 4,
            };
            for s in [1.5, 1.0, 0.576, 0.4] {
                let nodes = frame_at(&b, s);
                for (p, label) in quadrant_labels(&nodes).iter().enumerate() {
                    // The drawn node's own origin and measured width.
                    let half = fonts.face(label.style.weight).measure(
                        &label.lines.concat(),
                        label.style.size,
                        label.style.letter_spacing,
                    ) / 2.0;
                    let unit = polar(g.middle(p), 1.0);
                    let inner = label.origin.x - unit.x.signum() * half;
                    let point = polar(g.middle(p), 300.0 * s);
                    let along = (inner - point.x) * unit.x + (label.origin.y - point.y) * unit.y;
                    assert!(
                        along.abs() < 1e-9,
                        "N = {n}, {stem}{p} at {s}: {along} off the tangent"
                    );
                    if close(unit.y, 0.0) {
                        assert!(
                            close(inner, 300.0 * s * unit.x),
                            "N = {n}, {stem}{p} at {s}: {inner}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn short_names_barely_move() {
        let fonts = fonts();
        // At 3/9 o'clock a one-character name moves its full half-width,
        // under 6 units.
        let one = block(fonts, &csv_of(&["A", "B"], "Adopt"), None);
        let nodes = frame_at(&one, 1.0);
        for (label, middle) in quadrant_labels(&nodes).iter().zip([0.0, 180.0]) {
            let half = quadrant_width(fonts, &label.lines.concat()) / 2.0;
            let old = polar(middle, 300.0);
            let moved = (label.origin.x - old.x).hypot(label.origin.y - old.y);
            assert!(
                close(moved, half),
                "{moved} vs {half}: 3/9 o'clock moves the full half"
            );
            assert!(half < 6.0, "{half}");
        }
    }

    #[test]
    fn names_are_carried_raw() {
        let csv = format!("{HEADER}x,\"A<&\"\">B\",Adopt,d\ny,技术,Adopt,d\n");
        let labels = label_texts(&frame_at(&block(fonts(), &csv, None), 1.0));
        assert!(labels.iter().any(|l| l == "A<&\">B"), "{labels:?}");
        assert!(labels.iter().any(|l| l == "技术"), "{labels:?}");
    }

    #[test]
    fn a_placed_radar_is_the_origin_radar_translated() {
        let fonts = fonts();
        for b in [
            block(fonts, PRD_EXAMPLE, None),
            block(fonts, &marked_csv(), None),
        ] {
            assert_translates(&b, 1.0);
            assert_translates(&b, 0.5);
        }
    }
}
