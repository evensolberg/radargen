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

mod colours;
mod golden;
mod hostile;
mod html5;
mod identity;
mod page;
mod vocabulary;

use radargen::emit::html::{Page, page};
use radargen::layout::arrange::{Arrangement, Blocks, arrange};
use radargen::model::Catalogue;
use radargen::order::{Plotted, resolve};
use radargen::palette::Palette;
use radargen::svg::Document;
use radargen::svg::Target;
use radargen::text::Fonts;

const PRD_EXAMPLE: &str = include_str!("../fixtures/three-quadrant.csv");
const BOTH: [Arrangement; 2] = [Arrangement::Wide, Arrangement::Narrow];

/// A catalogue, its order and its blocks.
struct Built {
    catalogue: Catalogue,
    plotted: Plotted,
    blocks: Blocks,
}

impl Built {
    /// The page for `wide` and `narrow`, with this catalogue's table.
    fn page_of(&self, wide: &Document, narrow: &Document, title: Option<&str>) -> String {
        page(&Page {
            wide,
            narrow,
            catalogue: &self.catalogue,
            plotted: &self.plotted,
            title,
        })
    }

    /// The page for both arrangements of these blocks.
    fn page(&self, title: Option<&str>) -> String {
        self.page_of(
            &arrange(Arrangement::Wide, &self.blocks),
            &arrange(Arrangement::Narrow, &self.blocks),
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
