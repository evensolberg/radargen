//! The exported PDF's goldens (FR-110, NFR-4), as text through
//! [`readable`], light and dark as the SVG's are.

use radargen::emit::pdf::{Export, export};
use radargen::layout::arrange::{Arrangement, arrange};
use radargen::palette::{Palette, Theme};
use radargen::text::Fonts;

use super::golden::golden;
use super::pdf_text::readable;
use super::prd;
use super::version::PLACEHOLDER;

/// The PRD example as a PDF in `theme`, its producer the placeholder, so
/// a minor bump changes no byte.
fn prd_pdf(theme: Theme) -> String {
    let built = prd();
    let fonts = Fonts::embedded().expect("fonts");
    let doc = arrange(Arrangement::Wide, &built.blocks);
    let palette = Palette::built_in(theme);
    let bytes = export(
        &doc,
        &Export {
            palette: &palette,
            fonts: &fonts,
            width: None,
            title: Some("Technology Radar"),
            desc: &built.blocks.radar.name(),
            producer: PLACEHOLDER,
        },
    )
    .expect("exports");
    readable(&bytes)
}

/// The PRD example as a light PDF. One test per golden, since a blessing
/// run stops at the first.
#[test]
fn prd_pdf_matches_its_golden() {
    golden("pdf", "prd-light.pdf.txt", &prd_pdf(Theme::Light));
}

/// The same, dark (FR-56).
#[test]
fn prd_dark_pdf_matches_its_golden() {
    golden("pdf", "prd-dark.pdf.txt", &prd_pdf(Theme::Dark));
}
