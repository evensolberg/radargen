//! The SVG export (FR-54, FR-57, FR-71, D-88): the wide document with
//! literal colours for one theme over its ground, its text as outlines,
//! sized by `--width`, under the version comment (SC-25).

use crate::diagnostics::Fatal;
use crate::emit::{OUTPUT_VERSION, UNTITLED};
use crate::palette::Palette;
use crate::svg::{Document, Node, Semantics, Standalone, standalone};
use crate::text::Fonts;

/// What one export needs besides the document.
pub struct Export<'a> {
    /// The theme's palette (FR-56).
    pub palette: &'a Palette,
    /// The faces the text is shaped and outlined from.
    pub fonts: &'a Fonts,
    /// `--width` (FR-57).
    pub width: Option<u32>,
    /// FR-65's title, resolved.
    pub title: Option<&'a str>,
}

/// FR-57: `requested` wide and the height in proportion, or the
/// document's own size. Unrounded and unclamped (D-88).
#[must_use]
pub fn intrinsic_size(document: &Document, requested: Option<u32>) -> (f64, f64) {
    requested.map_or((document.width, document.height), |width| {
        let width = f64::from(width);
        (width, width * document.height / document.width)
    })
}

/// FR-54: `document` as a standalone SVG file (D-88).
///
/// # Errors
///
/// [`Fatal::Font`] when a glyph cannot be outlined (Ruling 6).
pub fn export(document: &Document, export: &Export<'_>) -> Result<String, Fatal> {
    let (width, height) = intrinsic_size(document, export.width);
    let root = standalone(
        document,
        &Standalone {
            palette: export.palette,
            fonts: export.fonts,
            width,
            height,
            title: export.title.unwrap_or(UNTITLED),
            desc: radar_name(&document.root),
        },
    )?;
    // SC-25: the emitter's line, before the root (§6).
    Ok(["<!-- ", OUTPUT_VERSION, " -->\n", &root].concat())
}

/// The radar's group name (D-75): the first named group, depth first.
/// The wide arrangement has exactly one, the radar's; an arrangement
/// with a second would need this to say which.
fn radar_name(node: &Node) -> Option<&str> {
    match node {
        Node::Group(group) => match &group.semantics {
            Semantics::Group(name) => Some(name),
            _ => group.children.iter().find_map(radar_name),
        },
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::arrange::{Arrangement, Blocks, arrange};
    use crate::layout::block::fonts;
    use crate::layout::test_support::{PRD_EXAMPLE, plotted};
    use crate::palette::Theme;

    fn prd(title: Option<&str>) -> Document {
        let (catalogue, order) = plotted(PRD_EXAMPLE, None);
        let blocks = Blocks::new(&catalogue, &order, title, None, fonts());
        arrange(Arrangement::Wide, &blocks)
    }

    /// FR-57, D-88: the height follows the width, unrounded.
    #[test]
    fn the_intrinsic_size_follows_the_width() {
        let doc = prd(None);
        assert_eq!(intrinsic_size(&doc, None), (doc.width, doc.height));
        let (w, h) = intrinsic_size(&doc, Some(320));
        assert_eq!(w, 320.0);
        assert_eq!(h, 320.0 * doc.height / doc.width);
    }

    /// D-88: no clamp; no pixmap is allocated.
    #[test]
    fn a_wide_default_is_not_clamped() {
        let doc = Document {
            width: 12_500.0,
            ..prd(None)
        };
        assert_eq!(intrinsic_size(&doc, None).0, 12_500.0);
    }

    /// SC-25, D-75, D-88: the comment, then the title or Technology Radar,
    /// then the radar's name.
    #[test]
    fn the_file_is_named() {
        let palette = Palette::built_in(Theme::Light);
        let export = |title| Export {
            palette: &palette,
            fonts: fonts(),
            width: None,
            title,
        };
        let titled =
            super::export(&prd(Some("Q3 & co")), &export(Some("Q3 & co"))).expect("exports");
        assert!(titled.starts_with(&format!("<!-- {} -->\n<svg ", crate::emit::OUTPUT_VERSION)));
        assert!(titled.contains("<title>Q3 &amp; co</title>"), "{titled}");
        let untitled = super::export(&prd(None), &export(None)).expect("exports");
        assert!(untitled.contains("<title>Technology Radar</title>"));
        assert!(
            untitled.contains("<desc>Technology radar: 3 quadrants, "),
            "{untitled}"
        );
    }
}
