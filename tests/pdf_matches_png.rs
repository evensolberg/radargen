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
use radargen::emit::pdf::{Export, export};
use radargen::emit::png::{PixelSize, pixel_size, rasterise};
use radargen::layout::arrange::{Arrangement, Blocks, arrange};
use radargen::order::resolve;
use radargen::palette::{Neutral, Palette, Role, Theme};
use radargen::svg::{Document, Node, Paint, Point, TextPaint};
use radargen::text::Fonts;
use resvg::tiny_skia::Pixmap;

#[path = "common/catalogue.rs"]
mod catalogue;

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

const LONG_TITLE: &str = "Engineering Technology Radar for the Platform and Delivery Groups";

/// `svg_matches_png.rs`'s five cases, a right-to-left name, names Inter
/// lacks (Review Focus 2), accented names, and marks the shaper raises
/// (so the raised-glyph text matrix's sign is checked), each with its
/// title.
const CASES: [(&str, &str, Option<&str>); 9] = [
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
    (
        "raised marks",
        "name,sector,ring,description\nA\u{300}\u{301}B\u{323}\u{302}x,Tools,Adopt,d\nKafka,Tools,Trial,d\n",
        Some("Ra\u{301}\u{300}dar"),
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

/// How far a channel inside a filled shape may differ: the two
/// rasterisers agree exactly on a flat fill, so this is rounding only.
const FLAT_DELTA: u8 = 3;
/// A shape smaller than this, in document units, may have no pixel that
/// antialiasing does not touch.
const MIN_SHAPE: f64 = 4.0;
/// How many pixels around a spot must show the shape's own fill in the
/// PNG for the spot to be looked at.
const CLEAR: u32 = 2;
/// Where inside a rectangle to look, as fractions of its sides: the
/// centre first, then a ring of points around it, since a legend row's
/// text covers the centre of most rows.
const RECT_SPOTS: [(f64, f64); 9] = [
    (0.5, 0.5),
    (0.5, 0.25),
    (0.5, 0.75),
    (0.25, 0.5),
    (0.75, 0.5),
    (0.3, 0.3),
    (0.7, 0.3),
    (0.3, 0.7),
    (0.7, 0.7),
];
/// Where inside a circle to look, as fractions of its radius from the
/// centre: the centre, then eight points between a marker's number and
/// its edge.
const CIRCLE_SPOTS: [(f64, f64); 9] = [
    (0.0, 0.0),
    (0.0, -0.7),
    (0.0, 0.7),
    (-0.7, 0.0),
    (0.7, 0.0),
    (-0.5, -0.5),
    (0.5, -0.5),
    (-0.5, 0.5),
    (0.5, 0.5),
];

/// A filled shape: the points to look at inside it, in document units,
/// and its fill's channels.
struct Filled {
    spots: Vec<Point>,
    rgb: [u8; 3],
}

/// Every filled rectangle and circle wider than [`MIN_SHAPE`] that the
/// static rendering draws, through its groups' offsets.
fn filled(node: &Node, at: Point, palette: &Palette, out: &mut Vec<Filled>) {
    if node.is_interactive_only() {
        return;
    }
    let (fill, spots): (Paint, Vec<Point>) = match node {
        Node::Group(group) => {
            let at = Point {
                x: at.x + group.offset.x,
                y: at.y + group.offset.y,
            };
            for child in &group.children {
                filled(child, at, palette, out);
            }
            return;
        }
        Node::Rect {
            origin,
            width,
            height,
            fill,
            ..
        } if width.min(*height) > MIN_SHAPE => (
            *fill,
            RECT_SPOTS
                .iter()
                .map(|&(fx, fy)| Point {
                    x: at.x + origin.x + width * fx,
                    y: at.y + origin.y + height * fy,
                })
                .collect(),
        ),
        Node::Circle {
            centre,
            radius,
            fill,
            ..
        } if radius * 2.0 > MIN_SHAPE => (
            *fill,
            CIRCLE_SPOTS
                .iter()
                .map(|&(fx, fy)| Point {
                    x: at.x + centre.x + radius * fx,
                    y: at.y + centre.y + radius * fy,
                })
                .collect(),
        ),
        _ => return,
    };
    let rgb = match fill {
        Paint::Role(role) => palette.rgb(role),
        Paint::NumberOn(index) => palette.number_ink_rgb(index),
        Paint::None => return,
    };
    out.push(Filled { spots, rgb });
}

/// Whether every channel of `a` and `b` is within [`FLAT_DELTA`].
fn flat(a: &[u8], b: &[u8]) -> bool {
    a.iter().zip(b).all(|(p, q)| p.abs_diff(*q) <= FLAT_DELTA)
}

/// What [`colours_off`] found.
struct Colours {
    /// The filled shapes the document draws.
    shapes: usize,
    /// How many had a clear spot to look at.
    looked: usize,
    /// Each sampled pixel the two disagree on.
    off: Vec<String>,
}

/// Each sampled pixel where hayro's drawing of `pdf` and the PNG differ
/// by more than [`FLAT_DELTA`] in a channel: in every filled shape, the
/// first spot where the PNG shows the shape's own fill (nothing drawn
/// over or near it), and the ground at two opposite corners.
fn colours_off(doc: &Document, pdf: &[u8], palette: &Palette) -> Colours {
    let size = pixel_size(doc, None).expect("fits");
    let png = Pixmap::decode_png(&rasterise(doc, palette, size).expect("renders")).expect("a PNG");
    let drawn = drawn(pdf, size, intrinsic_size(doc, None));
    let (w, h) = (size.width(), size.height());
    let pixel = |image: &[u8], (x, y): (u32, u32)| {
        let at = usize::try_from((y * w + x) * 4).expect("fits");
        image.get(at..at + 4).map(<[u8]>::to_vec)
    };
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::as_conversions,
        reason = "a test's pixel index: every spot is inside the page, so its floor fits u32"
    )]
    let to_pixel = |p: &Point| {
        (
            (p.x * f64::from(w) / doc.width).floor() as u32,
            (p.y * f64::from(h) / doc.height).floor() as u32,
        )
    };
    let mut shapes = Vec::new();
    filled(&doc.root, Point::ORIGIN, palette, &mut shapes);
    let mut seen = vec![(0, 0), (w - 1, h - 1)];
    // A spot is clear when the PNG shows the shape's own fill all around
    // it, so neither rasteriser's antialiasing of a nearby glyph reaches it.
    let clear = |(x, y): (u32, u32), own: &[u8]| {
        (x.saturating_sub(CLEAR)..=x + CLEAR).all(|x| {
            (y.saturating_sub(CLEAR)..=y + CLEAR)
                .all(|y| pixel(png.data(), (x, y)).is_some_and(|p| flat(&p, own)))
        })
    };
    seen.extend(shapes.iter().filter_map(|shape| {
        let own = [shape.rgb[0], shape.rgb[1], shape.rgb[2], 255];
        shape.spots.iter().map(to_pixel).find(|&at| clear(at, &own))
    }));
    let looked = seen.len() - 2;
    let mut off = Vec::new();
    for at in seen {
        match (pixel(png.data(), at), pixel(&drawn, at)) {
            (Some(a), Some(b)) if flat(&a, &b) => {}
            (a, b) => off.push(format!("{at:?}: PNG {a:?}, PDF {b:?}")),
        }
    }
    Colours {
        shapes: shapes.len(),
        looked,
        off,
    }
}

