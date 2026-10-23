//! The document tree blocks render into (architecture §5).
//!
//! A [`Node`] names colours by [`Role`], never by hex: the emitter binds a
//! role to a value late, so one tree serves every format and theme. Variants
//! and [`Semantics`] values are added by the PR that first needs them.

use std::num::NonZeroU8;

pub mod serialise;
mod tail;

pub use serialise::{Standalone, Target, serialise, standalone};
pub use tail::Tail;

use crate::palette::{Neutral, Role};
use crate::style::TextStyle;
use crate::text::{Fonts, TEXT_DESCENT};

/// How much of its advance a middle-anchored line starts before its
/// origin. A power of two, so it is exactly the `/ 2.0` resvg's anchor is.
const CENTRED: f64 = 0.5;

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
    /// An item's selection mark (FR-72, FR-73): a marker's square, inside
    /// its indicator, or a legend row's start bar. Shown by the page while
    /// the item is selected; interactive-only, and hidden until then.
    Selection(ItemKey),
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

impl Document {
    /// FR-57: the height at `width`, following the document's own aspect.
    #[must_use]
    pub fn height_at(&self, width: f64) -> f64 {
        width * self.height / self.width
    }

    /// The ground a static rendering paints under the drawing (D-87): the
    /// theme's `Surface1` over the whole document. The exported SVG and
    /// the PDF both paint this one rect.
    #[must_use]
    pub const fn ground(&self) -> Node {
        Node::Rect {
            origin: Point::ORIGIN,
            width: self.width,
            height: self.height,
            radius: 0.0,
            fill: Paint::Role(Role::Neutral(Neutral::Surface1)),
            stroke: Paint::None,
            stroke_width: 0.0,
        }
    }
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
        /// The text as written, whitespace collapsed: its name for a reader
        /// (D-88), which the lines cannot give back once a token is broken
        /// mid-word.
        words: String,
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
    /// as a browser places the `dx` tspan at default spacing. The exported
    /// SVG's outliner, which has no browser, places tails here (D-88). `None` for
    /// anything without a tail.
    #[must_use]
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

    /// Each line of a text with where it starts: the origin, one leading
    /// lower per line, a middle-anchored line moved back half its advance,
    /// as resvg places it. The exported SVG's outliner and the PDF both
    /// place lines here. Empty for anything but a text.
    #[expect(
        clippy::suboptimal_flops,
        reason = "NFR-4: plain * and + are bit-identical on every target; mul_add falls back to a libm fma where the CPU lacks one"
    )]
    #[must_use]
    pub fn line_starts(&self, fonts: &Fonts) -> Vec<(&str, Point)> {
        let Self::Text {
            origin,
            style,
            anchor,
            lines,
            leading,
            ..
        } = self
        else {
            return Vec::new();
        };
        (0_u32..)
            .zip(lines)
            .map(|(i, line)| {
                let x = match anchor {
                    Anchor::Start => origin.x,
                    Anchor::Middle => origin.x - style.advance(line, fonts) * CENTRED,
                };
                let y = origin.y + f64::from(i) * leading;
                (line.as_str(), Point { x, y })
            })
            .collect()
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
                Semantics::Indicator(_) | Semantics::FocusRing(_) | Semantics::Selection(_) => true,
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

    /// The runs this text draws, in paint order: each line in its style,
    /// then its tail in its own (D-78). Empty for anything but a text.
    #[must_use]
    fn text_runs(&self) -> Vec<(TextStyle, &str)> {
        match self {
            Self::Text {
                style, lines, tail, ..
            } => lines
                .iter()
                .map(|line| (*style, line.as_str()))
                .chain(tail.iter().map(|t| (t.style, t.text.as_str())))
                .collect(),
            Self::Group(_)
            | Self::Circle { .. }
            | Self::Rect { .. }
            | Self::Line { .. }
            | Self::Path { .. }
            | Self::HitArea { .. } => Vec::new(),
        }
    }

    /// Every run a static rendering draws, in document order: the
    /// runs of each text (its lines, then its tail) outside an
    /// interactive-only node. The exported SVG's glyph table (D-88) and
    /// the PDF's font subsets (FR-110) both collect through this, so
    /// neither can miss a glyph the writer then places.
    #[must_use]
    pub fn static_runs(&self) -> Vec<(TextStyle, &str)> {
        match self {
            _ if self.is_interactive_only() => Vec::new(),
            Self::Group(group) => group.children.iter().flat_map(Self::static_runs).collect(),
            _ => self.text_runs(),
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
    /// The text as written, whitespace collapsed: its name for a reader
    /// (D-88), which the lines cannot give back once a token is broken
    /// mid-word.
    pub words: String,
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
            words: self.words.clone(),
            leading: self.leading * scale.size,
            tail: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::palette::Neutral;
    use crate::text::Weight;

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
            words: "a".to_owned(),
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
            words: "a".to_owned(),
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
            words: lines.join(" "),
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

    fn style(weight: Weight) -> TextStyle {
        TextStyle {
            weight,
            size: 12.0,
            letter_spacing: 0.0,
        }
    }

    fn text(lines: &[&str], tail: Option<Tail>) -> Node {
        Node::Text {
            origin: Point::ORIGIN,
            style: style(Weight::Regular),
            ink: TextPaint::Fill(Paint::None),
            anchor: Anchor::Start,
            lines: lines.iter().map(|&l| l.to_owned()).collect(),
            words: lines.join(" "),
            leading: 14.0,
            tail,
        }
    }

    /// The runs a static rendering draws: lines in the text's style, then
    /// its tail in its own; nothing under an interactive-only node; document
    /// order (D-78, D-88).
    #[test]
    fn static_runs_are_what_a_static_rendering_draws() {
        let tail = Tail {
            gap: 4.0,
            style: style(Weight::SemiBold),
            ink: Paint::None,
            text: "\u{a0}meta".to_owned(),
        };
        let indicator = Node::Group(
            Group::new(Point::ORIGIN, vec![text(&["hidden"], None)])
                .with(Semantics::Indicator(ItemKey(3))),
        );
        let root = Node::Group(Group::new(
            Point::ORIGIN,
            vec![
                text(&["one", "two"], Some(tail)),
                indicator,
                Node::HitArea {
                    centre: Point::ORIGIN,
                    radius: 4.0,
                    key: ItemKey(3),
                },
                text(&["three"], None),
            ],
        ));
        let got: Vec<(Weight, &str)> = root
            .static_runs()
            .into_iter()
            .map(|(s, t)| (s.weight, t))
            .collect();
        assert_eq!(
            got,
            [
                (Weight::Regular, "one"),
                (Weight::Regular, "two"),
                (Weight::SemiBold, "\u{a0}meta"),
                (Weight::Regular, "three"),
            ]
        );
        assert!(
            Node::HitArea {
                centre: Point::ORIGIN,
                radius: 1.0,
                key: ItemKey(1)
            }
            .text_runs()
            .is_empty(),
            "only a text has runs"
        );
    }

    /// D-87: the ground is `Surface1` over the whole document.
    #[test]
    fn the_ground_covers_the_document() {
        let doc = Document {
            width: 300.0,
            height: 200.0,
            root: Node::Group(Group::new(Point::ORIGIN, Vec::new())),
        };
        assert_eq!(
            doc.ground(),
            Node::Rect {
                origin: Point::ORIGIN,
                width: 300.0,
                height: 200.0,
                radius: 0.0,
                fill: Paint::Role(Role::Neutral(Neutral::Surface1)),
                stroke: Paint::None,
                stroke_width: 0.0,
            }
        );
    }
}
