//! SC-30 (FR-110): the PDF, drawn by hayro — an independent PDF
//! rasteriser, so the check is not our own reading of the format —
//! draws what our PNG draws, to within antialiasing; and a PDF damaged
//! the ways a writer bug would damage it does not.
//!
//! hayro (`vello_cpu`) and tiny-skia antialias differently, so the check
//! is the largest single channel difference against a measured ceiling,
//! not a count of differing pixels; the damage tests are what show the
//! ceiling still separates a faithful PDF from a broken one.

use hayro::hayro_interpret::InterpreterSettings;
use hayro::hayro_interpret::util::TransformExt as _;
use hayro::hayro_syntax::Pdf;
use hayro::kurbo::Affine;
use hayro::vello_cpu::color::palette::css::TRANSPARENT;
use hayro::vello_cpu::{Pixmap as Drawn, RasterizerSettings, RenderContext, Resources, TargetInit};
use hayro::{RenderCache, RenderSettings, render_into};
use lopdf::Object;
use lopdf::content::{Content, Operation};
use radargen::emit::intrinsic_size;
use radargen::emit::pdf::{Export, export};
use radargen::emit::png::{PixelSize, pixel_size, rasterise};
use radargen::layout::arrange::{Arrangement, Blocks, arrange};
use radargen::order::resolve;
use radargen::palette::{Palette, Theme};
use radargen::svg::Document;
use radargen::text::Fonts;
use resvg::tiny_skia::Pixmap;

#[path = "common/catalogue.rs"]
mod catalogue;

/// The largest channel difference a faithful PDF shows. hayro
/// (`vello_cpu`) and tiny-skia antialias edges differently, so 1–2.6% of
/// subpixels differ in every faithful case (6% at `--width 320`) and a
/// pixel count cannot tell faithful from broken. The largest single
/// difference can: faithful cases peak at 77 (`--width 320`, dark; 45–74
/// at natural size), every damage test at 214 or more. 77 plus 16 of
/// room for another platform's antialiasing. Measured 2026-10-22, macOS
/// arm64.
const MAX_DELTA: u8 = 93;

const LONG_TITLE: &str = "Engineering Technology Radar for the Platform and Delivery Groups";

