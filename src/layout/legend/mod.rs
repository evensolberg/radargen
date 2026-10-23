//! The legend block (FR-35, FR-46, FR-50, FR-74, FR-75, FR-79, FR-94).
//!
//! A `Surface2` panel holding the laid-out headings, swatches and rows,
//! each row a control. Its layout, as data, lives in the `layout`
//! submodule, and one row's composition in `row`.

mod columns;
mod flow;
mod layout;
mod row;

use std::num::NonZeroU8;
use std::ops::Range;

use crate::layout::block::{Block, Placement, Size};
use crate::layout::marker::{number_paint, number_text};
use crate::model::Catalogue;
use crate::order::Plotted;
use crate::palette::{Neutral, Role};
use crate::style::{
    BADGE_HOLLOW_STROKE, BADGE_RADIUS, BADGE_STROKE, INDENT_ITEM, LEGEND_BADGE_NUMBER,
    LEGEND_FIRST_GROUP, LEGEND_PANEL_INSET_X, LEGEND_PANEL_INSET_Y, LEGEND_PANEL_RADIUS,
    LEGEND_ROW_INDICATOR_INSET, LEGEND_ROW_LEFT_OUTSET, LEGEND_ROW_MIN_HEIGHT, LEGEND_ROW_RADIUS,
    LEGEND_ROW_RIGHT_OUTSET, PAPER_ROW_MIN_HEIGHT, SELECTION_BAR_WIDTH, SWATCH_RADIUS,
};
use crate::svg::{
    Group, ItemKey, ListKey, Node, Paint, Point, Scale, Semantics, Structure, TextRun,
};
use crate::text::Fonts;

use layout::{Entry, Layout, layout};
use row::{MetaLine, Row};

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
    /// FR-100's columns, left to right; empty for the single column every
    /// legend has unless a static output flows it.
    columns: Vec<Column>,
}

/// One of FR-100's columns: a run of whole sector groups, drawn moved by
/// `offset` from where the single column has them.
#[derive(Debug, Clone, PartialEq)]
struct Column {
    /// The column's entries in [`Layout::entries`].
    entries: Range<usize>,
    /// From the single column's place to this column's, at scale 1.
    offset: Point,
}

impl LegendBlock {
    /// The legend for `catalogue`, measured and wrapped once with `fonts`.
    ///
    /// # Panics
    ///
    /// If `plotted` was not built from `catalogue`, so an item, sector or
    /// ring index is out of range for it.
    #[must_use]
    pub fn new(catalogue: &Catalogue, plotted: &Plotted, fonts: &Fonts) -> Self {
        Self::laid_out(layout(catalogue, plotted, LEGEND_ROW_MIN_HEIGHT, fonts))
    }

    /// The legend as paper draws it (FR-111, D-105): as [`new`](Self::new),
    /// its rows at least `PAPER_ROW_MIN_HEIGHT` tall, since paper has no
    /// touch target.
    ///
    /// # Panics
    ///
    /// As [`new`](Self::new).
    #[must_use]
    pub fn on_paper(catalogue: &Catalogue, plotted: &Plotted, fonts: &Fonts) -> Self {
        Self::laid_out(layout(catalogue, plotted, PAPER_ROW_MIN_HEIGHT, fonts))
    }

    /// `layout` as a single-column legend.
    fn laid_out(layout: Layout) -> Self {
        Self {
            size: own_panel(&layout),
            layout,
            columns: Vec::new(),
        }
    }

    /// The panel width, at scale 1, of every piece [`flowed`](Self::flowed)
    /// cuts: the legend's own, widened to fit each heading a piece may
    /// repeat marked continued, so the pieces line up.
    #[must_use]
    pub fn paper_width(&self, fonts: &Fonts) -> f64 {
        panel_size(flow::widest(&self.layout, fonts), 0.0).width
    }

