//! Shared by the PDF suites that look at paper (FR-111): a CSV laid out
//! on paper pages as the pipeline lays it out, and those pages as a PDF.

use radargen::emit::UNTITLED;
use radargen::emit::pdf::{Export, export_pages};
use radargen::layout::arrange::Blocks;
use radargen::layout::legend::LegendBlock;
use radargen::layout::paper::{Sheets, bodies, footed, page_size};
use radargen::options::{Paper, Table};
use radargen::order::resolve;
use radargen::palette::Palette;
use radargen::published::published;
use radargen::svg::Document;

use super::catalogue::catalogue;
use super::pdf_export::{PRODUCER, fonts};

/// `csv` on `paper`, titled `title` (or *Technology Radar*, D-105) with
/// `subtitle`, with `table`'s table after it for a full PDF (FR-112): the
/// pages, and the radar's name for `/Subject`.
pub fn paper_pages(
    csv: &str,
    title: Option<&str>,
    subtitle: Option<&str>,
    paper: Paper,
    table: Option<Table>,
) -> (Vec<Document>, String) {
    let catalogue = catalogue(csv, None);
    let plotted = resolve(&catalogue);
    let blocks = Blocks::new(&catalogue, &plotted, title, subtitle, fonts());
    let legend = LegendBlock::on_paper(&catalogue, &plotted, fonts());
    let published = table.map(|table| published(&catalogue, &plotted, table));
    let sheets = Sheets {
        radar: &blocks.radar,
        key: &blocks.key,
        legend: &legend,
        title: title.unwrap_or(UNTITLED),
        subtitle,
        size: page_size(paper.points().expect("a paper")),
        table: published.as_ref(),
    };
    let pages = footed(bodies(&sheets, fonts()), &sheets, fonts());
    (pages, blocks.radar.name())
}

/// `pages` as a PDF in `palette`, `/Title` `title`.
pub fn paper_pdf(
    pages: &[Document],
    desc: &str,
    palette: &Palette,
    title: Option<&str>,
) -> Vec<u8> {
    export_pages(
        pages,
        &Export {
            palette,
            fonts: fonts(),
            title,
            desc,
            producer: PRODUCER,
        },
    )
    .expect("exports")
}
