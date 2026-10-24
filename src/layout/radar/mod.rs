//! The radar block (FR-30–FR-36, FR-40, FR-42, FR-43, FR-48, FR-50, FR-70,
//! FR-94): ring circles, sector dividers, sector labels and ring names,
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
mod reach;

use crate::diagnostics::{Binding, plural};
use crate::layout::block::{Block, Placement, Size};
use crate::layout::geometry::{Geometry, growth, polar};
use crate::model::Catalogue;
use crate::options::RingShading;
use crate::order::Plotted;
use crate::palette::{Neutral, Role};
use crate::settings::Origin;
use crate::style::{
    DIVIDER_RADIUS, GRID_STROKE, LABEL_HALO, MIN_RADAR_WIDTH, RADAR_FLOOR, RING_LABEL,
    SECTOR_LABEL, SECTOR_LABEL_RADIUS, TextStyle,
};
use crate::svg::{Anchor, Extent, Group, Node, Paint, Point, Scale, Semantics, TextRun};
use crate::text::Fonts;
use markers::{Marker, hit_node, hit_reach, indicator_node, layer, marker_nodes, markers};
use reach::{half_extent, ring_reach_x, ring_reach_y, sector_reach_x, sector_reach_y};

/// Half the minimum radar width: the floor on each half of the block.
const MIN_RADAR_HALF: f64 = MIN_RADAR_WIDTH / 2.0;
/// How far a ring name's halo reaches past its glyphs.
const HALO_REACH: f64 = LABEL_HALO / 2.0;

/// FR-70's three scales for one radar scale `s` on a radar grown by
/// `growth` (D-86).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RadarScale {
    /// For positions and radii: `s × growth`.
    pub geometry: f64,
    /// For label text and hit areas: `max(s, 1)`, so neither shrinks below
    /// natural size.
    pub labels: f64,
    /// For strokes, markers, their numbers and focus indicators:
    /// `max(s, RADAR_FLOOR)` (D-69).
    pub marks: f64,
}

impl RadarScale {
    /// `geometry` = s × growth, `labels` = max(s, 1), `marks` =
    /// max(s, `RADAR_FLOOR`): growth moves things apart and enlarges
    /// nothing drawn.
    ///
    /// # Panics
    ///
    /// In debug builds, if `s` is not finite and positive: a NaN would
    /// otherwise vanish into `max` and leave only the floors.
    #[must_use]
    pub fn of(s: f64, growth: f64) -> Self {
        debug_assert!(s.is_finite() && s > 0.0, "radar scale {s}");
        Self {
            geometry: s * growth,
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
        let width = style.measure(text, fonts);
        Self {
            text: text.to_owned(),
            width,
            at,
        }
    }
}

/// A sector's name and how far FR-79 pushes it outward (D-70).
#[derive(Debug, Clone, PartialEq)]
struct SectorLabel {
    /// Anchored at the label point, on the sector's middle angle.
    label: Label,
    /// Along the middle angle, at natural size: half the name's width times
    /// `|cos|` of the angle, which puts a middle-anchored name's inner end on
    /// the label circle's tangent at the label point: on the circle at 3 and
    /// 9 o'clock, beyond it elsewhere, at `√(R² + h² sin² θ)` for a drawn
    /// half-width `h`. Nothing at 12 and 6 o'clock, half the width at 3 and 9.
    ///
    /// So the name's box, which reaches its ascent (11.625) above the
    /// baseline, comes no closer to the centre than `300 k − 11.625` for a
    /// drawn geometry `k <= 1`, which clears the outer ring (`260 k` plus
    /// half a stroke) from `k` of about 0.3 up. The narrow arrangement
    /// never draws below that: its least scale is `NARROW_MIN_SCALE` over
    /// the radar's growth (D-86). Below it a 6 o'clock name touches the
    /// ring's stroke whatever its length, unpushed.
    push: Point,
}

impl SectorLabel {
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
    sectors: Vec<SectorLabel>,
    /// In declared order, outermost first (D-60).
    rings: Vec<Label>,
    /// Where the rings were declared, for FR-95 (D-73).
    ring_origin: Origin,
    /// In FR-34's order.
    markers: Vec<Marker>,
    /// How much the geometry has grown to fit the markers (D-86): 1 until
    /// `Blocks::new` chooses otherwise.
    growth: f64,
    /// Whether every other ring is shaded (rg-74n): none until the run
    /// asks.
    shading: RingShading,
}

impl RadarBlock {
    /// The radar's accessible name, counting declared rings and plotted
    /// items (D-75): the group's label, and the exported SVG's `<desc>`.
    #[must_use]
    pub fn name(&self) -> String {
        label(self.sectors.len(), self.rings.len(), self.markers.len())
    }