    /// The legend cut into single-column pieces for paper (FR-111), each a
    /// legend of its own at scale 1, [`paper_width`](Self::paper_width)
    /// wide: the first at most `first` tall as a panel, every later one at
    /// most `rest`. Breaks fall between rows; a heading moves with what
    /// follows it; a piece that starts inside a sector or ring repeats its
    /// heading marked continued, and piece `k` opens with *Legend
    /// (continued)* when `opens_page(k)`. A row taller than `rest` stands
    /// alone. An empty legend has no pieces.
    #[must_use]
    pub fn flowed(
        &self,
        first: f64,
        rest: f64,
        opens_page: &dyn Fn(usize) -> bool,
        fonts: &Fonts,
    ) -> Vec<Self> {
        if self.layout.entries.is_empty() {
            return Vec::new();
        }
        let width = flow::widest(&self.layout, fonts);
        flow::pieces(&self.layout, width, first, rest, opens_page, fonts)
            .into_iter()
            .map(Self::laid_out)
            .collect()
    }

    /// Each row's CSV line and name, in order (rg-i95's warning names them).
    #[must_use]
    pub fn rows(&self) -> Vec<(usize, String)> {
        self.layout
            .entries
            .iter()
            .filter_map(|entry| match entry {
                Entry::Row(row) => Some((row.control.key.0, row.name.words.clone())),
                _ => None,
            })
            .collect()
    }

    /// The legend flowed into `columns` columns beside one another
    /// (FR-100, D-90), breaking only between sector groups, or as it is
    /// when that is one column.
    #[must_use]
    pub fn in_columns(&self, columns: usize) -> Self {
        let Layout {
            width,
            entries,
            groups,
            ..
        } = &self.layout;
        // Where group `g`'s entries start, and the column's lift that puts
        // its heading where the first group's is (FR-100).
        let first = |g: usize| groups.get(g).map_or(entries.len(), |start| start.entry);
        let lift = |g: usize| {
            groups
                .get(g)
                .map_or(0.0, |start| LEGEND_FIRST_GROUP - start.baseline)
        };
        // A column from group `g` to the next column's: the title, in the
        // first column, from the content's top.
        let span = |g: usize, end: usize| if g == 0 { 0 } else { first(g) }..first(end);
        let height = |run: Range<usize>| {
            let bottom = span(run.start, run.end)
                .last()
                .and_then(|i| entries.get(i))
                .map_or(0.0, Entry::bottom);
            bottom + lift(run.start)
        };
        let starts = columns::split(groups.len(), columns, height);
        if starts.len() < 2 {
            return self.clone();
        }
        let ends = starts.iter().skip(1).copied().chain([groups.len()]);
        let mut x = 0.0;
        let mut tallest: f64 = 0.0;
        let mut flowed = Vec::new();
        for (&g, end) in starts.iter().zip(ends) {
            if !flowed.is_empty() {
                x += width + LEGEND_PANEL_INSET_X;
            }
            tallest = tallest.max(height(g..end));
            flowed.push(Column {
                entries: span(g, end),
                offset: Point { x, y: lift(g) },
            });
        }
        Self {
            layout: self.layout.clone(),
            size: panel_size(*width + x, tallest),
            columns: flowed,
        }
    }

    /// The legend with its panel at least `width` wide at scale 1 and the
    /// content where it was: how the narrow arrangement spans its column
    /// (rg-zch). An empty legend stays empty.
    #[must_use]
    pub const fn spanning(&self, width: f64) -> LegendSpan<'_> {
        LegendSpan {
            legend: self,
            width,
        }
    }

    /// The panel at `panel`'s size, then the content inset in it, entries
    /// in reading order, which is document and Tab order. An empty legend
    /// is an empty group.
    fn draw(&self, panel: Size, at: Placement) -> Node {
        let s = at.scale;
        let children = if panel.is_empty() {
            Vec::new()
        } else {
            vec![
                panel_node(panel.scaled(s), s),
                content_node(&self.layout, &self.columns, s),
            ]
        };
        Node::Group(Group::new(at.origin, children))
    }
}

impl Block for LegendBlock {
    fn size(&self, scale: f64) -> Size {
        self.size.scaled(scale)
    }

    fn render(&self, at: Placement) -> Node {
        self.draw(self.size, at)
    }
}

/// A legend whose panel spans a width wider than its content
/// ([`LegendBlock::spanning`]).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LegendSpan<'a> {
    legend: &'a LegendBlock,
    width: f64,
}

impl LegendSpan<'_> {
    /// The panel at scale 1: the legend's, widened to `width`.
    fn panel(&self) -> Size {
        let own = self.legend.size;
        if own.is_empty() {
            own
        } else {
            Size {
                width: own.width.max(self.width),
                ..own
            }
        }
    }
}

