//! SC-30 (FR-110): the PDF, drawn by hayro — an independent PDF
//! rasteriser, so the check is not our own reading of the format —
//! draws what our PNG draws, to within antialiasing; and a PDF damaged
//! the ways a writer bug would damage it does not.
//!
//! hayro (`vello_cpu`) and tiny-skia antialias differently, so the check
//! is the largest single channel difference against a measured ceiling,
//! not a count of differing pixels; the damage tests are what show the
//! ceiling still separates a faithful PDF from a broken one.
//!
//! The ceiling is blind to a flat colour a little off and to a halo a
//! little thin or missing (each damage below stays under it), so two
//! more checks look where it cannot: inside every filled shape the PDF
//! is the PNG's colour, and every halo is one stroked outline group in
//! its colour and width.

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
use radargen::emit::pdf::POINTS_PER_PX;
use radargen::emit::png::{PixelSize, pixel_size, rasterise};
use radargen::palette::{Palette, Theme};
use radargen::svg::Document;
use resvg::tiny_skia::Pixmap;

#[path = "common/catalogue.rs"]
mod catalogue;
#[path = "common/difference.rs"]
mod difference;
#[path = "pdf_matches_png/fills.rs"]
mod fills;
#[path = "pdf_matches_png/halos.rs"]
mod halos;
#[path = "common/oracle.rs"]
mod oracle;
#[path = "common/pdf_export.rs"]
mod pdf_export;
#[path = "common/pdf_page.rs"]
mod pdf_page;

use difference::Difference;
use oracle::{CASES, wide};
use pdf_export::{fonts, pdf};
use pdf_page::first_page;

/// The largest channel difference a faithful PDF shows. hayro
/// (`vello_cpu`) and tiny-skia antialias edges differently, so 1–2.6% of
/// subpixels differ in every faithful case (6% at `--width 320`) and a
/// pixel count cannot tell faithful from broken. The largest single
/// difference can: faithful cases peak at 77 (`--width 320`, dark; 45–74
/// at natural size), every glyph and colour damage test at 214 or more
/// (the halo and panel damages stay under it; the checks below catch them). 77 plus 16 of
/// room for another platform's antialiasing. Measured 2026-10-22, macOS
/// arm64.
const MAX_DELTA: u8 = 93;

/// The PDF's own cases beside the shared ones: a right-to-left name,
/// names Inter lacks (Review Focus 2), accented names, and marks the
/// shaper raises (so the raised-glyph text matrix's sign is checked), each
/// with its title.
const PDF_CASES: [(&str, &str, Option<&str>); 4] = [
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
    (
        "raised marks",
        "name,sector,ring,description\nA\u{300}\u{301}B\u{323}\u{302}x,Tools,Adopt,d\nKafka,Tools,Trial,d\n",
        Some("Ra\u{301}\u{300}dar"),
    ),
];

/// Every case the PDF is checked on: the shared ones, then its own.
fn cases() -> impl Iterator<Item = (&'static str, &'static str, Option<&'static str>)> {
    CASES.into_iter().chain(PDF_CASES)
}

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

/// `doc`'s PNG in `png_palette` at `width`, and hayro's drawing of `pdf`
/// at the PNG's size.
fn rendered(
    doc: &Document,
    pdf: &[u8],
    png_palette: &Palette,
    width: Option<u32>,
) -> (PixelSize, Pixmap, Vec<u8>) {
    let size = pixel_size(doc, width).expect("fits");
    let png =
        Pixmap::decode_png(&rasterise(doc, png_palette, size).expect("renders")).expect("a PNG");
    let drawn = drawn(pdf, size, intrinsic_size(doc, width));
    (size, png, drawn)
}

/// SC-30's criterion: no channel differs by more than antialiasing does.
/// The differing subpixels are only reported, so a regression shows.
const fn within(d: &Difference) -> bool {
    d.worst <= MAX_DELTA
}

