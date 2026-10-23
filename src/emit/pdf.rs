//! The PDF export (FR-110, FR-111, D-103).
//!
//! The wide document as one vector page the document's size, or paper
//! pages, its text real text in embedded Inter subsets, tagged to PDF/UA-1
//! (FR-114), with no date or identifier (NFR-4).

use pdf_writer::types::TabOrder;
use pdf_writer::{Name, Pdf, Rect, Ref, TextStr, writers};

use super::{UNTITLED, intrinsic_size};
use crate::diagnostics::Fatal;
use crate::num::round_thousandths;
use crate::palette::Palette;
use crate::pdf::content::{Canvas, content};
use crate::pdf::font::Faces;
use crate::pdf::tag::Tagger;
use crate::pdf::{Sheet, real, tree, xmp};
use crate::style::POINTS_PER_PX;
use crate::svg::Document;
use crate::text::{Fonts, Weight};

/// The longest page side viewers accept, in points: 200 inches, Acrobat's
/// limit (PDF 1.7, Annex C).
const MAX_SIDE: f64 = 14_400.0;

/// What one export needs besides the document.
pub struct Export<'a> {
    /// The theme's palette (FR-56).
    pub palette: &'a Palette,
    /// The faces the text is placed and subset from.
    pub fonts: &'a Fonts,
    /// FR-65's title, resolved.
    pub title: Option<&'a str>,
    /// The radar's name (D-75), `/Subject`: what the SVG's `<desc>` holds.
    pub desc: &'a str,
    /// `/Producer`: [`OUTPUT_VERSION`](crate::emit::OUTPUT_VERSION) from
    /// the pipeline (SC-25); a test passes its placeholder.
    pub producer: &'a str,
}

/// `document` as a one-page PDF the drawing's size, `width` px wide when
/// `--width` gives one (FR-110, FR-57).
///
/// # Errors
///
/// [`Fatal::Font`] when a glyph cannot be measured or subset (NFR-15).
pub fn export(
    document: &Document,
    width: Option<u32>,
    export: &Export<'_>,
) -> Result<Vec<u8>, Fatal> {
    let (width, _) = intrinsic_size(document, width);
    write(&[(document, sheet(document, width))], export)
}

/// `pages` as a PDF on paper (FR-111): one page per document, each the
/// paper's exact size in points, with no `/UserUnit`.
///
/// # Errors
///
/// [`Fatal::Font`] when a glyph cannot be measured or subset (NFR-15).
pub fn export_pages(pages: &[Document], export: &Export<'_>) -> Result<Vec<u8>, Fatal> {
    let sheets: Vec<(&Document, Sheet)> = pages
        .iter()
        .map(|page| {
            (
                page,
                Sheet {
                    unit: 1.0,
                    scale: POINTS_PER_PX,
                    width: page.width * POINTS_PER_PX,
                    height: page.height * POINTS_PER_PX,
                },
            )
        })
        .collect();
    write(&sheets, export)
}

