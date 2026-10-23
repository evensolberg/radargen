//! The PDF export (FR-110, D-103): the wide document as one vector page
//! the document's size, its text real text in embedded Inter subsets,
//! untagged, with no date or identifier (NFR-4).

use pdf_writer::{Pdf, Rect, Ref, TextStr, writers};

use super::{UNTITLED, intrinsic_size};
use crate::diagnostics::Fatal;
use crate::num::{round_thousandths, saturating_u32};
use crate::palette::Palette;
use crate::pdf::content::{Canvas, content};
use crate::pdf::font::Faces;
use crate::pdf::real;
use crate::svg::Document;
use crate::text::{Fonts, Weight};

/// PDF points per CSS pixel: 72 per inch over 96.
const POINTS_PER_PX: f64 = 0.75;
/// The longest page side viewers accept, in points: 200 inches, Acrobat's
/// limit (PDF 1.7, Annex C).
const MAX_SIDE: f64 = 14_400.0;
/// The objects every light PDF numbers first, in this order: the catalog,
/// the page tree, the page, its content stream and the info dictionary.
/// The fonts' objects follow them.
const FIXED_OBJECTS: usize = 5;

/// What one export needs besides the document.
pub struct Export<'a> {
    /// The theme's palette (FR-56).
    pub palette: &'a Palette,
    /// The faces the text is placed and subset from.
    pub fonts: &'a Fonts,
    /// `--width` (FR-57), in px.
    pub width: Option<u32>,
    /// FR-65's title, resolved.
    pub title: Option<&'a str>,
    /// The radar's name (D-75), `/Subject`: what the SVG's `<desc>` holds.
    pub desc: &'a str,
    /// `/Producer`: [`OUTPUT_VERSION`](crate::emit::OUTPUT_VERSION) from
    /// the pipeline (SC-25); a test passes its placeholder.
    pub producer: &'a str,
}

/// `document` as a PDF file (FR-110).
///
/// # Errors
///
/// [`Fatal::Font`] when a glyph cannot be measured or subset (NFR-15).
pub fn export(document: &Document, export: &Export<'_>) -> Result<Vec<u8>, Fatal> {
    let (width, _) = intrinsic_size(document, export.width);
    let sheet = Sheet::of(document, width);
    let faces = Faces::of([&document.root], export.fonts)?;
    let mut next = Ref::new(1);
    let objects = Objects::numbered(&mut next);
    let mut pdf = Pdf::new();
    catalog(&mut pdf, &objects);
    pdf.pages(objects.tree).kids([objects.page]).count(1);
    let fonts = faces.write(&mut pdf, &mut next, export.fonts)?;
    page(&mut pdf, &objects, &sheet, &fonts);
    let drawn = content(&Canvas {
        document,
        palette: export.palette,
        fonts: export.fonts,
        faces: &faces,
        scale: sheet.scale,
        width: sheet.width,
        height: sheet.height,
    })?;
    pdf.stream(objects.stream, &drawn);
    info(&mut pdf, objects.info, export);
    Ok(pdf.finish())
}

/// The page in its own units: FR-57's size at [`POINTS_PER_PX`], and the
/// `/UserUnit` (PDF 1.6) that brings both sides under [`MAX_SIDE`], so a
/// tall legend's page keeps its size rather than being refused.
struct Sheet {
    /// 1, or the smallest whole number that brings both sides under
    /// [`MAX_SIDE`].
    unit: u16,
    /// The content-to-page scale, in the page's own units.
    scale: f64,
    /// The `MediaBox`'s width, in the page's own units: the drawing's,
    /// rounded up to a whole number.
    width: f64,
    /// The `MediaBox`'s height, likewise.
    height: f64,
}

impl Sheet {
    /// `document`'s page at `width` CSS px wide (FR-57).
    ///
    /// # Panics
    ///
    /// Past `u16::MAX`, a page over 900 km long: a layout bug.
    // qual:allow(complexity, error_handling) reason: "the documented # Panics: no layout is 65,535 × 200 inches long"
    fn of(document: &Document, width: f64) -> Self {
        let natural = POINTS_PER_PX * width / document.width;
        let longest = (document.width * natural).max(document.height * natural);
        // The smallest whole u with longest / u <= MAX_SIDE; 1 for a page
        // that already fits. `saturating_u32` saturates, so a page past
        // u16::MAX units fails the conversion rather than wrapping.
        let unit = u16::try_from(saturating_u32((longest / MAX_SIDE).ceil()).max(1))
            .expect("no layout is 65,535 × 200 inches long");
        let scale = natural / f64::from(unit);
        Self {
            unit,
            scale,
            width: whole(document.width * scale),
            height: whole(document.height * scale),
        }
    }
}

