//! The PDF export (FR-110, D-103): the wide document as one vector page
//! the document's size, its text real text in embedded Inter subsets,
//! untagged, with no date or identifier (NFR-4).

use pdf_writer::{Finish, Pdf, Rect, Ref, TextStr};

use super::{UNTITLED, intrinsic_size};
use crate::diagnostics::Fatal;
use crate::palette::Palette;
use crate::pdf::content::{Canvas, content};
use crate::pdf::font::Faces;
use crate::pdf::real;
use crate::svg::Document;
use crate::text::Fonts;

/// PDF points per CSS pixel: 72 per inch over 96.
pub const POINTS_PER_PX: f64 = 0.75;
/// The longest page side viewers accept, in points: 200 inches, Acrobat's
/// limit (PDF 1.7, Annex C).
const MAX_SIDE: f64 = 14_400.0;

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
    let natural = POINTS_PER_PX * width / document.width;
    let unit = user_unit(document.width * natural, document.height * natural);
    // The content-to-page scale, in the page's own units.
    let scale = natural / f64::from(unit);
    let faces = Faces::of([&document.root], export.fonts)?;
    let mut next = Ref::new(1);
    let [catalog, tree, page, stream, info] = [(); 5].map(|()| next.bump());
    let mut pdf = Pdf::new();

    let mut cat = pdf.catalog(catalog);
    cat.pages(tree).lang(TextStr("en"));
    cat.viewer_preferences().display_doc_title(true);
    cat.finish();

    pdf.pages(tree).kids([page]).count(1);

    let fonts = faces.write(&mut pdf, &mut next, export.fonts)?;
    let mut p = pdf.page(page);
    // The same real(side × scale) content() translates by, so the drawing
    // meets the MediaBox's edges (Review Focus 3).
    p.parent(tree)
        .media_box(Rect::new(
            0.0,
            0.0,
            real(document.width * scale),
            real(document.height * scale),
        ))
        .contents(stream);
    if unit > 1 {
        p.user_unit(f32::from(unit));
    }
    let mut resources = p.resources();
    let mut dict = resources.fonts();
    for (weight, font) in &fonts {
        dict.pair(Faces::resource(*weight), *font);
    }
    dict.finish();
    resources.finish();
    p.finish();

    let drawn = content(&Canvas {
        document,
        palette: export.palette,
        fonts: export.fonts,
        faces: &faces,
        scale,
    });
    pdf.stream(stream, &drawn);

    pdf.document_info(info)
        .title(TextStr(export.title.unwrap_or(UNTITLED)))
        .subject(TextStr(export.desc))
        .producer(TextStr(export.producer));
    Ok(pdf.finish())
}

/// The page's `/UserUnit` (PDF 1.6): 1, or the smallest whole number that
/// brings both sides of a `width` × `height` pt page under [`MAX_SIDE`],
/// so a tall legend's page keeps its size rather than being refused.
///
/// # Panics
///
/// Past `u16::MAX`, a page over 900 km long: a layout bug.
fn user_unit(width: f64, height: f64) -> u16 {
    let longest = width.max(height);
    (1_u16..=u16::MAX)
        .find(|&u| longest / f64::from(u) <= MAX_SIDE)
        .expect("no layout is 65,535 × 200 inches long")
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

    fn near(got: f32, want: f64) -> bool {
        (f64::from(got) - want).abs() <= 0.001
    }

    /// FR-110, FR-57: one page, the document's size at 0.75 pt per px,
    /// or `--width` px wide with the height following (Review Focus 3).
    #[test]
    fn the_page_is_the_documents_size() {
        let doc = prd(None);
        for width in [None, Some(320), Some(10_000)] {
            let [x0, y0, w, h] = media_box(&pdf(&doc, None, width, Theme::Light));
            let (want_w, want_h) = crate::emit::intrinsic_size(&doc, width);
            assert!(x0 == 0.0 && y0 == 0.0, "--width {width:?}");
            assert!(
                near(w, want_w * 0.75) && near(h, want_h * 0.75),
                "--width {width:?}: {w} × {h}"
            );
            assert!(
                page(&pdf(&doc, None, width, Theme::Light))
                    .get(b"UserUnit")
                    .is_err(),
                "under the limit, no UserUnit"
            );
        }
    }

    /// Review Focus 3: a side past the 14,400 pt viewers accept is written
    /// with a `/UserUnit` that brings it under, keeping the page's size.
    #[test]
    fn a_page_past_the_viewers_limit_gets_a_user_unit() {
        // 30,000 px tall is 22,500 pt: a long single-column legend's shape.
        let doc = Document {
            width: 100.0,
            height: 30_000.0,
            root: Node::Group(Group::new(Point::ORIGIN, Vec::new())),
        };
        let bytes = pdf(&doc, None, None, Theme::Light);
        let unit = page(&bytes)
            .get(b"UserUnit")
            .and_then(lopdf::Object::as_float)
            .expect("a UserUnit");
        assert!(
            (unit - 2.0).abs() < f32::EPSILON,
            "the smallest whole unit that fits: {unit}"
        );
        let [_, _, w, h] = media_box(&bytes);
        assert!(h <= 14_400.0, "{h}");
        assert!(
            near(w * unit, 75.0) && near(h * unit, 22_500.0),
            "{w} × {h} at {unit}"
        );
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
