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

use crate::layout::block::{Block, Placement, Size};
use crate::layout::geometry::{Geometry, place, polar};
use crate::layout::marker::{Glyph, StatusMark};
use crate::model::{Catalogue, Qualifier, Status};
use crate::order::Plotted;
use crate::palette::{Neutral, Role};
use crate::style::{
    BAND_OUTER_RADIUS, DIVIDER_RADIUS, FOCUS_RING_GAP, FOCUS_RING_STROKE, GRID_STROKE, HIT_RADIUS,
    INDICATOR_REACH, LABEL_HALO, MARKER_NUMBER, MARKER_RADIUS, MIN_RADAR_WIDTH, QUADRANT_LABEL,
    QUADRANT_LABEL_RADIUS, RADAR_FLOOR, RADAR_LABEL_PAD, RING_LABEL, TextStyle,
};
use crate::svg::{
    Anchor, Control, Group, ItemKey, Node, Paint, Point, Scale, Semantics, TextPaint, TextRun,
};
use crate::text::{Fonts, TEXT_ASCENT, TEXT_DESCENT};
use crate::trim;

/// Half the minimum radar width: the floor on each half of the block.
const MIN_RADAR_HALF: f64 = MIN_RADAR_WIDTH / 2.0;
/// How far a ring name's halo reaches past its glyphs.
const HALO_REACH: f64 = LABEL_HALO / 2.0;
/// How far below a marker's centre its number's baseline sits, per unit of
/// font size: the em box's centre (CSS `dominant-baseline: central`, as the
/// mockup has it). For Inter that is also half the cap height, because
/// 1984 - 494 = 1490 = capHeight, so lining digits sit centred on the marker.
const NUMBER_CENTRING: f64 = f64::midpoint(TEXT_ASCENT, -TEXT_DESCENT);

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

/// One plotted item, ready to draw.
#[derive(Debug, Clone, PartialEq)]
struct Marker {
    /// FR-33's centre, at scale 1, relative to the radar's centre.
    at: Point,
    /// Solid or hollow (FR-20).
    qualifier: Option<Qualifier>,
    /// The status ring, if any (FR-40).
    status: Option<Status>,
    /// The quadrant's FR-10 rank: its series index, unreduced (FR-67).
    series: usize,
    /// FR-34's number.
    number: usize,
    /// Degrees (SVG frame) from the marker toward the radar's centre.
    centreward: f64,
    /// Its key, accessible name (FR-50) and tooltip (FR-36).
    control: Control,
}

impl Marker {
    /// The marker's centre at `r`'s position scale, relative to the radar's
    /// centre.
    fn centre(&self, r: RadarScale) -> Point {
        self.at.scaled(r.geometry)
    }
}

