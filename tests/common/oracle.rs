//! Shared by the render oracles (the SVG's and the PDF's against the PNG)
//! and the PDF's text check: the documents they all look at, each in its
//! wide arrangement.

use radargen::layout::arrange::{Arrangement, Blocks, arrange};
use radargen::order::resolve;
use radargen::svg::Document;
use radargen::text::Fonts;

use super::catalogue::catalogue;

/// A title long enough to wrap onto two lines.
pub const LONG_TITLE: &str = "Engineering Technology Radar for the Platform and Delivery Groups";

/// The documents every suite checks, with the title each is drawn with.
pub const CASES: [(&str, &str, Option<&str>); 5] = [
    (
        "three-sector",
        include_str!("../fixtures/three-sector.csv"),
        Some("Technology Radar"),
    ),
    (
        "long-legend",
        include_str!("../fixtures/long-legend.csv"),
        None,
    ),
    (
        "text-spacing",
        include_str!("../fixtures/text-spacing.csv"),
        Some("Text spacing"),
    ),
    ("grown", include_str!("../fixtures/grown.csv"), None),
    (
        "wrapped title",
        include_str!("../fixtures/three-sector.csv"),
        Some(LONG_TITLE),
    ),
];

/// A case's wide document, and the radar's name: the SVG's `<desc>`, the
/// PDF's `/Subject`.
pub fn wide(csv: &str, title: Option<&str>, fonts: &Fonts) -> (Document, String) {
    let catalogue = catalogue(csv, None);
    let plotted = resolve(&catalogue);
    let blocks = Blocks::new(&catalogue, &plotted, title, None, fonts);
    (arrange(Arrangement::Wide, &blocks), blocks.radar.name())
}
