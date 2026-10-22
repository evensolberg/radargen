//! The radar's markers (FR-20, FR-33, FR-34, FR-36, FR-40, FR-48, FR-50,
//! FR-94): one per plotted item, and the nodes each draws: its glyph and
//! number, its pointer target and its focus indicator.

use crate::layout::geometry::place;
use crate::layout::marker::{Glyph, StatusMark, number_paint, number_text};
use crate::layout::radar::RadarScale;
use crate::model::{Catalogue, Qualifier, Status};
use crate::order::Plotted;
use crate::palette::{Neutral, Role};
use crate::style::{
    FOCUS_RING_GAP, FOCUS_RING_STROKE, HIT_RADIUS, INDICATOR_REACH, MARKER_NUMBER, MARKER_RADIUS,
};
use crate::svg::{Control, Group, ItemKey, Node, Paint, Point, Semantics};

/// One plotted item, ready to draw.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Marker {
    /// FR-33's centre, at scale 1, relative to the radar's centre.
    pub(super) at: Point,
    /// Solid or hollow (FR-20).
    qualifier: Option<Qualifier>,
    /// The status ring, if any (FR-40).
    status: Option<Status>,
    /// The sector's FR-10 rank: its series index, unreduced (FR-67).
    series: usize,
    /// FR-34's number.
    number: usize,
    /// Degrees (SVG frame) from the marker toward the radar's centre.
    centreward: f64,
    /// Its key, accessible name (FR-50) and tooltip (FR-36).
    pub(super) control: Control,
}

impl Marker {
    /// The marker's centre at `r`'s position scale, relative to the radar's
    /// centre.
    pub(super) fn centre(&self, r: RadarScale) -> Point {
        self.at.scaled(r.geometry)
    }
}

/// Every plotted item's marker, in FR-34's order: `place`'s points zipped
/// with [`Plotted::items`], numbered from 1 (FR-34).
///
/// # Panics
///
/// If `plotted` was not built from `catalogue`, so an item or its sector
/// is out of range for it, or `place` returns a different number of points
/// than there are plotted items.
// qual:allow(complexity, error_handling) reason: "the documented # Panics: a Plotted from another catalogue is a caller bug"
pub(super) fn markers(catalogue: &Catalogue, plotted: &Plotted) -> Vec<Marker> {
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
                .get(item.sector)
                .expect("rank covers every sector of the catalogue");
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
                    tooltip: Some(format!("{number}. {}", item.name)),
                },
            }
        })
        .collect()
}