/// Inside every filled shape, and at the ground's corners, the PDF is the
/// PNG's colour, in both themes. At least half the shapes have a clear
/// spot, so the check is never vacuous; a marker's number or a row's text
/// covers the rest.
#[test]
fn every_filled_shape_has_its_colour_inside() {
    let fonts = Fonts::embedded().expect("loads");
    for (name, csv, title) in CASES {
        let (doc, desc) = wide(csv, title, &fonts);
        for theme in [Theme::Light, Theme::Dark] {
            let palette = Palette::built_in(theme);
            let pdf = pdf_of(&doc, &desc, &palette, &fonts, title, None);
            let colours = colours_off(&doc, &pdf, &palette);
            eprintln!(
                "{name} {theme:?}: looked inside {} of {} shapes",
                colours.looked, colours.shapes
            );
            assert!(colours.looked * 2 >= colours.shapes, "{name} {theme:?}");
            assert!(
                colours.off.is_empty(),
                "{name} {theme:?}: {:?}",
                colours.off
            );
        }
    }
}

/// Each halo the document draws, in document order: its stroke's
/// channels as PDF reals and its width.
fn halos(node: &Node, palette: &Palette, out: &mut Vec<([f32; 3], f32)>) {
    if node.is_interactive_only() {
        return;
    }
    match node {
        Node::Group(group) => {
            for child in &group.children {
                halos(child, palette, out);
            }
        }
        Node::Text {
            ink: TextPaint::Halo { stroke, width },
            lines,
            ..
        } if !lines.is_empty() => {
            let Paint::Role(role) = stroke else {
                panic!("a halo strokes with a role");
            };
            #[expect(
                clippy::cast_possible_truncation,
                clippy::as_conversions,
                reason = "a test's expected value, compared within a thousandth"
            )]
            let width = *width as f32;
            out.push((palette.rgb(*role).map(|c| f32::from(c) / 255.0), width));
        }
        _ => {}
    }
}

