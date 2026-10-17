//! The one walker that finds the box a node paints in, and the assertions
//! built on it. Having one walker means no block's bounds test can quietly
//! measure less than another's.

use crate::layout::block::{Size, close};
use crate::palette::{Neutral, Role};
use crate::svg::{Group, Node, Paint, PathCommand, Point, TextPaint};
use crate::text::{Fonts, TEXT_DESCENT};

mod text;
use text::text_bounds;

/// How far outside its size a node may reach before it counts as outside:
/// float noise from scaling, not drawing room.
const SLACK: f64 = 1e-9;

/// An axis-aligned box in user units.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub left: f64,
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
}

impl Rect {
    /// The box of a disc of `reach` about `centre`.
    fn disc(centre: Point, reach: f64) -> Self {
        Self::spanning(centre, centre).grown(reach)
    }

    /// The smallest box holding both points.
    fn spanning(a: Point, b: Point) -> Self {
        Self {
            left: a.x.min(b.x),
            top: a.y.min(b.y),
            right: a.x.max(b.x),
            bottom: a.y.max(b.y),
        }
    }

    fn union(self, other: Self) -> Self {
        Self {
            left: self.left.min(other.left),
            top: self.top.min(other.top),
            right: self.right.max(other.right),
            bottom: self.bottom.max(other.bottom),
        }
    }

    /// The box pushed out by `by` on all four sides.
    fn grown(self, by: f64) -> Self {
        Self {
            left: self.left - by,
            top: self.top - by,
            right: self.right + by,
            bottom: self.bottom + by,
        }
    }

    /// Whether the box lies inside `size`, give or take float noise.
    pub(crate) fn fits(self, size: Size) -> bool {
        self.left >= -SLACK
            && self.top >= -SLACK
            && self.right <= size.width + SLACK
            && self.bottom <= size.height + SLACK
    }

    fn shifted(self, by: Point) -> Self {
        Self {
            left: self.left + by.x,
            top: self.top + by.y,
            right: self.right + by.x,
            bottom: self.bottom + by.y,
        }
    }
}

/// The smallest box holding everything `node` paints, in its parent's
/// coordinates; `None` when it paints nothing.
pub fn bounds(node: &Node, fonts: &Fonts) -> Option<Rect> {
    match node {
        Node::Group(group) => group
            .children
            .iter()
            .filter_map(|child| bounds(child, fonts))
            .reduce(Rect::union)
            .map(|inner| inner.shifted(group.offset)),
        Node::Circle {
            centre,
            radius,
            stroke_width,
            ..
        } => Some(Rect::disc(
            *centre,
            radius + f64::midpoint(0.0, *stroke_width),
        )),
        Node::Rect {
            origin,
            width,
            height,
            stroke_width,
            ..
        } => Some(
            Rect {
                left: origin.x,
                top: origin.y,
                right: origin.x + width,
                bottom: origin.y + height,
            }
            .grown(f64::midpoint(0.0, *stroke_width)),
        ),
        Node::HitArea { centre, radius, .. } => Some(Rect::disc(*centre, *radius)),
        Node::Line {
            from,
            to,
            stroke_width,
            ..
        } => Some(Rect::spanning(*from, *to).grown(f64::midpoint(0.0, *stroke_width))),
        Node::Path {
            commands,
            stroke_width,
            ..
        } => path_bounds(commands, *stroke_width),
        Node::Text { .. } => text_bounds(node, fonts),
    }
}

