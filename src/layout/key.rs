//! The encoding key (FR-66): what the marker shapes and status rings mean.
//! Its content is fixed whatever the dataset uses (D-25).

use std::num::{NonZeroU8, NonZeroUsize};

use crate::layout::block::{Block, Placement, Size};
use crate::layout::marker::{Glyph, StatusMark};
use crate::model::Qualifier;
use crate::palette::{Neutral, Role};
use crate::style::{
    KEY_BOTTOM, KEY_COLUMN_GAP, KEY_FIRST_ROW, KEY_HEADING, KEY_HEADING_BASELINE, KEY_LABEL,
    KEY_LABEL_DROP, KEY_LABEL_X, KEY_MARKER_RADIUS, KEY_ROW, KEY_SAMPLE_X,
    KEY_STATUS_MARKER_RADIUS, TextStyle,
};
use crate::svg::{Anchor, Group, Node, Point, Scale, Semantics, TextRun};
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
        label: "Moved in \u{2014} half-ring facing the centre",
        radius: KEY_STATUS_MARKER_RADIUS,
        qualifier: None,
        mark: Some(StatusMark::MovedIn { centreward: 180.0 }),
    },
    Row {
        label: "Moved out \u{2014} half-ring facing the outer edge",
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

/// The centre of the sample in grid row `index`, at scale 1, for an entry
/// in the column starting `column_x` from the key's left edge.
const fn sample_centre(index: usize, column_x: f64) -> Point {
    #[expect(
        clippy::cast_precision_loss,
        clippy::as_conversions,
        reason = "a row index is at most four, so the conversion is exact"
    )]
    let y = KEY_FIRST_ROW + (index as f64) * KEY_ROW;
    Point {
        x: column_x + KEY_SAMPLE_X,
        y,
    }
}

/// A text node of one line, muted or primary.
fn text(label: &str, origin: Point, style: TextStyle, ink: Neutral, scale: f64) -> Node {
    TextRun {
        origin,
        style,
        ink,
        anchor: Anchor::Start,
        lines: vec![label.to_owned()],
        leading: 0.0,
    }
    .node(Scale::uniform(scale))
}

/// One entry: its decorative sample, then its label, in grid row `index`
/// and the column starting `column_x`.
fn row_nodes(index: usize, column_x: f64, row: Row, scale: f64) -> [Node; 2] {
    let centre = sample_centre(index, column_x);
    let glyph = Glyph {
        centre,
        radius: row.radius,
        qualifier: row.qualifier,
        mark: row.mark,
        ink: SAMPLE_INK,
    }
    .draw(Scale::uniform(scale));
    let origin = Point {
        x: column_x + KEY_LABEL_X,
        y: centre.y + KEY_LABEL_DROP,
    };
    [
        Node::Group(Group::new(Point::ORIGIN, glyph).with(Semantics::Decorative)),
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
    Node::Group(Group::new(Point::ORIGIN, vec![node]).with(Semantics::Heading(HEADING_LEVEL)))
}

/// The key to the markers (FR-66): a heading and five fixed entries,
/// measured once and laid out in as many columns as the arrangement's
/// width holds (rg-zb9).
#[derive(Debug, Clone, PartialEq)]
pub struct KeyBlock {
    /// Each entry's scale-1 width: its indent and its label's layout width.
    entries: [f64; ROWS.len()],
    heading: f64,
}

impl KeyBlock {
    /// The key, its entries measured with `fonts`.
    #[must_use]
    pub fn new(fonts: &Fonts) -> Self {
        Self {
            entries: ROWS.map(|row| KEY_LABEL_X + KEY_LABEL.measure(row.label, fonts)),
            heading: KEY_HEADING.measure(HEADING, fonts),
        }
    }

    /// The most columns, at most one per entry, whose entries fit within
    /// `width` at scale 1; one when even a single column does not.
    #[must_use]
    pub fn columns_within(&self, width: f64) -> NonZeroUsize {
        (1..=ROWS.len())
            .rev()
            .filter_map(NonZeroUsize::new)
            .find(|&columns| self.grid(columns).entries_width() <= width)
            .unwrap_or(NonZeroUsize::MIN)
    }

    /// The key laid out in `columns`, entries filling each row in turn so
    /// the reading order is the entries' order.
    #[must_use]
    pub const fn grid(&self, columns: NonZeroUsize) -> KeyGrid<'_> {
        KeyGrid { key: self, columns }
    }
}

