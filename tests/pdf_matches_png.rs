//! SC-30 (FR-110): the PDF, drawn by hayro — an independent PDF
//! rasteriser, so the check is not our own reading of the format —
//! draws what our PNG draws, to within antialiasing; and a PDF damaged
//! the ways a writer bug would damage it does not.
//!
//! hayro (`vello_cpu`) and tiny-skia antialias differently, so both are
//! drawn at twice the size and averaged back down, which halves the
//! antialiasing noise while a shift or a colour change survives it. The
//! check is then the largest single channel difference against a
//! measured ceiling, and the subpixels past the noise against a measured
//! budget; the damage tests are what show the two still separate a
//! faithful PDF from a broken one.
//!
//! They are blind to a flat colour a few levels off and to a halo a
//! little thin or missing, so two more checks look where they cannot:
//! inside every filled shape the PDF is the PNG's colour, and every halo
//! is one stroked outline group in its colour and width.

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
use radargen::emit::png::{PixelSize, pixel_size, rasterise};
use radargen::palette::{Palette, Theme};
use radargen::style::POINTS_PER_PX;
use radargen::svg::Document;
use resvg::tiny_skia::Pixmap;

#[path = "common/catalogue.rs"]
mod catalogue;
#[path = "pdf_matches_png/fills.rs"]
mod fills;
#[path = "pdf_matches_png/halos.rs"]
mod halos;
#[path = "common/long_legend.rs"]
mod long_legend;
#[path = "common/oracle.rs"]
mod oracle;
#[path = "common/paper.rs"]
mod paper;
#[path = "pdf_matches_png/paper.rs"]
mod paper_pages;
#[path = "common/pdf_export.rs"]
mod pdf_export;
#[path = "common/pdf_page.rs"]
mod pdf_page;
#[path = "pdf_matches_png/table.rs"]
mod table_pages;

use oracle::{CASES, wide};
use pdf_export::{fonts, pdf};
use pdf_page::first_page;

/// The largest channel difference a faithful PDF shows, drawn at twice
/// the size and averaged down. Faithful cases peak at 36 (77 at 1x,
/// where a shift of a third of a pixel stayed under the ceiling); every
/// text shifted 0.1 px reaches 51, and 0.2 px 76. 36 plus 8 of room for
/// another platform's antialiasing. Measured 2026-10-22, macOS arm64.
const MAX_DELTA: u8 = 44;

/// A channel difference antialiasing alone reaches often: past it, a
/// subpixel counts against [`MAX_PPM`]. Faithful cases have none past 32
/// and up to 500 ppm past 16; a stroke colour 20 levels off has none
/// past 44, so a lower threshold is what lets the count see it.
const NOISE: u8 = 16;

/// How many subpixels per million a faithful PDF shows past [`NOISE`]:
/// faithful cases reach 500 (light; dark stays under 180), every stroke
/// colour 20 levels off (0.08) reaches 1,680, and every text colour
/// 1,100, though neither passes [`MAX_DELTA`]. A colour 10 levels off
/// stays under both. Measured 2026-10-22, macOS arm64.
const MAX_PPM: usize = 800;

/// [`MAX_PPM`] for a paper page holding only legend columns (FR-111,
/// SC-31): solid text has more antialiased edges than the wide
/// arrangement, whose radar dilutes the count. A faithful Letter page
/// reaches 848; the same page with its text 0.2 px off reaches 22,024,
/// and with its text colour 20 levels off 4,559. The maintainer's ruling,
/// 2026-10-22: legend pages get their own budget; the wide arrangement
/// and page 1 keep [`MAX_PPM`]. Measured 2026-10-22, macOS arm64.
const LEGEND_PAGE_MAX_PPM: usize = 1_600;

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
    drawn_page(pdf, 0, size, drawing)
}

