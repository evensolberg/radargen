//! The document tree blocks render into (architecture §5).
//!
//! A [`Node`] names colours by [`Role`], never by hex: the emitter binds a
//! role to a value late, so one tree serves every format and theme. Variants
//! and [`Semantics`] values are added by the PR that first needs them.

use std::num::NonZeroU8;

pub mod serialise;
mod tail;

pub use serialise::{Target, serialise};
pub use tail::Tail;

use crate::palette::{Neutral, Role};
use crate::style::TextStyle;
use crate::text::{Fonts, TEXT_DESCENT};

/// A point in SVG user units; the origin depends on the frame it is used in.
/// Geometry produces points before any block exists, so it lives here and
/// `layout` re-exports it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    /// Distance right of the frame's origin; negative is left.
    pub x: f64,
    /// Distance below the frame's origin (SVG's y-axis); negative is above.
    pub y: f64,
}

impl Point {
    /// The frame's origin.
    pub const ORIGIN: Self = Self { x: 0.0, y: 0.0 };

    /// Both coordinates multiplied by `k`.
    #[must_use]
    pub fn scaled(self, k: f64) -> Self {
        Self {
            x: self.x * k,
            y: self.y * k,
        }
    }
}

/// How far positions and sizes are multiplied; they differ inside the radar
/// (FR-70).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Scale {
    /// The factor for origins and offsets.
    pub position: f64,
    /// The factor for text sizes, letter-spacing and leading.
    pub size: f64,
}

impl Scale {
    /// Positions and sizes multiplied alike by `k`.
    #[must_use]
    pub const fn uniform(k: f64) -> Self {
        Self {
            position: k,
            size: k,
        }
    }
}

/// An item's identity in the output: its CSV line (architecture §5,
/// `data-row`). Not its number, which an unplotted row does not have.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ItemKey(pub usize);

/// An interactive item (FR-50).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Control {
    /// Which item this is.
    pub key: ItemKey,
    /// The accessible name.
    pub label: String,
    /// The text of the `<title>` tooltip; legend rows (PR 11) have none.
    pub tooltip: Option<String>,
}

/// A fill or stroke: a palette role, a number's ink on a series entry, or
/// nothing (architecture §5).
///
/// `None` is SVG's `none`, for stroked-but-unfilled shapes. It is not FR-20's
/// hollow marker, which fills with `Role::Neutral(Neutral::MarkerHollow)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Paint {
    /// Painted with this role; the emitter binds it to a value (FR-82).
    Role(Role),
    /// A marker's number ink on the series entry at this index: white or the
    /// dark ink, whichever contrasts more (FR-82). The index is unreduced,
    /// like [`Role::Series`]; the palette cycles it (FR-67).
    NumberOn(usize),
    /// Not painted: SVG's `none`.
    None,
}

/// How a text is painted.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TextPaint {
    /// Glyphs filled; no outline.
    Fill(Paint),
    /// An outline only, drawn under a filled copy so lines behind the text
    /// stop short of it. No fill, a round line join, and hidden from
    /// assistive technology: the filled copy is the one that is read.
    Halo {
        /// The outline's paint.
        stroke: Paint,
        /// The outline's width.
        width: f64,
    },
}

/// Which end of a text line sits at its origin.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Anchor {
    /// The line starts at the origin.
    Start,
    /// The line is centred on the origin.
    Middle,
}

/// What a group is for: its accessibility role, or its part in the
/// interactive layer (NFR-1, FR-50).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Semantics {
    /// Nothing to announce.
    #[default]
    None,
    /// Ornament, hidden from assistive technology.
    Decorative,
    /// A heading of this level; level zero does not exist.
    Heading(NonZeroU8),
    /// A named group: the radar, announced with its size before the user
    /// explores it (D-75). Kept in every format.
    Group(String),
    /// An interactive item; static formats keep its label and tooltip but
    /// not its role or focusability.
    Marker(Control),
    /// A legend row as an interactive control (FR-50): `data-row`, and no
    /// `data-marker`, which marks a radar marker. Static formats keep its
    /// label but not its role or focusability.
    LegendRow(Control),
    /// A focus and selection indicator. Two uses: the raised copy of an
    /// item's marker, and a legend row's inset rect. The emitter tells them
    /// apart by the enclosing control (`data-marker-indicator` or
    /// `data-row-indicator`). Interactive-only: static formats omit it.
    Indicator(ItemKey),
    /// A marker indicator's focus ring, marked so the page shows it apart
    /// from the rest of the indicator, while the marker has visible focus
    /// (FR-48, rg-jri). Interactive-only, inside an [`Semantics::Indicator`].
    FocusRing(ItemKey),
}