/// A marker's glyph, then its number on top.
pub(super) fn marker_nodes(m: &Marker, r: RadarScale) -> Vec<Node> {
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

/// A marker's FR-34 number, centred on it.
fn number_node(m: &Marker, r: RadarScale) -> Node {
    let hollow = m.qualifier.is_some_and(Qualifier::is_hollow);
    number_text(
        m.centre(r),
        MARKER_NUMBER,
        number_paint(hollow, m.series),
        m.number,
        r.marks,
    )
}

/// A marker's pointer target (FR-94), at natural size or larger.
pub(super) fn hit_node(m: &Marker, r: RadarScale) -> Node {
    Node::HitArea {
        centre: m.centre(r),
        radius: HIT_RADIUS * r.labels,
        key: m.control.key,
    }
}

/// A marker's focus indicator (FR-48): a Surface1 backing, the marker
/// `raised` over it, then the focus ring, so the ring is always seen
/// against Surface1 (SC-20, `FOCUS_RING_GAP`).
pub(super) fn indicator_node(m: &Marker, raised: Vec<Node>, r: RadarScale) -> Node {
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
    let selection = selection_node(m, centre, r);
    // Marked, so the page shows the ring apart from the rest (rg-jri).
    let ring = Node::Group(
        Group::new(Point::ORIGIN, vec![ring]).with(Semantics::FocusRing(m.control.key)),
    );
    let children = std::iter::once(backing)
        .chain(raised)
        .chain([selection, ring])
        .collect();
    Node::Group(Group::new(Point::ORIGIN, children).with(Semantics::Indicator(m.control.key)))
}

/// Half the selection square's side: its stroke's outer edge is the
/// indicator's reach, so the square adds nothing to what layout reserves.
const SELECTION_HALF: f64 = INDICATOR_REACH - FOCUS_RING_STROKE / 2.0;
/// The square's corner, small enough that it never reads as a ring, which
/// would be taken for a status (FR-40, FR-73).
const SELECTION_CORNER: f64 = MARKER_RADIUS / 4.0;
const _: () = assert!(
    SELECTION_CORNER < SELECTION_HALF / 2.0,
    "a selection square's corners stay under a quarter of its side"
);

/// A marker's selection mark (FR-72, FR-73): a rounded square in the text
/// colour, shown by the page while the item is selected, under the ring so
/// a focused, selected marker keeps its ring whole.
fn selection_node(m: &Marker, centre: Point, r: RadarScale) -> Node {
    let half = SELECTION_HALF * r.marks;
    let square = Node::Rect {
        origin: Point {
            x: centre.x - half,
            y: centre.y - half,
        },
        width: 2.0 * half,
        height: 2.0 * half,
        radius: SELECTION_CORNER * r.marks,
        fill: Paint::None,
        stroke: Paint::Role(Role::Neutral(Neutral::TextPrimary)),
        stroke_width: FOCUS_RING_STROKE * r.marks,
    };
    Node::Group(Group::new(Point::ORIGIN, vec![square]).with(Semantics::Selection(m.control.key)))
}

/// `nodes` as one plain group, or nothing when there are none.
pub(super) fn layer(nodes: Vec<Node>) -> Option<Node> {
    (!nodes.is_empty()).then(|| Node::Group(Group::new(Point::ORIGIN, nodes)))
}

/// How far a hit area whose marker sits `offset` from the centre on one
/// axis reaches along it.
#[expect(
    clippy::suboptimal_flops,
    reason = "NFR-4: plain * and + are bit-identical on every target; mul_add falls back to a libm fma where the CPU lacks one"
)]
pub(super) fn hit_reach(offset: f64, r: RadarScale) -> f64 {
    offset.abs() * r.geometry + HIT_RADIUS * r.labels
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::block::{Block, at, close, fonts};
    use crate::layout::geometry::{Geometry, polar};
    use crate::layout::radar::RadarBlock;
    use crate::layout::radar::tests::{
        Text, block, crowded, frame, frame_at, marked_csv, sectors, text,
    };
    use crate::layout::test_support::{
        HEAD, HEAD_META, PRD_EXAMPLE, csv_of, fill, near, plotted, plus,
    };
    use crate::model::Item;
    use crate::palette::{Palette, Theme};
    use crate::style::{
        MARKER_HOLLOW_STROKE, MARKER_STROKE, RADAR_FLOOR, STATUS_RING_GAP, STATUS_RING_STROKE,
    };
    use crate::svg::PathCommand;
    use crate::svg::{Anchor, TextPaint};
    use crate::text::NUMBER_CENTRING;

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
                assert_eq!(control.tooltip, Some(format!("{n}. {}", item.name)));
            }
            got.iter().map(|(_, c, _)| (*c).clone()).collect::<Vec<_>>()
        };
        assert_eq!(check(PRD_EXAMPLE).len(), 6);
        let kafka = check(&format!("{HEAD}Zeta,Q,Adopt,d\n\"  Kafka  \",Q,Adopt,d\n"));
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
        let csv = format!("{HEAD}\"A<&\"\">B\",Q,Adopt,d\n");
        let nodes = frame_at(&block(fonts(), &csv, None), 1.0);
        let got = controls(&nodes);
        let [(_, control, _)] = got.as_slice() else {
            panic!("one control");
        };
        assert!(control.label.contains("A<&\">B"), "{}", control.label);
        assert_eq!(control.tooltip.as_deref(), Some("1. A<&\">B"));
    }

    #[test]
    fn marker_colour_is_the_sector_rank() {
        let fonts = fonts();
        let csv = csv_of(&["Tool", "platform", "Practice"], "Adopt");
        let (catalogue, order) = plotted(&csv, None);
        assert_eq!(order.rank, [2, 0, 1]);
        let nodes = frame_at(&RadarBlock::new(&catalogue, &order, fonts), 1.0);
        let by_sector = |label: &str| {
            controls(&nodes)
                .into_iter()
                .find(|(_, c, _)| {
                    let item = item_of(&catalogue, c.key);
                    catalogue.sectors.get(item.sector).map(|q| q.label.as_str()) == Some(label)
                })
                .map(|(_, _, children)| disc(children).2)
                .expect("the sector's item is drawn")
        };
        assert_eq!(by_sector("platform"), Paint::Role(Role::Series(0)));
        assert_eq!(by_sector("Practice"), Paint::Role(Role::Series(1)));
        assert_eq!(by_sector("Tool"), Paint::Role(Role::Series(2)));

        let nine = frame_at(&sectors(fonts, 9), 1.0);
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
                let rank = *order.rank.get(item.sector).expect("ranked");
                positions.insert(rank);
                let ink = Paint::Role(Role::Series(rank));
                assert_disc(item, children, ink, marks);
                assert_status(item, children, ink, marks);
            }
            assert_eq!(positions.len(), 4, "every sector position of N = 4");
        }
    }

    #[test]
    fn numbers_name_the_entry_they_sit_on() {
        let fonts = fonts();
        let b = sectors(fonts, 6);
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
        let csv = format!("{HEAD_META}a,Q,Adopt,d,conditional,\nb,Q,Adopt,d,default,\n");
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
        let Some(Node::Group(ring)) = indicator.children.last() else {
            panic!("a focus ring group");
        };
        let [Node::Circle { radius: focus, .. }] = ring.children.as_slice() else {
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
            (0.4, 7.7778, 7.0, 18.0, 14.0, 2.54639),
            (1.0, 10.0, 9.0, 18.0, 18.0, 3.27393),
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
                    Node::Group(selection),
                    Node::Group(marked),
                ] = group.children.as_slice()
                else {
                    panic!(
                        "backing, raised marker, selection, ring: {:?}",
                        group.children
                    );
                };
                // FR-73: selection is a rounded square, not a ring, inside the
                // indicator's reach, drawn before the ring so it never breaks it.
                assert_eq!(selection.semantics, Semantics::Selection(control.key));
                let [
                    Node::Rect {
                        origin: square_at,
                        width: side,
                        height: side_y,
                        radius: corner,
                        fill: Paint::None,
                        stroke: Paint::Role(Role::Neutral(Neutral::TextPrimary)),
                        stroke_width: square_width,
                    },
                ] = selection.children.as_slice()
                else {
                    panic!("the square alone: {:?}", selection.children);
                };
                let half = (INDICATOR_REACH - FOCUS_RING_STROKE / 2.0) * marks;
                assert!(close(*side, 2.0 * half) && close(*side_y, 2.0 * half));
                assert!(near(
                    Point {
                        x: square_at.x + half,
                        y: square_at.y + half
                    },
                    c
                ));
                assert!(close(*corner, MARKER_RADIUS / 4.0 * marks));
                assert!(*corner < *side / 4.0, "never read as a ring");
                assert!(close(*square_width, FOCUS_RING_STROKE * marks));
                assert!(
                    close(half + *square_width / 2.0, INDICATOR_REACH * marks),
                    "the square's outer edge is the indicator's reach"
                );
                // rg-jri: the ring is marked, so the page can show it apart
                // from the rest of the indicator.
                assert_eq!(marked.semantics, Semantics::FocusRing(control.key));
                let [
                    Node::Circle {
                        centre: ring_at,
                        radius: ring,
                        fill: Paint::None,
                        stroke: Paint::Role(Role::Neutral(Neutral::Focus)),
                        stroke_width: ring_width,
                    },
                ] = marked.children.as_slice()
                else {
                    panic!("the ring alone: {:?}", marked.children);
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
        let csv = format!("{HEAD}Zed,Q,Adopt,d\nMid,Q,Nowhere,d\nAlpha,Q,Adopt,d\n");
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
}