/// Where each halo outline group is in `ops`: the `q` that opens it and
/// the `Q` that closes it. A halo group opens `q RG w 1 j`, as nothing
/// else in the stream does.
fn halo_groups(ops: &[Operation]) -> Vec<(usize, usize)> {
    let mut groups = Vec::new();
    for (i, window) in ops.windows(4).enumerate() {
        let names: Vec<&str> = window.iter().map(|op| op.operator.as_str()).collect();
        let round = window
            .get(3)
            .and_then(|j| j.operands.first())
            .and_then(|o| o.as_i64().ok())
            == Some(1);
        if names == ["q", "RG", "w", "j"] && round {
            let mut depth = 0_i32;
            let end = ops
                .iter()
                .enumerate()
                .skip(i)
                .find(|(_, op)| {
                    match op.operator.as_str() {
                        "q" => depth += 1,
                        "Q" => depth -= 1,
                        _ => {}
                    }
                    depth == 0
                })
                .map(|(at, _)| at)
                .expect("a halo group closes");
            groups.push((i, end));
        }
    }
    groups
}

/// The page's content operators.
fn operations(pdf: &[u8]) -> Vec<Operation> {
    let doc = lopdf::Document::load_mem(pdf).expect("parses");
    let page = *doc.get_pages().values().next().expect("a page");
    Content::decode(&doc.get_page_content(page))
        .expect("decodes")
        .operations
}

/// What is wrong with `pdf`'s halos against `doc`'s: a count that differs,
/// or a group not stroked in its halo's colour and width with no text.
fn halos_off(doc: &Document, pdf: &[u8], palette: &Palette) -> Vec<String> {
    let mut want = Vec::new();
    halos(&doc.root, palette, &mut want);
    let ops = operations(pdf);
    let groups = halo_groups(&ops);
    if groups.len() != want.len() {
        return vec![format!(
            "{} halo groups for {} halos",
            groups.len(),
            want.len()
        )];
    }
    let number = |op: &Operation, i: usize| {
        op.operands
            .get(i)
            .and_then(|o| o.as_float().ok())
            .expect("a number")
    };
    let mut off = Vec::new();
    for (&(start, end), (rgb, width)) in groups.iter().zip(&want) {
        let group = ops.get(start..=end).expect("in the stream");
        let (Some(stroke), Some(w)) = (group.get(1), group.get(2)) else {
            continue;
        };
        let got_rgb = [0, 1, 2].map(|i| number(stroke, i));
        let near = |a: f32, b: f32| (a - b).abs() < 0.001;
        if !got_rgb.iter().zip(rgb).all(|(a, b)| near(*a, *b)) || !near(number(w, 0), *width) {
            off.push(format!(
                "at op {start}: {got_rgb:?} {} for {rgb:?} {width}",
                number(w, 0)
            ));
        }
        let has = |name: &str| group.iter().any(|op| op.operator == name);
        if !has("S") || has("BT") {
            off.push(format!("at op {start}: not a stroked outline"));
        }
    }
    off
}

