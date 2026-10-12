//! The legend block (FR-35, FR-46, FR-50, FR-74, FR-75, FR-79, FR-94).
//!
//! A `Surface2` panel holding the laid-out headings, swatches and rows,
//! each row a control. Its layout, as data, lives in the `layout`
//! submodule, and one row's composition in `row`.

mod layout;
mod row;

use std::num::NonZeroU8;

use crate::layout::block::{Block, Placement, Size};
use crate::layout::marker::{number_paint, number_text};
use crate::model::Catalogue;
use crate::order::Plotted;
use crate::palette::{Neutral, Role};
use crate::style::{
    BADGE_HOLLOW_STROKE, BADGE_RADIUS, BADGE_STROKE, INDENT_ITEM, LEGEND_BADGE_NUMBER,
    LEGEND_PANEL_INSET_X, LEGEND_PANEL_INSET_Y, LEGEND_PANEL_RADIUS, LEGEND_ROW_INDICATOR_INSET,
    LEGEND_ROW_LEFT_OUTSET, LEGEND_ROW_RADIUS, LEGEND_ROW_RIGHT_OUTSET, SWATCH_RADIUS,
};
use crate::svg::{Group, ItemKey, Node, Paint, Point, Scale, Semantics, TextRun};
use crate::text::Fonts;

use layout::{Entry, Layout, layout};
use row::Row;

/// The panel's colour, and the row targets': invisible on it, yet they
/// receive the pointer in every format (a `none` fill would not).
const SURFACE2: Paint = Paint::Role(Role::Neutral(Neutral::Surface2));

/// The legend (FR-35): drawn linearly at any scale, and at scale 1 in both
/// arrangements (D-31).
#[derive(Debug, Clone, PartialEq)]
pub struct LegendBlock {
    layout: Layout,
    /// The panel at scale 1; zero for an empty legend (FR-42).
    size: Size,
}

impl LegendBlock {
    /// The legend for `catalogue`, measured and wrapped once with `fonts`.
    ///
    /// # Panics
    ///
    /// If `plotted` was not built from `catalogue`, so an item, quadrant or
    /// ring index is out of range for it.
    #[must_use]
    pub fn new(catalogue: &Catalogue, plotted: &Plotted, fonts: &Fonts) -> Self {
        let layout = layout(catalogue, plotted, fonts);
        Self {
            size: panel_size(&layout),
            layout,
        }
    }
}

impl Block for LegendBlock {
    fn size(&self, scale: f64) -> Size {
        self.size.scaled(scale)
    }

    /// The panel, then the content inset in it, entries in reading order,
    /// which is document and Tab order. An empty legend is an empty group.
    fn render(&self, at: Placement) -> Node {
        let s = at.scale;
        let children = if self.size.is_empty() {
            Vec::new()
        } else {
            vec![
                panel_node(self.size.scaled(s), s),
                content_node(&self.layout, s),
            ]
        };
        Node::Group(Group::new(at.origin, children))
    }
}

/// The panel at scale 1: the content plus its insets, or zero when there
/// is no content (FR-42: with no quadrants the legend is omitted).
#[expect(
    clippy::suboptimal_flops,
    reason = "NFR-4: plain * and + are bit-identical on every target; mul_add falls back to a libm fma where the CPU lacks one"
)]
// qual:allow(iosp) reason: "Size::default is the derived zero, not a step of ours; one FR-42 branch over arithmetic"
// qual:allow(complexity, magic_numbers) reason: "the panel insets the content on both sides"
fn panel_size(layout: &Layout) -> Size {
    if layout.entries.is_empty() {
        Size::default()
    } else {
        Size {
            width: layout.width + 2.0 * LEGEND_PANEL_INSET_X,
            height: layout.height + 2.0 * LEGEND_PANEL_INSET_Y,
        }
    }
}

/// An axis-aligned rectangle's frame.
#[derive(Debug, Clone, Copy)]
struct Area {
    origin: Point,
    width: f64,
    height: f64,
}

impl Area {
    /// The frame multiplied by `s`.
    fn scaled(self, s: f64) -> Self {
        Self {
            origin: self.origin.scaled(s),
            width: self.width * s,
            height: self.height * s,
        }
    }