/// `pdf` drawn by hayro against `doc`'s PNG in `png_palette` at `width`.
fn against_png(
    doc: &Document,
    pdf: &[u8],
    png_palette: &Palette,
    width: Option<u32>,
) -> Difference {
    let (_, png, drawn) = rendered(doc, pdf, png_palette, width);
    Difference::of(png.data(), &drawn)
}

/// SC-30: every case, both themes (Review Focus 2, 4 and 5).
#[test]
fn the_pdf_draws_what_the_png_draws() {
    for (name, csv, title) in cases() {
        let (doc, desc) = wide(csv, title, fonts());
        for theme in [Theme::Light, Theme::Dark] {
            let palette = Palette::built_in(theme);
            let d = against_png(
                &doc,
                &pdf(&doc, &desc, &palette, title, None),
                &palette,
                None,
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

/// Review Focus 3: at a set width the drawing scales with the page and
/// meets its edges, in the dark theme, where a sliver would show.
#[test]
fn the_pdf_at_a_set_width_draws_what_the_png_draws() {
    let (_, csv, title) = CASES[0];
    let (doc, desc) = wide(csv, title, fonts());
    let palette = Palette::built_in(Theme::Dark);
    for width in [320, 2400] {
        let d = against_png(
            &doc,
            &pdf(&doc, &desc, &palette, title, Some(width)),
            &palette,
            Some(width),
        );
        eprintln!("--width {width}: {} ppm, worst {}", d.ppm(), d.worst);
        assert!(within(&d), "--width {width}");
    }
}

/// `pdf` with its page's content operators edited, re-saved by lopdf.
fn damaged(pdf: &[u8], edit: impl FnOnce(&mut Vec<Operation>)) -> Vec<u8> {
    let mut doc = lopdf::Document::load_mem(pdf).expect("parses");
    let page = first_page(&doc);
    let mut content = Content::decode(&doc.get_page_content(page)).expect("decodes");
    edit(&mut content.operations);
    doc.change_page_content(page, content.encode().expect("encodes"))
        .expect("replaces");
    let mut out = Vec::new();
    doc.save_to(&mut out).expect("saves");
    out
}

/// The case the damage tests use: three sectors in `theme`, the
/// document, its PDF and the palette.
fn case(theme: Theme) -> (Document, Vec<u8>, Palette) {
    let (_, csv, title) = CASES[0];
    let (doc, desc) = wide(csv, title, fonts());
    let palette = Palette::built_in(theme);
    let pdf = pdf(&doc, &desc, &palette, title, None);
    (doc, pdf, palette)
}

/// The control: lopdf's re-encoding alone changes nothing hayro draws,
/// so each failure below is the damage's.
#[test]
fn a_round_trip_through_lopdf_still_passes() {
    let (doc, pdf, palette) = case(Theme::Light);
    assert!(within(&against_png(
        &doc,
        &damaged(&pdf, |_| {}),
        &palette,
        None
    )));
}

/// One glyph dropped from the first TJ fails.
#[test]
fn a_dropped_glyph_fails() {
    let (doc, pdf, palette) = case(Theme::Light);
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
        !within(&against_png(&doc, &broken, &palette, None)),
        "a dropped glyph must fail"
    );
}

/// Every TJ adjustment's sign flipped (kerning and letter-spacing the
/// wrong way) fails.
#[test]
fn flipped_adjustments_fail() {
    let (doc, pdf, palette) = case(Theme::Light);
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
        !within(&against_png(&doc, &broken, &palette, None)),
        "reversed kerning must fail"
    );
}

/// One fill colour changed (the first after the ground's) fails.
#[test]
fn a_wrong_colour_fails() {
    let (doc, pdf, palette) = case(Theme::Light);
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
        !within(&against_png(&doc, &broken, &palette, None)),
        "a wrong colour must fail"
    );
}

/// The light PDF against the dark PNG fails: the comparison sees the
/// ground.
#[test]
fn the_wrong_theme_fails() {
    let (doc, pdf, _) = case(Theme::Light);
    assert!(!within(&against_png(
        &doc,
        &pdf,
        &Palette::built_in(Theme::Dark),
        None
    )));
}