/// The key in a given number of columns, as the arrangement places it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KeyGrid<'a> {
    key: &'a KeyBlock,
    columns: NonZeroUsize,
}

impl KeyGrid<'_> {
    /// Each column's scale-1 width: its widest entry.
    fn column_widths(self) -> Vec<f64> {
        (0..self.columns.get())
            .map(|column| {
                self.key
                    .entries
                    .iter()
                    .skip(column)
                    .step_by(self.columns.get())
                    .fold(0.0, |widest, &w| f64::max(widest, w))
            })
            .collect()
    }

    /// Where each column starts from the key's left edge, at scale 1.
    fn column_starts(self) -> Vec<f64> {
        self.column_widths()
            .iter()
            .scan(0.0, |x, width| {
                let start = *x;
                *x += width + KEY_COLUMN_GAP;
                Some(start)
            })
            .collect()
    }

    /// The entries' scale-1 width: the columns and the gaps between them.
    fn entries_width(self) -> f64 {
        let widths = self.column_widths();
        #[expect(
            clippy::cast_precision_loss,
            clippy::as_conversions,
            reason = "a column count is at most five, so the conversion is exact"
        )]
        let gaps = (widths.len() - 1) as f64 * KEY_COLUMN_GAP;
        widths.iter().sum::<f64>() + gaps
    }

    /// The grid rows the entries take.
    const fn rows(self) -> usize {
        ROWS.len().div_ceil(self.columns.get())
    }
}

