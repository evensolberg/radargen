//! The exported PDF's goldens (FR-110, NFR-4), as text through
//! [`readable`], light and dark as the SVG's are.

use radargen::layout::arrange::{Arrangement, arrange};
use radargen::palette::{Palette, Theme};

use super::golden::golden;
use super::pdf_export::{PRODUCER, pdf};
use super::pdf_text::readable;
use super::prd;
use super::version::PLACEHOLDER;

/// The PRD example as a PDF in `theme`, its producer the placeholder
/// (`pdf_export`'s), so a minor bump changes no byte.
fn prd_pdf(theme: Theme) -> String {
    let built = prd();
    let doc = arrange(Arrangement::Wide, &built.blocks);
    readable(&pdf(
        &doc,
        &built.blocks.radar.name(),
        &Palette::built_in(theme),
        Some("Technology Radar"),
        None,
    ))
}

/// The PRD example as a light PDF. One test per golden, since a blessing
/// run stops at the first.
#[test]
fn the_producer_is_the_version_placeholder() {
    assert_eq!(PRODUCER, PLACEHOLDER);
}

#[test]
fn prd_pdf_matches_its_golden() {
    golden("pdf", "prd-light.pdf.txt", &prd_pdf(Theme::Light));
}

/// The same, dark (FR-56).
#[test]
fn prd_dark_pdf_matches_its_golden() {
    golden("pdf", "prd-dark.pdf.txt", &prd_pdf(Theme::Dark));
}
