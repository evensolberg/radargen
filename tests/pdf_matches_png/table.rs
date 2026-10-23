//! SC-33 (FR-112): every table page of a full PDF, drawn by hayro, draws
//! what the PNG of that page's document draws, under the legend pages'
//! budget; damage on the densest table page fails.

use lopdf::Object;
use radargen::options::{Paper, Table};
use radargen::palette::{Palette, Theme};
use radargen::svg::Document;

use super::paper::{paper_pages, paper_pdf};
use super::{against_page, damaged_page, within_legend_page};

/// The PRD example with 20 more rows of prose: two or three table pages.
fn csv() -> String {
    (0..20).fold(
        include_str!("../fixtures/three-sector.csv").to_owned(),
        |csv, i| {
            csv + &format!(
                "extra {i:02},Platforms,Adopt,\"A first paragraph of prose about extra {i}, long enough to wrap.\n\nA second.\",,data,\"Because it earns its place.\",,,\n"
            )
        },
    )
}

/// The full PDF on `paper` in `theme`: its pages, PDF bytes, palette, and
/// the index of its first table page.
fn on(paper: Paper, theme: Theme) -> (Vec<Document>, Vec<u8>, Palette, usize) {
    let title = Some("Technology Radar");
    let csv = csv();
    let leading = paper_pages(&csv, title, None, paper, None).0.len();
    let (pages, desc) = paper_pages(&csv, title, None, paper, Some(Table::Radar));
    let palette = Palette::built_in(theme);
    let pdf = paper_pdf(&pages, &desc, &palette, title);
    (pages, pdf, palette, leading)
}

fn every_table_page_draws_its_png(paper: Paper, theme: Theme) {
    let (pages, pdf, palette, first) = on(paper, theme);
    assert!(
        pages.len() >= first + 2,
        "{} pages, table from {first}",
        pages.len()
    );
    for (i, page) in pages.iter().enumerate().skip(first) {
        let c = against_page(page, &pdf, i, &palette, None);
        eprintln!(
            "{paper:?} {theme:?} table page {}: {} ppm past noise, worst {}",
            i + 1,
            c.loud_ppm,
            c.worst
        );
        assert!(within_legend_page(&c), "{paper:?} {theme:?} page {}", i + 1);
    }
}

#[test]
fn letter_light_table_pages_draw_their_pngs() {
    every_table_page_draws_its_png(Paper::Letter, Theme::Light);
}

#[test]
fn letter_dark_table_pages_draw_their_pngs() {
    every_table_page_draws_its_png(Paper::Letter, Theme::Dark);
}

#[test]
fn a4_light_table_pages_draw_their_pngs() {
    every_table_page_draws_its_png(Paper::A4, Theme::Light);
}

#[test]
fn a4_dark_table_pages_draw_their_pngs() {
    every_table_page_draws_its_png(Paper::A4, Theme::Dark);
}

/// The first table page, the densest, of the Letter light PDF with `edit`
/// applied to its operators, against its PNG: whether it passes.
fn damaged_table_page(edit: impl FnOnce(&mut Vec<lopdf::content::Operation>)) -> bool {
    let (pages, pdf, palette, first) = on(Paper::Letter, Theme::Light);
    let doc = lopdf::Document::load_mem(&pdf).expect("parses");
    let number = u32::try_from(first + 1).expect("few pages");
    let id = *doc.get_pages().get(&number).expect("the first table page");
    let broken = damaged_page(&pdf, id, edit);
    let c = against_page(
        pages.get(first).expect("a table page"),
        &broken,
        first,
        &palette,
        None,
    );
    eprintln!(
        "damaged table page: {} ppm past noise, worst {}",
        c.loud_ppm, c.worst
    );
    within_legend_page(&c)
}

/// Every text a fifth of a pixel to the right fails.
#[test]
fn text_a_fifth_of_a_pixel_off_fails_on_a_table_page() {
    assert!(!damaged_table_page(|ops| {
        for tm in ops.iter_mut().filter(|op| op.operator == "Tm") {
            let x = tm.operands.get_mut(4).expect("a Tm's x");
            *x = Object::Real(x.as_float().expect("a number") + 0.2);
        }
    }));
}

/// Every text colour 0.08 (20 levels) off fails.
#[test]
fn a_slightly_wrong_text_colour_fails_on_a_table_page() {
    assert!(!damaged_table_page(|ops| {
        let texts: Vec<usize> = ops
            .iter()
            .enumerate()
            .filter(|(_, op)| op.operator == "BT")
            .map(|(i, _)| i)
            .collect();
        assert_ne!(texts.len(), 0, "a table page has text");
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

/// The rules dropped (every stroke) fails: they are the table's new
/// geometry.
#[test]
fn a_dropped_rule_fails_on_a_table_page() {
    assert!(!damaged_table_page(|ops| {
        let strokes = ops.iter().filter(|op| op.operator == "S").count();
        assert_ne!(strokes, 0, "a table page has rules");
        ops.retain(|op| op.operator != "S");
    }));
}