/// One step of a path outline.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PathCommand {
    /// Start a new sub-path at this point.
    MoveTo(Point),
    /// A circular arc of `radius` from the current point to `end`.
    Arc {
        /// The arc's radius.
        radius: f64,
        /// Take the larger of the two arcs.
        large_arc: bool,
        /// Sweep clockwise on screen.
        sweep: bool,
        /// Where the arc ends.
        end: Point,
    },
}

/// One arranged document (architecture §5): its viewBox runs from (0, 0)
/// to (`width`, `height`), both finite and positive.
#[derive(Debug, Clone, PartialEq)]
pub struct Document {
    /// The viewBox's width, in user units.
    pub width: f64,
    /// The viewBox's height, in user units.
    pub height: f64,
    /// Everything drawn, in document order.
    pub root: Node,
}

/// Children translated together; a group never scales (FR-70).
#[derive(Debug, Clone, PartialEq)]
pub struct Group {
    /// Where the children's origin sits.
    pub offset: Point,
    /// What the group means to assistive technology.
    pub semantics: Semantics,
    /// The children, in paint order.
    pub children: Vec<Node>,
}

impl Group {
    /// A group with no semantics.
    #[must_use]
    pub const fn new(offset: Point, children: Vec<Node>) -> Self {
        Self {
            offset,
            semantics: Semantics::None,
            children,
        }
    }

    /// This group with `semantics`.
    #[must_use]
    pub fn with(self, semantics: Semantics) -> Self {
        Self { semantics, ..self }
    }
}

/// A drawn element. There is no `foreignObject` variant (FR-47), and no
/// colour is a hex value.
#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    /// Children translated by an offset.
    Group(Group),
    /// Lines of text; the first line's baseline is at `origin`.
    Text {
        /// Where the first line's baseline starts.
        origin: Point,
        /// The face, size and spacing the text is measured and drawn with (FR-85).
        style: TextStyle,
        /// How the text is painted.
        ink: TextPaint,
        /// Which end of each line sits at `origin`.
        anchor: Anchor,
        /// The lines, already wrapped.
        lines: Vec<String>,
        /// Distance from one baseline to the next.
        leading: f64,
        /// Text continuing the last line in its own style, or `None`.
        tail: Option<Tail>,
    },
    /// A circle.
    Circle {
        /// The circle's centre.
        centre: Point,
        /// The circle's radius.
        radius: f64,
        /// The interior paint.
        fill: Paint,
        /// The outline paint.
        stroke: Paint,
        /// The outline's width.
        stroke_width: f64,
    },
    /// A rectangle with rounded corners, `radius` on every corner.
    Rect {
        /// The top-left corner.
        origin: Point,
        /// The width, right of `origin`.
        width: f64,
        /// The height, below `origin`.
        height: f64,
        /// The corner radius; zero for square corners.
        radius: f64,
        /// The interior paint.
        fill: Paint,
        /// The outline paint.
        stroke: Paint,
        /// The outline's width.
        stroke_width: f64,
    },
    /// An invisible pointer target (FR-94). Interactive-only: static formats
    /// omit it.
    HitArea {
        /// The target's centre.
        centre: Point,
        /// The target's radius.
        radius: f64,
        /// The item whose control receives the pointer events.
        key: ItemKey,
    },
    /// A straight line.
    Line {
        /// Where the line starts.
        from: Point,
        /// Where the line ends.
        to: Point,
        /// The line's paint.
        stroke: Paint,
        /// The line's width.
        stroke_width: f64,
    },
    /// An outline built from commands.
    Path {
        /// The outline's steps.
        commands: Vec<PathCommand>,
        /// The interior paint.
        fill: Paint,
        /// The outline paint.
        stroke: Paint,
        /// The outline's width.
        stroke_width: f64,
    },
}

impl Node {
    /// Where a text's tail is drawn: the last line's drawn end plus the gap,
    /// as a browser places the `dx` tspan at default spacing. PR 17's
    /// outliner, which has no browser, places tails here. `None` for
    /// anything without a tail.
    #[must_use]
    // qual:api — PR 17's outliner is its first production caller
    pub fn tail_x(&self, fonts: &Fonts) -> Option<f64> {
        let Self::Text {
            origin,
            style,
            lines,
            tail: Some(tail),
            ..
        } = self
        else {
            return None;
        };
        let last = lines.last().map_or("", String::as_str);
        Some(origin.x + style.advance(last, fonts) + tail.gap)
    }