/// Every halo is one stroked outline group in its colour and width.
#[test]
fn every_halo_is_a_stroked_outline_group() {
    let fonts = Fonts::embedded().expect("loads");
    for (name, csv, title) in CASES {
        let (doc, desc) = wide(csv, title, &fonts);
        for theme in [Theme::Light, Theme::Dark] {
            let palette = Palette::built_in(theme);
            let pdf = pdf_of(&doc, &desc, &palette, &fonts, title, None);
            let mut want = Vec::new();
            halos(&doc.root, &palette, &mut want);
            assert!(
                !want.is_empty(),
                "{name}: the radar's ring labels are haloed"
            );
            let off = halos_off(&doc, &pdf, &palette);
            assert!(off.is_empty(), "{name} {theme:?}: {off:?}");
        }
    }
}

/// Three-sector in `theme`: the document, its PDF and the palette.
fn case(theme: Theme) -> (Document, Vec<u8>, Palette) {
    let fonts = Fonts::embedded().expect("loads");
    let (_, csv, title) = CASES[0];
    let (doc, desc) = wide(csv, title, &fonts);
    let palette = Palette::built_in(theme);
    let pdf = pdf_of(&doc, &desc, &palette, &fonts, title, None);
    (doc, pdf, palette)
}

/// The halos removed, dark, where they show most, fails.
#[test]
fn removed_halos_fail() {
    let (doc, pdf, palette) = case(Theme::Dark);
    let broken = damaged(&pdf, |ops| {
        for (start, end) in halo_groups(ops).into_iter().rev() {
            ops.drain(start..=end);
        }
    });
    assert!(
        !halos_off(&doc, &broken, &palette).is_empty()
            || !colours_off(&doc, &broken, &palette).off.is_empty(),
        "removed halos must fail"
    );
}

/// Every halo stroked at half its width fails.
#[test]
fn a_halved_halo_fails() {
    let (doc, pdf, palette) = case(Theme::Light);
    let broken = damaged(&pdf, |ops| {
        for (start, _) in halo_groups(ops) {
            if let Some(w) = ops
                .get_mut(start + 2)
                .and_then(|op| op.operands.first_mut())
            {
                let half = w.as_float().expect("a width") / 2.0;
                *w = Object::Real(half);
            }
        }
    });
    assert!(
        !halos_off(&doc, &broken, &palette).is_empty()
            || !colours_off(&doc, &broken, &palette).off.is_empty(),
        "a halved halo must fail"
    );
}

/// The legend's and key's `Surface2` panels left unfilled fails, in
/// both themes, though no pixel moves past [`MAX_DELTA`].
#[test]
fn unfilled_panels_fail() {
    for theme in [Theme::Light, Theme::Dark] {
        let (doc, pdf, palette) = case(theme);
        let surface = palette
            .rgb(Role::Neutral(Neutral::Surface2))
            .map(|c| f32::from(c) / 255.0);
        let broken = damaged(&pdf, |ops| {
            let mut panel = false;
            for op in ops.iter_mut() {
                match op.operator.as_str() {
                    "rg" => {
                        panel = op
                            .operands
                            .iter()
                            .zip(surface)
                            .all(|(o, c)| o.as_float().is_ok_and(|v| (v - c).abs() < 0.001));
                    }
                    "f" if panel => op.operator = "n".to_owned(),
                    _ => {}
                }
            }
        });
        assert!(
            !halos_off(&doc, &broken, &palette).is_empty()
                || !colours_off(&doc, &broken, &palette).off.is_empty(),
            "{theme:?}: unfilled panels must fail"
        );
    }
}