/// A drawing's side as the page box's: rounded up to a whole number, so
/// no viewer is left a fractional point to fill with a light edge on a
/// dark page; the drawing stays anchored top-left and the at most one
/// point more on the right and bottom is ground. Taken to thousandths
/// first, as [`real`] takes a coordinate, so a float residue past an
/// exact side (`240.00000000000003`) cannot add a whole point.
fn whole(side: f64) -> f64 {
    round_thousandths(side).ceil()
}

/// The fixed objects' numbers, the first [`FIXED_OBJECTS`] in order.
struct Objects {
    catalog: Ref,
    tree: Ref,
    page: Ref,
    stream: Ref,
    info: Ref,
}

impl Objects {
    /// The fixed objects numbered from `next`, which moves past them.
    fn numbered(next: &mut Ref) -> Self {
        let [catalog, tree, page, stream, info] = [(); FIXED_OBJECTS].map(|()| next.bump());
        Self {
            catalog,
            tree,
            page,
            stream,
            info,
        }
    }
}

/// The catalog: the page tree, `/Lang (en)`, and the title shown as the
/// window's (D-75).
fn catalog(pdf: &mut Pdf, objects: &Objects) {
    let mut cat = pdf.catalog(objects.catalog);
    cat.pages(objects.tree).lang(TextStr("en"));
    cat.viewer_preferences().display_doc_title(true);
}

/// The one page: its `MediaBox`, its `/UserUnit` when it needs one, its
/// content stream and its fonts.
fn page(pdf: &mut Pdf, objects: &Objects, sheet: &Sheet, fonts: &[(Weight, Ref)]) {
    let mut p = pdf.page(objects.page);
    p.parent(objects.tree)
        .media_box(media_box(sheet))
        .contents(objects.stream);
    user_unit(&mut p, sheet.unit);
    p.resources().fonts().pairs(
        fonts
            .iter()
            .map(|&(weight, font)| (Faces::resource(weight), font)),
    );
}

/// The `MediaBox`: the whole-number sides `content()` grounds and
/// translates by, so the ground meets its edges (Review Focus 3).
fn media_box(sheet: &Sheet) -> Rect {
    Rect::new(0.0, 0.0, real(sheet.width), real(sheet.height))
}

/// `/UserUnit` on `page`, written only when it is not PDF's default 1.
fn user_unit(page: &mut writers::Page<'_>, unit: u16) {
    if unit > 1 {
        page.user_unit(f32::from(unit));
    }
}