    /// The frame shrunk by `by` on every side.
    #[expect(
        clippy::suboptimal_flops,
        reason = "NFR-4: plain * and + are bit-identical on every target; mul_add falls back to a libm fma where the CPU lacks one"
    )]
    // qual:allow(complexity, magic_numbers) reason: "an inset is taken from both sides of a frame"
    fn inset(self, by: f64) -> Self {
        Self {
            origin: Point {
                x: self.origin.x + by,
                y: self.origin.y + by,
            },
            width: self.width - 2.0 * by,
            height: self.height - 2.0 * by,
        }
    }
}

/// `area` as a rectangle with `radius` corners, filled with `fill`,
/// unstroked.
const fn rect_node(area: Area, radius: f64, fill: Paint) -> Node {
    Node::Rect {
        origin: area.origin,
        width: area.width,
        height: area.height,
        radius,
        fill,
        stroke: Paint::None,
        stroke_width: 0.0,
    }
}

/// The panel, `size` already scaled by `s`.
fn panel_node(size: Size, s: f64) -> Node {
    let area = Area {
        origin: Point::ORIGIN,
        width: size.width,
        height: size.height,
    };
    rect_node(area, LEGEND_PANEL_RADIUS * s, SURFACE2)
}

/// Every entry at `s`, in the layout's order, inset in the panel.
fn content_node(layout: &Layout, s: f64) -> Node {
    let offset = Point {
        x: LEGEND_PANEL_INSET_X,
        y: LEGEND_PANEL_INSET_Y,
    }
    .scaled(s);
    let children = layout
        .entries
        .iter()
        .map(|entry| entry_node(entry, layout.width, s))
        .collect();
    Node::Group(Group::new(offset, children))
}

/// One entry at `s`; `width` is the content's, which a row's target spans.
fn entry_node(entry: &Entry, width: f64, s: f64) -> Node {
    match entry {
        Entry::Heading { run, level } => heading_node(run, *level, s),
        Entry::Swatch { centre, series } => swatch_node(*centre, *series, s),
        Entry::Row(row) => row_node(row, width, s),
    }
}

/// A heading at its level (2 "Legend", 3 a quadrant, 4 a ring).
fn heading_node(run: &TextRun, level: NonZeroU8, s: f64) -> Node {
    let text = run.node(Scale::uniform(s));
    Node::Group(Group::new(Point::ORIGIN, vec![text]).with(Semantics::Heading(level)))
}

/// A quadrant's colour swatch (FR-67): ornament, since its heading names
/// the quadrant.
fn swatch_node(centre: Point, series: usize, s: f64) -> Node {
    let circle = Node::Circle {
        centre: centre.scaled(s),
        radius: SWATCH_RADIUS * s,
        fill: Paint::Role(Role::Series(series)),
        stroke: Paint::None,
        stroke_width: 0.0,
    };
    Node::Group(Group::new(Point::ORIGIN, vec![circle]).with(Semantics::Decorative))
}

/// One row as a control (FR-50, FR-75): its target, its indicator, its
/// badge and number, its name and meta.
fn row_node(row: &Row, width: f64, s: f64) -> Node {
    let scale = Scale::uniform(s);
    let target = target(row, width).scaled(s);
    let children = [
        rect_node(target, LEGEND_ROW_RADIUS * s, SURFACE2),
        indicator_node(target, row.control.key, s),
        badge_node(row, s),
        number_node(row, s),
        row.name.node(scale),
    ]
    .into_iter()
    .chain(row.meta.as_ref().map(|meta| meta.node(scale)))
    .collect();
    Node::Group(Group::new(Point::ORIGIN, children).with(Semantics::LegendRow(row.control.clone())))
}

/// A row's target at scale 1 (FR-94): from just left of the badge to just
/// past the content's widest line, the row's full height.
fn target(row: &Row, width: f64) -> Area {
    let left = INDENT_ITEM - LEGEND_ROW_LEFT_OUTSET;
    Area {
        origin: Point {
            x: left,
            y: row.top,
        },
        width: width + LEGEND_ROW_RIGHT_OUTSET - left,
        height: row.height,
    }
}

/// The row's focus and selection indicator (FR-48, FR-73): a rectangle
/// inside `target` (already scaled by `s`), painted by the interactive
/// layer. Rows abut, so a stroke on the target would be covered by the
/// next row.
#[expect(
    clippy::suboptimal_flops,
    reason = "NFR-4: plain * and + are bit-identical on every target; mul_add falls back to a libm fma where the CPU lacks one"
)]
fn indicator_node(target: Area, key: ItemKey, s: f64) -> Node {
    let inset = LEGEND_ROW_INDICATOR_INSET * s;
    let ring = rect_node(
        target.inset(inset),
        LEGEND_ROW_RADIUS * s - inset,
        Paint::None,
    );
    Node::Group(Group::new(Point::ORIGIN, vec![ring]).with(Semantics::Indicator(key)))
}