/// The PDF file: the catalog, the page tree and the info dictionary, then
/// each page and its content stream, numbered in that order; the fonts,
/// subset once over every page, are numbered after them, then the
/// structure tree's root, the XMP metadata, the tree's elements and its
/// parent-tree arrays (FR-114). In the file the fonts come before the
/// pages, since each page names them, the catalog after the tree it names,
/// and the info dictionary last.
fn write(pages: &[(&Document, Sheet)], export: &Export<'_>) -> Result<Vec<u8>, Fatal> {
    let faces = Faces::of(pages.iter().map(|(page, _)| &page.root), export.fonts)?;
    let mut next = Ref::new(1);
    let [catalog_id, tree_id, info_id] = std::array::from_fn(|_| next.bump());
    let ids: Vec<(Ref, Ref)> = pages.iter().map(|_| (next.bump(), next.bump())).collect();
    let mut pdf = Pdf::new();
    pdf.pages(tree_id)
        .kids(ids.iter().map(|&(page, _)| page))
        .count(i32::try_from(ids.len()).unwrap_or(i32::MAX));
    let fonts = faces.write(&mut pdf, &mut next, export.fonts)?;
    let [root, metadata] = std::array::from_fn(|_| next.bump());
    // One tagger for every page, so an element can continue on the next.
    let mut tagger = Tagger::new();
    for (k, (&(document, sheet), &(page_id, stream))) in pages.iter().zip(&ids).enumerate() {
        page(&mut pdf, (page_id, tree_id, stream), (&sheet, k), &fonts);
        let drawn = content(
            &Canvas {
                document,
                palette: export.palette,
                fonts: export.fonts,
                faces: &faces,
                page: sheet,
            },
            &mut tagger,
        )?;
        pdf.stream(stream, &drawn);
    }
    let page_ids: Vec<Ref> = ids.iter().map(|&(page, _)| page).collect();
    tree::write(&mut pdf, &tagger.into_tree(), root, &mut next, &page_ids);
    // Cleaned once, so the info dictionary and the XMP agree.
    let title = xmp::clean(export.title.unwrap_or(UNTITLED));
    let packet = xmp::packet(&title);
    pdf.metadata(metadata, packet.as_bytes());
    catalog(&mut pdf, catalog_id, tree_id, (root, metadata));
    // Last in the file, though numbered third: the producer's length,
    // which moves with the version, then moves only the cross-reference
    // offset after it.
    info(&mut pdf, info_id, (&title, export));
    Ok(pdf.finish())
}