    /// Whether the node exists only for pointer and keyboard users: a hit
    /// area, an indicator group, or a non-empty plain (`Semantics::None`)
    /// group of nothing else. A group with any other semantics (a control, a
    /// heading) is never dropped whole. A static format (SVG, PNG) omits
    /// such nodes whole.
    #[must_use]
    pub fn is_interactive_only(&self) -> bool {
        match self {
            Self::HitArea { .. } => true,
            Self::Group(group) => match group.semantics {
                Semantics::Indicator(_) | Semantics::FocusRing(_) => true,
                Semantics::None => {
                    !group.children.is_empty()
                        && group.children.iter().all(Self::is_interactive_only)
                }
                Semantics::Decorative
                | Semantics::Heading(_)
                | Semantics::Group(_)
                | Semantics::Marker(_)
                | Semantics::LegendRow(_) => false,
            },
            Self::Circle { .. }
            | Self::Rect { .. }
            | Self::Line { .. }
            | Self::Path { .. }
            | Self::Text { .. } => false,
        }
    }
}

/// A block of text, described at scale 1.
#[derive(Debug, Clone, PartialEq)]
pub struct TextRun {
    /// Where the first line's baseline starts.
    pub origin: Point,
    /// The face, size and spacing.
    pub style: TextStyle,
    /// The text's neutral colour role.
    pub ink: Neutral,
    /// Which end of each line sits at `origin`.
    pub anchor: Anchor,
    /// The lines, already wrapped.
    pub lines: Vec<String>,
    /// Distance from one baseline to the next.
    pub leading: f64,
}

impl TextRun {
    /// The text node at `scale`: the origin is multiplied by the position
    /// factor; the style's sizes and the leading by the size factor. Both
    /// happen here, in one place.
    #[must_use]
    pub fn node(&self, scale: Scale) -> Node {
        self.painted(TextPaint::Fill(Paint::Role(Role::Neutral(self.ink))), scale)
    }

    /// The same text as an outline of `width` (at scale 1), to draw under
    /// [`node`](Self::node) in the surface colour so lines behind it stop
    /// short of the glyphs.
    #[must_use]
    pub fn halo(&self, width: f64, scale: Scale) -> Node {
        self.painted(
            TextPaint::Halo {
                stroke: Paint::Role(Role::Neutral(Neutral::Surface1)),
                width: width * scale.size,
            },
            scale,
        )
    }

