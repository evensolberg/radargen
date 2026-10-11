//! The document tree blocks render into (architecture §5).
//!
//! A [`Node`] names colours by [`Role`], never by hex: the emitter binds a
//! role to a value late, so one tree serves every format and theme. Variants
//! and [`Semantics`] values are added by the PR that first needs them.

use std::num::NonZeroU8;

use crate::palette::{Neutral, Role};
use crate::style::TextStyle;

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
    /// An interactive item; static formats keep its label and tooltip but
    /// not its role or focusability.
    Marker(Control),
    /// The raised copy of an item's marker, shown while the control has
    /// focus. Interactive-only: static formats omit it.
    Indicator(ItemKey),
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
                Semantics::Indicator(_) => true,
                Semantics::None => {
                    !group.children.is_empty()
                        && group.children.iter().all(Self::is_interactive_only)
                }
                Semantics::Decorative | Semantics::Heading(_) | Semantics::Marker(_) => false,
            },
            Self::Circle { .. } | Self::Line { .. } | Self::Path { .. } | Self::Text { .. } => {
                false
            }
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

    fn painted(&self, ink: TextPaint, scale: Scale) -> Node {
        Node::Text {
            origin: self.origin.scaled(scale.position),
            style: self.style.scaled(scale.size),
            ink,
            anchor: self.anchor,
            lines: self.lines.clone(),
            leading: self.leading * scale.size,
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

    #[test]
    fn interactive_only_nodes_say_so() {
        let group = |children| Group::new(Point::ORIGIN, children);
        assert!(hit(1).is_interactive_only());
        let indicator = group(vec![disc()]).with(Semantics::Indicator(ItemKey(1)));
        assert!(Node::Group(indicator).is_interactive_only());
        assert!(Node::Group(group(vec![hit(1), hit(2)])).is_interactive_only());
        assert!(!Node::Group(group(Vec::new())).is_interactive_only());
        assert!(!Node::Group(group(vec![hit(1), disc()])).is_interactive_only());
        let control = Control {
            key: ItemKey(1),
            label: "1".to_owned(),
            tooltip: None,
        };
        let control = group(vec![disc()]).with(Semantics::Marker(control));
        assert!(!Node::Group(control).is_interactive_only());
        let control = |children| {
            let control = Control {
                key: ItemKey(1),
                label: "1".to_owned(),
                tooltip: None,
            };
            Node::Group(group(children).with(Semantics::Marker(control)))
        };
        assert!(!control(vec![hit(1)]).is_interactive_only());
        let indicator = Node::Group(group(vec![disc()]).with(Semantics::Indicator(ItemKey(1))));
        assert!(!control(vec![indicator]).is_interactive_only());
        let heading = group(vec![hit(1)]).with(Semantics::Heading(LEVEL_1));
        assert!(!Node::Group(heading).is_interactive_only());
        assert!(!disc().is_interactive_only());
        let text = Node::Text {
            origin: Point::ORIGIN,
            style: crate::style::KEY_LABEL,
            ink: TextPaint::Fill(Paint::None),
            anchor: Anchor::Start,
            lines: vec!["a".to_owned()],
            leading: 0.0,
        };
        assert!(!text.is_interactive_only());
    }

    #[test]
    fn number_ink_is_a_paint_of_its_own() {
        assert_ne!(Paint::NumberOn(0), Paint::None);
        assert_ne!(Paint::NumberOn(0), Paint::Role(Role::Series(0)));
    }
}