/// The row's badge, drawn as its marker is (FR-20): solid in the series
/// colour, or hollow with a heavier series outline.
fn badge_node(row: &Row, s: f64) -> Node {
    let series = Paint::Role(Role::Series(row.series));
    let (fill, stroke_width) = if row.hollow {
        (
            Paint::Role(Role::Neutral(Neutral::MarkerHollow)),
            BADGE_HOLLOW_STROKE,
        )
    } else {
        (series, BADGE_STROKE)
    };
    Node::Circle {
        centre: row.badge.scaled(s),
        radius: BADGE_RADIUS * s,
        fill,
        stroke: series,
        stroke_width: stroke_width * s,
    }
}

/// The row's FR-34 number, centred on its badge.
fn number_node(row: &Row, s: f64) -> Node {
    number_text(
        row.badge.scaled(s),
        LEGEND_BADGE_NUMBER,
        number_paint(row.hollow, row.series),
        row.number,
        s,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::block::{assert_translates, at, close, fonts};
    use crate::layout::test_support::{
        HEAD_META, NAME_FULL_LINE, NAME_THREE_LINES, NAME_TWO_LINES, PRD_EXAMPLE, Rect,
        assert_finite, assert_inside, bounds, empty_catalogue, fill, near, plotted,
    };
    use crate::style::{MIN_PX_PER_UNIT, MIN_TARGET_PX};
    use crate::svg::{Anchor, Control, TextPaint};
    use crate::text::NUMBER_CENTRING;

    fn legend(csv: &str) -> (Catalogue, Plotted, LegendBlock) {
        let (catalogue, order) = plotted(csv, None);
        let block = LegendBlock::new(&catalogue, &order, fonts());
        (catalogue, order, block)
    }

    /// The legend for `csv`, drawn at scale 1.
    fn rendered(csv: &str) -> Node {
        let (_, _, block) = legend(csv);
        block.render(at(1.0))
    }

    /// A row's children, in order: its target, focus indicator, badge,
    /// number, name, and meta when it shows one.
    const TARGET: usize = 0;
    const INDICATOR: usize = 1;
    const BADGE: usize = 2;
    const NUMBER: usize = 3;
    const NAME: usize = 4;
    const META: usize = 5;

    fn group(node: &Node) -> &Group {
        let Node::Group(group) = node else {
            panic!("expected a group, got {node:?}");
        };
        group
    }

    fn child(nodes: &[Node], i: usize) -> &Node {
        nodes
            .get(i)
            .unwrap_or_else(|| panic!("no child {i} in {nodes:?}"))
    }

    /// The content group: the root's second child.
    fn content(root: &Node) -> &Group {
        group(child(&group(root).children, 1))
    }

    /// Every group in document order, depth first.
    fn groups(node: &Node) -> Vec<&Group> {
        match node {
            Node::Group(g) => std::iter::once(g)
                .chain(g.children.iter().flat_map(groups))
                .collect(),
            _ => Vec::new(),
        }
    }

    /// Every legend row's control and children, in document order.
    fn rows(root: &Node) -> Vec<(&Control, &[Node])> {
        groups(root)
            .into_iter()
            .filter_map(|g| match &g.semantics {
                Semantics::LegendRow(c) => Some((c, g.children.as_slice())),
                _ => None,
            })
            .collect()
    }

    /// A rect's (origin, width, height, radius, fill, stroke).
    fn rect(node: &Node) -> (Point, f64, f64, f64, Paint, Paint) {
        let Node::Rect {
            origin,
            width,
            height,
            radius,
            fill,
            stroke,
            ..
        } = node
        else {
            panic!("expected a rect, got {node:?}");
        };
        (*origin, *width, *height, *radius, *fill, *stroke)
    }

    /// A circle's (centre, radius, fill, stroke, stroke width).
    fn circle(node: &Node) -> (Point, f64, Paint, Paint, f64) {
        let Node::Circle {
            centre,
            radius,
            fill,
            stroke,
            stroke_width,
        } = node
        else {
            panic!("expected a circle, got {node:?}");
        };
        (*centre, *radius, *fill, *stroke, *stroke_width)
    }

    /// A text's (origin, style size, ink, anchor, lines).
    fn text(node: &Node) -> (Point, f64, TextPaint, Anchor, &[String]) {
        let Node::Text {
            origin,
            style,
            ink,
            anchor,
            lines,
            ..
        } = node
        else {
            panic!("expected text, got {node:?}");
        };
        (*origin, style.size, *ink, *anchor, lines.as_slice())
    }

    fn encloses(outer: Rect, inner: Rect) -> bool {
        inner.left >= outer.left
            && inner.top >= outer.top
            && inner.right <= outer.right
            && inner.bottom <= outer.bottom
    }

    #[test]
    fn a_rows_visible_meta_is_in_its_label() {
        let csv = format!(
            "{HEAD_META}A,Q,Adopt,d,default,new\nB,Q,Adopt,d,conditional,\nC,Q,Adopt,d,,no change\nD,Q,Adopt,d,,\n"
        );
        let root = rendered(&csv);
        let mut shown = 0;
        for (control, children) in rows(&root) {
            let Some(meta) = children.get(META) else {
                continue;
            };
            shown += 1;
            let visible = text(meta).4.join(" ");
            for part in visible.split(" \u{b7} ") {
                assert!(control.label.contains(part), "{part:?} not in {control:?}");
            }
        }
        assert_eq!(shown, 2, "the two rows with something to show");
    }

    #[test]
    fn rows_are_controls_with_the_ring_in_the_name() {
        let (catalogue, order, block) = legend(PRD_EXAMPLE);
        let root = block.render(at(1.0));
        let rows = rows(&root);
        assert_eq!(rows.len(), order.items.len());
        for ((control, _), (&index, number)) in rows.iter().zip(order.items.iter().zip(1_usize..)) {
            let item = catalogue.items.get(index).expect("an item");
            assert_eq!(control.key, ItemKey(item.line));
            assert_eq!(control.label, catalogue.accessible_name(item, Some(number)));
            let ring = item
                .ring
                .and_then(|r| catalogue.rings.get(r))
                .expect("a plotted item has a ring");
            assert!(control.label.contains(ring.as_str()), "{control:?}");
            assert!(!control.label.contains("No Change"), "{control:?}");
            assert_eq!(control.tooltip, None);
        }
        assert!(
            !groups(&root)
                .iter()
                .any(|g| matches!(g.semantics, Semantics::Marker(_))),
            "a legend row is not a marker"
        );
    }

    #[test]
    fn row_indicators_are_inset_and_interactive_only() {
        let root = rendered(PRD_EXAMPLE);
        for (control, children) in rows(&root) {
            let (origin, width, height, _, _, _) = rect(child(children, TARGET));
            let indicator = child(children, INDICATOR);
            let g = group(indicator);
            assert_eq!(g.semantics, Semantics::Indicator(control.key));
            assert!(near(g.offset, Point::ORIGIN));
            let [inset] = g.children.as_slice() else {
                panic!("an indicator holds one rect: {g:?}");
            };
            let (at, w, h, radius, fill, stroke) = rect(inset);
            assert!(
                close(radius, 5.0),
                "concentric with the target's 6: {radius}"
            );
            assert!(near(
                at,
                Point {
                    x: origin.x + 1.0,
                    y: origin.y + 1.0
                }
            ));
            assert!(close(w, width - 2.0) && close(h, height - 2.0), "{inset:?}");
            assert_eq!((fill, stroke), (Paint::None, Paint::None));
            assert!(indicator.is_interactive_only());
        }
    }

    /// FR-94 at the narrow floor: one-, two- and three-line names, and a
    /// meta on its own line, in one ring so the rows abut.
    #[test]
    fn row_targets_meet_fr94_and_tile() {
        let (two, three, own) = (NAME_TWO_LINES, NAME_THREE_LINES, NAME_FULL_LINE);
        let csv = format!(
            "{HEAD_META}A,Q,Adopt,d,,\n{two},Q,Adopt,d,,\n{three},Q,Adopt,d,,\n{own},Q,Adopt,d,default,new\n"
        );
        let root = rendered(&csv);
        let rows = rows(&root);
        // FR-34 orders the rows by name, so the cases are found, not indexed.
        let mut lines: Vec<usize> = rows
            .iter()
            .map(|(_, c)| text(child(c, NAME)).4.len())
            .collect();
        lines.sort_unstable();
        assert_eq!(lines, [1, 1, 2, 3], "the cases are the ones meant");
        let own_line = rows
            .iter()
            .filter(|(_, c)| c.get(META).is_some_and(|meta| close(text(meta).0.x, 68.0)))
            .count();
        assert_eq!(own_line, 1, "one row's meta has its own line");
        let mut bottom = None;
        for (_, children) in &rows {
            let (origin, width, height, _, fill, _) = rect(child(children, TARGET));
            assert!(height * MIN_PX_PER_UNIT >= MIN_TARGET_PX, "{height}");
            assert!(width * MIN_PX_PER_UNIT >= MIN_TARGET_PX, "{width}");
            assert_eq!(fill, SURFACE2);
            if let Some(previous) = bottom {
                assert!(close(origin.y, previous), "{origin:?} after {previous}");
            }
            bottom = Some(origin.y + height);
            let target = bounds(child(children, TARGET), fonts()).expect("a target");
            for part in children.iter().skip(BADGE) {
                let found = bounds(part, fonts()).expect("drawn");
                assert!(encloses(target, found), "{found:?} outside {target:?}");
            }
        }
    }

    #[test]
    fn document_order_is_reading_order() {
        let (catalogue, order, block) = legend(PRD_EXAMPLE);
        let root = block.render(at(1.0));
        let mut quadrant: Option<String> = None;
        let mut seen = Vec::new();
        let mut row_count = 0;
        for g in groups(&root) {
            match &g.semantics {
                Semantics::Heading(level) if level.get() == 3 => {
                    let (.., lines) = text(child(&g.children, 0));
                    let label = lines.join(" ");
                    seen.push(label.clone());
                    quadrant = Some(label);
                }
                Semantics::LegendRow(control) => {
                    row_count += 1;
                    let item = catalogue
                        .items
                        .iter()
                        .find(|i| ItemKey(i.line) == control.key)
                        .expect("a row's item");
                    let own = &catalogue
                        .quadrants
                        .get(item.quadrant)
                        .expect("a quadrant")
                        .label;
                    assert_eq!(quadrant.as_ref(), Some(own), "{control:?}");
                }
                _ => {}
            }
        }
        let fr10: Vec<String> = order
            .quadrants
            .iter()
            .map(|&q| {
                catalogue
                    .quadrants
                    .get(q)
                    .expect("a quadrant")
                    .label
                    .clone()
            })
            .collect();
        assert_eq!(seen, fr10, "each quadrant heading once, in FR-10 order");
        assert_eq!(row_count, order.items.len());
    }

    #[test]
    fn badges_match_the_markers() {
        let csv = format!("{HEAD_META}A,Q,Adopt,d,default,\nB,Q,Adopt,d,conditional,\n");
        let root = rendered(&csv);
        let rows = rows(&root);
        let [(_, solid), (_, hollow)] = rows.as_slice() else {
            panic!("two rows: {rows:?}");
        };
        let series = Paint::Role(Role::Series(0));
        let (centre, radius, disc, stroke, width) = circle(child(solid, BADGE));
        assert!(close(radius, BADGE_RADIUS));
        assert_eq!((disc, stroke), (series, series));
        assert!(close(width, BADGE_STROKE));
        let (origin, size, ink, anchor, lines) = text(child(solid, NUMBER));
        assert_eq!(
            (ink, anchor, lines),
            (
                TextPaint::Fill(Paint::NumberOn(0)),
                Anchor::Middle,
                ["1".to_owned()].as_slice()
            )
        );
        assert!(close(size, LEGEND_BADGE_NUMBER.size));
        assert!(near(origin, number_at(centre)), "{origin:?}");

        let (centre, _, disc, stroke, width) = circle(child(hollow, BADGE));
        assert_eq!(
            (disc, stroke),
            (Paint::Role(Role::Neutral(Neutral::MarkerHollow)), series)
        );
        assert!(close(width, BADGE_HOLLOW_STROKE));
        let (origin, _, ink, _, lines) = text(child(hollow, NUMBER));
        assert_eq!(
            (ink, lines),
            (fill(Neutral::TextPrimary), ["2".to_owned()].as_slice())
        );
        assert!(near(origin, number_at(centre)), "{origin:?}");
    }

    /// A badge number's origin: centred on the badge, its baseline
    /// dropped by `NUMBER_CENTRING` of the 9-unit size.
    #[expect(
        clippy::suboptimal_flops,
        reason = "NFR-4: plain * and + are bit-identical on every target; mul_add falls back to a libm fma where the CPU lacks one"
    )]
    fn number_at(centre: Point) -> Point {
        Point {
            x: centre.x,
            y: centre.y + NUMBER_CENTRING * 9.0,
        }
    }

    /// The headings' text inks: "Legend" and quadrants primary, rings muted.
    #[test]
    fn heading_inks() {
        let (_, order, block) = legend(PRD_EXAMPLE);
        let root = block.render(at(1.0));
        let headings: Vec<(u8, TextPaint)> = groups(&root)
            .iter()
            .filter_map(|g| match g.semantics {
                Semantics::Heading(level) => Some((level.get(), text(child(&g.children, 0)).2)),
                _ => None,
            })
            .collect();
        let primary = fill(Neutral::TextPrimary);
        let muted = fill(Neutral::TextMuted);
        assert_eq!(headings.first(), Some(&(2, primary)));
        let quadrants: Vec<_> = headings.iter().filter(|h| h.0 == 3).collect();
        assert_eq!(quadrants.len(), order.quadrants.len());
        assert!(quadrants.iter().all(|h| h.1 == primary));
        let rings: Vec<_> = headings.iter().filter(|h| h.0 == 4).collect();
        assert_ne!(rings.len(), 0, "the example has ring headings");
        assert!(rings.iter().all(|h| h.1 == muted));
        assert_eq!(headings.len(), 1 + quadrants.len() + rings.len());
    }

    #[test]
    fn swatches_are_series_circles() {
        let (_, order, block) = legend(PRD_EXAMPLE);
        let root = block.render(at(1.0));
        let swatches: Vec<(Point, f64, Paint, Paint, f64)> = groups(&root)
            .iter()
            .filter(|g| g.semantics == Semantics::Decorative)
            .map(|g| {
                let [one] = g.children.as_slice() else {
                    panic!("a swatch is one circle: {g:?}");
                };
                circle(one)
            })
            .collect();
        let fills: Vec<Paint> = swatches.iter().map(|s| s.2).collect();
        let want: Vec<Paint> = (0..order.quadrants.len())
            .map(|i| Paint::Role(Role::Series(i)))
            .collect();
        assert_eq!(fills, want);
        assert!(
            swatches
                .iter()
                .all(|s| close(s.1, SWATCH_RADIUS) && s.3 == Paint::None)
        );
    }

    #[test]
    fn row_text_inks() {
        let root = rendered(PRD_EXAMPLE);
        for (_, children) in rows(&root) {
            assert_eq!(
                text(child(children, NAME)).2,
                fill(Neutral::TextPrimary),
                "the name"
            );
            if let Some(meta) = children.get(META) {
                assert_eq!(text(meta).2, fill(Neutral::TextMuted), "the meta");
            }
        }
    }

    #[test]
    fn the_panel_is_surface2_and_holds_everything() {
        let (_, _, block) = legend(PRD_EXAMPLE);
        for s in [1.0, 0.7, 0.5] {
            let root = block.render(at(s));
            let size = block.size(s);
            assert_finite(&root);
            assert_inside(&root, size, fonts());
            let (origin, width, height, radius, fill, stroke) =
                rect(child(&group(&root).children, 0));
            assert!(near(origin, Point::ORIGIN));
            assert!(
                close(width, size.width) && close(height, size.height),
                "{size:?}"
            );
            assert!(close(radius, 8.0 * s));
            assert_eq!((fill, stroke), (SURFACE2, Paint::None));
            assert!(near(
                content(&root).offset,
                Point {
                    x: 12.0 * s,
                    y: 8.0 * s
                }
            ));
        }
    }

    /// The panel is the content plus the insets: 12 each side, 8 above and
    /// below.
    #[test]
    fn the_size_is_the_layout_plus_the_insets() {
        let (_, _, block) = legend(PRD_EXAMPLE);
        let size = block.size(1.0);
        assert!(close(size.width, block.layout.width + 24.0), "{size:?}");
        assert!(close(size.height, block.layout.height + 16.0), "{size:?}");
    }

    #[test]
    fn an_empty_legend_takes_no_space() {
        let catalogue = empty_catalogue();
        let block = LegendBlock::new(&catalogue, &Plotted::default(), fonts());
        assert!(block.size(1.0).is_empty());
        assert_eq!(group(&block.render(at(1.0))).children, Vec::new());
    }

    #[test]
    fn scales_linearly() {
        let (_, _, block) = legend(PRD_EXAMPLE);
        let one = block.size(1.0);
        assert!(!one.is_empty());
        for s in [0.5, 0.7, 1.3] {
            let got = block.size(s);
            assert!(
                close(got.width, one.width * s) && close(got.height, one.height * s),
                "{got:?} at {s}"
            );
        }
        assert_translates(&block, 1.0);
        assert_translates(&block, 0.5);
    }
}
