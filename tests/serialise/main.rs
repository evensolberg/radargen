//! Whole documents through the serialiser (PR 13): the vocabulary, late
//! binding as resvg sees it, identity by CSV line, hostile text, and a
//! golden file per target, and the minimal HTML page around it (NFR-4,
//! NFR-5, NFR-6, FR-54, architecture §5).

#[path = "../common/catalogue.rs"]
mod catalogue;
#[path = "../common/fonts.rs"]
mod fonts;
#[path = "../common/html.rs"]
mod html;
#[path = "../common/long_legend.rs"]
mod long_legend;
#[path = "../common/palette.rs"]
mod palette;
#[path = "../common/paper.rs"]
mod paper;
#[path = "../common/pdf_export.rs"]
mod pdf_export;
#[path = "../common/pdf.rs"]
mod pdf_text;
#[path = "../common/version.rs"]
mod version;

mod colours;
mod export;
mod golden;
mod hostile;
mod html5;
mod identity;
mod page;
mod palette_page;
mod pdf;
mod script;
mod table;
mod vocabulary;

use radargen::emit::html::{Page, page};
use radargen::layout::arrange::{
    Arrangement, Blocks, NarrowParts, WideParts, narrow_parts, wide_parts,
};
use radargen::model::Catalogue;
use radargen::options::{Legend, Table};
use radargen::order::{Plotted, resolve};
use radargen::palette::{Colours, Palette, Theme};
use radargen::svg::Target;
use radargen::text::Fonts;

const PRD_EXAMPLE: &str = include_str!("../fixtures/three-sector.csv");
const BOTH: [Arrangement; 2] = [Arrangement::Wide, Arrangement::Narrow];

/// A catalogue, its order and its blocks.
struct Built {
    catalogue: Catalogue,
    plotted: Plotted,
    blocks: Blocks,
}

impl Built {
    /// The page for `wide` and `narrow`, with this catalogue's table. The
    /// table is `--table all`'s, so the tests of every row and column read
    /// the whole catalogue; [`Built::page_table`] chooses.
    fn page_of(&self, wide: &WideParts, narrow: &NarrowParts, title: Option<&str>) -> String {
        self.page_in(
            wide,
            narrow,
            title,
            &Colours::BuiltIn(Theme::Light),
            Legend::Shown,
            Table::All,
        )
    }

    /// As [`Built::page_of`], in `colours`, the legend starting as `legend`
    /// and the table publishing `table`.
    fn page_in(
        &self,
        wide: &WideParts,
        narrow: &NarrowParts,
        title: Option<&str>,
        colours: &Colours,
        legend: Legend,
        table: Table,
    ) -> String {
        page(&Page {
            wide,
            narrow,
            catalogue: &self.catalogue,
            plotted: &self.plotted,
            title,
            colours,
            legend,
            table,
        })
    }

    /// The page for both arrangements of these blocks, titled `title`, its
    /// table publishing `table`.
    fn page_table(&self, title: Option<&str>, table: Table) -> String {
        self.page_in(
            &wide_parts(&self.blocks),
            &narrow_parts(&self.blocks),
            title,
            &Colours::BuiltIn(Theme::Light),
            Legend::Shown,
            table,
        )
    }

    /// The page for both arrangements of these blocks, its legend starting
    /// as `legend`.
    fn page_legend(&self, legend: Legend) -> String {
        self.page_in(
            &wide_parts(&self.blocks),
            &narrow_parts(&self.blocks),
            None,
            &Colours::BuiltIn(Theme::Light),
            legend,
            Table::All,
        )
    }

    /// The page for both arrangements of these blocks, in `colours`.
    fn page_with(&self, colours: &Colours) -> String {
        self.page_in(
            &wide_parts(&self.blocks),
            &narrow_parts(&self.blocks),
            None,
            colours,
            Legend::Shown,
            Table::All,
        )
    }

    /// The page for both arrangements of these blocks.
    fn page(&self, title: Option<&str>) -> String {
        self.page_of(
            &wide_parts(&self.blocks),
            &narrow_parts(&self.blocks),
            title,
        )
    }
}

/// `csv` with `--rings` and `--title`, laid out.
fn build(csv: &str, rings: Option<&str>, title: Option<&str>) -> Built {
    let catalogue = catalogue::catalogue(csv, rings);
    let plotted = resolve(&catalogue);
    let fonts = Fonts::embedded().expect("the vendored fonts parse");
    let blocks = Blocks::new(&catalogue, &plotted, title, None, &fonts);
    Built {
        catalogue,
        plotted,
        blocks,
    }
}

fn prd() -> Built {
    build(PRD_EXAMPLE, None, Some("Technology Radar"))
}

fn parse(markup: &str) -> roxmltree::Document<'_> {
    roxmltree::Document::parse(markup).unwrap_or_else(|e| panic!("{e}\n{markup}"))
}

/// Both targets, and whether each is interactive.
const fn targets(palette: &Palette) -> [(Target<'_>, bool); 2] {
    [
        (Target::Interactive(palette), true),
        (Target::Static(palette), false),
    ]
}