/// Every arc here is a half-circle, so its chord is a diameter, its midpoint
/// is the circle's centre and the circle bounds the arc.
///
/// Each half-circle is boxed as its full circle: a safe over-estimate that
/// can fail a correct drawing but never pass a wrong one. It is unsuitable
/// for a tightness check on a path.
fn path_bounds(commands: &[PathCommand], stroke_width: f64) -> Option<Rect> {
    let mut from = None;
    let mut boxes = Vec::new();
    for command in commands {
        match command {
            PathCommand::MoveTo(p) => from = Some(*p),
            PathCommand::Arc { radius, end, .. } => {
                let Some(start) = from else {
                    panic!("an arc must follow a move");
                };
                let chord = (end.x - start.x).hypot(end.y - start.y);
                assert!(
                    close(chord, 2.0 * radius),
                    "an arc of radius {radius} spans {chord}, so it is not a half-circle"
                );
                let centre = Point {
                    x: f64::midpoint(start.x, end.x),
                    y: f64::midpoint(start.y, end.y),
                };
                boxes.push(Rect::disc(
                    centre,
                    radius + f64::midpoint(0.0, stroke_width),
                ));
                from = Some(*end);
            }
        }
    }
    boxes.into_iter().reduce(Rect::union)
}

/// `a` moved by `b`.
pub fn plus(a: Point, b: Point) -> Point {
    Point {
        x: a.x + b.x,
        y: a.y + b.y,
    }
}

/// Whether two points coincide to within the tests' tolerance.
pub fn near(a: Point, b: Point) -> bool {
    close(a.x, b.x) && close(a.y, b.y)
}

/// Plain `ink` text paint, as the blocks' tests expect it.
pub const fn fill(ink: Neutral) -> TextPaint {
    TextPaint::Fill(Paint::Role(Role::Neutral(ink)))
}

/// Panics naming the first node whose painted box leaves `size`.
pub fn assert_inside(node: &Node, size: Size, fonts: &Fonts) {
    inside(node, Point::ORIGIN, size, fonts);
}

fn inside(node: &Node, offset: Point, size: Size, fonts: &Fonts) {
    if let Node::Group(group) = node {
        let offset = plus(offset, group.offset);
        for child in &group.children {
            inside(child, offset, size, fonts);
        }
        return;
    }
    let Some(found) = bounds(node, fonts) else {
        return;
    };
    let found = found.shifted(offset);
    assert!(
        found.fits(size),
        "{node:?} paints {found:?}, outside {size:?}"
    );
}

/// Panics on the first non-finite number in the tree.
pub fn assert_finite(node: &Node) {
    let numbers: Vec<f64> = match node {
        Node::Group(group) => {
            group.children.iter().for_each(assert_finite);
            vec![group.offset.x, group.offset.y]
        }
        Node::Text { .. } => text_numbers(node),
        Node::Circle {
            centre,
            radius,
            stroke_width,
            ..
        } => vec![centre.x, centre.y, *radius, *stroke_width],
        Node::Rect {
            origin,
            width,
            height,
            radius,
            stroke_width,
            ..
        } => vec![origin.x, origin.y, *width, *height, *radius, *stroke_width],
        Node::HitArea { centre, radius, .. } => vec![centre.x, centre.y, *radius],
        Node::Line {
            from,
            to,
            stroke_width,
            ..
        } => vec![from.x, from.y, to.x, to.y, *stroke_width],
        Node::Path {
            commands,
            stroke_width,
            ..
        } => path_numbers(commands, *stroke_width),
    };
    for number in numbers {
        assert!(number.is_finite(), "{number} in {node:?}");
    }
}

/// A text node's numbers: its position, sizes, leading, halo, and its
/// tail's gap and sizes.
fn text_numbers(node: &Node) -> Vec<f64> {
    let Node::Text {
        origin,
        style,
        ink,
        leading,
        tail,
        ..
    } = node
    else {
        return Vec::new();
    };
    let halo = match ink {
        TextPaint::Fill(_) => 0.0,
        TextPaint::Halo { width, .. } => *width,
    };
    let tail = tail
        .as_ref()
        .map_or([0.0; 3], |t| [t.gap, t.style.size, t.style.letter_spacing]);
    [
        origin.x,
        origin.y,
        style.size,
        style.letter_spacing,
        *leading,
        halo,
    ]
    .into_iter()
    .chain(tail)
    .collect()
}

