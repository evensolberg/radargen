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

/// A fill or stroke: a palette role, or nothing (architecture §5).
///
/// `None` is SVG's `none`, for stroked-but-unfilled shapes. It is not FR-20's
/// hollow marker, which fills with `Role::Neutral(Neutral::MarkerHollow)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Paint {
    /// Painted with this role; the emitter binds it to a value (FR-82).
    Role(Role),
    /// Not painted: SVG's `none`.
    None,
}

/// Which end of a text line sits at its origin.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Anchor {
    /// The line starts at the origin.
    Start,
    /// The line is centred on the origin.
    Middle,
}

/// What a group means to assistive technology (NFR-1, FR-50).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Semantics {
    /// Nothing to announce.
    #[default]
    None,
    /// Ornament, hidden from assistive technology.
    Decorative,
    /// A heading of this level; level zero does not exist.
    Heading(NonZeroU8),
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
        /// The text colour.
        ink: Paint,
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
    /// Start-anchored text at scale `scale`: `origin`, the style's sizes and
    /// `leading` are given at scale 1 and multiplied here, in one place.
    #[must_use]
    pub fn text(
        origin: Point,
        style: TextStyle,
        ink: Neutral,
        lines: Vec<String>,
        leading: f64,
        scale: f64,
    ) -> Self {
        Self::Text {
            origin: origin.scaled(scale),
            style: style.scaled(scale),
            ink: Paint::Role(Role::Neutral(ink)),
            anchor: Anchor::Start,
            lines,
            leading: leading * scale,
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

    #[test]
    fn text_scales_origin_style_and_leading_together() {
        let style = TextStyle {
            weight: crate::text::Weight::Regular,
            size: 10.0,
            letter_spacing: 1.0,
        };
        let node = Node::text(
            Point { x: 2.0, y: 4.0 },
            style,
            Neutral::TextMuted,
            vec!["a".to_owned()],
            12.0,
            0.5,
        );
        let Node::Text {
            origin,
            style,
            leading,
            anchor,
            ..
        } = node
        else {
            panic!("expected text");
        };
        assert_eq!(origin, Point { x: 1.0, y: 2.0 });
        assert!((style.size - 5.0).abs() < 1e-9 && (style.letter_spacing - 0.5).abs() < 1e-9);
        assert!((leading - 6.0).abs() < 1e-9);
        assert_eq!(anchor, Anchor::Start);
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
}
