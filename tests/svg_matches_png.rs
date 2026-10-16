//! SC-7 and SC-15 for the exported SVG (D-88): drawn by resvg with no
//! font at all, it draws what our PNG draws, to within antialiasing.
//!
//! Two criteria (Ruling 10): few subpixels differ, and none by more than
//! antialiasing does. A planted defect (a dropped glyph) differs by far
//! more than 64 levels; the antialiasing residue never does.

use std::sync::Arc;

use radargen::emit::png::{pixel_size, rasterise, usvg_options};
use radargen::emit::svg::{Export, export};
use radargen::layout::arrange::{Arrangement, Blocks, arrange};
use radargen::order::resolve;
use radargen::palette::{Palette, Theme};
use radargen::svg::Document;
use radargen::text::Fonts;
use resvg::tiny_skia::{Pixmap, Transform};
use resvg::usvg;

#[path = "common/catalogue.rs"]
mod catalogue;

/// 1.5× the worst case's residue, measured 2026-10-16 on macOS arm64:
/// 82 ppm (`text-spacing`); the others 19 to 51.
const BUDGET_PPM: usize = 123;
/// The spike's antialiasing ceiling (`docs/spikes/svg-font.md`). Measured
/// here at most 56 (2026-10-16); every planted defect the plan's QA review
/// tried differed by more than 64, so the ceiling separates the two with
/// room for another platform's antialiasing.
const MAX_DELTA: u8 = 64;

const LONG_TITLE: &str = "Engineering Technology Radar for the Platform and Delivery Groups";

/// The documents compared, with the title each is drawn with.
const CASES: [(&str, &str, Option<&str>); 5] = [
    (
        "three-quadrant",
        include_str!("fixtures/three-quadrant.csv"),
        Some("Technology Radar"),
    ),
    (
        "long-legend",
        include_str!("fixtures/long-legend.csv"),
        None,
    ),
    (
        "text-spacing",
        include_str!("fixtures/text-spacing.csv"),
        Some("Text spacing"),
    ),
    ("grown", include_str!("fixtures/grown.csv"), None),
    // A title long enough to wrap onto two lines.
    (
        "wrapped title",
        include_str!("fixtures/three-quadrant.csv"),
        Some(LONG_TITLE),
    ),
];

/// Our options with an empty font database: any text the SVG still
/// carried would draw as nothing.
fn fontless() -> usvg::Options<'static> {
    usvg::Options {
        fontdb: Arc::new(usvg::fontdb::Database::new()),
        ..usvg_options()
    }
}

fn wide(csv: &str, title: Option<&str>, fonts: &Fonts) -> Document {
    let catalogue = catalogue::catalogue(csv, None);
    let plotted = resolve(&catalogue);
    arrange(
        Arrangement::Wide,
        &Blocks::new(&catalogue, &plotted, title, None, fonts),
    )
}

/// Differing subpixels, the worst channel delta, and the total.
struct Difference {
    differ: usize,
    worst: u8,
    total: usize,
}

impl Difference {
    const fn within(&self) -> bool {
        self.differ * 1_000_000 <= BUDGET_PPM * self.total && self.worst <= MAX_DELTA
    }
}

/// `svg` drawn fontless at the PNG's size, against the PNG.
#[expect(
    clippy::cast_possible_truncation,
    clippy::as_conversions,
    reason = "tiny-skia transforms are f32, as `rasterise` narrows them"
)]
fn against_png(doc: &Document, svg: &str, palette: &Palette) -> Difference {
    let size = pixel_size(doc, None).expect("fits");
    let png = Pixmap::decode_png(&rasterise(doc, palette, size).expect("renders")).expect("a PNG");
    let tree = usvg::Tree::from_str(svg, &fontless()).expect("usvg parses the export");
    let mut drawn = Pixmap::new(size.width(), size.height()).expect("a pixmap");
    // The scale `rasterise` uses, so the two differ only in the text.
    resvg::render(
        &tree,
        Transform::from_scale(
            (f64::from(size.width()) / doc.width) as f32,
            (f64::from(size.height()) / doc.height) as f32,
        ),
        &mut drawn.as_mut(),
    );
    let pairs = || png.data().iter().zip(drawn.data());
    Difference {
        differ: pairs().filter(|(a, b)| a != b).count(),
        worst: pairs().map(|(a, b)| a.abs_diff(*b)).max().unwrap_or(0),
        total: png.data().len(),
    }
}

fn svg_of(doc: &Document, palette: &Palette, fonts: &Fonts, title: Option<&str>) -> String {
    export(
        doc,
        &Export {
            palette,
            fonts,
            width: None,
            title,
        },
    )
    .expect("exports")
}

/// SC-7 for SVG: every case, both themes.
#[test]
fn the_svg_draws_what_the_png_draws() {
    let fonts = Fonts::embedded().expect("loads");
    for (name, csv, title) in CASES {
        for theme in [Theme::Light, Theme::Dark] {
            let doc = wide(csv, title, &fonts);
            let palette = Palette::built_in(theme);
            let d = against_png(&doc, &svg_of(&doc, &palette, &fonts, title), &palette);
            eprintln!(
                "{name} {theme:?}: {} of {} subpixels ({} ppm), worst {}",
                d.differ,
                d.total,
                d.differ * 1_000_000 / d.total,
                d.worst
            );
            assert!(d.within(), "{name} {theme:?}");
        }
    }
}

/// The comparison is not vacuous: one glyph dropped fails it.
#[test]
fn the_comparison_catches_a_dropped_glyph() {
    let fonts = Fonts::embedded().expect("loads");
    let (_, csv, title) = CASES[0];
    let doc = wide(csv, title, &fonts);
    let palette = Palette::built_in(Theme::Light);
    let svg = svg_of(&doc, &palette, &fonts, title);
    let at = svg.find("<use ").expect("a glyph");
    let end = at + svg.get(at..).and_then(|s| s.find('\n')).expect("its line") + 1;
    let damaged = [
        svg.get(..at).expect("before"),
        svg.get(end..).expect("after"),
    ]
    .concat();
    assert!(
        !against_png(&doc, &damaged, &palette).within(),
        "a dropped glyph must fail"
    );
}

/// NFR-7: a name Inter lacks draws the shaper's `.notdef`, as resvg does.
#[test]
fn a_name_inter_lacks_draws_as_the_png_does() {
    let fonts = Fonts::embedded().expect("loads");
    let csv = "name,quadrant,ring,description\n東京,Tools,Adopt,d\n😀,Tools,Trial,d\nKafka,Tools,Assess,d\n";
    let doc = wide(csv, None, &fonts);
    let palette = Palette::built_in(Theme::Light);
    assert!(against_png(&doc, &svg_of(&doc, &palette, &fonts, None), &palette).within());
}

/// The table's invariant (Task 2): every glyph the writer places was
/// collected, over every case; an `expect` in the writer would fire.
#[test]
fn every_drawn_glyph_is_in_the_table() {
    let fonts = Fonts::embedded().expect("loads");
    let palette = Palette::built_in(Theme::Dark);
    for (_, csv, title) in CASES {
        let doc = wide(csv, title, &fonts);
        let svg = svg_of(&doc, &palette, &fonts, title);
        assert!(svg.contains("<use "), "drew glyphs");
    }
}
