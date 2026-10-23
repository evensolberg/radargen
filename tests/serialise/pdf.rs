//! The exported PDF's goldens (FR-110, NFR-4), as text through
//! [`readable`], light and dark as the SVG's are.

use radargen::layout::arrange::{Arrangement, arrange};
use radargen::palette::{Palette, Theme};

use radargen::options::{Paper, Table};

use super::golden::golden;
use super::long_legend::long_csv;
use super::paper::{paper_pages, paper_pdf};
use super::pdf_export::{PRODUCER, pdf};
use super::pdf_text::readable;
use super::prd;
use super::version::PLACEHOLDER;

/// The PRD example's CSV.
const PRD: &str = include_str!("../fixtures/three-sector.csv");

/// The PRD example as a PDF in `theme`, its producer the placeholder
/// (`pdf_export`'s), so a minor bump changes no byte.
fn prd_pdf(theme: Theme) -> String {
    let built = prd();
    let doc = arrange(Arrangement::Wide, &built.blocks);
    readable(&pdf(
        &doc,
        &built.blocks.radar.name(),
        &Palette::built_in(theme),
        Some("Technology Radar"),
        None,
    ))
}

/// The PRD example as a light PDF. One test per golden, since a blessing
/// run stops at the first.
#[test]
fn the_producer_is_the_version_placeholder() {
    assert_eq!(PRODUCER, PLACEHOLDER);
}

#[test]
fn prd_pdf_matches_its_golden() {
    golden("pdf", "prd-light.pdf.txt", &prd_pdf(Theme::Light));
}

/// The same, dark (FR-56).
#[test]
fn prd_dark_pdf_matches_its_golden() {
    golden("pdf", "prd-dark.pdf.txt", &prd_pdf(Theme::Dark));
}

/// The PRD example on Letter (FR-111, SC-31): the paper pages' golden.
#[test]
fn prd_paper_pdf_matches_its_golden() {
    let title = Some("Technology Radar");
    let (pages, desc) = paper_pages(
        include_str!("../fixtures/three-sector.csv"),
        title,
        None,
        Paper::Letter,
        None,
    );
    let pdf = paper_pdf(&pages, &desc, &Palette::built_in(Theme::Light), title);
    golden("pdf", "prd-paper.pdf.txt", &readable(&pdf));
}

/// The long legend on `paper`, subtitled, as text: pins the continuation
/// pages, their openers and grouping before PR 21b's refactors (RG-021B).
fn long_paper(paper: Paper) -> String {
    let title = Some("Technology Radar");
    let (pages, desc) = paper_pages(&long_csv(), title, Some("Edition 1"), paper, None);
    assert!(pages.len() >= 3, "{}", pages.len());
    readable(&paper_pdf(
        &pages,
        &desc,
        &Palette::built_in(Theme::Light),
        title,
    ))
}

#[test]
fn prd_long_paper_letter_matches_its_golden() {
    golden(
        "pdf",
        "prd-long-paper-letter.pdf.txt",
        &long_paper(Paper::Letter),
    );
}

#[test]
fn prd_long_paper_a4_matches_its_golden() {
    golden("pdf", "prd-long-paper-a4.pdf.txt", &long_paper(Paper::A4));
}

/// `csv` on `paper` with `table`'s table, titled, in the light theme.
fn on_paper(csv: &str, paper: Paper, table: Option<Table>) -> Vec<u8> {
    let title = Some("Technology Radar");
    let (pages, desc) = paper_pages(csv, title, None, paper, table);
    paper_pdf(&pages, &desc, &Palette::built_in(Theme::Light), title)
}

/// The PRD example as a full PDF on Letter (FR-112, SC-33): the leading
/// pages, then the table's.
#[test]
fn prd_full_pdf_matches_its_golden() {
    let pdf = on_paper(PRD, Paper::Letter, Some(Table::Radar));
    golden("pdf", "prd-full.pdf.txt", &readable(&pdf));
}

/// rg-p82 (QA 11): a sector label too wide for the radar to sit beside the
/// legend: the radar takes page 1 alone and the legend starts on page 2
/// (`paper::tests::a_wide_sector_starts_the_legend_on_page_two` pins the
/// placement; this pins the bytes).
#[test]
fn prd_legend_after_page_one_matches_its_golden() {
    let title = Some("Technology Radar");
    let csv = format!(
        "name,sector,ring,description\na,{},Adopt,d\nb,Tools,Trial,d\nc,Data,Assess,d\n",
        "S".repeat(64)
    );
    let (pages, desc) = paper_pages(&csv, title, None, Paper::Letter, None);
    assert!(pages.len() >= 2, "{}", pages.len());
    let pdf = paper_pdf(&pages, &desc, &Palette::built_in(Theme::Light), title);
    golden("pdf", "prd-legend-after-page-one.pdf.txt", &readable(&pdf));
}

/// rg-p82 (QA 9): the PRD example as a full PDF on A4.
#[test]
fn prd_full_a4_pdf_matches_its_golden() {
    let pdf = on_paper(PRD, Paper::A4, Some(Table::Radar));
    golden("pdf", "prd-full-a4.pdf.txt", &readable(&pdf));
}

/// `csv` on `paper` with `table`'s table, its structure tree as a golden
/// a person can review in a diff (FR-114).
fn tree_golden(name: &str, csv: &str, paper: Paper, table: Option<Table>) {
    let pdf = on_paper(csv, paper, table);
    golden(
        "pdf",
        &format!("{name}.tree.txt"),
        &super::pdf_tree::tree_outline(&pdf),
    );
}

#[test]
fn the_full_pdfs_tree_matches_its_golden() {
    tree_golden("prd-full", PRD, Paper::Letter, Some(Table::Radar));
}

#[test]
fn the_long_legends_tree_matches_its_golden() {
    tree_golden("prd-long-paper-a4", &long_csv(), Paper::A4, None);
}

/// A row split across pages: one `TR`, its cells on two pages.
#[test]
fn a_split_rows_tree_matches_its_golden() {
    let words: Vec<String> = (0..3_000).map(|i| format!("w{i}")).collect();
    let csv = format!(
        "name,sector,ring,description\nAlpha,Tools,Adopt,d\nBeta,Tools,Adopt,{}\nGamma,Tools,Trial,d\n",
        words.join(" ")
    );
    tree_golden("split-row", &csv, Paper::Letter, Some(Table::Radar));
}
