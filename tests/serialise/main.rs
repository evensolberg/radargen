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
mod identity;
mod page;
mod vocabulary;

use radargen::layout::arrange::{Arrangement, Blocks};
use radargen::layout::key::KeyBlock;
use radargen::layout::legend::LegendBlock;
use radargen::layout::radar::RadarBlock;
use radargen::layout::title::TitleBlock;
use radargen::model::Catalogue;
use radargen::order::{Plotted, resolve};
use radargen::palette::Palette;
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

/// `csv` with `--rings` and `--title`, laid out.
fn build(csv: &str, rings: Option<&str>, title: Option<&str>) -> Built {
    let catalogue = catalogue::catalogue(csv, rings);
    let plotted = resolve(&catalogue);
    let fonts = Fonts::embedded().expect("the vendored fonts parse");
    let blocks = Blocks {
        title: TitleBlock::new(title, None, &fonts),
        radar: RadarBlock::new(&catalogue, &plotted, &fonts),
        key: KeyBlock::new(&fonts),
        legend: LegendBlock::new(&catalogue, &plotted, &fonts),
    };
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