    /// The radar for `catalogue`, its sectors in `plotted`'s FR-10 order,
    /// every declared ring drawn, empty or not (FR-19), and a marker for
    /// every plotted item. Labels are measured once here, at natural size.
    ///
    /// # Panics
    ///
    /// If `plotted` names a sector or item `catalogue` lacks
    /// (`order::resolve` guarantees it does not).
    #[must_use]
    // qual:allow(complexity, error_handling) reason: "the documented # Panics: a Plotted from another catalogue is a caller bug"
    pub fn new(catalogue: &Catalogue, plotted: &Plotted, fonts: &Fonts) -> Self {
        let geometry = Geometry::of(catalogue);
        let sectors = plotted
            .sectors
            .iter()
            .enumerate()
            .map(|(p, &q)| {
                let sector = catalogue
                    .sectors
                    .get(q)
                    .expect("FR-10's order indexes the catalogue's sectors");
                let middle = geometry.middle(p);
                let at = polar(middle, SECTOR_LABEL_RADIUS);
                let label = Label::measured(&sector.label, SECTOR_LABEL, at, fonts);
                SectorLabel::new(label, middle)
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
            sectors,
            rings,
            ring_origin: catalogue.ring_origin.clone(),
            markers: markers(catalogue, plotted),
            growth: 1.0,
            shading: RingShading::None,
        }
    }

    /// How much the geometry has grown (D-86).
    #[must_use]
    pub const fn growth(&self) -> f64 {
        self.growth
    }

    /// Sets the geometry's growth to `growth`, one of D-86's steps.
    pub(crate) const fn set_growth(&mut self, growth: f64) {
        self.growth = growth;
    }

    /// Shades the rings as `shading` says (rg-74n).
    pub const fn set_ring_shading(&mut self, shading: RingShading) {
        self.shading = shading;
    }

    /// Whether ring `i`, outermost first, is shaded: every other one,
    /// counting from the innermost, which always is (rg-74n).
    const fn is_shaded(&self, i: usize) -> bool {
        matches!(self.shading, RingShading::Alternate)
            && (self.rings.len() - 1 - i).is_multiple_of(2)
    }

    /// The colour under ring `i`'s name: its band's, or the ground's.
    const fn ground_of(&self, i: usize) -> Paint {
        if self.is_shaded(i) {
            Paint::Role(Role::RingBand)
        } else {
            Paint::Role(Role::Neutral(Neutral::Surface1))
        }
    }

    /// The least growth step at which every two markers clear (D-86),
    /// from the markers' natural-size centres.
    #[must_use]
    pub fn needed_growth(&self) -> f64 {
        let centres: Vec<Point> = self.markers.iter().map(|m| m.at).collect();
        growth(&centres)
    }

    /// FR-70's three scales at radar scale `s`, with this radar's growth.
    #[must_use]
    pub fn scale(&self, s: f64) -> RadarScale {
        RadarScale::of(s, self.growth)
    }

    /// How far each label reaches left or right of the centre, with its
    /// text and, for a ring, where it was declared: sectors in FR-10 order,
    /// then rings. The one
    /// source for both the radar's width and the label that sets it.
    fn reaches_x(&self, r: RadarScale) -> impl Iterator<Item = (f64, Option<&Origin>, &str)> + '_ {
        let sectors = self
            .sectors
            .iter()
            .map(move |q| (sector_reach_x(q, r), None, q.label.text.as_str()));
        let rings = self.rings.iter().map(move |l| {
            (
                ring_reach_x(l.width, r),
                Some(&self.ring_origin),
                l.text.as_str(),
            )
        });
        sectors.chain(rings)
    }

    /// The furthest any label reaches left or right of the centre.
    fn label_half_width(&self, r: RadarScale) -> f64 {
        self.reaches_x(r)
            .map(|(reach, ..)| reach)
            .fold(0.0, f64::max)
    }

