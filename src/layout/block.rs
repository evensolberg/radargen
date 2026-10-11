//! The `Block` abstraction (architecture §4): a laid-out region that knows
//! its size and can be drawn anywhere.

use crate::svg::{Node, Point};

/// A width and height in user units.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Size {
    /// Extent along x.
    pub width: f64,
    /// Extent along y.
    pub height: f64,
}

impl Size {
    /// Both sides multiplied by `scale`.
    #[must_use]
    pub fn scaled(self, scale: f64) -> Self {
        Self {
            width: self.width * scale,
            height: self.height * scale,
        }
    }

    /// Whether this is the zero size, which a block with nothing to draw has.
    #[must_use]
    pub fn is_empty(self) -> bool {
        self == Self::default()
    }
}

/// Where and how large a block is drawn (FR-70).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placement {
    /// Where the block's top-left lands.
    pub origin: Point,
    /// The scale applied to the block's coordinates.
    pub scale: f64,
}

impl Placement {
    /// Placement at `origin` at scale 1.
    #[must_use]
    pub const fn at(origin: Point) -> Self {
        Self { origin, scale: 1.0 }
    }
}

/// A laid-out region that knows its own size and can be drawn anywhere.
/// Size comes from measured text (FR-84), never a guess, so an arrangement
/// can place blocks without re-running layout.
pub trait Block {
    /// Size when drawn at `scale`.
    fn size(&self, scale: f64) -> Size;
    /// The block drawn at `at.scale`, translated to `at.origin`. Scale is
    /// applied to coordinates, never as an SVG transform (FR-70).
    fn render(&self, at: Placement) -> Node;
}

/// Test helper: the vendored fonts.
#[cfg(test)]
pub(crate) fn fonts() -> crate::text::Fonts {
    crate::text::Fonts::embedded().expect("the vendored fonts parse")
}

/// How close two computed lengths must be to count as equal in tests.
#[cfg(test)]
const TOLERANCE: f64 = 1e-9;

/// Test helper: whether `a` and `b` agree to within [`TOLERANCE`].
#[cfg(test)]
pub(crate) fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < TOLERANCE
}

/// Test helper: a placement at the origin at `scale`.
#[cfg(test)]
pub(crate) const fn at(scale: f64) -> Placement {
    Placement {
        origin: Point::ORIGIN,
        scale,
    }
}

#[cfg(test)]
const TRANSLATED_TO: Point = Point { x: 5.0, y: 7.0 };

/// Test helper: `block` rendered at (5, 7) is its origin rendering with only
/// the root group's offset changed.
///
/// # Panics
///
/// If the block does not render a group, or the placement changes anything
/// but the offset.
#[cfg(test)]
pub(crate) fn assert_translates(block: &impl Block) {
    let rendered = |origin| {
        let at = Placement { origin, scale: 1.0 };
        let Node::Group(group) = block.render(at) else {
            panic!("a block renders a group");
        };
        group
    };
    let home = rendered(Point::ORIGIN);
    let placed = rendered(TRANSLATED_TO);
    assert_eq!(placed.offset, TRANSLATED_TO);
    assert_eq!(placed.children, home.children);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scaled_multiplies_both_sides() {
        let size = Size {
            width: 3.0,
            height: 4.0,
        }
        .scaled(2.0);
        assert_eq!(
            size,
            Size {
                width: 6.0,
                height: 8.0
            }
        );
    }

    #[test]
    fn only_the_default_size_is_empty() {
        assert!(Size::default().is_empty());
        assert!(
            !Size {
                width: 1.0,
                height: 0.0
            }
            .is_empty()
        );
    }

    #[test]
    fn at_places_at_scale_one() {
        let at = Placement::at(Point { x: 5.0, y: 6.0 });
        assert_eq!(at.origin, Point { x: 5.0, y: 6.0 });
        assert!((at.scale - 1.0).abs() < f64::EPSILON);
    }
}
