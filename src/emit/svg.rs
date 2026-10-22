//! The SVG export (FR-54, FR-57, FR-71, D-88): the wide document with
//! literal colours for one theme over its ground, its text as outlines,
//! sized by `--width`, under the version comment (SC-25).

use crate::diagnostics::Fatal;
use crate::emit::{OUTPUT_VERSION, UNTITLED};
use crate::palette::Palette;
use crate::svg::{Document, Standalone, standalone};
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
    /// The radar's name (D-75), the root's `<desc>`: `RadarBlock::name`.
    pub desc: &'a str,
}

/// FR-57: `requested` wide and the height in proportion, or the
/// document's own size. Unrounded and unclamped: no pixmap is allocated
/// (D-88).
#[must_use]
pub fn intrinsic_size(document: &Document, requested: Option<u32>) -> (f64, f64) {
    requested.map_or((document.width, document.height), |width| {
        let width = f64::from(width);
        (width, document.height_at(width))
    })
}

/// FR-54: `document` as a standalone SVG file (D-88).
///
/// # Errors
///
/// [`Fatal::Font`] when a glyph cannot be outlined (NFR-15).
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
            desc: export.desc,
        },
    )?;
    // SC-25: the emitter's line, before the root (§6).
    Ok(["<!-- ", OUTPUT_VERSION, " -->\n", &root].concat())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::arrange::{Arrangement, Blocks, arrange};
    use crate::layout::block::fonts;
    use crate::layout::test_support::{PRD_EXAMPLE, plotted};
    use crate::palette::Theme;

    fn prd(title: Option<&str>) -> (Document, String) {
        let (catalogue, order) = plotted(PRD_EXAMPLE, None);
        let blocks = Blocks::new(&catalogue, &order, title, None, fonts());
        (arrange(Arrangement::Wide, &blocks), blocks.radar.name())
    }

    /// FR-57, D-88: the height follows the width, unrounded; by default
    /// the document's own size, however wide, since nothing is clamped.
    #[test]
    fn the_intrinsic_size_follows_the_width() {
        let (doc, _) = prd(None);
        assert_eq!(intrinsic_size(&doc, None), (doc.width, doc.height));
        let (w, h) = intrinsic_size(&doc, Some(320));
        assert_eq!(w, 320.0);
        assert_eq!(h, 320.0 * doc.height / doc.width);
    }

    /// SC-25, D-75, D-88: the comment, then the title or Technology Radar,
    /// then the radar's name.
    #[test]
    fn the_file_is_named() {
        let palette = Palette::built_in(Theme::Light);
        for (title, want) in [
            (Some("Q3 & co"), "<title>Q3 &amp; co</title>"),
            (None, "<title>Technology Radar</title>"),
        ] {
            let (doc, desc) = prd(title);
            let export = Export {
                palette: &palette,
                fonts: fonts(),
                width: None,
                title,
                desc: &desc,
            };
            let svg = super::export(&doc, &export).expect("exports");
            assert!(svg.starts_with(&format!("<!-- {OUTPUT_VERSION} -->\n<svg ")));
            assert!(svg.contains(want), "{svg}");
            assert!(svg.contains("<desc>Technology radar: 3 sectors, "), "{svg}");
        }
    }
}
