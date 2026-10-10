//! Colour roles and palettes (architecture §5). No `Document` node carries a
//! colour value: it names a `Role`, and the emitter binds it — `var()` for
//! HTML, which carries both themes, and a literal for SVG and PNG, which are
//! fixed snapshots and whose consumers do not resolve custom properties.

use std::fmt;

/// FR-82's eight neutral roles, in the PRD's order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Neutral {
    Surface1,
    Surface2,
    TextPrimary,
    TextMuted,
    StrokeRing,
    StrokeDivider,
    /// FR-20's conditional fill. Not `Paint::None`: FR-66's key draws it on a
    /// different background, where the two differ (architecture §5).
    MarkerHollow,
    Focus,
}

impl Neutral {
    pub const ALL: [Self; 8] = [
        Self::Surface1,
        Self::Surface2,
        Self::TextPrimary,
        Self::TextMuted,
        Self::StrokeRing,
        Self::StrokeDivider,
        Self::MarkerHollow,
        Self::Focus,
    ];
}

impl fmt::Display for Neutral {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Surface1 => "--surface-1",
            Self::Surface2 => "--surface-2",
            Self::TextPrimary => "--text-primary",
            Self::TextMuted => "--text-muted",
            Self::StrokeRing => "--stroke-ring",
            Self::StrokeDivider => "--stroke-divider",
            Self::MarkerHollow => "--marker-hollow",
            Self::Focus => "--focus",
        })
    }
}

/// A colour a node asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Neutral(Neutral),
    /// The quadrant's 0-based index in FR-10's order — **not** reduced modulo
    /// the palette's length. FR-87 makes that length a property of a file the
    /// layout never reads; reducing here would make one radar two different
    /// documents under two palettes (architecture §5).
    Series(usize),
}

impl fmt::Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Neutral(n) => n.fmt(f),
            Self::Series(i) => write!(f, "--series-{}", i + 1),
        }
    }
}

/// FR-56. Picks the palette a static snapshot is bound to; HTML carries both.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum Theme {
    Light,
    Dark,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roles_display_as_their_css_custom_property() {
        let names: Vec<String> = Neutral::ALL.iter().map(ToString::to_string).collect();
        assert_eq!(
            names,
            [
                "--surface-1",
                "--surface-2",
                "--text-primary",
                "--text-muted",
                "--stroke-ring",
                "--stroke-divider",
                "--marker-hollow",
                "--focus",
            ]
        );
        assert_eq!(Role::Neutral(Neutral::Focus).to_string(), "--focus");
        assert_eq!(Role::Series(0).to_string(), "--series-1");
        // Unreduced: the role names the quadrant's index, not a palette slot.
        assert_eq!(Role::Series(8).to_string(), "--series-9");
    }
}
