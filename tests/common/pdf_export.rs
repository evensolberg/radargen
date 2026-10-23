//! Shared by the PDF suites: the vendored fonts, parsed once, and one
//! export through them.

use std::sync::LazyLock;

use radargen::emit::pdf::{Export, export};
use radargen::palette::Palette;
use radargen::svg::Document;
use radargen::text::Fonts;

/// The vendored fonts, parsed once for every test in the binary.
static FONTS: LazyLock<Fonts> =
    LazyLock::new(|| Fonts::embedded().expect("the vendored fonts parse"));

/// The producer every test PDF names: the goldens' placeholder, so a
/// golden's offsets do not move with the version. Only the `serialise`
/// binary includes `version.rs`, so `serialise/pdf.rs` asserts this equals
/// its `PLACEHOLDER`.
pub const PRODUCER: &str = "radargen {version}";

/// The vendored fonts.
pub fn fonts() -> &'static Fonts {
    &FONTS
}

/// `doc` as a PDF in `palette`, `--width` `width`, with the radar's name
/// `desc` and the title `title`.
pub fn pdf(
    doc: &Document,
    desc: &str,
    palette: &Palette,
    title: Option<&str>,
    width: Option<u32>,
) -> Vec<u8> {
    export(
        doc,
        width,
        &Export {
            palette,
            fonts: fonts(),
            title,
            desc,
            producer: PRODUCER,
        },
    )
    .expect("exports")
}