    /// The label whose reach left or right sets the radar's width at
    /// `scale`, for FR-95 at the narrow scale and FR-105 at 1 (D-73, D-94):
    /// the first furthest in FR-10 order,
    /// sectors before rings. `None` only with no labels at all, which
    /// `rings.rs` rules out by refusing an empty ring list.
    #[must_use]
    pub fn binding_label(&self, scale: f64) -> Option<Binding> {
        self.reaches_x(self.scale(scale))
            .reduce(|best, next| if next.0 > best.0 { next } else { best })
            .map(|(_, ring, text)| {
                let name = text.to_owned();
                match ring {
                    Some(origin) => Binding::Ring {
                        name,
                        origin: origin.clone(),
                    },
                    None => Binding::Sector(name),
                }
            })
    }

    /// The furthest any label reaches above or below the centre.
    fn label_half_height(&self, r: RadarScale) -> f64 {
        let sectors = self.sectors.iter().map(|label| sector_reach_y(label, r));
        let rings = self.rings.iter().map(|label| ring_reach_y(-label.at.y, r));
        sectors.chain(rings).fold(0.0, f64::max)
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

    /// The shading under the rings (rg-74n): a filled disc per ring from
    /// the outermost shaded one inward, each covering its ring and all
    /// inside it, so the next one in cuts it down to its band. A shaded
    /// ring's disc is the band's colour and an unshaded one's the ground's.
    /// None when the rings are unshaded.
    fn bands(&self, r: RadarScale) -> impl Iterator<Item = Node> + '_ {
        let first = (0..self.rings.len())
            .find(|&i| self.is_shaded(i))
            .unwrap_or(self.rings.len());
        (first..self.rings.len()).map(move |i| Node::Circle {
            centre: Point::ORIGIN,
            radius: self.geometry.band(i).outer * r.geometry,
            fill: self.ground_of(i),
            stroke: Paint::None,
            stroke_width: 0.0,
        })
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

    /// One divider per sector (FR-77); none with no sectors (FR-42).
    fn dividers(&self, r: RadarScale) -> impl Iterator<Item = Node> + '_ {
        (0..self.sectors.len()).map(move |p| Node::Line {
            from: Point::ORIGIN,
            to: polar(self.geometry.divider(p), DIVIDER_RADIUS * r.geometry),
            stroke: Paint::Role(Role::Neutral(Neutral::StrokeDivider)),
            stroke_width: GRID_STROKE * r.marks,
        })
    }