/// As [`drawn`], for the PDF's page `index`.
fn drawn_page(pdf: &[u8], index: usize, size: PixelSize, drawing: (f64, f64)) -> Vec<u8> {
    let pdf = Pdf::new(pdf.to_vec()).expect("hayro parses the PDF");
    let page = pdf.pages().get(index).expect("the page");
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

/// The supersampled comparison: the largest channel difference, and the
/// subpixels past [`NOISE`] in parts per million.
struct Compared {
    worst: u8,
    loud_ppm: usize,
}

/// SC-30's criterion: no channel differs by more than antialiasing does,
/// and few differ by more than its noise.
const fn within(c: &Compared) -> bool {
    c.worst <= MAX_DELTA && c.loud_ppm <= MAX_PPM
}

/// SC-31's criterion for a paper page of legend columns alone: as
/// [`within`], with [`LEGEND_PAGE_MAX_PPM`].
const fn within_legend_page(c: &Compared) -> bool {
    c.worst <= MAX_DELTA && c.loud_ppm <= LEGEND_PAGE_MAX_PPM
}

/// `pixels` (RGBA, `width` wide) with each 2×2 block averaged into one.
/// An odd last row is averaged with itself, so the bottom edge, where a
/// sliver would show, is still compared; the width is always even.
fn halved(pixels: &[u8], width: usize) -> Vec<u8> {
    let row = width * 4;
    let padded;
    let pixels = if (pixels.len() / row) % 2 == 1 {
        let last = pixels.rchunks_exact(row).next().expect("a row");
        padded = [pixels, last].concat();
        &padded
    } else {
        pixels
    };
    let (pixels, _) = pixels.as_chunks::<4>();
    pixels
        .chunks_exact(width * 2)
        .flat_map(|rows| {
            let (top, bottom) = rows.split_at(width);
            top.as_chunks::<2>().0.iter().zip(bottom.as_chunks::<2>().0)
        })
        .flat_map(|(top, bottom)| {
            let sums = top
                .iter()
                .chain(bottom)
                .fold([0_u32; 4], |mut sums, pixel| {
                    for (sum, channel) in sums.iter_mut().zip(pixel) {
                        *sum += u32::from(*channel);
                    }
                    sums
                });
            sums.map(|sum| u8::try_from((sum + 2) / 4).expect("an average of four bytes"))
        })
        .collect()
}

/// `pdf` drawn by hayro against `doc`'s PNG in `png_palette` at `width`,
/// both drawn at twice the size and averaged down.
fn against_png(doc: &Document, pdf: &[u8], png_palette: &Palette, width: Option<u32>) -> Compared {
    against_page(doc, pdf, 0, png_palette, width)
}

/// As [`against_png`], for the PDF's page `index`, `doc` being that
/// page's document.
fn against_page(
    doc: &Document,
    pdf: &[u8],
    index: usize,
    png_palette: &Palette,
    width: Option<u32>,
) -> Compared {
    let once = pixel_size(doc, width).expect("fits");
    let twice = pixel_size(doc, Some(once.width() * 2)).expect("fits");
    let png =
        Pixmap::decode_png(&rasterise(doc, png_palette, twice).expect("renders")).expect("a PNG");
    let drawn = drawn_page(pdf, index, twice, intrinsic_size(doc, width));
    let wide = usize::try_from(twice.width()).expect("a test page fits usize");
    let (png, drawn) = (halved(png.data(), wide), halved(&drawn, wide));
    let deltas = || png.iter().zip(&drawn).map(|(a, b)| a.abs_diff(*b));
    Compared {
        worst: deltas().max().unwrap_or(0),
        loud_ppm: deltas().filter(|&d| d > NOISE).count() * 1_000_000 / png.len(),
    }
}

/// SC-30: every case, both themes (Review Focus 2, 4 and 5).
#[test]
fn the_pdf_draws_what_the_png_draws() {
    for (name, csv, title) in cases() {
        let (doc, desc) = wide(csv, title, fonts());
        for theme in [Theme::Light, Theme::Dark] {
            let palette = Palette::built_in(theme);
            let c = against_png(
                &doc,
                &pdf(&doc, &desc, &palette, title, None),
                &palette,
                None,
            );
            eprintln!(
                "{name} {theme:?}: {} ppm past noise, worst {}",
                c.loud_ppm, c.worst
            );
            assert!(within(&c), "{name} {theme:?}");
        }
    }
}

/// Review Focus 3: at a set width the drawing scales with the page and
/// meets its edges, in the dark theme, where a sliver would show. At 320
/// only the ceiling applies: the drawing is a third of its size, so
/// nearly every pixel is an edge, and [`MAX_PPM`], measured at natural
/// size, does not hold (8,000 ppm past noise, worst 32).
#[test]
fn the_pdf_at_a_set_width_draws_what_the_png_draws() {
    let (_, csv, title) = CASES[0];
    let (doc, desc) = wide(csv, title, fonts());
    let palette = Palette::built_in(Theme::Dark);
    for width in [320, 2400] {
        let c = against_png(
            &doc,
            &pdf(&doc, &desc, &palette, title, Some(width)),
            &palette,
            Some(width),
        );
        eprintln!(
            "--width {width}: {} ppm past noise, worst {}",
            c.loud_ppm, c.worst
        );
        let passes = if width == 320 {
            c.worst <= MAX_DELTA
        } else {
            within(&c)
        };
        assert!(passes, "--width {width}");
    }
}

/// `pdf` with its page's content operators edited, re-saved by lopdf.
fn damaged(pdf: &[u8], edit: impl FnOnce(&mut Vec<Operation>)) -> Vec<u8> {
    let doc = lopdf::Document::load_mem(pdf).expect("parses");
    damaged_page(pdf, first_page(&doc), edit)
}

/// As [`damaged`], for the page `page`.
fn damaged_page(
    pdf: &[u8],
    page: lopdf::ObjectId,
    edit: impl FnOnce(&mut Vec<Operation>),
) -> Vec<u8> {
    let mut doc = lopdf::Document::load_mem(pdf).expect("parses");
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

/// Every text a fifth of a pixel to the right fails, in both themes: a
/// placement error the 1x ceiling let through.
#[test]
fn text_a_fifth_of_a_pixel_off_fails() {
    for theme in [Theme::Light, Theme::Dark] {
        let (doc, pdf, palette) = case(theme);
        let broken = damaged(&pdf, |ops| {
            for tm in ops.iter_mut().filter(|op| op.operator == "Tm") {
                let x = tm.operands.get_mut(4).expect("a Tm's x");
                *x = Object::Real(x.as_float().expect("a number") + 0.2);
            }
        });
        assert!(
            !within(&against_png(&doc, &broken, &palette, None)),
            "{theme:?}: shifted text must fail"
        );
    }
}

/// Every stroke colour 0.08 (20 levels) off fails, in both themes: no
/// filled shape shows it, and no single subpixel passes the ceiling.
#[test]
fn a_slightly_wrong_stroke_colour_fails() {
    for theme in [Theme::Light, Theme::Dark] {
        let (doc, pdf, palette) = case(theme);
        let broken = damaged(&pdf, |ops| {
            for stroke in ops.iter_mut().filter(|op| op.operator == "RG") {
                let red = stroke.operands.first_mut().expect("a channel");
                let v = red.as_float().expect("a number");
                *red = Object::Real(if v > 0.5 { v - 0.08 } else { v + 0.08 });
            }
        });
        assert!(
            !within(&against_png(&doc, &broken, &palette, None)),
            "{theme:?}: a slightly wrong stroke colour must fail"
        );
    }
}

/// Halving keeps an odd last row, averaged with itself, so the bottom
/// edge is compared.
#[test]
fn halving_keeps_an_odd_last_row() {
    let pixel = |v: u8| [v; 4];
    // Two pixels wide, three rows: 0 0 / 4 4 / 9 9.
    let rows: Vec<u8> = [0, 0, 4, 4, 9, 9].into_iter().flat_map(pixel).collect();
    assert_eq!(halved(&rows, 2), [pixel(2), pixel(9)].concat());
}