/// The info dictionary: `/Title`, `/Subject` and `/Producer`, and no date
/// (NFR-4).
fn info(pdf: &mut Pdf, id: Ref, export: &Export<'_>) {
    pdf.document_info(id)
        .title(TextStr(export.title.unwrap_or(UNTITLED)))
        .subject(TextStr(export.desc))
        .producer(TextStr(export.producer));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::arrange::{Arrangement, Blocks, arrange};
    use crate::layout::block::fonts;
    use crate::layout::test_support::{PRD_EXAMPLE, plotted};
    use crate::palette::Theme;
    use crate::svg::{Group, Node, Point};

    fn prd(title: Option<&str>) -> Document {
        let (catalogue, order) = plotted(PRD_EXAMPLE, None);
        let blocks = Blocks::new(&catalogue, &order, title, None, fonts());
        arrange(Arrangement::Wide, &blocks)
    }

    fn pdf(doc: &Document, title: Option<&str>, width: Option<u32>, theme: Theme) -> Vec<u8> {
        let palette = Palette::built_in(theme);
        export(
            doc,
            &Export {
                palette: &palette,
                fonts: fonts(),
                width,
                title,
                desc: "Technology radar: 3 sectors",
                producer: "radargen 9.9",
            },
        )
        .expect("exports")
    }

    fn page(bytes: &[u8]) -> lopdf::Dictionary {
        let doc = lopdf::Document::load_mem(bytes).expect("parses");
        let page = *doc.get_pages().values().next().expect("one page");
        doc.get_dictionary(page).expect("a page dictionary").clone()
    }

    fn media_box(bytes: &[u8]) -> [f32; 4] {
        let dict = page(bytes);
        let mb = dict
            .get(b"MediaBox")
            .and_then(lopdf::Object::as_array)
            .expect("a MediaBox");
        let n = |i: usize| mb.get(i).and_then(|o| o.as_float().ok()).expect("a number");
        [n(0), n(1), n(2), n(3)]
    }

    /// FR-110, FR-57: one page, the document's size at 0.75 pt per px,
    /// or `--width` px wide with the height following (Review Focus 3),
    /// each side rounded up to whole points so no viewer leaves a
    /// fractional sliver of paper at an edge.
    #[test]
    fn the_page_is_the_documents_size_in_whole_points() {
        let doc = prd(None);
        for width in [None, Some(320), Some(10_000)] {
            let [x0, y0, w, h] = media_box(&pdf(&doc, None, width, Theme::Light));
            let (want_w, want_h) = crate::emit::intrinsic_size(&doc, width);
            assert!(x0 == 0.0 && y0 == 0.0, "--width {width:?}");
            for (got, want) in [(w, want_w * POINTS_PER_PX), (h, want_h * POINTS_PER_PX)] {
                let got = f64::from(got);
                assert!(
                    got > want - 0.001 && got < want + 1.0 && got.fract().to_bits() == 0,
                    "--width {width:?}: {got} for {want}"
                );
            }
            assert!(
                page(&pdf(&doc, None, width, Theme::Light))
                    .get(b"UserUnit")
                    .is_err(),
                "under the limit, no UserUnit"
            );
        }
    }

    /// Review Focus 3: a side past the 14,400 pt viewers accept is written
    /// with a `/UserUnit` that brings it under, keeping the page's size,
    /// at the document's own width and at `--width`. The PRD document,
    /// forced 30,000 px tall (a long single-column legend's shape), so the
    /// content is real: its flip scales by 0.75 pt per px over the unit,
    /// and translates by the `MediaBox`'s own height.
    #[test]
    fn a_page_past_the_viewers_limit_gets_a_user_unit() {
        let mut doc = prd(None);
        doc.height = 30_000.0;
        for width in [None, Some(2_000)] {
            let bytes = pdf(&doc, None, width, Theme::Light);
            let unit = page(&bytes)
                .get(b"UserUnit")
                .and_then(lopdf::Object::as_float)
                .expect("a UserUnit");
            let (want_w, want_h) = crate::emit::intrinsic_size(&doc, width);
            let longest = want_w.max(want_h) * POINTS_PER_PX;
            let smallest = (longest / MAX_SIDE).ceil();
            assert!(
                smallest > 1.0 && f64::from(unit).to_bits() == smallest.to_bits(),
                "--width {width:?}: the smallest whole unit that fits, {smallest}, not {unit}"
            );
            let [_, _, w, h] = media_box(&bytes);
            assert!(h <= 14_400.0, "{h}");
            // Each side is whole in the page's units, so in points it is
            // the intended size, rounded up to a whole unit.
            let kept = |side: f32, want: f64| {
                let points = f64::from(side * unit);
                side.fract().to_bits() == 0 && points >= want && points < want + f64::from(unit)
            };
            assert!(
                kept(w, want_w * POINTS_PER_PX) && kept(h, want_h * POINTS_PER_PX),
                "--width {width:?}: {w} × {h} at {unit}"
            );
            let parsed = lopdf::Document::load_mem(&bytes).expect("parses");
            let id = *parsed.get_pages().values().next().expect("a page");
            let ops = lopdf::content::Content::decode(&parsed.get_page_content(id))
                .expect("decodes")
                .operations;
            let cm = ops.iter().find(|op| op.operator == "cm").expect("the flip");
            let operand = |i: usize| {
                cm.operands
                    .get(i)
                    .and_then(|o| o.as_float().ok())
                    .expect("a number")
            };
            let a = POINTS_PER_PX * want_w / doc.width / f64::from(unit);
            assert!(
                (f64::from(operand(0)) - a).abs() < a * 1e-6,
                "--width {width:?}: cm a {} for {a}",
                operand(0)
            );
            assert_eq!(operand(5).to_bits(), h.to_bits(), "--width {width:?}");
        }
    }

    /// FR-110, NFR-4: the same bytes twice; no date, no file identifier;
    /// the title, subject, language and producer are set (D-75, SC-25).
    #[test]
    fn the_file_is_reproducible_and_named() {
        let doc = prd(None);
        let first = pdf(&doc, None, None, Theme::Dark);
        assert!(first == pdf(&doc, None, None, Theme::Dark), "NFR-4");
        let parsed = lopdf::Document::load_mem(&first).expect("parses");
        assert!(parsed.trailer.get(b"ID").is_err(), "no file identifier");
        let info = parsed
            .trailer
            .get(b"Info")
            .and_then(lopdf::Object::as_reference)
            .expect("an Info");
        let info = parsed.get_dictionary(info).expect("an info dictionary");
        for absent in [&b"CreationDate"[..], b"ModDate"] {
            assert!(
                info.get(absent).is_err(),
                "{}",
                String::from_utf8_lossy(absent)
            );
        }
        let text = |key: &[u8]| {
            lopdf::decode_text_string(info.get(key).expect("set")).expect("a text string")
        };
        assert_eq!(text(b"Title"), "Technology Radar");
        assert_eq!(text(b"Subject"), "Technology radar: 3 sectors");
        assert_eq!(text(b"Producer"), "radargen 9.9");
        let raw = String::from_utf8_lossy(&first);
        assert!(
            raw.contains("/Lang (en)") && raw.contains("/DisplayDocTitle true"),
            "the catalog"
        );
        assert!(first.starts_with(b"%PDF-1.7\n"));
    }

    /// No TJ carries a float-residue adjustment: every one written moves a
    /// glyph by at least a hundredth of a thousandth of an em.
    #[test]
    fn no_adjustment_is_float_residue() {
        let doc =
            lopdf::Document::load_mem(&pdf(&prd(None), None, None, Theme::Light)).expect("parses");
        let page = *doc.get_pages().values().next().expect("a page");
        let ops = lopdf::content::Content::decode(&doc.get_page_content(page))
            .expect("decodes")
            .operations;
        let adjustments: Vec<f32> = ops
            .iter()
            .filter(|op| op.operator == "TJ")
            .filter_map(|op| op.operands.first().and_then(|a| a.as_array().ok()))
            .flatten()
            .filter_map(|item| item.as_float().ok())
            .collect();
        assert!(!adjustments.is_empty(), "the PRD kerns");
        assert!(
            adjustments.iter().all(|a| a.abs() >= 0.01),
            "{adjustments:?}"
        );
    }

    /// A halo and its fill, as the radar draws a ring label: the label
    /// extracts once, since only the fill is text.
    #[test]
    fn a_haloed_label_extracts_once() {
        use crate::palette::{Neutral, Role};
        use crate::style::TextStyle;
        use crate::svg::{Anchor, Paint, TextPaint};
        use crate::text::Weight;
        let label = |ink| Node::Text {
            origin: Point { x: 10.0, y: 20.0 },
            style: TextStyle {
                weight: Weight::Regular,
                size: 10.0,
                letter_spacing: 0.0,
            },
            ink,
            anchor: Anchor::Start,
            lines: vec!["Adopt".to_owned()],
            words: "Adopt".to_owned(),
            leading: 12.0,
            tail: None,
        };
        let doc = Document {
            width: 100.0,
            height: 40.0,
            root: Node::Group(Group::new(
                Point::ORIGIN,
                vec![
                    label(TextPaint::Halo {
                        stroke: Paint::Role(Role::Neutral(Neutral::Surface1)),
                        width: 3.0,
                    }),
                    label(TextPaint::Fill(Paint::Role(Role::Neutral(
                        Neutral::TextPrimary,
                    )))),
                ],
            )),
        };
        let parsed =
            lopdf::Document::load_mem(&pdf(&doc, None, None, Theme::Light)).expect("parses");
        let text = parsed.extract_text(&[1]).expect("extracts");
        assert_eq!(text.matches("Adopt").count(), 1, "{text}");
    }

    /// Review Focus 1: a title with PDF string delimiters, an unbalanced
    /// parenthesis, line breaks and non-ASCII reads back exactly.
    #[test]
    fn a_hostile_title_reads_back() {
        let title = "Q3 (draft) \\ Ærø — 中 ) \r\n end";
        let doc = prd(Some(title));
        let bytes = pdf(&doc, Some(title), None, Theme::Light);
        let parsed = lopdf::Document::load_mem(&bytes).expect("parses");
        let info = parsed
            .trailer
            .get(b"Info")
            .and_then(lopdf::Object::as_reference)
            .expect("an Info");
        let got = parsed
            .get_dictionary(info)
            .and_then(|d| d.get(b"Title"))
            .expect("a Title");
        assert_eq!(
            lopdf::decode_text_string(got).expect("a text string"),
            title
        );
    }
}