/// The radar (FR-30–FR-36): its rings, dividers, labels and markers.
#[derive(Debug, Clone, PartialEq)]
pub struct RadarBlock {
    geometry: Geometry,
    /// In FR-10 order: label `p` belongs to the sector at position `p`.
    quadrants: Vec<Label>,
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
                let at = polar(geometry.middle(p), QUADRANT_LABEL_RADIUS);
                Label::measured(&quadrant.label, QUADRANT_LABEL, at, fonts)
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
            .map(|label| quadrant_reach_x(label.at, label.width, r));
        let rings = self.rings.iter().map(|label| ring_reach_x(label.width, r));
        quadrants.chain(rings).fold(0.0, f64::max)
    }

    /// The furthest any label reaches above or below the centre.
    fn label_half_height(&self, r: RadarScale) -> f64 {
        let quadrants = self
            .quadrants
            .iter()
            .map(|label| quadrant_reach_y(label.at, r));
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

    /// Each quadrant's name, mid-sector beyond the rim.
    fn quadrant_labels(&self, r: RadarScale) -> impl Iterator<Item = Node> + '_ {
        self.quadrants.iter().map(move |label| {
            TextRun {
                origin: label.at,
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
        self.rings.iter().flat_map(move |label| {
            let run = TextRun {
                origin: label.at,
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
        // Each marker's glyph and number, drawn once: the control holds
        // them and its indicator raises a copy.
        let drawn: Vec<Vec<Node>> = self.markers.iter().map(|m| marker_nodes(m, r)).collect();
        let hits = layer(self.markers.iter().map(|m| hit_node(m, r)).collect());
        let controls = self.markers.iter().zip(&drawn).map(|(m, nodes)| {
            Node::Group(
                Group::new(Point::ORIGIN, nodes.clone()).with(Semantics::Marker(m.control.clone())),
            )
        });
        let indicators = layer(
            self.markers
                .iter()
                .zip(drawn.iter().cloned())
                .map(|(m, nodes)| indicator_node(m, nodes, r))
                .collect(),
        );
        let children = frame
            .chain(hits)
            .chain(controls)
            .chain(indicators)
            .collect();
        let centred = Node::Group(Group::new(self.centre(r), children));
        Node::Group(Group::new(at.origin, vec![centred]))
    }
}

/// Every plotted item's marker, in FR-34's order: `place`'s points zipped
/// with [`Plotted::items`], numbered from 1 (FR-34).
///
/// # Panics
///
/// If `plotted` was not built from `catalogue`, so an item or its quadrant
/// is out of range for it, or `place` returns a different number of points
/// than there are plotted items.
// qual:allow(complexity, error_handling) reason: "the documented # Panics: a Plotted from another catalogue is a caller bug"
fn markers(catalogue: &Catalogue, plotted: &Plotted) -> Vec<Marker> {
    let placed = place(catalogue, plotted);
    assert_eq!(
        placed.len(),
        plotted.items.len(),
        "place yields one point per plotted item, or the zip below drops markers"
    );
    placed
        .into_iter()
        .zip(plotted.items.iter().zip(1_usize..))
        .map(|(at, (&index, number))| {
            let item = catalogue
                .items
                .get(index)
                .expect("FR-34's order indexes the catalogue's items");
            let series = *plotted
                .rank
                .get(item.quadrant)
                .expect("rank covers every quadrant of the catalogue");
            Marker {
                at,
                qualifier: item.qualifier,
                status: item.status,
                series,
                number,
                centreward: (-at.y).atan2(-at.x).to_degrees(),
                control: Control {
                    key: ItemKey(item.line),
                    label: catalogue.accessible_name(item, Some(number)),
                    tooltip: Some(format!("{number}. {}", trim(&item.name))),
                },
            }
        })
        .collect()
}

/// A marker's glyph, then its number on top.
fn marker_nodes(m: &Marker, r: RadarScale) -> Vec<Node> {
    glyph_nodes(m, r)
        .into_iter()
        .chain([number_node(m, r)])
        .collect()
}

/// A marker's status ring (if any) and disc (FR-20, FR-40). A half-ring
/// faces the direction from the marker toward the centre.
fn glyph_nodes(m: &Marker, r: RadarScale) -> Vec<Node> {
    Glyph {
        centre: m.at,
        radius: MARKER_RADIUS,
        qualifier: m.qualifier,
        mark: m.status.and_then(|s| StatusMark::of(s, m.centreward)),
        ink: Role::Series(m.series),
    }
    .draw(r.marker())
}

/// A marker's FR-34 number, centred on it: in primary text on a hollow
/// marker, else in the ink its series entry calls for (D-54).
#[expect(
    clippy::suboptimal_flops,
    reason = "NFR-4: plain * and + are bit-identical on every target; mul_add falls back to a libm fma where the CPU lacks one"
)]
fn number_node(m: &Marker, r: RadarScale) -> Node {
    let paint = if m.qualifier == Some(Qualifier::Conditional) {
        Paint::Role(Role::Neutral(Neutral::TextPrimary))
    } else {
        Paint::NumberOn(m.series)
    };
    let centre = m.centre(r);
    Node::Text {
        origin: Point {
            x: centre.x,
            y: centre.y + NUMBER_CENTRING * MARKER_NUMBER.size * r.marks,
        },
        style: MARKER_NUMBER.scaled(r.marks),
        ink: TextPaint::Fill(paint),
        anchor: Anchor::Middle,
        lines: vec![m.number.to_string()],
        leading: 0.0,
    }
}

/// A marker's pointer target (FR-94), at natural size or larger.
fn hit_node(m: &Marker, r: RadarScale) -> Node {
    Node::HitArea {
        centre: m.centre(r),
        radius: HIT_RADIUS * r.labels,
        key: m.control.key,
    }
}

/// A marker's focus indicator (FR-48): a Surface1 backing, the marker
/// `raised` over it, then the focus ring, so the ring is always seen
/// against Surface1 (SC-20, `FOCUS_RING_GAP`).
fn indicator_node(m: &Marker, raised: Vec<Node>, r: RadarScale) -> Node {
    let centre = m.centre(r);
    let backing = Node::Circle {
        centre,
        radius: INDICATOR_REACH * r.marks,
        fill: Paint::Role(Role::Neutral(Neutral::Surface1)),
        stroke: Paint::None,
        stroke_width: 0.0,
    };
    let ring = Node::Circle {
        centre,
        radius: (MARKER_RADIUS + FOCUS_RING_GAP) * r.marks,
        fill: Paint::None,
        stroke: Paint::Role(Role::Neutral(Neutral::Focus)),
        stroke_width: FOCUS_RING_STROKE * r.marks,
    };
    let children = std::iter::once(backing)
        .chain(raised)
        .chain([ring])
        .collect();
    Node::Group(Group::new(Point::ORIGIN, children).with(Semantics::Indicator(m.control.key)))
}

/// `nodes` as one plain group, or nothing when there are none.
fn layer(nodes: Vec<Node>) -> Option<Node> {
    (!nodes.is_empty()).then(|| Node::Group(Group::new(Point::ORIGIN, nodes)))
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

/// How far a hit area whose marker sits `offset` from the centre on one
/// axis reaches along it.
#[expect(
    clippy::suboptimal_flops,
    reason = "NFR-4: plain * and + are bit-identical on every target; mul_add falls back to a libm fma where the CPU lacks one"
)]
fn hit_reach(offset: f64, r: RadarScale) -> f64 {
    offset.abs() * r.geometry + HIT_RADIUS * r.labels
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
/// under the hit area's `|p| s + 16` because `INDICATOR_REACH * RADAR_FLOOR
/// <= HIT_RADIUS`; from `RADAR_FLOOR` up it reaches `(|p| + 19) s`, under
/// the floor's `330 s` because `|p| < BAND_OUTER_RADIUS` and
/// `BAND_OUTER_RADIUS + INDICATOR_REACH <= MIN_RADAR_HALF`. Both are
/// asserted at compile time, below `half_extent`. The hit term itself binds only
/// for `s < 16 / (330 - |p|)`, about 0.23 at most.
///
/// The block's size is non-decreasing in `s` from 0.0034 up. The only
/// decreasing term is a below-centre quadrant label's ascent, at most
/// 11.625; the outermost ring name's reach, at least `130 * s + 11.19`,
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
    use crate::layout::test_support::{assert_finite, bounds, fill, near, plotted, plus};
    use crate::model::Item;
    use crate::palette::{Palette, Theme};
    use crate::style::{MARKER_HOLLOW_STROKE, MARKER_STROKE, STATUS_RING_GAP, STATUS_RING_STROKE};
    use crate::svg::{PathCommand, TextPaint};
    use crate::text::Weight;

    const HEADER: &str = "name,quadrant,ring,description\n";
    const PRD_EXAMPLE: &str = include_str!("../../tests/fixtures/three-quadrant.csv");
    /// How near the block's edge the drawn labels must come: float noise.
    const TIGHT: f64 = 1e-6;

    /// One row per quadrant name, all in `ring`.
    fn csv_of(quadrants: &[&str], ring: &str) -> String {
        quadrants
            .iter()
            .enumerate()
            .fold(HEADER.to_owned(), |text, (i, q)| {
                format!("{text}item {i},{q},{ring},d\n")
            })
    }

    fn block(fonts: &Fonts, csv: &str, rings: Option<&str>) -> RadarBlock {
        let (catalogue, order) = plotted(csv, rings);
        RadarBlock::new(&catalogue, &order, fonts)
    }

    /// `n` short quadrants, `Q0`… in FR-10 order, default rings.
    fn quadrants(fonts: &Fonts, n: usize) -> RadarBlock {
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
    fn crowded(fonts: &Fonts) -> RadarBlock {
        block(fonts, &csv_of(&["Q0"; 30], "Adopt"), None)
    }

    const MARKED_HEADER: &str = "name,quadrant,ring,description,qualifier,status\n";
    const QUALIFIERS: [&str; 2] = ["", "conditional"];
    const STATUSES: [&str; 5] = ["", "No Change", "New", "Moved In", "Moved Out"];

    /// N = 4, each quadrant holding every qualifier × status pair in Trial.
    fn marked_csv() -> String {
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
                assert!(near(label.origin, polar(g.middle(i), 300.0)), "{i}");
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
        let mut names = 0;
        for (i, node) in nodes.iter().enumerate() {
            let Some(name) = text(node).filter(|t| t.ink == fill(Neutral::TextMuted)) else {
                continue;
            };
            names += 1;
            let halo = i
                .checked_sub(1)
                .and_then(|j| nodes.get(j))
                .and_then(text)
                .expect("a text precedes every ring name");
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
        let b = quadrants(fonts(), 4);
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
                assert!(
                    near(label.origin, polar(g.middle(p), 300.0 * s)),
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
            for (i, t) in texts.iter().enumerate() {
                let band = g.band(i / 2);
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

        // N = 8: middle −67.5° puts a label above the centre, so its ascent
        // sets the height once the floor (198 at 0.3) is passed.
        let s = 0.3;
        let above = 300.0 * s * 67.5_f64.to_radians().sin();
        let want = 2.0 * (TEXT_ASCENT * 12.0 + above + 10.0);
        let got = quadrants(fonts, 8).size(s).height;
        assert!(close(got, want), "{got} vs {want}");
        assert!(want > 660.0 * s && (want - 209.55).abs() < 0.01, "{want}");

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
        let w = fonts.face(QUADRANT_LABEL.weight).measure(
            &long(40),
            QUADRANT_LABEL.size,
            QUADRANT_LABEL.letter_spacing,
        );
        let b = block(fonts, &csv_of(&["B", "C", "D", &long(40)], "Adopt"), None);
        for s in [1.0, 0.576] {
            let want = 2.0 * (polar(45.0, 300.0).x.abs() * s + w / 2.0 + 10.0);
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
        // (near 0.135 for the Assess fixture), then 0.30–1.00 in 0.05.
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

    /// Every marker control among the centring group's children: its index
    /// there, its control and its children.
    fn controls(nodes: &[Node]) -> Vec<(usize, &Control, &[Node])> {
        nodes
            .iter()
            .enumerate()
            .filter_map(|(i, n)| match n {
                Node::Group(Group {
                    semantics: Semantics::Marker(c),
                    children,
                    ..
                }) => Some((i, c, children.as_slice())),
                _ => None,
            })
            .collect()
    }

    /// A marker's disc, its one filled circle: (centre, radius, fill,
    /// stroke, stroke width).
    fn disc(children: &[Node]) -> (Point, f64, Paint, Paint, f64) {
        children
            .iter()
            .find_map(|n| match n {
                Node::Circle {
                    centre,
                    radius,
                    fill,
                    stroke,
                    stroke_width,
                } if *fill != Paint::None => {
                    Some((*centre, *radius, *fill, *stroke, *stroke_width))
                }
                _ => None,
            })
            .expect("a marker has a disc")
    }

    /// A marker's number: its one text.
    fn number(children: &[Node]) -> Text<'_> {
        children
            .iter()
            .find_map(text)
            .expect("a marker has a number")
    }

    /// The hit layer: a plain group of hit areas only. Its index and children.
    fn hit_layer(nodes: &[Node]) -> Option<(usize, &[Node])> {
        nodes.iter().enumerate().find_map(|(i, n)| match n {
            Node::Group(Group {
                semantics: Semantics::None,
                children,
                ..
            }) if !children.is_empty()
                && children.iter().all(|c| matches!(c, Node::HitArea { .. })) =>
            {
                Some((i, children.as_slice()))
            }
            _ => None,
        })
    }

    /// The indicator layer: the last child, a plain group.
    fn indicator_layer(nodes: &[Node]) -> &[Node] {
        let Some(Node::Group(layer)) = nodes.last() else {
            panic!("the indicator layer is the last child");
        };
        assert_eq!(layer.semantics, Semantics::None);
        &layer.children
    }

    /// The catalogue item with CSV line `key`.
    fn item_of(catalogue: &Catalogue, key: ItemKey) -> &Item {
        catalogue
            .items
            .iter()
            .find(|i| i.line == key.0)
            .expect("a control names a row")
    }

    #[test]
    fn every_plotted_item_is_a_control() {
        let fonts = fonts();
        let check = |csv: &str| {
            let (catalogue, order) = plotted(csv, None);
            let b = RadarBlock::new(&catalogue, &order, fonts);
            let nodes = frame(&b.render(at(1.0)));
            let got = controls(&nodes);
            assert_eq!(got.len(), order.items.len());
            for (n, ((_, control, _), &index)) in got.iter().zip(&order.items).enumerate() {
                let n = n + 1;
                let item = catalogue.items.get(index).expect("in range");
                assert_eq!(control.key, ItemKey(item.line));
                assert_eq!(control.label, catalogue.accessible_name(item, Some(n)));
                assert_eq!(control.tooltip, Some(format!("{n}. {}", trim(&item.name))));
            }
            got.iter().map(|(_, c, _)| (*c).clone()).collect::<Vec<_>>()
        };
        assert_eq!(check(PRD_EXAMPLE).len(), 6);
        let kafka = check(&format!(
            "{HEADER}Zeta,Q,Adopt,d\n\"  Kafka  \",Q,Adopt,d\n"
        ));
        let tooltips: Vec<_> = kafka.iter().map(|c| c.tooltip.clone()).collect();
        assert_eq!(
            tooltips,
            [Some("1. Kafka".to_owned()), Some("2. Zeta".to_owned())]
        );
    }

    #[test]
    fn markers_sit_where_fr33_puts_them() {
        let fonts = fonts();
        let (catalogue, order) = plotted(PRD_EXAMPLE, None);
        let b = RadarBlock::new(&catalogue, &order, fonts);
        let g = Geometry::of(&catalogue);
        // FR-10: platform 0, Practice 1, Tool 2; default rings Assess 0,
        // Trial 1, Adopt 2, Retire 3; FR-34 orders names within a band.
        let anchors = [
            ("Kubernetes", g.marker(0, 2, 0, 1)),
            ("Trunk-Based Development", g.marker(1, 2, 0, 1)),
            ("Pulumi", g.marker(2, 2, 0, 2)),
            ("Terraform", g.marker(2, 2, 1, 2)),
            ("Ansible", g.marker(2, 1, 0, 1)),
            ("Chef", g.marker(2, 3, 0, 1)),
        ];
        for s in [1.0_f64, 0.5] {
            let nodes = frame_at(&b, s);
            let got = controls(&nodes);
            assert_eq!(got.len(), anchors.len());
            for (name, want) in anchors {
                let (_, _, children) = got
                    .iter()
                    .find(|(_, c, _)| item_of(&catalogue, c.key).name == name)
                    .expect("every named item is drawn");
                let (centre, ..) = disc(children);
                assert!(
                    near(centre.scaled(1.0 / s), want),
                    "{name} at {s}: {centre:?}"
                );
            }
        }
    }

    #[test]
    fn controls_carry_names_raw() {
        let csv = format!("{HEADER}\"A<&\"\">B\",Q,Adopt,d\n");
        let nodes = frame_at(&block(fonts(), &csv, None), 1.0);
        let got = controls(&nodes);
        let [(_, control, _)] = got.as_slice() else {
            panic!("one control");
        };
        assert!(control.label.contains("A<&\">B"), "{}", control.label);
        assert_eq!(control.tooltip.as_deref(), Some("1. A<&\">B"));
    }

    #[test]
    fn marker_colour_is_the_quadrant_rank() {
        let fonts = fonts();
        let csv = csv_of(&["Tool", "platform", "Practice"], "Adopt");
        let (catalogue, order) = plotted(&csv, None);
        assert_eq!(order.rank, [2, 0, 1]);
        let nodes = frame_at(&RadarBlock::new(&catalogue, &order, fonts), 1.0);
        let by_quadrant = |label: &str| {
            controls(&nodes)
                .into_iter()
                .find(|(_, c, _)| {
                    let item = item_of(&catalogue, c.key);
                    catalogue
                        .quadrants
                        .get(item.quadrant)
                        .map(|q| q.label.as_str())
                        == Some(label)
                })
                .map(|(_, _, children)| disc(children).2)
                .expect("the quadrant's item is drawn")
        };
        assert_eq!(by_quadrant("platform"), Paint::Role(Role::Series(0)));
        assert_eq!(by_quadrant("Practice"), Paint::Role(Role::Series(1)));
        assert_eq!(by_quadrant("Tool"), Paint::Role(Role::Series(2)));

        let nine = frame_at(&quadrants(fonts, 9), 1.0);
        let last = controls(&nine)
            .last()
            .map(|(_, _, children)| disc(children).2);
        assert_eq!(last, Some(Paint::Role(Role::Series(8))), "unreduced");
    }

    /// A half-ring's (start, end).
    fn half_ring_ends(node: &Node) -> (Point, Point) {
        let Node::Path { commands, .. } = node else {
            panic!("expected a half-ring, got {node:?}");
        };
        let [PathCommand::MoveTo(start), PathCommand::Arc { end, .. }] = commands.as_slice() else {
            panic!("unexpected commands {commands:?}");
        };
        (*start, *end)
    }

    /// FR-20: a solid disc fills with `ink` inside a Surface1 stroke; a
    /// hollow one fills with `MarkerHollow` inside an `ink` stroke.
    fn assert_disc(item: &Item, children: &[Node], ink: Paint, marks: f64) {
        let (_, radius, fill, stroke, width) = disc(children);
        assert!(close(radius, MARKER_RADIUS * marks), "{}", item.name);
        let want = if item.hollow() {
            (
                Paint::Role(Role::Neutral(Neutral::MarkerHollow)),
                ink,
                MARKER_HOLLOW_STROKE,
            )
        } else {
            (
                ink,
                Paint::Role(Role::Neutral(Neutral::Surface1)),
                MARKER_STROKE,
            )
        };
        assert_eq!((fill, stroke), (want.0, want.1), "{}", item.name);
        assert!(close(width, want.2 * marks), "{}", item.name);
    }

    /// FR-40: none, a full ring, or a half-ring toward (`MovedIn`) or away
    /// from (`MovedOut`) the centre, under the disc and number.
    fn assert_status(item: &Item, children: &[Node], ink: Paint, marks: f64) {
        let ring = (MARKER_RADIUS + STATUS_RING_GAP) * marks;
        let c = disc(children).0;
        let centreward = (-c.y).atan2(-c.x).to_degrees();
        let toward = plus(c, polar(centreward, ring));
        assert!(toward.x.hypot(toward.y) < c.x.hypot(c.y), "{}", item.name);
        let ends_at = |path: &Node, start: f64, end: f64| {
            let (s, e) = half_ring_ends(path);
            assert!(
                near(s, plus(c, polar(centreward + start, ring))),
                "{}",
                item.name
            );
            assert!(
                near(e, plus(c, polar(centreward + end, ring))),
                "{}",
                item.name
            );
        };
        match (item.status, children) {
            (None | Some(Status::NoChange), [Node::Circle { .. }, Node::Text { .. }]) => {}
            (
                Some(Status::New),
                [
                    Node::Circle {
                        centre,
                        radius,
                        fill: Paint::None,
                        stroke,
                        stroke_width,
                    },
                    Node::Circle { .. },
                    Node::Text { .. },
                ],
            ) => {
                assert!(near(*centre, c) && close(*radius, ring));
                assert_eq!(*stroke, ink);
                assert!(close(*stroke_width, STATUS_RING_STROKE * marks));
            }
            (Some(Status::MovedIn), [path, Node::Circle { .. }, Node::Text { .. }]) => {
                ends_at(path, -90.0, 90.0);
            }
            (Some(Status::MovedOut), [path, Node::Circle { .. }, Node::Text { .. }]) => {
                ends_at(path, 90.0, 270.0);
            }
            (status, other) => panic!("{}: {status:?} drew {other:?}", item.name),
        }
    }

    #[test]
    fn qualifier_and_status_draw_per_fr20_fr40() {
        let fonts = fonts();
        let (catalogue, order) = plotted(&marked_csv(), None);
        let b = RadarBlock::new(&catalogue, &order, fonts);
        for s in [1.0_f64, 0.5] {
            let marks = s.max(RADAR_FLOOR);
            let nodes = frame_at(&b, s);
            let got = controls(&nodes);
            assert_eq!(got.len(), 40);
            let mut positions = std::collections::BTreeSet::new();
            for (_, control, children) in got {
                let item = item_of(&catalogue, control.key);
                let rank = *order.rank.get(item.quadrant).expect("ranked");
                positions.insert(rank);
                let ink = Paint::Role(Role::Series(rank));
                assert_disc(item, children, ink, marks);
                assert_status(item, children, ink, marks);
            }
            assert_eq!(positions.len(), 4, "every quadrant position of N = 4");
        }
    }

    #[test]
    fn numbers_name_the_entry_they_sit_on() {
        let fonts = fonts();
        let b = quadrants(fonts, 6);
        for (s, offset) in [(1.0_f64, 3.27393), (0.4, 2.54639)] {
            let marks = s.max(RADAR_FLOOR);
            let nodes = frame_at(&b, s);
            let got = controls(&nodes);
            assert_eq!(got.len(), 6);
            for (n, (_, _, children)) in got.iter().enumerate() {
                let t = number(children);
                assert_eq!(t.ink, TextPaint::Fill(Paint::NumberOn(n)), "rank {n}");
                assert_eq!(t.lines, [(n + 1).to_string()]);
                assert_eq!(t.style, MARKER_NUMBER.scaled(marks));
                assert_eq!(t.anchor, Anchor::Middle);
                let (c, ..) = disc(children);
                assert!(close(t.origin.x, c.x));
                assert!(
                    (t.origin.y - c.y - offset).abs() < 1e-5,
                    "{s}: {:?}",
                    t.origin
                );
                assert!(close(
                    t.origin.y - c.y,
                    NUMBER_CENTRING * MARKER_NUMBER.size * marks
                ));
            }
        }
        // Rank 5 is the green entry: its number binds to the dark ink.
        for theme in [Theme::Light, Theme::Dark] {
            assert_eq!(Palette::built_in(theme).number_ink(5), "#16191F");
        }
    }

    #[test]
    fn hollow_markers_number_in_primary_text() {
        let csv = format!("{MARKED_HEADER}a,Q,Adopt,d,conditional,\nb,Q,Adopt,d,default,\n");
        let nodes = frame_at(&block(fonts(), &csv, None), 1.0);
        let inks: Vec<TextPaint> = controls(&nodes)
            .iter()
            .map(|(_, _, children)| number(children).ink)
            .collect();
        assert_eq!(
            inks,
            [
                fill(Neutral::TextPrimary),
                TextPaint::Fill(Paint::NumberOn(0))
            ]
        );
    }

    /// What `marker_scales` measures on the first marker of a frame.
    struct Probe<'a> {
        centre: Point,
        disc: f64,
        number: Text<'a>,
        hit: f64,
        focus: f64,
    }

    fn probe(nodes: &[Node]) -> Probe<'_> {
        let (_, _, children) = *controls(nodes).first().expect("markers");
        let (centre, disc, ..) = disc(children);
        let Some((_, [Node::HitArea { radius: hit, .. }, ..])) = hit_layer(nodes) else {
            panic!("a hit layer");
        };
        let Some(Node::Group(indicator)) = indicator_layer(nodes).first() else {
            panic!("an indicator");
        };
        let Some(Node::Circle { radius: focus, .. }) = indicator.children.last() else {
            panic!("a focus ring");
        };
        Probe {
            centre,
            disc,
            number: number(children),
            hit: *hit,
            focus: *focus,
        }
    }

    #[test]
    fn marker_scales() {
        let b = block(fonts(), PRD_EXAMPLE, None);
        // (s, disc, number, hit, indicator ring, number offset)
        let printed = [
            (0.4, 7.7778, 7.0, 16.0, 14.0, 2.54639),
            (1.0, 10.0, 9.0, 16.0, 18.0, 3.27393),
        ];
        for (s, d, size, h, ring, offset) in printed {
            let nodes = frame_at(&b, s);
            let p = probe(&nodes);
            assert!((p.disc - d).abs() < 1e-4, "{s}: disc {}", p.disc);
            assert!(
                close(p.number.style.size, size),
                "{s}: number {}",
                p.number.style.size
            );
            assert!(close(p.hit, h), "{s}: hit {}", p.hit);
            assert!(close(p.focus, ring), "{s}: ring {}", p.focus);
            assert!(
                (p.number.origin.y - p.centre.y - offset).abs() < 1e-5,
                "{s}: offset"
            );
        }
        for s in [RADAR_FLOOR, 1.5] {
            let (marks, labels) = (s.max(RADAR_FLOOR), s.max(1.0));
            let nodes = frame_at(&b, s);
            let p = probe(&nodes);
            assert!(close(p.disc, MARKER_RADIUS * marks), "{s}");
            assert!(
                close(p.number.style.size, MARKER_NUMBER.size * marks),
                "{s}"
            );
            assert!(close(p.hit, HIT_RADIUS * labels), "{s}");
            assert!(
                close(p.focus, (MARKER_RADIUS + FOCUS_RING_GAP) * marks),
                "{s}"
            );
            assert!(
                close(
                    p.number.origin.y - p.centre.y,
                    NUMBER_CENTRING * MARKER_NUMBER.size * marks
                ),
                "{s}"
            );
        }
    }

    #[test]
    fn hit_layer_paints_before_every_marker() {
        let nodes = frame_at(&block(fonts(), PRD_EXAMPLE, None), 1.0);
        let (index, hits) = hit_layer(&nodes).expect("a hit layer");
        let got = controls(&nodes);
        assert_eq!(hits.len(), got.len());
        for (hit, (i, control, children)) in hits.iter().zip(&got) {
            assert!(index < *i, "the hit layer paints before control {i}");
            let Node::HitArea {
                centre,
                radius,
                key,
            } = hit
            else {
                panic!("expected a hit area, got {hit:?}");
            };
            assert!(close(*radius, HIT_RADIUS));
            assert!(near(*centre, disc(children).0));
            assert_eq!(*key, control.key);
        }
    }

    #[test]
    fn indicators_raise_the_marker_over_surface() {
        let b = block(fonts(), &marked_csv(), None);
        for s in [1.0_f64, 0.5] {
            let marks = s.max(RADAR_FLOOR);
            let nodes = frame_at(&b, s);
            let got = controls(&nodes);
            let indicators = indicator_layer(&nodes);
            assert_eq!(indicators.len(), got.len());
            for (indicator, (_, control, children)) in indicators.iter().zip(&got) {
                let Node::Group(group) = indicator else {
                    panic!("an indicator is a group: {indicator:?}");
                };
                assert_eq!(group.semantics, Semantics::Indicator(control.key));
                assert_eq!(group.offset, Point::ORIGIN);
                let c = disc(children).0;
                let [
                    Node::Circle {
                        centre: backing_at,
                        radius: backing,
                        fill: Paint::Role(Role::Neutral(Neutral::Surface1)),
                        stroke: Paint::None,
                        stroke_width: backing_width,
                    },
                    raised @ ..,
                    Node::Circle {
                        centre: ring_at,
                        radius: ring,
                        fill: Paint::None,
                        stroke: Paint::Role(Role::Neutral(Neutral::Focus)),
                        stroke_width: ring_width,
                    },
                ] = group.children.as_slice()
                else {
                    panic!("backing, raised marker, ring: {:?}", group.children);
                };
                assert!(near(*backing_at, c) && near(*ring_at, c));
                assert!(close(*backing, INDICATOR_REACH * marks));
                assert!(close(*backing_width, 0.0));
                assert_eq!(raised, *children, "the raised copy is the marker");
                assert!(close(*ring, (MARKER_RADIUS + FOCUS_RING_GAP) * marks));
                assert!(close(*ring_width, FOCUS_RING_STROKE * marks));
            }
        }
    }

    #[test]
    fn a_crowded_band_draws_every_marker() {
        let nodes = frame_at(&crowded(fonts()), 1.0);
        let got = controls(&nodes);
        assert_eq!(got.len(), 30);
        assert_eq!(hit_layer(&nodes).map(|(_, h)| h.len()), Some(30));
        assert_eq!(indicator_layer(&nodes).len(), 30);
        let numbers: Vec<String> = got
            .iter()
            .map(|(_, _, children)| number(children).lines.concat())
            .collect();
        let want: Vec<String> = (1..=30).map(|n: usize| n.to_string()).collect();
        assert_eq!(numbers, want);
    }

    #[test]
    fn unplotted_rows_draw_nothing() {
        let csv = format!("{HEADER}Zed,Q,Adopt,d\nMid,Q,Nowhere,d\nAlpha,Q,Adopt,d\n");
        let (catalogue, order) = plotted(&csv, None);
        let nodes = frame_at(&RadarBlock::new(&catalogue, &order, fonts()), 1.0);
        let got = controls(&nodes);
        let keys: Vec<ItemKey> = got.iter().map(|(_, c, _)| c.key).collect();
        assert_eq!(keys, [ItemKey(4), ItemKey(2)]);
        let numbers: Vec<String> = got
            .iter()
            .map(|(_, _, children)| number(children).lines.concat())
            .collect();
        assert_eq!(numbers, ["1", "2"]);
        assert_eq!(hit_layer(&nodes).map(|(_, h)| h.len()), Some(2));
        assert_eq!(indicator_layer(&nodes).len(), 2);
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
        assert!((size.width - 74.5595).abs() < 1e-3, "{}", size.width);
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
