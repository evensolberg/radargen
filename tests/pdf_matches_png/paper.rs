//! SC-31 (FR-111): every paper page, drawn by hayro, draws what the PNG
//! of that page's document draws; damage on a continuation page fails;
//! the ground fills every page to its edges.

use lopdf::Object;
use radargen::emit::png::{pixel_size, rasterise};
use radargen::options::Paper;
use radargen::palette::{Palette, Theme};
use radargen::svg::Document;
use resvg::tiny_skia::Pixmap;

use super::long_legend::long_csv;
use super::paper::{paper_pages, paper_pdf};
use super::{against_page, damaged_page, drawn_page, within, within_legend_page};

/// The long legend on `paper` in `theme`: pages, PDF, palette.
fn on(paper: Paper, theme: Theme) -> (Vec<Document>, Vec<u8>, Palette) {
    let title = Some("Technology Radar");
    let (pages, desc) = paper_pages(&long_csv(), title, Some("Edition 1"), paper);
    let palette = Palette::built_in(theme);
    let pdf = paper_pdf(&pages, &desc, &palette, title);
    (pages, pdf, palette)
}

/// Every page against its PNG: page 1 under the wide arrangement's gate,
/// the legend pages under their own budget.
fn every_page_draws_its_png(paper: Paper, theme: Theme) {
    let (pages, pdf, palette) = on(paper, theme);
    let hayro = hayro::hayro_syntax::Pdf::new(pdf.clone()).expect("hayro parses the PDF");
    assert_eq!(hayro.pages().len(), pages.len(), "a page per document");
    assert!(pages.len() >= 3, "{}", pages.len());
    for (i, page) in pages.iter().enumerate() {
        let c = against_page(page, &pdf, i, &palette, None);
        eprintln!(
            "{paper:?} {theme:?} page {}: {} ppm past noise, worst {}",
            i + 1,
            c.loud_ppm,
            c.worst
        );
        let passes = if i == 0 {
            within(&c)
        } else {
            within_legend_page(&c)
        };
        assert!(passes, "{paper:?} {theme:?} page {}", i + 1);
    }
}

#[test]
fn letter_light_pages_draw_their_pngs() {
    every_page_draws_its_png(Paper::Letter, Theme::Light);
}

#[test]
fn letter_dark_pages_draw_their_pngs() {
    every_page_draws_its_png(Paper::Letter, Theme::Dark);
}

#[test]
fn a4_light_pages_draw_their_pngs() {
    every_page_draws_its_png(Paper::A4, Theme::Light);
}

#[test]
fn a4_dark_pages_draw_their_pngs() {
    every_page_draws_its_png(Paper::A4, Theme::Dark);
}

/// The second page, a continuation, of the long legend on Letter, light,
/// with `edit` applied to its operators, against its PNG.
fn damaged_continuation(edit: impl FnOnce(&mut Vec<lopdf::content::Operation>)) -> bool {
    let (pages, pdf, palette) = on(Paper::Letter, Theme::Light);
    let doc = lopdf::Document::load_mem(&pdf).expect("parses");
    let second = *doc.get_pages().get(&2).expect("a second page");
    let broken = damaged_page(&pdf, second, edit);
    within_legend_page(&against_page(
        pages.get(1).expect("a second page"),
        &broken,
        1,
        &palette,
        None,
    ))
}

/// Every text a fifth of a pixel to the right on a continuation page
/// fails, as it does on the wide arrangement.
#[test]
fn text_a_fifth_of_a_pixel_off_fails_on_paper() {
    assert!(!damaged_continuation(|ops| {
        for tm in ops.iter_mut().filter(|op| op.operator == "Tm") {
            let x = tm.operands.get_mut(4).expect("a Tm's x");
            *x = Object::Real(x.as_float().expect("a number") + 0.2);
        }
    }));
}

/// Every text colour 0.08 (20 levels) off on a continuation page fails.
#[test]
fn a_slightly_wrong_text_colour_fails_on_paper() {
    assert!(!damaged_continuation(|ops| {
        let texts: Vec<usize> = ops
            .iter()
            .enumerate()
            .filter(|(_, op)| op.operator == "BT")
            .map(|(i, _)| i)
            .collect();
        assert!(!texts.is_empty(), "a continuation page has text");
        for bt in texts {
            let fill = (0..bt)
                .rev()
                .find(|&i| ops.get(i).is_some_and(|op| op.operator == "rg"));
            if let Some(op) = fill.and_then(|i| ops.get_mut(i)) {
                let red = op.operands.first_mut().expect("a channel");
                let v = red.as_float().expect("a number");
                *red = Object::Real(if v > 0.5 { v - 0.08 } else { v + 0.08 });
            }
        }
    }));
}

/// Review Focus 4: on dark A4, every page's four corners are the ground
/// in the PDF as in the page's PNG: the ground fills the `MediaBox`.
#[test]
fn the_ground_fills_every_page_to_its_edges() {
    let (pages, pdf, palette) = on(Paper::A4, Theme::Dark);
    for (i, page) in pages.iter().enumerate() {
        let size = pixel_size(page, None).expect("fits");
        let png =
            Pixmap::decode_png(&rasterise(page, &palette, size).expect("renders")).expect("a PNG");
        let drawn = drawn_page(&pdf, i, size, (page.width, page.height));
        let (w, h) = (size.width(), size.height());
        for (x, y) in [(0, 0), (w - 1, 0), (0, h - 1), (w - 1, h - 1)] {
            let at = usize::try_from((y * w + x) * 4).expect("fits");
            let (a, b) = (png.data().get(at..at + 4), drawn.get(at..at + 4));
            assert!(
                a.zip(b)
                    .is_some_and(|(a, b)| a.iter().zip(b).all(|(p, q)| p.abs_diff(*q) <= 3)),
                "page {} corner {x},{y}: PNG {a:?}, PDF {b:?}",
                i + 1
            );
        }
    }
}
