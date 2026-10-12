//! Text that continues another text's last line (FR-74, D-78).

use super::Paint;
use crate::style::TextStyle;
use crate::text::Fonts;

/// Text that continues a text node's last line in its own style: the
/// legend's inline qualifier and status (FR-74).
///
/// Written inside the same `<text>`, so a browser places it after the
/// line's rendered end. A reader's text spacing (WCAG 1.4.12) then pushes it
/// along rather than into the line (D-78). Only start-anchored, filled text
/// carries one.
#[derive(Debug, Clone, PartialEq)]
pub struct Tail {
    /// From the last line's end to the tail's start, where its text begins
    /// (usually with a space, so the two read as separate words).
    pub gap: f64,
    /// The tail's face, size and spacing.
    pub style: TextStyle,
    /// The tail's paint.
    pub ink: Paint,
    /// The tail's text.
    pub text: String,
}

impl Tail {
    /// How far the tail reaches past its line's end at the layout width:
    /// the gap and its own layout width (D-78).
    #[must_use]
    pub fn layout_width(&self, fonts: &Fonts) -> f64 {
        self.gap + self.style.measure(&self.text, fonts)
    }

    /// The tail with its gap and sizes multiplied by `size` (FR-70).
    #[must_use]
    pub fn scaled(&self, size: f64) -> Self {
        Self {
            gap: self.gap * size,
            style: self.style.scaled(size),
            ink: self.ink,
            text: self.text.clone(),
        }
    }
}