fn path_numbers(commands: &[PathCommand], stroke_width: f64) -> Vec<f64> {
    let mut numbers = vec![stroke_width];
    for command in commands {
        match command {
            PathCommand::MoveTo(p) => numbers.extend([p.x, p.y]),
            PathCommand::Arc { radius, end, .. } => numbers.extend([*radius, end.x, end.y]),
        }
    }
    numbers
}

/// Every group in `node`'s tree, depth first, in document order.
pub fn groups(node: &Node) -> Vec<&Group> {
    match node {
        Node::Group(g) => std::iter::once(g)
            .chain(g.children.iter().flat_map(groups))
            .collect(),
        _ => Vec::new(),
    }
}

// Redundant for the compiler (the module is already test-only), but it is
// how rustqual tells this module's own tests from the helpers it checks.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::block::fonts;
    use crate::style::KEY_LABEL;
    use crate::svg::{Anchor, Group, ItemKey, Tail};
    use crate::text::TEXT_ASCENT;

    const INK: Paint = Paint::Role(Role::Neutral(Neutral::TextMuted));

    fn text(anchor: Anchor, ink: TextPaint, lines: &[&str], leading: f64) -> Node {
        Node::Text {
            origin: Point { x: 100.0, y: 50.0 },
            style: KEY_LABEL,
            ink,
            anchor,
            lines: lines.iter().map(|line| (*line).to_owned()).collect(),
            words: lines.join(" "),
            leading,
            tail: None,
        }
    }

    fn line(from: Point, to: Point, stroke_width: f64) -> Node {
        Node::Line {
            from,
            to,
            stroke: INK,
            stroke_width,
        }
    }

    fn same(got: Rect, want: [f64; 4]) {
        let [left, top, right, bottom] = want;
        assert!(
            close(got.left, left)
                && close(got.top, top)
                && close(got.right, right)
                && close(got.bottom, bottom),
            "{got:?} is not {want:?}"
        );
    }

    fn width_of(fonts: &Fonts, label: &str) -> f64 {
        KEY_LABEL.measure(label, fonts)
    }

    #[test]
    #[expect(
        clippy::suboptimal_flops,
        reason = "NFR-4: plain * and + are bit-identical on every target; mul_add falls back to a libm fma where the CPU lacks one"
    )]
    fn a_middle_anchored_text_is_centred_on_its_origin() {
        let fonts = fonts();
        let node = text(Anchor::Middle, TextPaint::Fill(INK), &["Adopt"], 0.0);
        let got = bounds(&node, fonts).expect("text paints");
        let half = f64::midpoint(0.0, width_of(fonts, "Adopt"));
        let size = KEY_LABEL.size;
        same(
            got,
            [
                100.0 - half,
                50.0 - TEXT_ASCENT * size,
                100.0 + half,
                50.0 + TEXT_DESCENT * size,
            ],
        );
    }

    #[test]
    fn a_start_anchored_text_begins_at_its_origin() {
        let fonts = fonts();
        let node = text(Anchor::Start, TextPaint::Fill(INK), &["Adopt"], 0.0);
        let got = bounds(&node, fonts).expect("text paints");
        assert!(close(got.left, 100.0) && close(got.right, 100.0 + width_of(fonts, "Adopt")));
    }

    #[test]
    #[expect(
        clippy::suboptimal_flops,
        reason = "NFR-4: plain * and + are bit-identical on every target; mul_add falls back to a libm fma where the CPU lacks one"
    )]
    fn later_lines_step_down_by_the_leading() {
        let fonts = fonts();
        let node = text(Anchor::Start, TextPaint::Fill(INK), &["a", "b", "c"], 13.0);
        let got = bounds(&node, fonts).expect("text paints");
        assert!(close(
            got.bottom,
            50.0 + 26.0 + TEXT_DESCENT * KEY_LABEL.size
        ));
        assert!(close(got.top, 50.0 - TEXT_ASCENT * KEY_LABEL.size));
    }

    #[test]
    fn a_halo_widens_the_text_by_half_its_width_on_every_side() {
        let fonts = fonts();
        let plain = text(Anchor::Middle, TextPaint::Fill(INK), &["Adopt"], 0.0);
        let halo = TextPaint::Halo {
            stroke: INK,
            width: 4.0,
        };
        let haloed = text(Anchor::Middle, halo, &["Adopt"], 0.0);
        let (plain, haloed) = (
            bounds(&plain, fonts).expect("paints"),
            bounds(&haloed, fonts).expect("paints"),
        );
        same(
            haloed,
            [
                plain.left - 2.0,
                plain.top - 2.0,
                plain.right + 2.0,
                plain.bottom + 2.0,
            ],
        );
    }

    #[test]
    fn a_line_reaches_half_its_width_past_its_ends() {
        let fonts = fonts();
        let node = line(Point::ORIGIN, Point { x: 10.0, y: 0.0 }, 2.0);
        same(
            bounds(&node, fonts).expect("paints"),
            [-1.0, -1.0, 11.0, 1.0],
        );
    }

    #[test]
    fn a_circle_reaches_half_its_stroke_past_its_radius() {
        let fonts = fonts();
        let node = Node::Circle {
            centre: Point { x: 10.0, y: 20.0 },
            radius: 5.0,
            fill: INK,
            stroke: INK,
            stroke_width: 2.0,
        };
        same(
            bounds(&node, fonts).expect("paints"),
            [4.0, 14.0, 16.0, 26.0],
        );
    }

    #[test]
    fn a_rect_reaches_half_its_stroke_past_its_sides() {
        let fonts = fonts();
        let node = Node::Rect {
            origin: Point { x: 10.0, y: 20.0 },
            width: 30.0,
            height: 8.0,
            radius: 6.0,
            fill: INK,
            stroke: INK,
            stroke_width: 2.0,
        };
        same(
            bounds(&node, fonts).expect("paints"),
            [9.0, 19.0, 41.0, 29.0],
        );
    }

    #[test]
    fn a_hit_area_is_its_circle() {
        let fonts = fonts();
        let node = Node::HitArea {
            centre: Point { x: 10.0, y: 20.0 },
            radius: 16.0,
            key: ItemKey(3),
        };
        same(
            bounds(&node, fonts).expect("paints"),
            [-6.0, 4.0, 26.0, 36.0],
        );
    }

    #[test]
    fn a_half_circle_arc_is_bounded_by_its_circle() {
        let fonts = fonts();
        let node = Node::Path {
            commands: vec![
                PathCommand::MoveTo(Point { x: 0.0, y: 0.0 }),
                PathCommand::Arc {
                    radius: 5.0,
                    large_arc: false,
                    sweep: true,
                    end: Point { x: 0.0, y: 10.0 },
                },
            ],
            fill: INK,
            stroke: INK,
            stroke_width: 2.0,
        };
        same(
            bounds(&node, fonts).expect("paints"),
            [-6.0, -1.0, 6.0, 11.0],
        );
    }

    #[test]
    fn a_group_offset_shifts_the_box_and_nothing_paints_nothing() {
        let fonts = fonts();
        let inner = line(Point::ORIGIN, Point { x: 10.0, y: 0.0 }, 2.0);
        let node = Node::Group(Group::new(Point { x: 3.0, y: 4.0 }, vec![inner]));
        same(bounds(&node, fonts).expect("paints"), [2.0, 3.0, 14.0, 5.0]);
        let empty = Node::Group(Group::new(Point { x: 3.0, y: 4.0 }, Vec::new()));
        assert_eq!(bounds(&empty, fonts), None);
    }

    /// The text a panic carried; fails if the closure did not panic.
    fn panic_message(result: std::thread::Result<()>) -> String {
        let payload = result.expect_err("the check must panic");
        payload
            .downcast_ref::<String>()
            .cloned()
            .or_else(|| payload.downcast_ref::<&str>().map(|s| (*s).to_owned()))
            .expect("a panic message")
    }

    #[test]
    fn assert_inside_accepts_the_edge_and_names_what_leaves() {
        let fonts = fonts();
        let inner = line(Point::ORIGIN, Point { x: 10.0, y: 0.0 }, 2.0);
        let node = Node::Group(Group::new(Point { x: 1.0, y: 1.0 }, vec![inner]));
        let fits = Size {
            width: 12.0,
            height: 2.0,
        };
        assert_inside(&node, fits, fonts);
        let result = std::panic::catch_unwind(|| {
            assert_inside(
                &node,
                Size {
                    width: 11.0,
                    height: 2.0,
                },
                fonts,
            );
        });
        let message = panic_message(result);
        assert!(
            message.contains("Line") && message.contains("outside"),
            "the failure must name the node and say it is outside: {message}"
        );
    }

    #[test]
    fn assert_finite_refuses_nan_and_infinity() {
        let ok = line(Point::ORIGIN, Point { x: 1.0, y: 1.0 }, 1.0);
        assert_finite(&ok);
        let hit = |x| Node::HitArea {
            centre: Point { x, y: 0.0 },
            radius: 16.0,
            key: ItemKey(0),
        };
        assert_finite(&hit(1.0));
        let rect = |height| Node::Rect {
            origin: Point::ORIGIN,
            width: 1.0,
            height,
            radius: 0.0,
            fill: INK,
            stroke: INK,
            stroke_width: 1.0,
        };
        assert_finite(&rect(1.0));
        let message = panic_message(std::panic::catch_unwind(|| {
            assert_finite(&rect(f64::INFINITY));
        }));
        assert!(message.contains("inf"), "{message}");
        let message = panic_message(std::panic::catch_unwind(|| assert_finite(&hit(f64::NAN))));
        assert!(message.contains("NaN"), "{message}");
        for bad in [f64::NAN, f64::INFINITY] {
            let node = Node::Group(Group::new(
                Point::ORIGIN,
                vec![line(Point::ORIGIN, Point { x: bad, y: 1.0 }, 1.0)],
            ));
            let message = panic_message(std::panic::catch_unwind(|| assert_finite(&node)));
            assert!(
                message.contains(&bad.to_string()),
                "the failure must name {bad}: {message}"
            );
        }
    }

    /// A tail extends its text's last line from where that line ends at its
    /// layout width, not from the widest line: here it reaches past the
    /// first line, and the right edge is the tail's own.
    #[test]
    #[expect(
        clippy::suboptimal_flops,
        reason = "NFR-4: plain * and + are bit-identical on every target; mul_add falls back to a libm fma where the CPU lacks one"
    )]
    fn bounds_include_the_tail() {
        let fonts = fonts();
        let tail = Tail {
            gap: 5.0,
            style: KEY_LABEL,
            ink: INK,
            text: " Conditional · Moved In, and more".to_owned(),
        };
        let mut node = text(
            Anchor::Start,
            TextPaint::Fill(INK),
            &["a long first line", "b"],
            15.0,
        );
        if let Node::Text { tail: slot, .. } = &mut node {
            *slot = Some(tail.clone());
        }
        let rect = bounds(&node, fonts).expect("bounds");
        // From the last line's layout end, where a reader's spacing puts it
        // (D-78), not its drawn end.
        let tail_end =
            100.0 + KEY_LABEL.measure("b", fonts) + tail.gap + KEY_LABEL.measure(&tail.text, fonts);
        let first_end = 100.0 + KEY_LABEL.measure("a long first line", fonts);
        assert!(
            tail_end > first_end,
            "the fixture's tail passes the first line"
        );
        assert!((rect.right - tail_end).abs() < 1e-9, "{rect:?}");
        assert!((rect.bottom - (65.0 + TEXT_DESCENT * KEY_LABEL.size)).abs() < 1e-9);
    }
}