    /// Each sector's name, mid-sector beyond the rim, pushed out by its
    /// own half-width (D-70). The centre is already at `r`, so only the size
    /// scales.
    fn sector_labels(&self, r: RadarScale) -> impl Iterator<Item = Node> + '_ {
        let scale = Scale {
            position: 1.0,
            size: r.labels,
        };
        self.sectors.iter().map(move |sector| {
            TextRun {
                origin: sector.centre(r),
                style: SECTOR_LABEL,
                ink: Neutral::TextPrimary,
                anchor: Anchor::Middle,
                lines: vec![sector.label.text.clone()],
                words: sector.label.text.clone(),
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
            words: label.text.clone(),
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
            .enumerate()
            .map(move |(i, run)| run.halo_on(self.ground_of(i), LABEL_HALO, r.text()));
        let names = self.ring_runs().map(move |run| run.node(r.text()));
        halos.chain(names)
    }
}

impl Block for RadarBlock {
    // qual:allow(complexity, magic_numbers) reason: "a box centred on its centre spans twice the centre offset"
    fn size(&self, scale: f64) -> Size {
        let centre = self.centre(self.scale(scale));
        Size {
            width: 2.0 * centre.x,
            height: 2.0 * centre.y,
        }
    }

    fn render(&self, at: Placement) -> Node {
        let r = self.scale(at.scale);
        let frame = self
            .bands(r)
            .chain(self.ring_circles(r))
            .chain(self.dividers(r))
            .chain(self.sector_labels(r))
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
        let centre = self.centre(r);
        // The block's box about the centre the children are drawn from.
        let extent = Extent {
            min: Point {
                x: -centre.x,
                y: -centre.y,
            },
            max: centre,
        };
        let centred = Node::Group(
            Group::new(centre, children)
                .with(Semantics::Group(self.name()))
                .spanning(extent),
        );
        Node::Group(Group::new(at.origin, vec![centred]))
    }
}

/// The radar's accessible name (D-75): its sectors, declared rings and
/// plotted items, so assistive technology announces its size first.
fn label(sectors: usize, rings: usize, items: usize) -> String {
    format!(
        "Technology radar: {sectors} {}, {rings} {}, {items} {}",
        plural(sectors, "sector", "sectors"),
        plural(rings, "ring", "rings"),
        plural(items, "item", "items")
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::block::{assert_translates, at, close, fonts};
    use crate::layout::geometry::place;
    use crate::layout::test_support::{
        HEAD, HEAD_META, PRD_EXAMPLE, Rect, assert_finite, bounds, csv_of, fill, groups, near,
        plotted, plus,
    };
    use crate::style::{
        BAND_OUTER_RADIUS, HIT_RADIUS, INDICATOR_REACH, MARKER_RADIUS, NARROW_MIN_SCALE,
        RADAR_LABEL_PAD,
    };
    use crate::svg::TextPaint;
    use crate::text::{TEXT_ASCENT, TEXT_DESCENT, Weight};

    /// How near the block's edge the drawn labels must come: float noise.
    const TIGHT: f64 = 1e-6;

    /// rg-2qs: the figure's group carries the block's own box about its
    /// centre, and everything it draws lies inside it.
    #[test]
    fn the_figure_spans_what_it_draws() {
        let fonts = fonts();
        let radar = block(fonts, PRD_EXAMPLE, None);
        for scale in [1.0, NARROW_MIN_SCALE] {
            let Node::Group(outer) = radar.render(at(scale)) else {
                panic!("a group");
            };
            let Some(Node::Group(figure)) = outer.children.first() else {
                panic!("the figure's group");
            };
            let extent = figure.extent.expect("the figure's box");
            let size = radar.size(scale);
            assert!(close(extent.max.x - extent.min.x, size.width));
            assert!(close(extent.max.y - extent.min.y, size.height));
            for rect in figure.children.iter().filter_map(|n| bounds(n, fonts)) {
                assert!(
                    rect.left >= extent.min.x - TIGHT
                        && rect.right <= extent.max.x + TIGHT
                        && rect.top >= extent.min.y - TIGHT
                        && rect.bottom <= extent.max.y + TIGHT,
                    "{scale}: {rect:?} outside {extent:?}"
                );
            }
        }
    }

    pub(super) fn block(fonts: &Fonts, csv: &str, rings: Option<&str>) -> RadarBlock {
        let (catalogue, order) = plotted(csv, rings);
        RadarBlock::new(&catalogue, &order, fonts)
    }

    /// `n` short sectors, `Q0`… in FR-10 order, default rings.
    pub(super) fn sectors(fonts: &Fonts, n: usize) -> RadarBlock {
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

    const QUALIFIERS: [&str; 2] = ["", "conditional"];
    const STATUSES: [&str; 5] = ["", "No Change", "New", "Moved In", "Moved Out"];

    /// N = 4, each sector holding every qualifier × status pair in Trial.
    pub(super) fn marked_csv() -> String {
        let rows = ["Q0", "Q1", "Q2", "Q3"].into_iter().flat_map(|q| {
            QUALIFIERS.into_iter().flat_map(move |qualifier| {
                STATUSES.into_iter().map(move |status| {
                    format!("{q}/{qualifier}/{status},{q},Trial,d,{qualifier},{status}\n")
                })
            })
        });
        std::iter::once(HEAD_META.to_owned()).chain(rows).collect()
    }

    fn fixtures(fonts: &Fonts) -> Vec<(&'static str, RadarBlock)> {
        // N = 4 with one sector named 40 × "W"; no sectors and one ring
        // named 80 × "W".
        let long_sector = csv_of(&["B", "C", "D", &long(40)], "Adopt");
        vec![
            ("prd", block(fonts, PRD_EXAMPLE, None)),
            ("n1", sectors(fonts, 1)),
            ("n2", sectors(fonts, 2)),
            ("n6", sectors(fonts, 6)),
            ("n8", sectors(fonts, 8)),
            ("zero", block(fonts, HEAD, None)),
            ("long sector", block(fonts, &long_sector, None)),
            ("long ring", block(fonts, HEAD, Some(&long(80)))),
            ("assess", assess(fonts)),
            ("crowded", crowded(fonts)),
            ("marked", block(fonts, &marked_csv(), None)),
            ("grown", grown(block(fonts, &marked_csv(), None))),
            ("grown crowded", grown(crowded(fonts))),
        ]
    }

    /// `b` at D-86's largest growth.
    fn grown(mut b: RadarBlock) -> RadarBlock {
        b.set_growth(crate::style::MAX_GROWTH);
        b
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

    /// Sector labels are the only `SemiBold` texts in the frame.
    fn sector_labels(nodes: &[Node]) -> Vec<Text<'_>> {
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

    /// A sector name's width at natural size.
    fn sector_width(fonts: &Fonts, name: &str) -> f64 {
        SECTOR_LABEL.measure(name, fonts)
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
        sector_labels(nodes)
            .iter()
            .map(|t| t.lines.concat())
            .collect()
    }

    #[test]
    fn radar_scale() {
        let is = |s: f64, [geometry, labels, marks]: [f64; 3]| {
            let r = RadarScale::of(s, 1.0);
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

    /// rg-06b, D-86: growth scales the radar's geometry and nothing else.
    #[test]
    fn growth_scales_geometry_only() {
        let mut b = block(fonts(), &marked_csv(), None);
        assert!(close(b.growth(), 1.0));
        b.set_growth(2.0);
        assert!(close(b.growth(), 2.0));
        for s in [0.15, 0.5, RADAR_FLOOR, 1.0] {
            let r = b.scale(s);
            assert!(
                close(r.geometry, 2.0 * s)
                    && close(r.labels, s.max(1.0))
                    && close(r.marks, s.max(RADAR_FLOOR)),
                "{s}: {r:?}"
            );
        }
    }

    /// Every leaf node in `b` drawn at 1.
    fn leaves(b: &RadarBlock) -> Vec<Node> {
        let root = b.render(at(1.0));
        groups(&root)
            .into_iter()
            .flat_map(|g| g.children.iter().filter(|n| !matches!(n, Node::Group(_))))
            .cloned()
            .collect()
    }

    /// Every hit area in `b` drawn at 1, with its radius.
    fn hit_areas(b: &RadarBlock) -> Vec<(Point, f64)> {
        leaves(b)
            .into_iter()
            .filter_map(|n| match n {
                Node::HitArea { centre, radius, .. } => Some((centre, radius)),
                _ => None,
            })
            .collect()
    }

    /// Markers spread apart as the radar grows but keep their size: each
    /// hit area sits `g` times as far out, as large as before, and every
    /// marker's circle keeps its radius.
    #[test]
    fn markers_keep_their_size_as_the_radar_grows() {
        let ungrown = block(fonts(), &marked_csv(), None);
        let mut grown = ungrown.clone();
        grown.set_growth(1.5);
        let (before, after) = (hit_areas(&ungrown), hit_areas(&grown));
        assert!(!before.is_empty() && before.len() == after.len());
        for ((c0, r0), (c1, r1)) in before.iter().zip(&after) {
            assert!(near(c0.scaled(1.5), *c1), "{c0:?} grows to {c1:?}");
            assert!(close(*r0, *r1), "the hit radius stays {r0}, got {r1}");
        }
        let markers = |b: &RadarBlock| {
            leaves(b)
                .iter()
                .filter(
                    |n| matches!(n, Node::Circle { radius, .. } if close(*radius, MARKER_RADIUS)),
                )
                .count()
        };
        assert_eq!(markers(&ungrown), markers(&grown));
        assert!(markers(&grown) > 0);
    }

    /// The rings grow with the geometry, so a grown block is wider, while
    /// its labels keep their natural size.
    #[test]
    fn a_grown_radar_is_larger_and_its_labels_are_not() {
        let ungrown = block(fonts(), PRD_EXAMPLE, None);
        let mut grown = ungrown.clone();
        grown.set_growth(2.0);
        assert!(grown.size(1.0).width > ungrown.size(1.0).width);
        let sizes = |b: &RadarBlock| -> Vec<f64> {
            sector_labels(&frame_at(b, 1.0))
                .iter()
                .map(|t| t.style.size)
                .collect()
        };
        let (before, after) = (sizes(&ungrown), sizes(&grown));
        assert!(!before.is_empty() && before.len() == after.len());
        assert!(
            before.iter().zip(&after).all(|(a, b)| close(*a, *b)),
            "{before:?} vs {after:?}"
        );
    }

    #[test]
    fn zero_sectors_draws_rings_only() {
        let b = block(fonts(), HEAD, None);
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
    fn one_sector_has_one_divider() {
        let nodes = frame_at(&sectors(fonts(), 1), 1.0);
        assert_eq!(lines(&nodes).len(), 1);
        let labels = sector_labels(&nodes);
        let [label] = labels.as_slice() else {
            panic!("one sector label");
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
            ends_are(&lines(&frame_at(&sectors(fonts, n), 1.0)), angles, 262.0);
        }
    }

    #[test]
    fn sector_labels_follow_fr10() {
        let fonts = fonts();
        let check = |csv: &str| {
            let nodes = frame_at(&block(fonts, csv, None), 1.0);
            assert_eq!(label_texts(&nodes), ["platform", "Practice", "Tool"]);
            let g = Geometry {
                sectors: 3,
                rings: 4,
            };
            for (i, label) in sector_labels(&nodes).iter().enumerate() {
                let width = sector_width(fonts, &label.lines.concat());
                assert!(near(label.origin, pushed(g.middle(i), width, 1.0)), "{i}");
            }
            assert_eq!(lines(&nodes).len(), 3);
        };
        check(&csv_of(&["Tool", "platform", "Practice"], "Adopt"));
        // FR-42: every row in an undeclared ring plots nothing, but the
        // sectors still exist and are still drawn.
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

    /// rg-74n: the filled discs that shade the rings, as (radius, fill), in
    /// drawing order. A ring circle is never filled, so these are the rest.
    fn bands(nodes: &[Node]) -> Vec<(f64, Paint)> {
        nodes
            .iter()
            .filter_map(|n| match n {
                Node::Circle {
                    centre,
                    radius,
                    fill,
                    stroke: Paint::None,
                    ..
                } => {
                    assert_eq!(*centre, Point::ORIGIN, "a band is centred");
                    Some((*radius, *fill))
                }
                _ => None,
            })
            .collect()
    }

    /// The ring circles' radii, outermost first.
    fn ring_radii(nodes: &[Node]) -> Vec<f64> {
        nodes
            .iter()
            .filter_map(|n| match n {
                Node::Circle {
                    radius,
                    stroke: Paint::Role(Role::Neutral(Neutral::StrokeRing)),
                    ..
                } => Some(*radius),
                _ => None,
            })
            .collect()
    }

    const BAND: Paint = Paint::Role(Role::RingBand);
    const GROUND: Paint = Paint::Role(Role::Neutral(Neutral::Surface1));

    fn shaded(rings: Option<&str>) -> RadarBlock {
        let mut b = block(fonts(), PRD_EXAMPLE, rings);
        b.set_ring_shading(RingShading::Alternate);
        b
    }

    /// rg-74n: off by default, as the standard radar draws it.
    #[test]
    fn rings_are_unshaded_by_default() {
        let b = block(fonts(), PRD_EXAMPLE, None);
        assert_eq!(bands(&frame_at(&b, 1.0)), []);
    }

    /// rg-74n: every other ring from the centre. Each disc covers its ring
    /// and everything inside, so the next one in cuts the band down to its
    /// ring: a ground disc under an unshaded ring, a band disc under a
    /// shaded one, and nothing outside the outermost shaded ring.
    #[test]
    fn alternate_shades_every_other_ring_from_the_centre() {
        // Four rings, Assess outermost: Trial and Retire are shaded.
        let nodes = frame_at(&shaded(None), 1.0);
        let r = ring_radii(&nodes);
        let want = |picks: &[(usize, Paint)]| -> Vec<(f64, Paint)> {
            picks
                .iter()
                .map(|&(i, paint)| (*r.get(i).expect("a ring"), paint))
                .collect()
        };
        assert_eq!(bands(&nodes), want(&[(1, BAND), (2, GROUND), (3, BAND)]));
        // Three: the outermost is shaded too, counting from the centre.
        let nodes = frame_at(&shaded(Some("Adopt,Trial,Assess")), 1.0);
        let r = ring_radii(&nodes);
        let picks = [(0, BAND), (1, GROUND), (2, BAND)];
        let want: Vec<(f64, Paint)> = picks
            .iter()
            .map(|&(i, paint)| (*r.get(i).expect("a ring"), paint))
            .collect();
        assert_eq!(bands(&nodes), want);
    }

    /// rg-74n: the edge cases of counting from the centre. One ring is
    /// one band; two shade the inner only, with nothing outside it.
    #[test]
    fn one_and_two_rings_shade_from_the_centre() {
        let nodes = frame_at(&shaded(Some("Adopt")), 1.0);
        let r = ring_radii(&nodes);
        assert_eq!(bands(&nodes), [(*r.first().expect("a ring"), BAND)]);
        let nodes = frame_at(&shaded(Some("Adopt,Trial")), 1.0);
        let r = ring_radii(&nodes);
        assert_eq!(bands(&nodes), [(*r.get(1).expect("two rings"), BAND)]);
    }

    /// rg-74n: the bands lie under everything else the radar draws.
    #[test]
    fn bands_are_drawn_first() {
        let nodes = frame_at(&shaded(None), 1.0);
        let n = bands(&nodes).len();
        assert!(n > 0);
        assert!(nodes.iter().take(n).all(|node| matches!(
            node,
            Node::Circle {
                stroke: Paint::None,
                ..
            }
        )));
        assert!(!nodes.iter().skip(n).any(|node| matches!(
            node,
            Node::Circle {
                stroke: Paint::None,
                ..
            }
        )));
    }

    /// rg-74n: a ring name's halo is its band's colour, so no pale patch
    /// shows round a name on a shaded ring.
    #[test]
    fn a_shaded_rings_name_sits_on_a_band_halo() {
        let nodes = frame_at(&shaded(None), 1.0);
        let texts = ring_texts(&nodes);
        let (halos, _) = texts.split_at(texts.len() / 2);
        let strokes: Vec<Paint> = halos
            .iter()
            .map(|halo| match halo.ink {
                TextPaint::Halo { stroke, .. } => stroke,
                other @ TextPaint::Fill(_) => panic!("expected a halo, got {other:?}"),
            })
            .collect();
        // Outermost first: Assess, Trial, Adopt, Retire.
        assert_eq!(strokes, [GROUND, BAND, GROUND, BAND]);
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
            .expect("three sectors draw dividers");
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
    fn sector_labels_are_primary_and_middle_anchored() {
        let nodes = frame_at(&block(fonts(), PRD_EXAMPLE, None), 1.0);
        let labels = sector_labels(&nodes);
        assert_eq!(labels.len(), 3);
        for label in labels {
            assert_eq!(label.ink, fill(Neutral::TextPrimary));
            assert_eq!(label.anchor, Anchor::Middle);
            assert_eq!(label.style, SECTOR_LABEL);
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
        let b = sectors(fonts, 4);
        let g = Geometry {
            sectors: 4,
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
            let labels = sector_labels(&nodes);
            assert_eq!(labels.len(), 4);
            for (p, label) in labels.iter().enumerate() {
                let width = sector_width(fonts, &label.lines.concat());
                assert!(
                    near(label.origin, pushed(g.middle(p), width, s)),
                    "{s} label {p}"
                );
                assert!(close(label.style.size, SECTOR_LABEL.size));
                assert!(close(
                    label.style.letter_spacing,
                    SECTOR_LABEL.letter_spacing
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
        let zero = block(fonts, HEAD, None).size(0.5);
        assert!(
            close(zero.width, 330.0) && close(zero.height, 330.0),
            "{zero:?}"
        );

        // N = 8: middles −67.5° (Q0) and 247.5° (Q7) put labels above the
        // centre, so the higher one's ascent sets the height once the floor
        // (198 at 0.3) is passed. The push raises it by the wider name's
        // half-width × cos 67.5° × sin 67.5°, at natural size.
        let s = 0.3;
        let width = sector_width(fonts, "Q0").max(sector_width(fonts, "Q7"));
        let above = -pushed(-67.5, width, s).y;
        let want = 2.0 * (TEXT_ASCENT * 12.0 + above + 10.0);
        let got = sectors(fonts, 8).size(s).height;
        assert!(close(got, want), "{got} vs {want}");
        // 215.80 before D-78: the wider name's layout width now carries the
        // spacing allowance, so its push is larger.
        assert!(want > 660.0 * s && (want - 216.81).abs() < 0.01, "{want}");

        // N = 1: the label hangs below 6 o'clock, so its descent counts.
        let s = 0.4_f64;
        let want = (660.0 * s).max(2.0 * (TEXT_DESCENT * 12.0 + 300.0 * s + 10.0));
        let got = sectors(fonts, 1).size(s).height;
        assert!(close(got, want), "{got} vs {want}");
    }

    #[test]
    #[expect(
        clippy::suboptimal_flops,
        reason = "NFR-4: plain * and + are bit-identical on every target; mul_add falls back to a libm fma where the CPU lacks one"
    )]
    fn a_long_sector_label_widens_the_block() {
        let fonts = fonts();
        let w = sector_width(fonts, &long(40));
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
        let w = RING_LABEL.measure(&long(80), fonts);
        let want = 2.0 * (w / 2.0 + 1.5 + 10.0);
        let got = block(fonts, HEAD, Some(&long(80))).size(1.0).width;
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
                let r = b.scale(s);
                let pad = RADAR_LABEL_PAD * r.labels;
                let floor = MIN_RADAR_HALF * r.geometry;
                // How far the rings and dividers can reach: below about
                // s = 0.15 they can pass the labels without passing the floor.
                let grid = DIVIDER_RADIUS * r.geometry + f64::midpoint(0.0, GRID_STROKE * r.marks);
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
        let r = b.scale(s);
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
        let none = block(fonts, HEAD, None);
        let s = 0.02;
        let r = none.scale(s);
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
                let ring =
                    BAND_OUTER_RADIUS * s + f64::midpoint(0.0, GRID_STROKE * b.scale(s).marks);
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
                sectors: n,
                rings: 4,
            };
            for s in [1.5, 1.0, 0.576, 0.4] {
                let nodes = frame_at(&b, s);
                for (p, label) in sector_labels(&nodes).iter().enumerate() {
                    // The drawn node's own origin and measured width.
                    let half = label.style.measure(&label.lines.concat(), fonts) / 2.0;
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
        for (label, middle) in sector_labels(&nodes).iter().zip([0.0, 180.0]) {
            let half = sector_width(fonts, &label.lines.concat()) / 2.0;
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
        let csv = format!("{HEAD}x,\"A<&\"\">B\",Adopt,d\ny,技术,Adopt,d\n");
        let labels = label_texts(&frame_at(&block(fonts(), &csv, None), 1.0));
        assert!(labels.iter().any(|l| l == "A<&\">B"), "{labels:?}");
        assert!(labels.iter().any(|l| l == "技术"), "{labels:?}");
    }

    /// rg-eae: the centring group names the radar, counting declared rings
    /// and plotted items, so the PRD example's unplotted row is not counted.
    #[test]
    fn the_radar_is_a_named_group() {
        let rendered = block(fonts(), PRD_EXAMPLE, None).render(at(1.0));
        let Node::Group(root) = &rendered else {
            panic!("the radar renders a group, got {rendered:?}");
        };
        let [Node::Group(centred)] = root.children.as_slice() else {
            panic!("the root holds one centring group: {:?}", root.children);
        };
        assert_eq!(
            centred.semantics,
            Semantics::Group("Technology radar: 3 sectors, 4 rings, 6 items".into())
        );
    }

    /// FR-95: a label on a diagonal reaches further than a longer one at 6
    /// o'clock, which reaches only half its width (D-71). With three
    /// sectors, FR-10's second sits at 6 o'clock.
    #[test]
    fn the_binding_label_is_not_always_the_longest() {
        let diagonal = format!("A{}", "W".repeat(30));
        let six = format!("B{}", "W".repeat(36));
        assert!(six.len() > diagonal.len(), "the 6 o'clock name is longer");
        let b = block(fonts(), &csv_of(&[&diagonal, &six, "C"], "Adopt"), None);
        assert!(
            close(b.geometry.middle(1), 90.0),
            "position 1 is at 6 o'clock: {}",
            b.geometry.middle(1)
        );
        assert_eq!(
            b.binding_label(NARROW_MIN_SCALE),
            Some(Binding::Sector(diagonal))
        );
    }

    /// D-73: ring names count, and reach the same at every angle.
    #[test]
    fn a_long_ring_binds() {
        let ring = "W".repeat(70);
        let b = block(fonts(), HEAD, Some(&format!("{ring},Adopt")));
        assert_eq!(
            b.binding_label(NARROW_MIN_SCALE),
            Some(Binding::Ring {
                name: ring,
                origin: Origin::rings(crate::settings::Source::Flag),
            })
        );
    }

    #[test]
    fn label_counts_singular_and_plural() {
        assert_eq!(label(1, 1, 1), "Technology radar: 1 sector, 1 ring, 1 item");
        assert_eq!(
            label(0, 4, 0),
            "Technology radar: 0 sectors, 4 rings, 0 items"
        );
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