/// `svg_matches_png.rs`'s five cases, a right-to-left name, and names
/// Inter lacks (Review Focus 2), each with its title.
const CASES: [(&str, &str, Option<&str>); 8] = [
    (
        "three-sector",
        include_str!("fixtures/three-sector.csv"),
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
    (
        "wrapped title",
        include_str!("fixtures/three-sector.csv"),
        Some(LONG_TITLE),
    ),
    (
        "text-spacing-rtl",
        include_str!("fixtures/text-spacing-rtl.csv"),
        None,
    ),
    (
        "not in Inter",
        "name,sector,ring,description\n東京,Tools,Adopt,d\n😀,Tools,Trial,d\nKafka,Tools,Assess,d\n",
        None,
    ),
    (
        "accented",
        "name,sector,ring,description\nCafé Zürich,Tools,Adopt,d\nÅngström,Tools,Trial,d\nNaïve façade,Tools,Assess,d\n",
        Some("Ærø — Q3"),
    ),
];

/// A case's wide document and the radar's name.
fn wide(csv: &str, title: Option<&str>, fonts: &Fonts) -> (Document, String) {
    let catalogue = catalogue::catalogue(csv, None);
    let plotted = resolve(&catalogue);
    let blocks = Blocks::new(&catalogue, &plotted, title, None, fonts);
    (arrange(Arrangement::Wide, &blocks), blocks.radar.name())
}

fn pdf_of(
    doc: &Document,
    desc: &str,
    palette: &Palette,
    fonts: &Fonts,
    title: Option<&str>,
    width: Option<u32>,
) -> Vec<u8> {
    export(
        doc,
        &Export {
            palette,
            fonts,
            width,
            title,
            desc,
            producer: "t",
        },
    )
    .expect("exports")
}

/// PDF points per CSS pixel, as the export sizes its drawing.
const POINTS_PER_PX: f64 = 0.75;

/// `pdf` drawn by hayro at `size`, premultiplied RGBA like tiny-skia's.
/// The scale is the drawing's, `drawing` px (FR-57's size) at 0.75 pt
/// each, not the page box's: the page is rounded up to whole points, and
/// the fraction past the drawing's right and bottom edges falls off the
/// pixmap. x and y are scaled separately, as the PNG's transform is,
/// since the PNG's height is rounded to whole pixels.
fn drawn(pdf: &[u8], size: PixelSize, drawing: (f64, f64)) -> Vec<u8> {
    let pdf = Pdf::new(pdf.to_vec()).expect("hayro parses the PDF");
    let page = pdf.pages().first().expect("one page");
    let w = u16::try_from(size.width()).expect("a test page fits u16");
    let h = u16::try_from(size.height()).expect("a test page fits u16");
    let fit = Affine::scale_non_uniform(
        f64::from(w) / (drawing.0 * POINTS_PER_PX),
        f64::from(h) / (drawing.1 * POINTS_PER_PX),
    );
    let cache = RenderCache::new();
    let mut ctx = RenderContext::new(w, h);
    render_into(
        page,
        &cache,
        &InterpreterSettings::default(),
        &RenderSettings::default(),
        &mut ctx,
        fit * page.initial_transform(true).to_kurbo(),
    );
    ctx.flush();
    let mut out = Drawn::new(w, h);
    ctx.render_with(
        &mut out,
        &mut Resources::default(),
        RasterizerSettings {
            target_init: TargetInit::Clear(TRANSPARENT),
            ..Default::default()
        },
    );
    out.data_as_u8_slice().to_vec()
}

/// Differing subpixels and the total, reported so a regression shows;
/// and the worst channel delta, which the check gates on.
struct Difference {
    differ: usize,
    worst: u8,
    total: usize,
}

impl Difference {
    const fn within(&self) -> bool {
        self.worst <= MAX_DELTA
    }
}

/// `pdf` drawn by hayro against `doc`'s PNG in `png_palette` at `width`.
fn against_png(
    doc: &Document,
    pdf: &[u8],
    png_palette: &Palette,
    width: Option<u32>,
) -> Difference {
    let size = pixel_size(doc, width).expect("fits");
    let png =
        Pixmap::decode_png(&rasterise(doc, png_palette, size).expect("renders")).expect("a PNG");
    let drawn = drawn(pdf, size, intrinsic_size(doc, width));
    let pairs = || png.data().iter().zip(&drawn);
    Difference {
        differ: pairs().filter(|(a, b)| a != b).count(),
        worst: pairs().map(|(a, b)| a.abs_diff(*b)).max().unwrap_or(0),
        total: png.data().len(),
    }
}

/// SC-30: every case, both themes (Review Focus 2, 4 and 5).
#[test]
fn the_pdf_draws_what_the_png_draws() {
    let fonts = Fonts::embedded().expect("loads");
    for (name, csv, title) in CASES {
        let (doc, desc) = wide(csv, title, &fonts);
        for theme in [Theme::Light, Theme::Dark] {
            let palette = Palette::built_in(theme);
            let d = against_png(
                &doc,
                &pdf_of(&doc, &desc, &palette, &fonts, title, None),
                &palette,
                None,
            );
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

/// Review Focus 3: at a set width the drawing scales with the page and
/// meets its edges, in the dark theme, where a sliver would show.
#[test]
fn the_pdf_at_a_set_width_draws_what_the_png_draws() {
    let fonts = Fonts::embedded().expect("loads");
    let (_, csv, title) = CASES[0];
    let (doc, desc) = wide(csv, title, &fonts);
    let palette = Palette::built_in(Theme::Dark);
    for width in [320, 2400] {
        let d = against_png(
            &doc,
            &pdf_of(&doc, &desc, &palette, &fonts, title, Some(width)),
            &palette,
            Some(width),
        );
        eprintln!(
            "--width {width}: {} ppm, worst {}",
            d.differ * 1_000_000 / d.total,
            d.worst
        );
        assert!(d.within(), "--width {width}");
    }
}

/// `pdf` with its page's content operators edited, re-saved by lopdf.
fn damaged(pdf: &[u8], edit: impl FnOnce(&mut Vec<Operation>)) -> Vec<u8> {
    let mut doc = lopdf::Document::load_mem(pdf).expect("parses");
    let page = *doc.get_pages().values().next().expect("a page");
    let mut content = Content::decode(&doc.get_page_content(page)).expect("decodes");
    edit(&mut content.operations);
    doc.change_page_content(page, content.encode().expect("encodes"))
        .expect("replaces");
    let mut out = Vec::new();
    doc.save_to(&mut out).expect("saves");
    out
}

/// The case the damage tests use: three sectors, light.
fn good() -> (Document, Vec<u8>, Palette) {
    let fonts = Fonts::embedded().expect("loads");
    let (_, csv, title) = CASES[0];
    let (doc, desc) = wide(csv, title, &fonts);
    let palette = Palette::built_in(Theme::Light);
    let pdf = pdf_of(&doc, &desc, &palette, &fonts, title, None);
    (doc, pdf, palette)
}

/// The control: lopdf's re-encoding alone changes nothing hayro draws,
/// so each failure below is the damage's.
#[test]
fn a_round_trip_through_lopdf_still_passes() {
    let (doc, pdf, palette) = good();
    assert!(against_png(&doc, &damaged(&pdf, |_| {}), &palette, None).within());
}

/// One glyph dropped from the first TJ fails.
#[test]
fn a_dropped_glyph_fails() {
    let (doc, pdf, palette) = good();
    let broken = damaged(&pdf, |ops| {
        let tj = ops.iter_mut().find(|op| op.operator == "TJ").expect("a TJ");
        let Some(Object::Array(items)) = tj.operands.first_mut() else {
            panic!("a TJ array")
        };
        let Some(Object::String(bytes, _)) =
            items.iter_mut().find(|o| matches!(o, Object::String(..)))
        else {
            panic!("a glyph string")
        };
        bytes.drain(..2);
    });
    assert!(
        !against_png(&doc, &broken, &palette, None).within(),
        "a dropped glyph must fail"
    );
}

/// Every TJ adjustment's sign flipped (kerning and letter-spacing the
/// wrong way) fails.
#[test]
fn flipped_adjustments_fail() {
    let (doc, pdf, palette) = good();
    let broken = damaged(&pdf, |ops| {
        for op in ops.iter_mut().filter(|op| op.operator == "TJ") {
            if let Some(Object::Array(items)) = op.operands.first_mut() {
                for item in items {
                    match item {
                        Object::Real(v) => *v = -*v,
                        Object::Integer(v) => *v = -*v,
                        _ => {}
                    }
                }
            }
        }
    });
    assert!(
        !against_png(&doc, &broken, &palette, None).within(),
        "reversed kerning must fail"
    );
}

/// One fill colour changed (the first after the ground's) fails.
#[test]
fn a_wrong_colour_fails() {
    let (doc, pdf, palette) = good();
    let broken = damaged(&pdf, |ops| {
        let rg = ops
            .iter_mut()
            .filter(|op| op.operator == "rg")
            .nth(1)
            .expect("a second fill");
        let red = rg
            .operands
            .first()
            .and_then(|o| o.as_float().ok())
            .expect("a channel");
        if let Some(first) = rg.operands.first_mut() {
            *first = Object::Real(1.0 - red);
        }
    });
    assert!(
        !against_png(&doc, &broken, &palette, None).within(),
        "a wrong colour must fail"
    );
}

/// The light PDF against the dark PNG fails: the comparison sees the
/// ground.
#[test]
fn the_wrong_theme_fails() {
    let (doc, pdf, _) = good();
    assert!(!against_png(&doc, &pdf, &Palette::built_in(Theme::Dark), None).within());
}