/// `document`'s page at `width` CSS px wide (FR-57), in its own units:
/// FR-57's size at [`POINTS_PER_PX`], and the `/UserUnit` (PDF 1.6) that
/// brings both sides under [`MAX_SIDE`], so a tall legend's page keeps its
/// size rather than being refused.
fn sheet(document: &Document, width: f64) -> Sheet {
    let natural = POINTS_PER_PX * width / document.width;
    let longest = (document.width * natural).max(document.height * natural);
    // The smallest whole u with longest / u <= MAX_SIDE; 1 for a page
    // that already fits.
    let unit = (longest / MAX_SIDE).ceil().max(1.0);
    let scale = natural / unit;
    Sheet {
        unit,
        scale,
        width: whole(document.width * scale),
        height: whole(document.height * scale),
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

/// The catalog: the page tree, `/Lang (en)`, the title shown as the
/// window's (D-75), and the tagging PDF/UA-1 asks for: marked, the
/// structure tree's `root` and the XMP `metadata` (FR-114).
fn catalog(pdf: &mut Pdf, id: Ref, tree: Ref, (root, metadata): (Ref, Ref)) {
    let mut cat = pdf.catalog(id);
    cat.pages(tree).lang(TextStr("en"));
    cat.viewer_preferences().display_doc_title(true);
    cat.mark_info().marked(true);
    // A reference to the root, not the inline dictionary pdf-writer
    // starts with `struct_tree_root`.
    cat.pair(Name(b"StructTreeRoot"), root);
    cat.metadata(metadata);
}

/// A page: its `MediaBox`, its `/UserUnit` when it needs one, its content
/// stream and its fonts, and as page `k` its key in the parent tree and a
/// tab order by structure (FR-114).
fn page(
    pdf: &mut Pdf,
    (id, tree, stream): (Ref, Ref, Ref),
    (sheet, k): (&Sheet, usize),
    fonts: &[(Weight, Ref)],
) {
    let mut p = pdf.page(id);
    p.parent(tree).media_box(media_box(sheet)).contents(stream);
    p.struct_parents(i32::try_from(k).unwrap_or(i32::MAX))
        .tab_order(TabOrder::StructureOrder);
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
fn user_unit(page: &mut writers::Page<'_>, unit: f64) {
    if unit > 1.0 {
        page.user_unit(real(unit));
    }
}

/// The info dictionary: `/Title`, `/Subject` and `/Producer`, and no date
/// (NFR-4).
fn info(pdf: &mut Pdf, id: Ref, (title, export): (&str, &Export<'_>)) {
    pdf.document_info(id)
        .title(TextStr(title))
        .subject(TextStr(export.desc))
        .producer(TextStr(export.producer));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::arrange::{Arrangement, Blocks, arrange};
    use crate::layout::block::fonts;
    use crate::layout::test_support::{PRD_EXAMPLE, plotted};
    use crate::options::Paper;
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
            width,
            &Export {
                palette: &palette,
                fonts: fonts(),
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
            let ops = crate::pdf::test_support::operations(&bytes);
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
        let ops = crate::pdf::test_support::operations(&pdf(&prd(None), None, None, Theme::Light));
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

    /// 21c-2 review: a control character XML cannot hold is dropped from
    /// the title once, so the info dictionary's `/Title` and the XMP's
    /// `dc:title` say the same.
    #[test]
    fn the_info_title_matches_the_xmp() {
        let title = "bell\u{7} rings";
        let bytes = pdf(&prd(Some(title)), Some(title), None, Theme::Light);
        let parsed = lopdf::Document::load_mem(&bytes).expect("parses");
        let info = parsed
            .trailer
            .get(b"Info")
            .and_then(lopdf::Object::as_reference)
            .and_then(|r| parsed.get_dictionary(r))
            .and_then(|d| d.get(b"Title"))
            .expect("a Title");
        assert_eq!(
            lopdf::decode_text_string(info).expect("a text string"),
            "bell rings"
        );
        let packet = String::from_utf8_lossy(&bytes);
        assert!(packet.contains("x-default\">bell rings</rdf:li>"));
    }

    /// The PRD example's radar beside a legend of 300 items on `paper`:
    /// several pages.
    fn paper_pages(paper: Paper) -> Vec<Document> {
        use crate::layout::legend::LegendBlock;
        use crate::layout::paper::{Sheets, bodies, footed, page_size};
        let (catalogue, order) = crate::layout::test_support::long(1, 1, 300);
        let legend = LegendBlock::on_paper(&catalogue, &order, fonts());
        let (prd_catalogue, prd_order) = plotted(PRD_EXAMPLE, None);
        let blocks = Blocks::new(&prd_catalogue, &prd_order, None, None, fonts());
        let sheets = Sheets {
            radar: &blocks.radar,
            key: &blocks.key,
            legend: &legend,
            title: "Technology Radar",
            subtitle: None,
            size: page_size(paper.points().expect("a paper")),
            table: None,
        };
        footed(bodies(&sheets, fonts()), &sheets, fonts())
    }

    fn paper_pdf(pages: &[Document], theme: Theme) -> Vec<u8> {
        let palette = Palette::built_in(theme);
        export_pages(
            pages,
            &Export {
                palette: &palette,
                fonts: fonts(),
                title: None,
                desc: "Technology radar: 1 sector",
                producer: "radargen 9.9",
            },
        )
        .expect("exports")
    }

    /// `object`'s reference as its number.
    fn number(object: Option<&lopdf::Object>) -> u32 {
        object
            .and_then(|o| o.as_reference().ok())
            .map(|(n, _)| n)
            .expect("a reference")
    }

    /// The object order every PDF takes, light or paper: catalog, page
    /// tree, info, each page and its stream, then the fonts (21c's
    /// document-level objects will follow them).
    #[test]
    fn objects_are_catalog_tree_info_pages_then_fonts() {
        let pages = paper_pages(Paper::Letter);
        assert!(pages.len() >= 3, "{}", pages.len());
        let bytes = paper_pdf(&pages, Theme::Light);
        let doc = lopdf::Document::load_mem(&bytes).expect("parses");
        assert_eq!(number(doc.trailer.get(b"Root").ok()), 1);
        assert_eq!(number(doc.trailer.get(b"Info").ok()), 3);
        let tree = doc.get_dictionary((2, 0)).expect("the page tree");
        let kids: Vec<u32> = tree
            .get(b"Kids")
            .and_then(lopdf::Object::as_array)
            .expect("kids")
            .iter()
            .map(|kid| number(Some(kid)))
            .collect();
        let want: Vec<u32> = (0..pages.len())
            .map(|k| 4 + 2 * u32::try_from(k).expect("few pages"))
            .collect();
        assert_eq!(kids, want);
        for &kid in &kids {
            let page = doc.get_dictionary((kid, 0)).expect("a page");
            assert_eq!(number(page.get(b"Contents").ok()), kid + 1);
        }
        let last_stream = want.last().expect("a page") + 1;
        let fonts: Vec<u32> = doc
            .objects
            .iter()
            .filter(|(_, object)| {
                object.as_dict().is_ok_and(|d| {
                    d.get(b"Type")
                        .and_then(lopdf::Object::as_name)
                        .is_ok_and(|t| t == b"Font")
                })
            })
            .map(|(&(n, _), _)| n)
            .collect();
        assert!(
            !fonts.is_empty() && fonts.iter().all(|&n| n > last_stream),
            "{fonts:?}"
        );
        let light =
            lopdf::Document::load_mem(&pdf(&prd(None), None, None, Theme::Light)).expect("parses");
        assert_eq!(number(light.trailer.get(b"Info").ok()), 3);
    }

    /// FR-111: every page is the paper exactly, with no `/UserUnit`; the
    /// flip is 0.75 and hangs the drawing from the page's top; the ground
    /// fills the `MediaBox`.
    #[test]
    fn a_paper_page_is_the_paper() {
        let bytes = paper_pdf(&paper_pages(Paper::A4), Theme::Dark);
        let doc = lopdf::Document::load_mem(&bytes).expect("parses");
        let pages = doc.get_pages();
        assert!(pages.len() >= 3);
        for &id in pages.values() {
            let dict = doc.get_dictionary(id).expect("a page");
            let mb: Vec<u32> = dict
                .get(b"MediaBox")
                .and_then(lopdf::Object::as_array)
                .expect("a MediaBox")
                .iter()
                .map(|o| o.as_float().expect("a number").to_bits())
                .collect();
            assert_eq!(mb, [0.0_f32, 0.0, 841.89, 595.276].map(f32::to_bits));
            assert!(dict.get(b"UserUnit").is_err());
            let content = doc.get_page_content(id);
            let ops = lopdf::content::Content::decode(&content)
                .expect("decodes")
                .operations;
            let floats = |op: &lopdf::content::Operation| -> Vec<u32> {
                op.operands
                    .iter()
                    .map(|o| o.as_float().expect("a number").to_bits())
                    .collect()
            };
            // The ground: the first path, its points spanning the box.
            let ground: Vec<f32> = ops
                .iter()
                .take_while(|op| op.operator != "f")
                .filter(|op| matches!(op.operator.as_str(), "m" | "l" | "c"))
                .flat_map(|op| op.operands.iter().map(|o| o.as_float().expect("a number")))
                .collect();
            let (xs, ys): (Vec<f32>, Vec<f32>) = ground
                .as_chunks::<2>()
                .0
                .iter()
                .map(|&pair| <(f32, f32)>::from(pair))
                .unzip();
            let span = |v: &[f32]| {
                (
                    v.iter().copied().fold(f32::INFINITY, f32::min).to_bits(),
                    v.iter()
                        .copied()
                        .fold(f32::NEG_INFINITY, f32::max)
                        .to_bits(),
                )
            };
            assert_eq!(span(&xs), (0.0_f32.to_bits(), 841.89_f32.to_bits()));
            assert_eq!(span(&ys), (0.0_f32.to_bits(), 595.276_f32.to_bits()));
            let cm = ops.iter().find(|op| op.operator == "cm").expect("the flip");
            assert_eq!(
                floats(cm),
                [0.75_f32, 0.0, 0.0, -0.75, 0.0, 595.276].map(f32::to_bits)
            );
        }
    }

    /// NFR-4: a paper PDF written twice is the same bytes.
    #[test]
    fn two_paper_exports_are_identical() {
        let pages = paper_pages(Paper::Letter);
        assert!(
            paper_pdf(&pages, Theme::Light) == paper_pdf(&pages, Theme::Light),
            "NFR-4"
        );
    }
}