impl Block for LegendSpan<'_> {
    fn size(&self, scale: f64) -> Size {
        self.panel().scaled(scale)
    }

    fn render(&self, at: Placement) -> Node {
        self.legend.draw(self.panel(), at)
    }
}

/// The single column's panel at scale 1, or zero when there is no content
/// (FR-42: with nothing plotted the legend is omitted).
// qual:allow(iosp) reason: "Size::default is the derived zero, not a step of ours; one FR-42 branch over arithmetic"
fn own_panel(layout: &Layout) -> Size {
    if layout.entries.is_empty() {
        Size::default()
    } else {
        panel_size(layout.width, layout.height)
    }
}

/// The panel at scale 1 around content `width` by `height`: the content
/// plus its insets.
#[expect(
    clippy::suboptimal_flops,
    reason = "NFR-4: plain * and + are bit-identical on every target; mul_add falls back to a libm fma where the CPU lacks one"
)]
// qual:allow(complexity, magic_numbers) reason: "the panel insets the content on both sides"
fn panel_size(width: f64, height: f64) -> Size {
    Size {
        width: width + 2.0 * LEGEND_PANEL_INSET_X,
        height: height + 2.0 * LEGEND_PANEL_INSET_Y,
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

/// Every entry at `s`, in the layout's order, inset in the panel: as one
/// column, or as FR-100's `columns`, each a group at its offset.
fn content_node(layout: &Layout, columns: &[Column], s: f64) -> Node {
    let offset = Point {
        x: LEGEND_PANEL_INSET_X,
        y: LEGEND_PANEL_INSET_Y,
    }
    .scaled(s);
    let entries = |range: Range<usize>| -> Vec<Node> {
        layout
            .entries
            .get(range)
            .unwrap_or_default()
            .chunk_by(|a, b| list_of(a).is_some() && list_of(a) == list_of(b))
            .flat_map(|run| {
                let nodes = run.iter().map(|entry| entry_node(entry, layout.width, s));
                match run.first().and_then(list_of) {
                    // A ring's adjacent rows are one list (FR-114).
                    Some(key) => vec![Node::Group(
                        Group::new(Point::ORIGIN, nodes.collect())
                            .with(Semantics::Structure(Structure::List(key))),
                    )],
                    None => nodes.collect(),
                }
            })
            .collect()
    };
    let children = if columns.is_empty() {
        entries(0..layout.entries.len())
    } else {
        columns
            .iter()
            .map(|column| {
                Node::Group(Group::new(
                    column.offset.scaled(s),
                    entries(column.entries.clone()),
                ))
            })
            .collect()
    };
    Node::Group(Group::new(offset, children))
}

/// The list a legend row belongs to; `None` for any other entry.
const fn list_of(entry: &Entry) -> Option<ListKey> {
    match entry {
        Entry::Row(row) => Some(row.list),
        _ => None,
    }
}

/// One entry at `s`; `width` is the content's, which a row's target spans.
fn entry_node(entry: &Entry, width: f64, s: f64) -> Node {
    match entry {
        Entry::Heading {
            run,
            level,
            continued: true,
        } => Node::Group(
            Group::new(Point::ORIGIN, vec![heading_node(run, *level, s)])
                .with(Semantics::Pagination),
        ),
        Entry::Heading { run, level, .. } => heading_node(run, *level, s),
        Entry::Swatch { centre, series } => swatch_node(*centre, *series, s),
        Entry::Row(row) => row_node(row, width, s),
    }
}

/// A heading at its level (2 "Legend", 3 a sector, 4 a ring).
fn heading_node(run: &TextRun, level: NonZeroU8, s: f64) -> Node {
    let text = run.node(Scale::uniform(s));
    Node::Group(Group::new(Point::ORIGIN, vec![text]).with(Semantics::Heading(level)))
}

/// A sector's colour swatch (FR-67): ornament, since its heading names
/// the sector.
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
    // A list item's label and body to a tagged PDF (FR-114): its number,
    // then its name and meta.
    let part = |structure, children| {
        Node::Group(Group::new(Point::ORIGIN, children).with(Semantics::Structure(structure)))
    };
    let children = vec![
        rect_node(target, LEGEND_ROW_RADIUS * s, SURFACE2),
        indicator_node(target, row.control.key, s),
        selection_node(target, row.control.key, s),
        badge_node(row, s),
        part(Structure::Label, vec![number_node(row, s)]),
        part(Structure::Body, text_nodes(row, scale)),
    ];
    Node::Group(Group::new(Point::ORIGIN, children).with(Semantics::LegendRow(row.control.clone())))
}

/// A row's name and meta: one text when the meta continues the name, two
/// when it has its own line (FR-74, D-78).
fn text_nodes(row: &Row, scale: Scale) -> Vec<Node> {
    match &row.meta {
        Some(MetaLine::Inline(tail)) => vec![row.name.node_with_tail(tail, scale)],
        Some(MetaLine::OwnLine(run)) => vec![row.name.node(scale), run.node(scale)],
        None => vec![row.name.node(scale)],
    }
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

/// The row's selection mark (FR-72, FR-73): a bar at the start of
/// `target` (already scaled by `s`), inside the indicator's edge and clear of
/// the row's rounded corners, shown by the page while the item is selected.
#[expect(
    clippy::suboptimal_flops,
    reason = "NFR-4: plain * and + are bit-identical on every target; mul_add falls back to a libm fma where the CPU lacks one"
)]
fn selection_node(target: Area, key: ItemKey, s: f64) -> Node {
    let corner = LEGEND_ROW_RADIUS * s;
    let bar = rect_node(
        Area {
            origin: Point {
                x: target.origin.x + LEGEND_ROW_INDICATOR_INSET * s,
                y: target.origin.y + corner,
            },
            width: SELECTION_BAR_WIDTH * s,
            height: target.height - 2.0 * corner,
        },
        0.0,
        Paint::Role(Role::Neutral(Neutral::TextPrimary)),
    );
    Node::Group(Group::new(Point::ORIGIN, vec![bar]).with(Semantics::Selection(key)))
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
    use super::layout::GroupStart;
    use super::*;
    use crate::layout::block::{assert_translates, at, close, fonts};
    use crate::layout::test_support::{
        HEAD, HEAD_META, NAME_FULL_LINE, NAME_THREE_LINES, NAME_TWO_LINES, PRD_EXAMPLE, Rect,
        assert_finite, assert_inside, bounds, csv_of, empty_catalogue, fill, groups, near, plotted,
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
    const SELECTION: usize = 2;
    const BADGE: usize = 3;
    const NUMBER: usize = 4;
    const NAME: usize = 5;
    const META: usize = 6;

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

    /// Every legend row's control and children, in document order, its
    /// label and body unwrapped as the SVG writes them.
    fn rows(root: &Node) -> Vec<(&Control, Vec<Node>)> {
        groups(root)
            .into_iter()
            .filter_map(|g| match &g.semantics {
                Semantics::LegendRow(c) => Some((c, unwrapped(&g.children))),
                _ => None,
            })
            .collect()
    }

    fn unwrapped(nodes: &[Node]) -> Vec<Node> {
        nodes
            .iter()
            .flat_map(|node| match node {
                Node::Group(group) if matches!(group.semantics, Semantics::Structure(_)) => {
                    unwrapped(&group.children)
                }
                node => vec![node.clone()],
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

    /// A row's meta as drawn, and its paint: the name's tail, or its own line.
    fn meta_of(children: &[Node]) -> Option<(String, Paint)> {
        if let Some(own) = children.get(META) {
            let (_, _, ink, _, lines) = text(own);
            let TextPaint::Fill(paint) = ink else {
                panic!("a filled meta");
            };
            return Some((lines.concat(), paint));
        }
        match child(children, NAME) {
            Node::Text {
                tail: Some(tail), ..
            } => Some((tail.text.clone(), tail.ink)),
            _ => None,
        }
    }

    fn encloses(outer: Rect, inner: Rect) -> bool {
        inner.left >= outer.left
            && inner.top >= outer.top
            && inner.right <= outer.right
            && inner.bottom <= outer.bottom
    }

    /// The target, and so the focus ring, covers an inline meta where it
    /// ends under a reader's text spacing: after the name's layout width
    /// (D-78; the crumb's "Default reaches the ring" note).
    #[test]
    fn the_row_target_covers_the_meta() {
        let fonts = fonts();
        let csv = format!(
            "{HEAD_META}Trunk-Based Development,Q,Adopt,d,default,\nA,Q,Adopt,d,conditional,new\n"
        );
        let root = rendered(&csv);
        let mut tails = 0;
        for (_, children) in rows(&root) {
            let name = child(&children, NAME);
            if !matches!(name, Node::Text { tail: Some(_), .. }) {
                continue;
            }
            tails += 1;
            let end = bounds(name, fonts).expect("bounds").right;
            let (target, width, ..) = rect(child(&children, TARGET));
            assert!(target.x + width >= end, "{end} past {}", target.x + width);
        }
        assert_eq!(tails, 2, "both rows' metas ride inline");
    }

    #[test]
    fn a_rows_visible_meta_is_in_its_label() {
        let csv = format!(
            "{HEAD_META}A,Q,Adopt,d,default,new\nB,Q,Adopt,d,conditional,\nC,Q,Adopt,d,,no change\nD,Q,Adopt,d,,\n"
        );
        let root = rendered(&csv);
        let mut shown = 0;
        for (control, children) in rows(&root) {
            let Some((visible, _)) = meta_of(&children) else {
                continue;
            };
            shown += 1;
            for part in visible.trim_start_matches('\u{a0}').split(" \u{b7} ") {
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
            let (origin, width, height, _, _, _) = rect(child(&children, TARGET));
            let indicator = child(&children, INDICATOR);
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

            // FR-72, FR-73: selection is a bar at the row's start, inside the
            // indicator's edge, so focus keeps the outline and the two never
            // merge into one thicker edge.
            let bar = child(&children, SELECTION);
            let b = group(bar);
            assert_eq!(b.semantics, Semantics::Selection(control.key));
            assert!(bar.is_interactive_only());
            let [mark] = b.children.as_slice() else {
                panic!("a bar holds one rect: {b:?}");
            };
            // Inset from the row's rounded corners, so its square ends never
            // cross the target's or the outline's curve.
            let (bar_at, bar_w, bar_h, _, bar_fill, _) = rect(mark);
            assert!(near(
                bar_at,
                Point {
                    x: origin.x + 1.0,
                    y: origin.y + 6.0
                }
            ));
            assert!(close(bar_w, 3.0) && close(bar_h, height - 12.0), "{mark:?}");
            assert_eq!(bar_fill, Paint::Role(Role::Neutral(Neutral::TextPrimary)));
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
        let mut sector: Option<String> = None;
        let mut seen = Vec::new();
        let mut row_count = 0;
        for g in groups(&root) {
            match &g.semantics {
                Semantics::Heading(level) if level.get() == 3 => {
                    let (.., lines) = text(child(&g.children, 0));
                    let label = lines.join(" ");
                    seen.push(label.clone());
                    sector = Some(label);
                }
                Semantics::LegendRow(control) => {
                    row_count += 1;
                    let item = catalogue
                        .items
                        .iter()
                        .find(|i| ItemKey(i.line) == control.key)
                        .expect("a row's item");
                    let own = &catalogue.sectors.get(item.sector).expect("a sector").label;
                    assert_eq!(sector.as_ref(), Some(own), "{control:?}");
                }
                _ => {}
            }
        }
        let fr10: Vec<String> = order
            .sectors
            .iter()
            .map(|&q| catalogue.sectors.get(q).expect("a sector").label.clone())
            .collect();
        assert_eq!(seen, fr10, "each sector heading once, in FR-10 order");
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

    /// The headings' text inks: "Legend" and sectors primary, rings muted.
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
        let sectors: Vec<_> = headings.iter().filter(|h| h.0 == 3).collect();
        assert_eq!(sectors.len(), order.sectors.len());
        assert!(sectors.iter().all(|h| h.1 == primary));
        let rings: Vec<_> = headings.iter().filter(|h| h.0 == 4).collect();
        assert_ne!(rings.len(), 0, "the example has ring headings");
        assert!(rings.iter().all(|h| h.1 == muted));
        assert_eq!(headings.len(), 1 + sectors.len() + rings.len());
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
        let want: Vec<Paint> = (0..order.sectors.len())
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
                text(child(&children, NAME)).2,
                fill(Neutral::TextPrimary),
                "the name"
            );
            if let Some((_, paint)) = meta_of(&children) {
                assert_eq!(
                    paint,
                    Paint::Role(Role::Neutral(Neutral::TextMuted)),
                    "the meta"
                );
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

    /// rg-zch: the narrow arrangement spans the panel across its column;
    /// the content stays where it was, and never narrows.
    #[test]
    fn spanning_widens_the_panel_not_the_content() {
        let (_, _, block) = legend(PRD_EXAMPLE);
        let natural = block.size(1.0);
        let wider = natural.width + 50.0;
        for s in [1.0, 0.5] {
            let spanning = block.spanning(wider);
            let size = spanning.size(s);
            assert!(close(size.width, wider * s) && close(size.height, natural.height * s));
            let root = spanning.render(at(s));
            let (_, width, height, ..) = rect(child(&group(&root).children, 0));
            assert!(close(width, size.width) && close(height, size.height));
            assert_eq!(content(&root), content(&block.render(at(s))));
        }
        assert_eq!(block.spanning(10.0).size(1.0), natural);
    }

    #[test]
    fn an_empty_legend_takes_no_space() {
        let catalogue = empty_catalogue();
        let block = LegendBlock::new(&catalogue, &Plotted::default(), fonts());
        assert!(block.size(1.0).is_empty());
        assert_eq!(group(&block.render(at(1.0))).children, Vec::new());
        assert!(block.spanning(400.0).size(1.0).is_empty());
        assert_eq!(
            group(&block.spanning(400.0).render(at(1.0))).children,
            Vec::new()
        );
    }

    /// Four sectors of 6, 2, 2 and 6 rows: two columns break between
    /// the second and third, where the columns are equal.
    fn flowed() -> String {
        [("A", 6), ("B", 2), ("C", 2), ("D", 6)]
            .iter()
            .flat_map(|&(q, n)| (0..n).map(move |i| format!("{q}{i},{q},Adopt,d\n")))
            .fold(HEAD.to_owned(), |text, row| text + &row)
    }

    fn start(block: &LegendBlock, group: usize) -> GroupStart {
        *block.layout.groups.get(group).expect("a group")
    }

    fn controls(root: &Node) -> Vec<Control> {
        rows(root).into_iter().map(|(c, _)| c.clone()).collect()
    }

    /// Every group under `node` with its semantics, in document order.
    fn all_semantics(node: &Node, out: &mut Vec<Semantics>) {
        if let Node::Group(group) = node {
            out.push(group.semantics.clone());
            group
                .children
                .iter()
                .for_each(|child| all_semantics(child, out));
        }
    }

    /// A legend row's label holds its number and its body its name.
    fn assert_label_and_body(row: &Group) {
        let Semantics::LegendRow(control) = &row.semantics else {
            panic!("a legend row");
        };
        let words = |s: Structure| -> Vec<String> {
            row.children
                .iter()
                .filter_map(|c| match c {
                    Node::Group(g) if g.semantics == Semantics::Structure(s) => Some(&g.children),
                    _ => None,
                })
                .flatten()
                .filter_map(|n| match n {
                    Node::Text { words, .. } => Some(words.clone()),
                    _ => None,
                })
                .collect()
        };
        let label = words(Structure::Label);
        assert!(
            matches!(label.as_slice(), [n] if n.parse::<usize>().is_ok()),
            "{label:?}"
        );
        let body = words(Structure::Body);
        let name = body.first().expect("a name");
        assert!(
            control.label.contains(name.as_str()),
            "{name} in {}",
            control.label
        );
    }

    /// PR 21c-1: each ring's rows are one list, wherever a page breaks it;
    /// a row's number is its label and its name and meta its body; a
    /// continued heading is page furniture.
    #[test]
    fn rings_are_lists_of_labelled_rows() {
        let (_, _, block) = legend(&flowed());
        let pieces = block.flowed(150.0, 150.0, &|k| k > 0, fonts());
        assert!(pieces.len() > 2, "{}", pieces.len());
        let mut keys = Vec::new();
        let mut pagination = 0;
        for piece in &pieces {
            let root = piece.render(at(1.0));
            for list in groups(&root).into_iter().filter_map(|g| match g.semantics {
                Semantics::Structure(Structure::List(key)) => Some((key, g)),
                _ => None,
            }) {
                let (key, group) = list;
                keys.push(key);
                for row in &group.children {
                    let Node::Group(row) = row else {
                        panic!("a row")
                    };
                    assert!(matches!(row.semantics, Semantics::LegendRow(_)));
                    let mut inner = Vec::new();
                    row.children
                        .iter()
                        .for_each(|c| all_semantics(c, &mut inner));
                    let count = |s: Structure| {
                        inner
                            .iter()
                            .filter(|x| **x == Semantics::Structure(s))
                            .count()
                    };
                    assert_eq!((count(Structure::Label), count(Structure::Body)), (1, 1));
                    assert_label_and_body(row);
                }
            }
            let mut all = Vec::new();
            all_semantics(&root, &mut all);
            pagination += all.iter().filter(|s| **s == Semantics::Pagination).count();
            // No row outside a list.
            let listed: usize = groups(&root)
                .iter()
                .filter(|g| matches!(g.semantics, Semantics::Structure(Structure::List(_))))
                .map(|g| g.children.len())
                .sum();
            assert_eq!(listed, rows(&root).len());
        }
        assert!(pagination > 0, "a continued heading is furniture");
        // Some ring is broken across pieces: its key appears twice.
        let mut sorted = keys.clone();
        sorted.sort_by_key(|k| format!("{k:?}"));
        sorted.dedup();
        assert!(sorted.len() < keys.len(), "{keys:?}");
    }

    /// FR-100: whole groups, side by side, each a single column's width
    /// apart by the panel's inset, on one panel spanning them.
    #[test]
    fn columns_stand_beside_one_another_on_one_panel() {
        let (_, _, block) = legend(&flowed());
        let two = block.in_columns(2);
        let [first, second] = two.columns.as_slice() else {
            panic!("two columns: {:?}", two.columns);
        };
        let third = start(&block, 2);
        assert_eq!(first.entries, 0..third.entry);
        assert_eq!(second.entries, third.entry..block.layout.entries.len());
        assert!(near(first.offset, Point::ORIGIN), "{first:?}");
        let width = block.layout.width;
        assert!(
            close(second.offset.x, width + LEGEND_PANEL_INSET_X),
            "{second:?}"
        );
        assert!(
            close(third.baseline + second.offset.y, start(&block, 0).baseline),
            "the third sector's heading is level with the first's"
        );
        let size = two.size(1.0);
        assert!(
            close(size.width, width + width + LEGEND_PANEL_INSET_X + 24.0),
            "{size:?}"
        );
        assert!(size.height < block.size(1.0).height, "{size:?}");
        let root = two.render(at(1.0));
        assert_inside(&root, size, fonts());
        let (_, panel_width, panel_height, ..) = rect(child(&group(&root).children, 0));
        assert!(close(panel_width, size.width) && close(panel_height, size.height));
        let offsets: Vec<Point> = content(&root)
            .children
            .iter()
            .map(|column| group(column).offset)
            .collect();
        assert_eq!(offsets, [first.offset, second.offset]);
        assert_eq!(
            controls(&root),
            controls(&block.render(at(1.0))),
            "FR-35's order"
        );
        let half = two.render(at(0.5));
        assert_inside(&half, two.size(0.5), fonts());
    }

    /// FR-100: one column per group when asked for more.
    #[test]
    fn more_columns_than_groups_gives_one_each() {
        let (_, _, block) = legend(&flowed());
        assert_eq!(block.in_columns(4).columns.len(), 4);
        let (_, _, three) = legend(&csv_of(&["A", "B", "C"], "Adopt"));
        assert_eq!(three.in_columns(4).columns.len(), 3);
    }

    /// One column, one group or none: the legend as it is.
    #[test]
    fn a_single_column_is_the_legend_unchanged() {
        let (_, _, block) = legend(&flowed());
        assert_eq!(block.in_columns(1), block);
        let (_, _, one) = legend(&csv_of(&["A", "A"], "Adopt"));
        assert_eq!(one.in_columns(4), one);
        let catalogue = empty_catalogue();
        let empty = LegendBlock::new(&catalogue, &Plotted::default(), fonts());
        assert_eq!(empty.in_columns(3), empty);
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