impl Block for KeyGrid<'_> {
    fn size(&self, scale: f64) -> Size {
        Size {
            width: self.entries_width().max(self.key.heading),
            height: sample_centre(self.rows() - 1, 0.0).y + KEY_BOTTOM,
        }
        .scaled(scale)
    }

    fn render(&self, at: Placement) -> Node {
        let starts = self.column_starts();
        let columns = self.columns.get();
        let rows = ROWS.iter().enumerate().flat_map(|(index, row)| {
            let column_x = starts.get(index % columns).copied().unwrap_or_default();
            row_nodes(index / columns, column_x, *row, at.scale)
        });
        let children = std::iter::once(heading_node(at.scale))
            .chain(rows)
            .collect();
        Node::Group(Group::new(at.origin, children))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::block::{at, close, fonts};
    use crate::layout::geometry::polar;
    use crate::layout::test_support::{self, assert_inside, fill};
    use crate::style::{KEY_COLUMN_GAP, STATUS_RING_GAP, STATUS_RING_STROKE};
    use crate::svg::Paint;
    use crate::svg::PathCommand;
    use crate::svg::TextPaint;

    const UNIT: Scale = Scale::uniform(1.0);

    const fn columns(n: usize) -> NonZeroUsize {
        match NonZeroUsize::new(n) {
            Some(n) => n,
            None => panic!("a column count is not zero"),
        }
    }

    fn rendered_in(n: usize, scale: f64) -> Vec<Node> {
        let Node::Group(group) = KeyBlock::new(fonts()).grid(columns(n)).render(at(scale)) else {
            panic!("the key renders a group");
        };
        group.children
    }

    /// One column: the key as it was before rg-zb9, so these tests pin it.
    fn rendered(scale: f64) -> Vec<Node> {
        rendered_in(1, scale)
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

    fn text_parts(node: &Node) -> (Point, &[String], &TextStyle, TextPaint) {
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
        test_support::near(p, Point { x, y })
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
            "Moved in \u{2014} half-ring facing the centre",
            "Moved out \u{2014} half-ring facing the outer edge",
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
            .draw(UNIT),
            Glyph {
                centre: c(64.0),
                radius: 7.0,
                qualifier: Some(Qualifier::Conditional),
                mark: None,
                ink,
            }
            .draw(UNIT),
            Glyph {
                centre: c(88.0),
                radius: 5.0,
                qualifier: None,
                mark: Some(StatusMark::New),
                ink,
            }
            .draw(UNIT),
            Glyph {
                centre: c(112.0),
                radius: 5.0,
                qualifier: None,
                mark: Some(StatusMark::MovedIn { centreward: 180.0 }),
                ink,
            }
            .draw(UNIT),
            Glyph {
                centre: c(136.0),
                radius: 5.0,
                qualifier: None,
                mark: Some(StatusMark::MovedOut { centreward: 180.0 }),
                ink,
            }
            .draw(UNIT),
        ];
        for (i, want) in expected.iter().enumerate() {
            assert_eq!(sample(&nodes, i), want.as_slice());
        }
    }

    #[test]
    fn key_status_rows_literal_geometry() {
        let nodes = rendered(1.0);
        let (start, end) = arc_ends(child(sample(&nodes, 3), 0));
        assert!(near(start, 10.0, 121.0) && near(end, 10.0, 103.0));
        let (start, end) = arc_ends(child(sample(&nodes, 4), 0));
        assert!(near(start, 10.0, 127.0) && near(end, 10.0, 145.0));
        let (centre, radius, _, _) = circle(child(sample(&nodes, 2), 0));
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
            let (start, end) = arc_ends(child(sample(&nodes, row), 0));
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
        let (_, _, fill, _) = circle(child(sample(&nodes, 0), 0));
        assert_eq!(fill, muted);
        let (_, _, fill, stroke) = circle(child(sample(&nodes, 1), 0));
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
        assert_eq!(ink, fill(Neutral::TextPrimary));
        for (i, y) in [40.0, 64.0, 88.0, 112.0, 136.0].into_iter().enumerate() {
            let (origin, _, style, ink) = text_parts(label(&nodes, i));
            assert!(close(origin.x, 28.0) && close(origin.y, y + 4.0));
            assert_eq!(*style, KEY_LABEL);
            assert_eq!(ink, fill(Neutral::TextMuted));
        }
        let size = KeyBlock::new(fonts()).grid(columns(1)).size(1.0);
        assert!(close(size.height, 146.0));
        // Measured at a literal 11 and no spacing, not through KEY_LABEL, so
        // the test pins the label size independently of the constant; plus
        // D-78's allowance for a reader's text spacing.
        let fonts = fonts();
        let widest = ROWS
            .iter()
            .map(|row| {
                fonts.face(KEY_LABEL.weight).measure(row.label, 11.0, 0.0)
                    + crate::style::spacing_allowance(row.label, 11.0)
            })
            .fold(0.0, f64::max);
        assert!(close(size.width, 28.0 + widest));
    }

    #[test]
    fn the_samples_scale_with_the_key() {
        let nodes = rendered(0.5);
        let ink = Role::Neutral(Neutral::TextMuted);
        for (i, row) in ROWS.iter().enumerate() {
            let want = Glyph {
                centre: sample_centre(i, 0.0),
                radius: row.radius,
                qualifier: row.qualifier,
                mark: row.mark,
                ink,
            }
            .draw(Scale::uniform(0.5));
            assert_eq!(sample(&nodes, i), want.as_slice());
        }
        let Node::Circle {
            radius,
            stroke_width,
            ..
        } = child(sample(&nodes, 2), 0)
        else {
            panic!("row 3 starts with its ring");
        };
        assert!(close(*radius, 4.5));
        assert!(close(*stroke_width, STATUS_RING_STROKE / 2.0));
    }

    #[test]
    fn a_placed_key_is_the_origin_key_translated() {
        let key = KeyBlock::new(fonts());
        for n in 1..=ROWS.len() {
            crate::layout::block::assert_translates(&key.grid(columns(n)), 1.0);
        }
    }

    #[test]
    fn size_scales_linearly() {
        let key = KeyBlock::new(fonts());
        let b = key.grid(columns(2));
        let (one, half) = (b.size(1.0), b.size(0.5));
        assert!(close(half.width, one.width / 2.0));
        assert!(close(half.height, one.height / 2.0));
    }

    #[test]
    fn everything_drawn_lies_inside_the_size() {
        let fonts = fonts();
        let key = KeyBlock::new(fonts);
        for n in 1..=ROWS.len() {
            let grid = key.grid(columns(n));
            for scale in [1.0, 0.5] {
                assert_inside(&grid.render(at(scale)), grid.size(scale), fonts);
            }
        }
    }

    /// Each entry's width at scale 1: its indent and its label's layout
    /// width, D-78's allowance included.
    fn entry_widths() -> Vec<f64> {
        ROWS.iter()
            .map(|row| KEY_LABEL_X + KEY_LABEL.measure(row.label, fonts()))
            .collect()
    }

    fn width_at(widths: &[f64], i: usize) -> f64 {
        *widths.get(i).expect("an entry")
    }

    /// rg-zb9: as many columns as fit, at least one, at most one per entry.
    #[test]
    fn the_key_takes_as_many_columns_as_fit() {
        let key = KeyBlock::new(fonts());
        let w = entry_widths();
        // Two columns, filled row by row: entries 0, 2, 4 and 1, 3.
        let first = [0, 2, 4]
            .map(|i| width_at(&w, i))
            .into_iter()
            .fold(0.0, f64::max);
        let second = [1, 3]
            .map(|i| width_at(&w, i))
            .into_iter()
            .fold(0.0, f64::max);
        let two = first + KEY_COLUMN_GAP + second;
        assert_eq!(key.columns_within(two).get(), 2);
        assert_eq!(key.columns_within(two - 0.01).get(), 1);
        assert_eq!(key.columns_within(f64::INFINITY).get(), ROWS.len());
        assert_eq!(key.columns_within(0.0).get(), 1, "never fewer than one");
        // The widths the arrangements give at the smallest radar: the wide
        // key's rule spans 640 units, the narrow one's 380.
        assert_eq!(key.columns_within(640.0).get(), 2);
        assert_eq!(key.columns_within(380.0).get(), 1);
    }

    #[test]
    fn two_columns_run_in_reading_order() {
        let nodes = rendered_in(2, 1.0);
        let w = entry_widths();
        let first = [0, 2, 4]
            .map(|i| width_at(&w, i))
            .into_iter()
            .fold(0.0, f64::max);
        let second_x = first + KEY_COLUMN_GAP;
        for (i, x, y) in [
            (0, 0.0, 40.0),
            (1, second_x, 40.0),
            (2, 0.0, 64.0),
            (3, second_x, 64.0),
            (4, 0.0, 88.0),
        ] {
            let row = ROWS.get(i).expect("a row");
            let want = Glyph {
                centre: Point {
                    x: x + KEY_SAMPLE_X,
                    y,
                },
                radius: row.radius,
                qualifier: row.qualifier,
                mark: row.mark,
                ink: SAMPLE_INK,
            }
            .draw(UNIT);
            assert_eq!(sample(&nodes, i), want.as_slice(), "entry {i}");
            let (origin, ..) = text_parts(label(&nodes, i));
            assert!(
                near(origin, x + KEY_LABEL_X, y + KEY_LABEL_DROP),
                "entry {i}: {origin:?}"
            );
        }
        let size = KeyBlock::new(fonts()).grid(columns(2)).size(1.0);
        let second = [1, 3]
            .map(|i| width_at(&w, i))
            .into_iter()
            .fold(0.0, f64::max);
        assert!(close(size.width, second_x + second), "{size:?}");
        assert!(close(size.height, 88.0 + KEY_BOTTOM), "{size:?}");
    }

    #[test]
    fn key_bottom_is_the_status_ring_edge() {
        assert!(close(
            KEY_BOTTOM,
            KEY_STATUS_MARKER_RADIUS + STATUS_RING_GAP + STATUS_RING_STROKE / 2.0
        ));
    }
}