    /// The baseline of the last line, at scale 1.
    #[must_use]
    pub const fn last_baseline(&self) -> f64 {
        #[expect(
            clippy::cast_precision_loss,
            clippy::as_conversions,
            reason = "a line count is far below 2^53, so the conversion is exact"
        )]
        let gaps = self.lines.len().saturating_sub(1) as f64;
        self.origin.y + gaps * self.leading
    }

    /// The lowest extent of the last line's descenders, at scale 1.
    #[must_use]
    pub const fn bottom(&self) -> f64 {
        self.last_baseline() + TEXT_DESCENT * self.style.size
    }

    /// The text node at `scale`, its last line continued by `tail` (FR-74).
    #[must_use]
    pub fn node_with_tail(&self, tail: &Tail, scale: Scale) -> Node {
        debug_assert_eq!(
            self.anchor,
            Anchor::Start,
            "a tail follows start-anchored text"
        );
        let mut node = self.node(scale);
        if let Node::Text { tail: slot, .. } = &mut node {
            *slot = Some(tail.scaled(scale.size));
        }
        node
    }

    /// The furthest right any line reaches at its layout width (D-78), at
    /// scale 1.
    #[must_use]
    pub fn right_edge(&self, fonts: &Fonts) -> f64 {
        self.lines
            .iter()
            .map(|line| self.origin.x + self.style.measure(line, fonts))
            .fold(0.0, f64::max)
    }

    fn painted(&self, ink: TextPaint, scale: Scale) -> Node {
        Node::Text {
            origin: self.origin.scaled(scale.position),
            style: self.style.scaled(scale.size),
            ink,
            anchor: self.anchor,
            lines: self.lines.clone(),
            leading: self.leading * scale.size,
            tail: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::palette::Neutral;

    const LEVEL_1: NonZeroU8 = NonZeroU8::MIN;

    #[test]
    fn none_is_not_the_hollow_marker_role() {
        assert_ne!(
            Paint::None,
            Paint::Role(Role::Neutral(Neutral::MarkerHollow))
        );
    }

    #[test]
    fn a_new_group_has_no_semantics_and_with_sets_them() {
        let group = Group::new(Point { x: 1.0, y: 2.0 }, Vec::new());
        assert_eq!(group.semantics, Semantics::None);
        assert_eq!(
            group.with(Semantics::Heading(LEVEL_1)).semantics,
            Semantics::Heading(LEVEL_1)
        );
    }

    fn run(anchor: Anchor) -> TextRun {
        TextRun {
            origin: Point { x: 10.0, y: 20.0 },
            style: TextStyle {
                weight: crate::text::Weight::Regular,
                size: 12.0,
                letter_spacing: 0.48,
            },
            ink: Neutral::TextMuted,
            anchor,
            lines: vec!["a".to_owned()],
            leading: 15.0,
        }
    }

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    #[test]
    fn scale_splits_position_from_size() {
        let Node::Text {
            origin,
            style,
            anchor,
            leading,
            ..
        } = run(Anchor::Middle).node(Scale {
            position: 0.5,
            size: 1.0,
        })
        else {
            panic!("expected text");
        };
        assert!(close(origin.x, 5.0) && close(origin.y, 10.0), "{origin:?}");
        assert!(close(style.size, 12.0) && close(style.letter_spacing, 0.48));
        assert!(close(leading, 15.0));
        assert_eq!(anchor, Anchor::Middle);

        let Node::Text {
            origin,
            style,
            leading,
            ..
        } = run(Anchor::Middle).node(Scale::uniform(2.0))
        else {
            panic!("expected text");
        };
        assert!(close(origin.x, 20.0) && close(origin.y, 40.0), "{origin:?}");
        assert!(close(style.size, 24.0) && close(style.letter_spacing, 0.96));
        assert!(close(leading, 30.0));
    }

    #[test]
    fn halo_is_an_outline_of_the_same_text() {
        let scale = Scale {
            position: 0.5,
            size: 2.0,
        };
        let text = run(Anchor::Middle);
        let halo = text.halo(3.0, scale);
        let Node::Text { ink, .. } = &halo else {
            panic!("expected text");
        };
        assert_eq!(
            *ink,
            TextPaint::Halo {
                stroke: Paint::Role(Role::Neutral(Neutral::Surface1)),
                width: 6.0
            }
        );
        // Both come from the same arithmetic, so with the halo's ink the
        // filled text is the halo exactly.
        let mut filled = text.node(scale);
        let Node::Text {
            ink: filled_ink, ..
        } = &mut filled
        else {
            panic!("expected text");
        };
        *filled_ink = *ink;
        assert_eq!(filled, halo);
    }

    #[test]
    fn ordinary_text_is_filled() {
        let Node::Text { ink, .. } = run(Anchor::Start).node(Scale::uniform(1.0)) else {
            panic!("expected text");
        };
        assert_eq!(
            ink,
            TextPaint::Fill(Paint::Role(Role::Neutral(Neutral::TextMuted)))
        );
    }

    #[test]
    fn scaled_multiplies_both_coordinates() {
        let p = Point { x: 3.0, y: -4.0 }.scaled(0.5);
        assert_eq!(p, Point { x: 1.5, y: -2.0 });
    }

    #[test]
    fn a_group_keeps_its_children_in_order() {
        let circle = |radius| Node::Circle {
            centre: Point { x: 0.0, y: 0.0 },
            radius,
            fill: Paint::None,
            stroke: Paint::None,
            stroke_width: 1.0,
        };
        let group = Group::new(Point { x: 0.0, y: 0.0 }, vec![circle(1.0), circle(2.0)]);
        assert_eq!(group.children, vec![circle(1.0), circle(2.0)]);
    }

    fn hit(key: usize) -> Node {
        Node::HitArea {
            centre: Point::ORIGIN,
            radius: 16.0,
            key: ItemKey(key),
        }
    }

    fn disc() -> Node {
        Node::Circle {
            centre: Point::ORIGIN,
            radius: 1.0,
            fill: Paint::None,
            stroke: Paint::None,
            stroke_width: 1.0,
        }
    }

    fn rect() -> Node {
        Node::Rect {
            origin: Point::ORIGIN,
            width: 4.0,
            height: 3.0,
            radius: 1.0,
            fill: Paint::None,
            stroke: Paint::None,
            stroke_width: 1.0,
        }
    }

    #[test]
    fn interactive_only_nodes_say_so() {
        let group = |children| Group::new(Point::ORIGIN, children);
        assert!(hit(1).is_interactive_only());
        let indicator = group(vec![disc()]).with(Semantics::Indicator(ItemKey(1)));
        assert!(Node::Group(indicator).is_interactive_only());
        assert!(Node::Group(group(vec![hit(1), hit(2)])).is_interactive_only());
        assert!(!Node::Group(group(Vec::new())).is_interactive_only());
        assert!(!Node::Group(group(vec![hit(1), disc()])).is_interactive_only());
        let control = |children| {
            let control = Control {
                key: ItemKey(1),
                label: "1".to_owned(),
                tooltip: None,
            };
            Node::Group(group(children).with(Semantics::Marker(control)))
        };
        assert!(!control(vec![disc()]).is_interactive_only());
        assert!(!control(vec![hit(1)]).is_interactive_only());
        let indicator = Node::Group(group(vec![disc()]).with(Semantics::Indicator(ItemKey(1))));
        assert!(!control(vec![indicator]).is_interactive_only());
        let heading = group(vec![hit(1)]).with(Semantics::Heading(LEVEL_1));
        assert!(!Node::Group(heading).is_interactive_only());
        assert!(!disc().is_interactive_only());
        assert!(!rect().is_interactive_only());
        let row = Control {
            key: ItemKey(1),
            label: "1".to_owned(),
            tooltip: None,
        };
        let row = group(vec![rect(), hit(1)]).with(Semantics::LegendRow(row));
        assert!(!Node::Group(row).is_interactive_only());
        let text = Node::Text {
            origin: Point::ORIGIN,
            style: crate::style::KEY_LABEL,
            ink: TextPaint::Fill(Paint::None),
            anchor: Anchor::Start,
            lines: vec!["a".to_owned()],
            leading: 0.0,
            tail: None,
        };
        assert!(!text.is_interactive_only());
    }

    #[test]
    fn number_ink_is_a_paint_of_its_own() {
        assert_ne!(Paint::NumberOn(0), Paint::None);
        assert_ne!(Paint::NumberOn(0), Paint::Role(Role::Series(0)));
    }

    fn lines_run(lines: &[&str]) -> TextRun {
        TextRun {
            origin: Point { x: 10.0, y: 20.0 },
            style: crate::style::LEGEND_NAME,
            ink: Neutral::TextPrimary,
            anchor: Anchor::Start,
            lines: lines.iter().map(|&l| l.to_owned()).collect(),
            leading: 15.0,
        }
    }

    fn tail() -> Tail {
        Tail {
            gap: 5.0,
            style: crate::style::LEGEND_META,
            ink: Paint::Role(Role::Neutral(Neutral::TextMuted)),
            text: " Default".to_owned(),
        }
    }

    #[test]
    #[expect(
        clippy::suboptimal_flops,
        reason = "NFR-4: plain * and + are bit-identical on every target; mul_add falls back to a libm fma where the CPU lacks one"
    )]
    fn a_tail_scales_with_its_run() {
        let doubled = lines_run(&["Kafka"]).node_with_tail(
            &tail(),
            Scale {
                position: 1.0,
                size: 2.0,
            },
        );
        let Node::Text {
            tail: Some(scaled), ..
        } = doubled
        else {
            panic!("a tailed text: {doubled:?}");
        };
        assert!((scaled.gap - 10.0).abs() < 1e-9, "the gap doubles");
        assert!((scaled.style.size - 2.0 * crate::style::LEGEND_META.size).abs() < 1e-9);
    }

    /// The tail starts at the last line's drawn end, not the widest line's.
    #[test]
    fn tail_x_follows_the_last_line() {
        let fonts = crate::text::Fonts::embedded().expect("the vendored fonts parse");
        let node = lines_run(&["A much longer first line", "two"])
            .node_with_tail(&tail(), Scale::uniform(1.0));
        let end = crate::style::LEGEND_NAME.advance("two", &fonts);
        let x = node.tail_x(&fonts).expect("a tail");
        assert!((x - (10.0 + end + 5.0)).abs() < 1e-9, "{x}");
        assert_eq!(
            lines_run(&["two"]).node(Scale::uniform(1.0)).tail_x(&fonts),
            None
        );
    }

    #[test]
    fn a_halo_has_no_tail() {
        let halo = lines_run(&["Kafka"]).halo(3.0, Scale::uniform(1.0));
        assert!(matches!(halo, Node::Text { tail: None, .. }), "{halo:?}");
    }
}
