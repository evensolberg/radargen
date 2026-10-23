//! SC-7 and SC-15 for the exported SVG (D-88): drawn by resvg with no
//! font at all, it draws what our PNG draws, to within antialiasing.
//!
//! Two criteria (D-88): few subpixels differ, and none by more than
//! antialiasing does. A planted defect (a dropped glyph) differs by far
//! more than 64 levels; the antialiasing residue never does.

use std::sync::Arc;

use radargen::emit::png::{pixel_size, rasterise, transform, usvg_options};
use radargen::emit::svg::{Export, export};
use radargen::palette::{Palette, Theme};
use radargen::svg::Document;
use radargen::text::Fonts;
use resvg::tiny_skia::Pixmap;
use resvg::usvg;

#[path = "common/catalogue.rs"]
mod catalogue;
#[path = "common/difference.rs"]
mod difference;
#[path = "common/oracle.rs"]
mod oracle;

use difference::Difference;
use oracle::{CASES, wide};

/// 1.5× the worst case's residue, measured 2026-10-16 on macOS arm64:
/// 82 ppm (`text-spacing`); the others 19 to 51.
const BUDGET_PPM: usize = 123;
/// The spike's antialiasing ceiling (`docs/spikes/svg-font.md`). Measured
/// here at most 56 (2026-10-16); every planted defect tried while planning
/// (a dropped or shifted glyph, a dropped tail) differed by more than 64,
/// so the ceiling separates the two with room for another platform's
/// antialiasing.
const MAX_DELTA: u8 = 64;

/// Our options with an empty font database: any text the SVG still
/// carried would draw as nothing.
fn fontless() -> usvg::Options<'static> {
    usvg::Options {
        fontdb: Arc::new(usvg::fontdb::Database::new()),
        ..usvg_options()
    }
}

/// SC-7's two criteria (D-88): few subpixels differ, and none by more
/// than antialiasing does.
const fn within(d: &Difference) -> bool {
    d.differ * 1_000_000 <= BUDGET_PPM * d.total && d.worst <= MAX_DELTA
}

/// `svg` drawn fontless at the PNG's size and scale, against the PNG.
fn against_png(doc: &Document, svg: &str, palette: &Palette) -> Difference {
    let size = pixel_size(doc, None).expect("fits");
    let png = Pixmap::decode_png(&rasterise(doc, palette, size).expect("renders")).expect("a PNG");
    let tree = usvg::Tree::from_str(svg, &fontless()).expect("usvg parses the export");
    let mut drawn = Pixmap::new(size.width(), size.height()).expect("a pixmap");
    resvg::render(&tree, transform(doc, size), &mut drawn.as_mut());
    Difference::of(png.data(), drawn.data())
}

fn svg_of(
    doc: &Document,
    desc: &str,
    palette: &Palette,
    fonts: &Fonts,
    title: Option<&str>,
) -> String {
    export(
        doc,
        &Export {
            palette,
            fonts,
            width: None,
            title,
            desc,
        },
    )
    .expect("exports")
}

/// SC-7 for SVG: every case, both themes.
#[test]
fn the_svg_draws_what_the_png_draws() {
    let fonts = Fonts::embedded().expect("loads");
    for (name, csv, title) in CASES {
        let (doc, desc) = wide(csv, title, &fonts);
        for theme in [Theme::Light, Theme::Dark] {
            let palette = Palette::built_in(theme);
            let d = against_png(
                &doc,
                &svg_of(&doc, &desc, &palette, &fonts, title),
                &palette,
            );
            eprintln!(
                "{name} {theme:?}: {} of {} subpixels ({} ppm), worst {}",
                d.differ,
                d.total,
                d.ppm(),
                d.worst
            );
            assert!(within(&d), "{name} {theme:?}");
        }
    }
}

/// The comparison is not vacuous: one glyph dropped fails it.
#[test]
fn the_comparison_catches_a_dropped_glyph() {
    let fonts = Fonts::embedded().expect("loads");
    let (_, csv, title) = CASES[0];
    let (doc, desc) = wide(csv, title, &fonts);
    let palette = Palette::built_in(Theme::Light);
    let svg = svg_of(&doc, &desc, &palette, &fonts, title);
    let at = svg.find("<use ").expect("a glyph");
    let end = at + svg.get(at..).and_then(|s| s.find('\n')).expect("its line") + 1;
    let damaged = [
        svg.get(..at).expect("before"),
        svg.get(end..).expect("after"),
    ]
    .concat();
    assert!(
        !within(&against_png(&doc, &damaged, &palette)),
        "a dropped glyph must fail"
    );
}

/// NFR-7: a name Inter lacks draws the shaper's `.notdef`, as resvg does.
#[test]
fn a_name_inter_lacks_draws_as_the_png_does() {
    let fonts = Fonts::embedded().expect("loads");
    let csv = "name,sector,ring,description\n東京,Tools,Adopt,d\n😀,Tools,Trial,d\nKafka,Tools,Assess,d\n";
    let (doc, desc) = wide(csv, None, &fonts);
    let palette = Palette::built_in(Theme::Light);
    assert!(within(&against_png(
        &doc,
        &svg_of(&doc, &desc, &palette, &fonts, None),
        &palette
    )));
}
