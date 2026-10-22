//! Writing rendered output to disk (architecture §3).

pub mod html;
pub mod pdf;
pub mod png;
pub mod svg;
pub mod write;

pub use write::write_atomic;

use crate::svg::Document;

/// The generator string every output carries: `major.minor`, never the
/// patch, so a patch release does not change a byte of output (NFR-12,
/// D-52).
pub const OUTPUT_VERSION: &str = concat!(
    "radargen ",
    env!("CARGO_PKG_VERSION_MAJOR"),
    ".",
    env!("CARGO_PKG_VERSION_MINOR")
);

/// D-75: what names an untitled output: the page's tab and hidden heading,
/// the exported SVG's `<title>`.
pub const UNTITLED: &str = "Technology Radar";

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::arrange::{Arrangement, Blocks, arrange};
    use crate::layout::block::fonts;
    use crate::layout::test_support::{PRD_EXAMPLE, plotted};

    fn prd() -> Document {
        let (catalogue, order) = plotted(PRD_EXAMPLE, None);
        let blocks = Blocks::new(&catalogue, &order, None, None, fonts());
        arrange(Arrangement::Wide, &blocks)
    }

    /// FR-57, D-88: the height follows the width, unrounded; by default
    /// the document's own size, however wide, since nothing is clamped.
    #[test]
    fn the_intrinsic_size_follows_the_width() {
        let doc = prd();
        assert_eq!(intrinsic_size(&doc, None), (doc.width, doc.height));
        let (w, h) = intrinsic_size(&doc, Some(320));
        assert_eq!(w, 320.0);
        assert_eq!(h, 320.0 * doc.height / doc.width);
    }
}
