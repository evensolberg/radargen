//! A PDF on paper (FR-111, D-105).
//!
//! Landscape Letter or A4 pages, each an arranged document the page's
//! size. The first page holds the title,
//! the radar, the key and the legend's first piece; the legend continues
//! on the pages after it; [`footed`] adds each page's footer once the
//! page count is known.

use std::num::NonZeroUsize;

use crate::layout::arrange::{Rule, fit_scale, rule_node};
use crate::layout::block::{Block, Placement};
use crate::layout::geometry::real;
use crate::layout::key::KeyBlock;
use crate::layout::legend::LegendBlock;
use crate::layout::radar::RadarBlock;
use crate::layout::table::TableBlock;
use crate::layout::title::{Wrap, cut};
use crate::palette::Neutral;
use crate::published::Published;
use crate::style::{
    COLUMN_GAP, FOOTER, FOOTER_BASELINE, LEGEND_WIDE_DROP, LINE_HEIGHT, NARROW_MIN_SCALE,
    PAPER_FOOT_ROOM, PAPER_LEGEND_SCALE, PAPER_MARGIN, POINTS_PER_PX, RADAR_LABEL_PAD,
    TABLE_POINTER_ABOVE,
};
use crate::svg::{Anchor, Document, Group, Node, Point, Scale, Semantics, Structure, TextRun};
use crate::text::{Fonts, words};

/// Between the footer's title and subtitle.
const FOOTER_SEPARATOR: &str = " \u{b7} ";

/// What the paper pages are made of.
#[derive(Debug, Clone, Copy)]
pub struct Sheets<'a> {
    /// The radar, grown as every arrangement has it (D-86).
    pub radar: &'a RadarBlock,
    /// The encoding key (FR-66).
    pub key: &'a KeyBlock,
    /// The legend as paper lays it out ([`LegendBlock::on_paper`]).
    pub legend: &'a LegendBlock,
    /// The title drawn: FR-65's, or *Technology Radar* (D-105).
    pub title: &'a str,
    /// FR-65's subtitle, if one shows.
    pub subtitle: Option<&'a str>,
    /// The page in px, width first ([`page_size`]).
    pub size: (f64, f64),
    /// The full PDF's table (FR-112); `None` for the radar PDF.
    pub table: Option<&'a Published<'a>>,
}

/// A paper's landscape `points`, width first, as a page in px.
#[must_use]
pub fn page_size(points: (f64, f64)) -> (f64, f64) {
    (points.0 / POINTS_PER_PX, points.1 / POINTS_PER_PX)
}

/// What a page holds (FR-111, FR-112).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageKind {
    /// Page 1: the title, radar, key and the legend's start.
    Front,
    /// A page of the legend continued.
    Legend,
    /// A page of the full PDF's table.
    Table,
}

/// The pages' contents, before their footers.
#[derive(Debug, Clone, PartialEq)]
pub struct Bodies {
    /// Each page's nodes, page 1 first, in reading order.
    pub pages: Vec<Vec<Node>>,
    /// What each page holds, one per page.
    pub kinds: Vec<PageKind>,
    /// When the table's prose columns are narrower than they should be
    /// even with its label columns at their floor (FR-112), its drawn
    /// columns, the number column included.
    pub table_cramped: Option<usize>,
    /// Each table row split across pages: its CSV line and name.
    pub tall_table_rows: Vec<(usize, String)>,
    /// Each legend row taller than a page: its CSV line and name.
    pub tall_legend_rows: Vec<(usize, String)>,
    /// Whether the radar, at its smallest, still runs past the first
    /// page's margins: labels never shrink, so a long enough sector name
    /// can be wider than the page.
    pub radar_too_wide: bool,
}

/// Where page 1's parts go.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct FirstPage {
    /// The radar, at the scale that fits.
    pub radar: Placement,
    /// The rule over the key; the key hangs from its start, at scale 1.
    pub rule: Rule,
    /// The key's columns.
    pub key_columns: NonZeroUsize,
    /// The legend's first piece, at `PAPER_LEGEND_SCALE`; `None` when the
    /// legend is empty or starts on page 2.
    pub legend: Option<Placement>,
    /// Whether the radar fits at the scale it has.
    pub fits: bool,
}

/// The page's content box, in px: the margins inset, and the footer's room
/// at the bottom.
#[derive(Debug, Clone, Copy)]
struct Content {
    width: f64,
    height: f64,
}

impl Content {
    fn of(sheets: &Sheets<'_>) -> Self {
        let (w, h) = sheets.size;
        Self {
            width: w - PAPER_MARGIN - PAPER_MARGIN,
            height: h - PAPER_MARGIN - PAPER_FOOT_ROOM,
        }
    }
}

/// The title block for `sheets`, wrapped across the content box.
fn title(sheets: &Sheets<'_>, fonts: &Fonts) -> Wrap {
    Wrap::capped(
        Some(sheets.title),
        sheets.subtitle,
        Content::of(sheets).width,
        PAPER_TITLE_LINES,
        fonts,
    )
}

/// The most lines a title and a subtitle take on paper (rg-i95): enough
/// for any real title, short of pushing the radar off its page.
const PAPER_TITLE_LINES: (usize, usize) = (3, 2);

/// One legend piece's width on the page.
fn piece_width(sheets: &Sheets<'_>, fonts: &Fonts) -> f64 {
    if sheets.legend.size(1.0).is_empty() {
        0.0
    } else {
        sheets.legend.paper_width(fonts) * PAPER_LEGEND_SCALE
    }
}

/// The radar's column: as wide as the page leaves beside the legend (or
/// the whole content box), widened by the unpainted label pad on each
/// side, and as tall as the title leaves.
fn column(sheets: &Sheets<'_>, beside: bool, fonts: &Fonts) -> (f64, f64) {
    let content = Content::of(sheets);
    let legend = if beside {
        piece_width(sheets, fonts) + COLUMN_GAP
    } else {
        0.0
    };
    let title = title(sheets, fonts).size(1.0).height;
    (
        content.width - legend + RADAR_LABEL_PAD + RADAR_LABEL_PAD,
        content.height - title,
    )
}

/// The largest scale, down to the narrow arrangement's floor (FR-70), at
/// which the radar fits `column` with a key under it, for any of the key's
/// column counts; `None` when none does.
///
/// For a fixed count the fit holds from the floor up to some largest
/// scale, so that scale is found by halving; the key's own width must
/// then fit under the radar's ink there, since ink only narrows below it.
fn largest(sheets: &Sheets<'_>, (width, height): (f64, f64)) -> Option<f64> {
    let least = least(sheets);
    (1..=5)
        .filter_map(NonZeroUsize::new)
        .filter_map(|columns| {
            let key = sheets.key.grid(columns).size(1.0);
            let fits = |s: f64| {
                let radar = sheets.radar.size(s);
                radar.width <= width && radar.height + key.height <= height
            };
            let s = fit_scale(least, fits);
            let ink = sheets.radar.size(s).width - RADAR_LABEL_PAD - RADAR_LABEL_PAD;
            (fits(s) && key.width <= ink).then_some(s)
        })
        .reduce(f64::max)
}

/// The radar's smallest scale on paper: the narrow arrangement's floor
/// (FR-70), lower for a grown radar (D-86).
fn least(sheets: &Sheets<'_>) -> f64 {
    NARROW_MIN_SCALE / sheets.radar.growth()
}

/// Page 1's placements: the radar beside the legend's first piece when
/// it fits there, else across the whole page with the legend starting on
/// page 2, else at its floor, marked as not fitting.
pub(crate) fn first_page(sheets: &Sheets<'_>, fonts: &Fonts) -> FirstPage {
    let has_legend = !sheets.legend.size(1.0).is_empty();
    let beside_col = column(sheets, true, fonts);
    let beside_fit = has_legend.then(|| largest(sheets, beside_col)).flatten();
    let beside = beside_fit.is_some();
    let col = if beside {
        beside_col
    } else {
        column(sheets, false, fonts)
    };
    let found = beside_fit.or_else(|| largest(sheets, col));
    let s = found.unwrap_or_else(|| least(sheets));
    let radar = sheets.radar.size(s);
    let ink = radar.width - RADAR_LABEL_PAD - RADAR_LABEL_PAD;
    let key_columns = sheets.key.columns_within(ink);
    let (page_width, _) = sheets.size;
    let t = title(sheets, fonts).size(1.0).height;
    let x = (PAPER_MARGIN + (col.0 - radar.width) / 2.0 - RADAR_LABEL_PAD)
        .max(PAPER_MARGIN - RADAR_LABEL_PAD);
    let top = PAPER_MARGIN + t;
    let key_y = top + radar.height;
    FirstPage {
        radar: Placement {
            origin: Point { x, y: top },
            scale: s,
        },
        rule: Rule {
            from: Point {
                x: x + RADAR_LABEL_PAD,
                y: key_y,
            },
            to: Point {
                x: x + radar.width - RADAR_LABEL_PAD,
                y: key_y,
            },
        },
        key_columns,
        legend: beside.then(|| Placement {
            origin: Point {
                x: page_width - PAPER_MARGIN - piece_width(sheets, fonts),
                y: top + LEGEND_WIDE_DROP,
            },
            scale: PAPER_LEGEND_SCALE,
        }),
        fits: found.is_some(),
    }
}

/// How many pieces sit side by side on a continuation page: as many as
/// fit across the content box, at least one.
fn per_page(sheets: &Sheets<'_>, fonts: &Fonts) -> usize {
    let width = piece_width(sheets, fonts);
    let room = Content::of(sheets).width;
    // Each further column's right edge, while it is inside the room.
    std::iter::successors(Some(width), |used| Some(used + COLUMN_GAP + width))
        .skip(1)
        .take_while(|&used| used <= room)
        .count()
        + 1
}

/// Which page each legend piece lands on (rg-mi2): decided once, so the
/// pieces that open a page and the pages' columns cannot disagree.
#[derive(Debug, Clone, Copy)]
struct PieceSlots {
    /// The first piece sits beside the radar on page 1.
    on_first: bool,
    /// Pieces side by side on each later page.
    per_page: usize,
}

impl PieceSlots {
    /// Piece `k`'s page, 0 being page 1.
    const fn page_of(self, k: usize) -> usize {
        match (self.on_first, k) {
            (true, 0) => 0,
            (true, k) => 1 + (k - 1) / self.per_page,
            (false, k) => 1 + k / self.per_page,
        }
    }

    /// Whether piece `k` is the first on its page: such a piece opens
    /// with *Legend (continued)*, unless it starts the legend.
    const fn opens_page(self, k: usize) -> bool {
        match k {
            0 => !self.on_first,
            k => self.page_of(k) != self.page_of(k - 1),
        }
    }
}

/// Every page's content, before footers (FR-111).
#[must_use]
pub fn bodies(sheets: &Sheets<'_>, fonts: &Fonts) -> Bodies {
    let first = first_page(sheets, fonts);
    let content = Content::of(sheets);
    let n = per_page(sheets, fonts);
    let t = title(sheets, fonts).size(1.0).height;
    let slots = PieceSlots {
        on_first: first.legend.is_some(),
        per_page: n,
    };
    // Every piece hangs where page 1's legend does, below the title's
    // height, so every piece has the same room (the maintainer, 2026-10-23).
    let room = (content.height - t - LEGEND_WIDE_DROP).max(0.0) / PAPER_LEGEND_SCALE;
    let pieces = sheets
        .legend
        .flowed(room, room, &|k| slots.opens_page(k), fonts);
    // rg-i95: a piece taller than its room holds a row that could not fit
    // even alone, so it runs into the foot room.
    let tall_legend_rows: Vec<(usize, String)> = pieces
        .iter()
        .enumerate()
        .filter(|(_, piece)| piece.size(1.0).height > room)
        .flat_map(|(_, piece)| piece.rows())
        .collect();
    let mut pages = legend_pages(sheets, &first, pieces, slots, fonts);
    let mut kinds = vec![PageKind::Front];
    kinds.resize(pages.len(), PageKind::Legend);
    // FR-112: the table starts on a page of its own, after the legend's.
    let table = sheets
        .table
        .map(|published| TableBlock::new(published, content.width, content.height, fonts));
    if let Some(table) = &table {
        let origin = Point {
            x: PAPER_MARGIN,
            y: PAPER_MARGIN,
        };
        for page in table.pages() {
            pages.push(table.render(page, origin, fonts));
            kinds.push(PageKind::Table);
        }
    }
    Bodies {
        pages,
        kinds,
        table_cramped: table.as_ref().and_then(TableBlock::cramped),
        tall_table_rows: table
            .as_ref()
            .map(|table| table.tall_rows().to_vec())
            .unwrap_or_default(),
        tall_legend_rows,
        radar_too_wide: !first.fits,
    }
}

/// Page 1 and the legend's continuation pages: each piece on the page
/// `slots` gives it, page 1's beside the radar, the rest side by side.
fn legend_pages(
    sheets: &Sheets<'_>,
    first: &FirstPage,
    pieces: Vec<LegendBlock>,
    slots: PieceSlots,
    fonts: &Fonts,
) -> Vec<Vec<Node>> {
    let mut page_one = vec![
        title(sheets, fonts).render(Placement::at(Point {
            x: PAPER_MARGIN,
            y: PAPER_MARGIN,
        })),
        sheets.radar.render(first.radar),
        rule_node(first.rule),
        sheets
            .key
            .grid(first.key_columns)
            .render(Placement::at(first.rule.from)),
    ];
    let mut later: Vec<Vec<LegendBlock>> = Vec::new();
    for (k, piece) in pieces.into_iter().enumerate() {
        match (slots.page_of(k), first.legend) {
            (0, Some(at)) => page_one.push(piece.render(at)),
            (page, _) => {
                if later.len() < page {
                    later.push(Vec::new());
                }
                if let Some(columns) = later.last_mut() {
                    columns.push(piece);
                }
            }
        }
    }
    let width = piece_width(sheets, fonts);
    let top = legend_top(sheets, fonts);
    let mut pages = vec![page_one];
    pages.extend(
        later
            .iter()
            .map(|columns| continuation(columns, width, sheets, top)),
    );
    pages
}

/// Where every legend piece's panel starts: page 1's, below the title by
/// the wide arrangement's drop.
fn legend_top(sheets: &Sheets<'_>, fonts: &Fonts) -> f64 {
    PAPER_MARGIN + title(sheets, fonts).size(1.0).height + LEGEND_WIDE_DROP
}

/// A continuation page: `pieces` side by side, `width` each, hung from
/// `top`, the last on the right margin: where page 1's legend sits.
#[expect(
    clippy::suboptimal_flops,
    reason = "NFR-4: plain * and + are bit-identical on every target; mul_add falls back to a libm fma where the CPU lacks one"
)]
fn continuation(pieces: &[LegendBlock], width: f64, sheets: &Sheets<'_>, top: f64) -> Vec<Node> {
    let (page_width, _) = sheets.size;
    // `n` columns and the `n − 1` gaps between them, the last ending where
    // page 1's legend ends: on the right margin.
    let n = real(pieces.len());
    let span = width * n + COLUMN_GAP * (n - 1.0);
    let mut x = page_width - PAPER_MARGIN - span;
    pieces
        .iter()
        .map(|piece| {
            let node = piece.render(Placement {
                origin: Point { x, y: top },
                scale: PAPER_LEGEND_SCALE,
            });
            x += width + COLUMN_GAP;
            node
        })
        .collect()
}

/// `bodies` as page documents, each with its footer (FR-111).
///
/// The footer holds the title, and the subtitle when given, at the left,
/// cut to fit, and *Page n of m* at the right. The footer is one group, so
/// a tagged PDF can mark it as a pagination artifact. A full PDF's first
/// page also names the page its table starts on (FR-112).
#[must_use]
pub fn footed(bodies: Bodies, sheets: &Sheets<'_>, fonts: &Fonts) -> Vec<Document> {
    let (width, height) = sheets.size;
    let count = bodies.pages.len();
    let table = bodies
        .kinds
        .iter()
        .position(|kind| *kind == PageKind::Table);
    bodies
        .pages
        .into_iter()
        .enumerate()
        .map(|(i, mut children)| {
            if let Some(start) = table.filter(|_| i == 0) {
                // Drawn in the foot room, read straight after the title
                // and subtitle, before the radar (FR-114): a tagged PDF
                // reads in document order.
                let at = after_title(&children);
                children.insert(at, pointer(start + 1, height));
            }
            children.push(footer(sheets, i + 1, count, fonts));
            Document {
                width,
                height,
                root: Node::Group(Group::new(Point::ORIGIN, children)),
            }
        })
        .collect()
}

/// Where page 1's title block ends among its `children`: just after the
/// child holding the level-1 heading, or the start when there is none.
fn after_title(children: &[Node]) -> usize {
    fn holds(node: &Node) -> bool {
        match node {
            Node::Group(group) => {
                matches!(group.semantics, Semantics::Heading(level) if level.get() == 1)
                    || group.children.iter().any(holds)
            }
            _ => false,
        }
    }
    children.iter().position(holds).map_or(0, |i| i + 1)
}

/// Page 1's line naming `page`, where the table starts (FR-112): in the
/// foot room, above the footer.
fn pointer(page: usize, height: f64) -> Node {
    let text = format!("Each item's description and rationale: page {page} onward.");
    let at = Point {
        x: PAPER_MARGIN,
        y: height - FOOTER_BASELINE - TABLE_POINTER_ABOVE,
    };
    // Information, not a repeat: a paragraph to a tagged PDF (FR-114).
    Node::Group(
        Group::new(Point::ORIGIN, vec![foot_line(&text, at, Anchor::Start)])
            .with(Semantics::Structure(Structure::Paragraph)),
    )
}

/// One muted line in the footer's style at `at`: a footer run, or page 1's
/// pointer.
fn foot_line(text: &str, at: Point, anchor: Anchor) -> Node {
    TextRun {
        origin: at,
        style: FOOTER,
        ink: Neutral::TextMuted,
        anchor,
        lines: vec![text.to_owned()],
        words: words(text),
        leading: LINE_HEIGHT,
    }
    .node(Scale::uniform(1.0))
}

/// Page `page` of `count`'s footer.
fn footer(sheets: &Sheets<'_>, page: usize, count: usize, fonts: &Fonts) -> Node {
    let (width, height) = sheets.size;
    let baseline = height - FOOTER_BASELINE;
    let numbered = format!("Page {page} of {count}");
    // Whitespace folded, as every other text is: a line break or tab a
    // title may hold would draw as a box in one line.
    let named = sheets.subtitle.map_or_else(
        || words(sheets.title),
        |subtitle| {
            format!(
                "{}{FOOTER_SEPARATOR}{}",
                words(sheets.title),
                words(subtitle)
            )
        },
    );
    let room = Content::of(sheets).width - FOOTER.measure(&numbered, fonts) - COLUMN_GAP;
    let named = cut(FOOTER, &named, room, fonts);
    let run =
        |text: &str, x: f64, anchor: Anchor| foot_line(text, Point { x, y: baseline }, anchor);
    Node::Group(
        Group::new(
            Point::ORIGIN,
            vec![
                run(&named, PAPER_MARGIN, Anchor::Start),
                run(&numbered, width - PAPER_MARGIN, Anchor::End),
            ],
        )
        .with(Semantics::Pagination),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::arrange::Blocks;

    /// Whether the radar at `s`, with the key wrapped to its ink, fits the
    /// column `first` was laid out for.
    fn fits(sheets: &Sheets<'_>, first: &FirstPage, s: f64, fonts: &Fonts) -> bool {
        let (width, height) = column(sheets, first.legend.is_some(), fonts);
        let radar = sheets.radar.size(s);
        let ink = radar.width - RADAR_LABEL_PAD - RADAR_LABEL_PAD;
        let key = sheets.key.grid(sheets.key.columns_within(ink)).size(1.0);
        radar.width <= width && radar.height + key.height <= height
    }
    use crate::layout::block::fonts;
    use crate::layout::legend::LegendBlock;
    use crate::layout::test_support::{HEAD, PRD_EXAMPLE, Rect, bounds, plotted};
    use crate::model::Catalogue;
    use crate::options::{Paper, Table};
    use crate::order::Plotted;
    use crate::published::{ALL_HEADING, Published, RADAR_HEADING, published};
    use crate::style::PAPER_MARGIN;
    use crate::svg::{Document, Node};

    /// The PRD example's radar and key, with `legend` beside them.
    struct Fixture {
        blocks: Blocks,
        legend: LegendBlock,
    }

    impl Fixture {
        fn sheets<'a>(
            &'a self,
            paper: Paper,
            title: &'a str,
            subtitle: Option<&'a str>,
        ) -> Sheets<'a> {
            Sheets {
                radar: &self.blocks.radar,
                key: &self.blocks.key,
                legend: &self.legend,
                title,
                subtitle,
                size: page_size(paper.points().expect("a paper")),
                table: None,
            }
        }
    }

    /// `csv`'s radar, legend and catalogue, for a full PDF's pages.
    fn full(csv: &str) -> (Fixture, (Catalogue, Plotted)) {
        let shape = plotted(csv, None);
        let fixture = Fixture {
            blocks: Blocks::new(&shape.0, &shape.1, None, None, fonts()),
            legend: LegendBlock::on_paper(&shape.0, &shape.1, fonts()),
        };
        (fixture, shape)
    }

    /// `n` items with a paragraph or two each, in two sectors.
    fn prose_csv(n: usize) -> String {
        let rows = (0..n)
            .map(|i| {
                let sector = if i < n / 2 { "Tools" } else { "Platforms" };
                format!("item {i:03},{sector},Adopt,\"About item {i}, at some length.\n\nWhy it sits here.\"\n")
            })
            .collect::<Vec<_>>()
            .concat();
        format!("{HEAD}{rows}")
    }

    /// The full PDF's pages for `fixture` with `table`, on Letter.
    fn full_sheets<'a>(fixture: &'a Fixture, table: &'a Published<'a>, paper: Paper) -> Sheets<'a> {
        Sheets {
            table: Some(table),
            ..fixture.sheets(paper, "Technology Radar", None)
        }
    }

    /// The pointer page 1 carries to the table on page `p`.
    fn pointer(p: usize) -> String {
        format!("Each item's description and rationale: page {p} onward.")
    }

    #[test]
    fn the_table_starts_on_a_page_of_its_own() {
        let nothing = format!("{HEAD}a,Tools,Gone,d\n");
        for (csv, table) in [
            (PRD_EXAMPLE.to_owned(), Table::Radar),
            (prose_csv(60), Table::Radar),
            (nothing.clone(), Table::Radar),
            (HEAD.to_owned(), Table::All),
            (nothing, Table::All),
        ] {
            let (fixture, shape) = full(&csv);
            let published = published(&shape.0, &shape.1, table);
            let sheets = full_sheets(&fixture, &published, Paper::Letter);
            let bodies = bodies(&sheets, fonts());
            assert_eq!(bodies.kinds.len(), bodies.pages.len());
            assert_eq!(bodies.kinds.first(), Some(&PageKind::Front));
            let first_table = bodies
                .kinds
                .iter()
                .position(|kind| *kind == PageKind::Table)
                .expect("a table page");
            assert!(
                bodies
                    .kinds
                    .get(1..first_table)
                    .unwrap_or_default()
                    .iter()
                    .all(|k| *k == PageKind::Legend)
            );
            assert!(
                bodies
                    .kinds
                    .get(first_table..)
                    .unwrap_or_default()
                    .iter()
                    .all(|k| *k == PageKind::Table)
            );
            for (page, kind) in bodies.pages.iter().zip(&bodies.kinds) {
                let words: Vec<String> = page.iter().flat_map(texts).collect();
                let headed = words
                    .first()
                    .is_some_and(|w| w == RADAR_HEADING || w == ALL_HEADING);
                assert_eq!(headed, *kind == PageKind::Table, "{csv:.30} {kind:?}");
            }
        }
    }

    #[test]
    fn every_table_page_is_footed_and_numbered() {
        let (fixture, shape) = full(&prose_csv(120));
        let published = published(&shape.0, &shape.1, Table::Radar);
        let sheets = full_sheets(&fixture, &published, Paper::A4);
        let pages = footed(bodies(&sheets, fonts()), &sheets, fonts());
        let count = pages.len();
        assert!(count > 3);
        for (i, page) in pages.iter().enumerate() {
            let words = texts(&page.root);
            assert!(words.contains(&format!("Page {} of {count}", i + 1)), "{i}");
        }
    }

    #[test]
    fn the_pointer_names_the_first_table_page() {
        let (fixture, shape) = full(&prose_csv(60));
        let published = published(&shape.0, &shape.1, Table::Radar);
        let sheets = full_sheets(&fixture, &published, Paper::Letter);
        let bodies = bodies(&sheets, fonts());
        let p = 1 + bodies
            .kinds
            .iter()
            .position(|k| *k == PageKind::Table)
            .expect("a table");
        let pages = footed(bodies, &sheets, fonts());
        let page_one = pages.first().expect("page 1");
        let node = children(page_one)
            .iter()
            .find(|node| texts(node) == [pointer(p)])
            .expect("the pointer");
        // Read straight after the title block, before the radar (FR-114).
        let at = children(page_one)
            .iter()
            .position(|node| texts(node) == [pointer(p)]);
        assert_eq!(at, Some(after_title(children(page_one))));
        assert!(at.is_some_and(|i| i > 0), "a title comes first");
        // A paragraph to a tagged PDF, and every footer furniture (PR 21c-1).
        assert!(matches!(
            node,
            Node::Group(g) if g.semantics == Semantics::Structure(Structure::Paragraph)
        ));
        for page in &pages {
            assert!(matches!(
                children(page).last(),
                Some(Node::Group(g)) if g.semantics == Semantics::Pagination
            ));
        }
        let at = rect(node);
        let (width, height) = sheets.size;
        assert!(at.top >= height - PAPER_FOOT_ROOM, "{at:?}");
        assert!(
            at.bottom <= height - FOOTER_BASELINE - FOOTER.size,
            "{at:?}"
        );
        assert!(at.left >= PAPER_MARGIN - 1e-9 && at.right <= width - PAPER_MARGIN);
        for later in pages.iter().skip(1) {
            assert!(!texts(&later.root).contains(&pointer(p)));
        }
    }

    #[test]
    fn table_pages_stay_clear_of_the_footer_and_margins() {
        let others = format!(
            "name,sector,ring,description,rationale,notes,more\n{}",
            (0..80)
                .map(|i| format!(
                    "item {i:02},Tools,{},\"Some prose.\n\nMore.\",Why.,n,m\n",
                    if i % 3 == 0 { "Gone" } else { "Adopt" }
                ))
                .collect::<Vec<_>>()
                .concat()
        );
        for (csv, table) in [(prose_csv(300), Table::Radar), (others, Table::All)] {
            let (fixture, shape) = full(&csv);
            let published = published(&shape.0, &shape.1, table);
            for paper in [Paper::Letter, Paper::A4] {
                let sheets = full_sheets(&fixture, &published, paper);
                let bodies = bodies(&sheets, fonts());
                let (width, height) = sheets.size;
                for (page, kind) in bodies.pages.iter().zip(&bodies.kinds) {
                    if *kind != PageKind::Table {
                        continue;
                    }
                    for node in page {
                        let Some(at) = bounds(node, fonts()) else {
                            continue;
                        };
                        let reach = crate::style::TABLE_RULE_HEAVY / 2.0 + 1e-9;
                        assert!(
                            at.left >= PAPER_MARGIN - reach
                                && at.right <= width - PAPER_MARGIN + reach,
                            "{at:?}"
                        );
                        assert!(
                            at.top >= PAPER_MARGIN - 1e-9
                                && at.bottom <= height - PAPER_FOOT_ROOM + reach,
                            "{at:?}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn the_radar_pdf_has_no_table_and_no_pointer() {
        let fixture = long(1, 1, 300);
        let sheets = fixture.sheets(Paper::Letter, "Technology Radar", None);
        let bodies = bodies(&sheets, fonts());
        assert!(bodies.kinds.iter().all(|k| *k != PageKind::Table));
        let pages = footed(bodies, &sheets, fonts());
        for page in &pages {
            assert!(
                texts(&page.root)
                    .iter()
                    .all(|w| !w.starts_with("Each item's"))
            );
        }
    }

    fn prd() -> Fixture {
        let (catalogue, order) = plotted(PRD_EXAMPLE, None);
        Fixture {
            blocks: Blocks::new(&catalogue, &order, None, None, fonts()),
            legend: LegendBlock::on_paper(&catalogue, &order, fonts()),
        }
    }

    /// The PRD example's radar beside a legend of `sectors` × `rings` ×
    /// `per_ring` items.
    fn long(sectors: usize, rings: usize, per_ring: usize) -> Fixture {
        let (catalogue, order) = crate::layout::test_support::long(sectors, rings, per_ring);
        let (prd_catalogue, prd_order) = plotted(PRD_EXAMPLE, None);
        Fixture {
            blocks: Blocks::new(&prd_catalogue, &prd_order, None, None, fonts()),
            legend: LegendBlock::on_paper(&catalogue, &order, fonts()),
        }
    }

    fn documents(sheets: &Sheets<'_>) -> Vec<Document> {
        footed(bodies(sheets, fonts()), sheets, fonts())
    }

    /// Every text's words, in document order.
    fn texts(node: &Node) -> Vec<String> {
        match node {
            Node::Group(group) => group.children.iter().flat_map(texts).collect(),
            Node::Text { words, .. } => vec![words.clone()],
            _ => Vec::new(),
        }
    }

    /// A page's children.
    fn children(page: &Document) -> &[Node] {
        match &page.root {
            Node::Group(group) => &group.children,
            other => panic!("a page is a group: {other:?}"),
        }
    }

    fn rect(node: &Node) -> Rect {
        bounds(node, fonts()).expect("drawn")
    }

    fn disjoint(a: Rect, b: Rect) -> bool {
        a.right <= b.left || b.right <= a.left || a.bottom <= b.top || b.bottom <= a.top
    }

    const LONG_TITLE: &str = "A title long enough to wrap onto three lines across a landscape \
        page of either size, which the radar makes room for by drawing smaller, as it must when \
        a team writes a title this long for its edition";

    /// rg-mi2: each piece's page is decided once, and its openers follow.
    #[test]
    fn piece_slots_assign_pages_once() {
        let pages = |slots: PieceSlots| (0..7).map(|k| slots.page_of(k)).collect::<Vec<_>>();
        let openers =
            |slots: PieceSlots| (0..7).filter(|&k| slots.opens_page(k)).collect::<Vec<_>>();
        let beside = PieceSlots {
            on_first: true,
            per_page: 3,
        };
        assert_eq!(pages(beside), [0, 1, 1, 1, 2, 2, 2]);
        assert_eq!(openers(beside), [1, 4]);
        let after = PieceSlots {
            on_first: false,
            per_page: 3,
        };
        assert_eq!(pages(after), [1, 1, 1, 2, 2, 2, 3]);
        assert_eq!(openers(after), [0, 3, 6]);
    }

    #[test]
    fn paper_sizes_in_points() {
        assert_eq!(Paper::Letter.points(), Some((792.0, 612.0)));
        let (w, h) = Paper::A4.points().expect("A4");
        assert!(
            (w - 841.889_763_779_527_6).abs() < 1e-9 && (h - 595.275_590_551_181_2).abs() < 1e-9
        );
        assert_eq!(Paper::None.points(), None);
    }

    #[test]
    fn a_page_is_the_paper_in_px() {
        let fixture = prd();
        let page = documents(&fixture.sheets(Paper::Letter, "T", None))
            .into_iter()
            .next()
            .expect("a page");
        assert_eq!(
            (page.width.to_bits(), page.height.to_bits()),
            (1056.0_f64.to_bits(), 816.0_f64.to_bits())
        );
        let (w, h) = page_size(Paper::A4.points().expect("A4"));
        assert!(
            (w - 841.889_763_779_527_6 / 0.75).abs() < 1e-9
                && (h - 595.275_590_551_181_2 / 0.75).abs() < 1e-9
        );
    }

    #[test]
    fn the_radar_fits_at_the_largest_scale() {
        let fixture = prd();
        for paper in [Paper::Letter, Paper::A4] {
            let sheets = fixture.sheets(paper, "T", None);
            let first = first_page(&sheets, fonts());
            let s = first.radar.scale;
            assert!(first.fits && s <= 1.0 && s > 0.5, "{paper:?}: {s}");
            assert!(fits(&sheets, &first, s, fonts()));
            assert!(
                s >= 1.0 || !fits(&sheets, &first, s + 1e-3, fonts()),
                "{paper:?}: {s} is not the largest"
            );
        }
        let short = first_page(&fixture.sheets(Paper::Letter, "T", None), fonts())
            .radar
            .scale;
        let long = first_page(
            &fixture.sheets(Paper::Letter, LONG_TITLE, Some("S")),
            fonts(),
        )
        .radar
        .scale;
        assert!(long < short, "{long} < {short}");
    }

    /// rg-i95: a title of 2,000 words is cut on paper, so the radar keeps
    /// its page.
    #[test]
    fn an_absurd_title_leaves_the_radar_its_page() {
        let fixture = prd();
        let title = "word ".repeat(2_000);
        let sheets = fixture.sheets(Paper::Letter, &title, Some("subtitle"));
        let first = first_page(&sheets, fonts());
        assert!(first.fits);
        let (_, height) = sheets.size;
        let radar = fixture.blocks.radar.size(first.radar.scale);
        assert!(first.radar.origin.y + radar.height <= height - PAPER_FOOT_ROOM);
    }

    /// rg-i95: a legend row taller than a page is reported with its line
    /// and name.
    #[test]
    fn a_legend_row_taller_than_a_page_is_reported() {
        let name = "word ".repeat(1_500);
        let csv = format!("{HEAD}Short,Tools,Adopt,d\n{name},Tools,Adopt,d\n");
        let (fixture, _) = full(&csv);
        let sheets = fixture.sheets(Paper::Letter, "Technology Radar", None);
        let bodies = bodies(&sheets, fonts());
        assert_eq!(bodies.tall_legend_rows, [(3, words(&name))]);
        let fine = long(1, 1, 300);
        let sheets = fine.sheets(Paper::Letter, "Technology Radar", None);
        assert_eq!(bodies_of(&sheets).tall_legend_rows.len(), 0);
    }

    /// rg-p82 (QA 11): a sector label too wide to sit beside the legend:
    /// the radar takes page 1 alone, and the legend's first piece opens
    /// page 2 as the legend's start, not its continuation.
    #[test]
    fn a_wide_sector_starts_the_legend_on_page_two() {
        let csv = format!(
            "{HEAD}a,{},Adopt,d\nb,Tools,Trial,d\nc,Data,Assess,d\n",
            "S".repeat(64)
        );
        let (catalogue, order) = plotted(&csv, None);
        let fixture = Fixture {
            blocks: Blocks::new(&catalogue, &order, None, None, fonts()),
            legend: LegendBlock::on_paper(&catalogue, &order, fonts()),
        };
        let sheets = fixture.sheets(Paper::Letter, "T", None);
        let first = first_page(&sheets, fonts());
        assert!(first.legend.is_none() && first.fits);
        let bodies = bodies_of(&sheets);
        assert_eq!(bodies.kinds.get(1), Some(&PageKind::Legend));
    }

    fn bodies_of(sheets: &Sheets<'_>) -> Bodies {
        bodies(sheets, fonts())
    }

    #[test]
    fn a_very_wide_radar_falls_back() {
        let name = "S".repeat(120);
        let (catalogue, order) = plotted(&format!("{HEAD}x,{name},Adopt,d\n"), None);
        let fixture = Fixture {
            blocks: Blocks::new(&catalogue, &order, None, None, fonts()),
            legend: LegendBlock::on_paper(&catalogue, &order, fonts()),
        };
        let first = first_page(&fixture.sheets(Paper::Letter, "T", None), fonts());
        assert!(first.legend.is_none(), "the legend starts on page 2");
        assert!(!first.fits, "and the radar still does not fit");
        assert!(bodies(&fixture.sheets(Paper::Letter, "T", None), fonts()).radar_too_wide);
        let prd = prd();
        let sheets = prd.sheets(Paper::Letter, "T", None);
        assert!(first_page(&sheets, fonts()).fits && !bodies(&sheets, fonts()).radar_too_wide);
        let pages = documents(&fixture.sheets(Paper::Letter, "T", None));
        assert!(pages.len() >= 2);
        assert!(
            texts(&pages.get(1).expect("page 2").root)
                .iter()
                .any(|t| t == "Legend")
        );
    }

    #[test]
    fn page_one_holds_only_its_parts() {
        let fixture = long(1, 1, 300);
        let pages = documents(&fixture.sheets(Paper::Letter, "T", None));
        let first = pages.first().expect("page 1");
        // Title, radar, rule, key, the legend's first piece, the footer.
        assert_eq!(children(first).len(), 6);
        assert!(matches!(children(first).last(), Some(Node::Group(_))));
        let empty = prd_without_legend();
        let first = documents(&empty.sheets(Paper::Letter, "T", None))
            .into_iter()
            .next()
            .expect("page 1");
        assert_eq!(children(&first).len(), 5);
        assert!(!texts(&first.root).iter().any(|t| t.starts_with("Legend")));
    }

    fn prd_without_legend() -> Fixture {
        let (prd_catalogue, prd_order) = plotted(PRD_EXAMPLE, None);
        let (catalogue, order) = (
            crate::layout::test_support::empty_catalogue(),
            crate::order::Plotted::default(),
        );
        Fixture {
            blocks: Blocks::new(&prd_catalogue, &prd_order, None, None, fonts()),
            legend: LegendBlock::on_paper(&catalogue, &order, fonts()),
        }
    }

    #[test]
    fn nothing_overlaps_on_page_one() {
        for fixture in [prd(), long(1, 1, 300)] {
            for paper in [Paper::Letter, Paper::A4] {
                for title in ["T", LONG_TITLE] {
                    let pages = documents(&fixture.sheets(paper, title, Some("S")));
                    let first = pages.first().expect("page 1");
                    let parts: Vec<Rect> = children(first)
                        .iter()
                        .take(children(first).len() - 1)
                        .filter_map(|node| bounds(node, fonts()))
                        .collect();
                    for (i, a) in parts.iter().enumerate() {
                        assert!(
                            a.left >= PAPER_MARGIN - 1e-6 && a.top >= PAPER_MARGIN - 1e-6,
                            "{a:?}"
                        );
                        assert!(a.right <= first.width - PAPER_MARGIN + 1e-6, "{a:?}");
                        assert!(
                            a.bottom <= first.height - PAPER_MARGIN + 1e-6,
                            "{paper:?} {a:?}"
                        );
                        for b in parts.iter().skip(i + 1) {
                            assert!(disjoint(*a, *b), "{paper:?} {title}: {a:?} {b:?}");
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn continuation_pages_sit_where_page_one_legend_sits() {
        let fixture = long(1, 1, 300);
        for paper in [Paper::Letter, Paper::A4] {
            let sheets = fixture.sheets(paper, "T", None);
            let first = first_page(&sheets, fonts())
                .legend
                .expect("a legend beside the radar");
            let right = first.origin.x + piece_width(&sheets, fonts());
            let pages = documents(&sheets);
            assert!(pages.len() >= 3, "{paper:?}: {}", pages.len());
            let columns = |page: &Document| -> Vec<Rect> {
                children(page)
                    .iter()
                    .take(children(page).len() - 1)
                    .map(rect)
                    .collect()
            };
            let full = columns(pages.get(1).expect("page 2"));
            assert!(full.len() >= 2, "{paper:?}");
            for page in pages.iter().skip(1) {
                let cols = columns(page);
                assert!(cols.len() <= full.len());
                // The last column's right edge is page 1's legend's.
                let last = cols.last().expect("a column");
                assert!(
                    (last.right - right).abs() < 1.0,
                    "{paper:?}: {} vs {right}",
                    last.right
                );
                for col in &cols {
                    assert!((col.top - first.origin.y).abs() < 1e-6, "{col:?}");
                    assert!(col.bottom <= page.height - PAPER_FOOT_ROOM + 1e-6);
                }
                for pair in cols.windows(2) {
                    if let [a, b] = pair {
                        assert!(disjoint(*a, *b));
                    }
                }
            }
        }
    }

    /// Every legend row's name on `pages`, in order.
    fn row_names(pages: &[Document], legend: &LegendBlock) -> Vec<String> {
        let expected = names_of(legend);
        pages
            .iter()
            .flat_map(|page| texts(&page.root))
            .filter(|t| expected.contains(t))
            .collect()
    }

    fn names_of(legend: &LegendBlock) -> Vec<String> {
        texts(&legend.render(crate::layout::block::Placement::at(
            crate::svg::Point::ORIGIN,
        )))
        .into_iter()
        .filter(|t| t.contains('-'))
        .collect()
    }

    #[test]
    fn every_name_once_across_pages() {
        let fixture = long(4, 3, 30);
        for paper in [Paper::Letter, Paper::A4] {
            let pages = documents(&fixture.sheets(paper, "T", None));
            assert_eq!(
                row_names(&pages, &fixture.legend),
                names_of(&fixture.legend)
            );
        }
    }

    #[test]
    fn every_page_is_numbered() {
        let fixture = long(1, 1, 300);
        let pages = documents(&fixture.sheets(Paper::A4, "T", None));
        let m = pages.len();
        assert!(m >= 3);
        for (i, page) in pages.iter().enumerate() {
            assert!(
                texts(&page.root).contains(&format!("Page {} of {m}", i + 1)),
                "page {i}"
            );
        }
    }

    #[test]
    fn the_title_and_subtitle_head_page_one_and_every_footer() {
        let fixture = long(1, 1, 300);
        let pages = documents(&fixture.sheets(Paper::Letter, "Technology Radar", Some("S")));
        let first = texts(&pages.first().expect("page 1").root);
        assert_eq!(
            first.get(..2),
            Some(&["Technology Radar".to_owned(), "S".to_owned()][..])
        );
        for page in &pages {
            let footer = children(page).last().map(texts).unwrap_or_default();
            assert_eq!(
                footer.first().map(String::as_str),
                Some("Technology Radar · S")
            );
        }
    }

    #[test]
    fn the_footer_sits_in_the_bottom_margin() {
        let fixture = prd();
        for paper in [Paper::Letter, Paper::A4] {
            let page = documents(&fixture.sheets(paper, "T", Some("S")))
                .into_iter()
                .next()
                .expect("page 1");
            let footer = rect(children(&page).last().expect("a footer"));
            assert!(footer.top >= page.height - PAPER_MARGIN, "{footer:?}");
            assert!(footer.bottom <= page.height - 1.0);
            // The right edge is the page number's ink, pinned by
            // `the_page_number_ends_at_the_margin`; its layout box runs on
            // by the room left for text spacing (D-78).
            assert!(footer.left >= PAPER_MARGIN - 1e-6);
        }
    }

    #[test]
    fn a_long_title_is_cut_in_the_footer_not_on_the_page() {
        let fixture = prd();
        let long_title = LONG_TITLE.repeat(3);
        for paper in [Paper::Letter, Paper::A4] {
            let page = documents(&fixture.sheets(paper, &long_title, None))
                .into_iter()
                .next()
                .expect("page 1");
            let footer = children(&page).last().map(texts).unwrap_or_default();
            let left = footer.first().expect("the footer's title");
            assert!(left.ends_with('…'), "{left}");
            assert!(
                long_title.starts_with(left.trim_end_matches(['…', ' '])),
                "cut at a word: {left}"
            );
            assert_eq!(footer.get(1).map(String::as_str), Some("Page 1 of 1"));
            let whole: String = texts(&page.root).first().cloned().unwrap_or_default();
            assert_eq!(whole, long_title, "page 1's title is whole");
        }
        let word = "W".repeat(300);
        let page = documents(&fixture.sheets(Paper::Letter, &word, None))
            .into_iter()
            .next()
            .expect("page 1");
        let footer = children(&page).last().map(texts).unwrap_or_default();
        let left = footer.first().expect("the footer's title");
        assert!(left.ends_with('…') && left.len() < word.len(), "{left}");
    }

    /// The footer's two runs: (where drawn from, line) for the title and
    /// the page number.
    fn footer_runs(page: &Document) -> Vec<(crate::svg::Point, String)> {
        let Some(Node::Group(footer)) = children(page).last() else {
            panic!("a footer group")
        };
        footer
            .children
            .iter()
            .map(|node| {
                // Where the line is drawn from, its anchor resolved.
                let starts = node.line_starts(fonts());
                let (line, start) = starts.first().expect("footer text");
                (*start, (*line).to_owned())
            })
            .collect()
    }

    /// Final review (Rust): a title holding a line break or tab, as a TOML
    /// string can, reads as words in the footer, not as boxes.
    #[test]
    fn the_footer_folds_whitespace() {
        let fixture = prd();
        let page = documents(&fixture.sheets(Paper::A4, "Line one\nLine\ttwo", Some("Sub  title")))
            .into_iter()
            .next()
            .expect("page 1");
        let runs = footer_runs(&page);
        assert_eq!(
            runs.first().map(|(_, line)| line.as_str()),
            Some("Line one Line two · Sub title")
        );
    }

    /// Final review (UI): *Page n of m*'s ink ends at the right margin.
    #[test]
    fn the_page_number_ends_at_the_margin() {
        let fixture = long(1, 1, 300);
        for paper in [Paper::Letter, Paper::A4] {
            for page in documents(&fixture.sheets(paper, "T", None)) {
                let runs = footer_runs(&page);
                let (origin, line) = runs.get(1).expect("the page number");
                let end = origin.x + crate::style::FOOTER.advance(line, fonts());
                assert!(
                    (end - (page.width - PAPER_MARGIN)).abs() < 1e-6,
                    "{paper:?} {line}: ends at {end}, margin at {}",
                    page.width - PAPER_MARGIN
                );
            }
        }
    }

    /// Final review (UI): the page's content ends `PAPER_FOOT_ROOM` above
    /// the bottom edge, at least 6 mm clear of the footer, so the footer
    /// does not read as one more key row.
    #[test]
    fn content_stays_clear_of_the_footer() {
        let six_mm = 6.0 * 96.0 / 25.4;
        for fixture in [prd(), long(1, 1, 300)] {
            for paper in [Paper::Letter, Paper::A4] {
                for page in documents(&fixture.sheets(paper, LONG_TITLE, Some("S"))) {
                    let footer = rect(children(&page).last().expect("a footer"));
                    let bottom = children(&page)
                        .iter()
                        .take(children(&page).len() - 1)
                        .filter_map(|node| bounds(node, fonts()))
                        .map(|r| r.bottom)
                        .fold(0.0, f64::max);
                    assert!(
                        bottom <= page.height - crate::style::PAPER_FOOT_ROOM + 1e-6,
                        "{paper:?}: content to {bottom}"
                    );
                    assert!(
                        footer.top - bottom >= six_mm,
                        "{paper:?}: {bottom} vs {}",
                        footer.top
                    );
                }
            }
        }
    }

    /// Final review: a cut footer title fits the room it was cut for,
    /// however small, so it cannot run into the page number.
    #[test]
    fn a_cut_title_fits_its_room() {
        let word = "W".repeat(300);
        for text in [LONG_TITLE, word.as_str(), "Technology Radar · Edition 1"] {
            for room in [400.0, 120.0, 20.0, 8.0, 0.0, -5.0] {
                let got = cut(FOOTER, text, room, fonts());
                assert!(
                    FOOTER.measure(&got, fonts()) <= room || got == "\u{2026}",
                    "{room}: {got:?} is {} wide",
                    FOOTER.measure(&got, fonts())
                );
                assert!(got == text || got.ends_with('\u{2026}'), "{got:?}");
            }
        }
    }

    #[test]
    fn the_legend_draws_at_sixteen_fifteenths() {
        fn sizes(node: &Node, out: &mut Vec<f64>) {
            match node {
                Node::Group(group) => group.children.iter().for_each(|c| sizes(c, out)),
                Node::Text { style, .. } => out.push(style.size),
                _ => {}
            }
        }
        let fixture = prd();
        let pages = documents(&fixture.sheets(Paper::Letter, "T", None));
        let page_one = pages.first().expect("page 1");
        // A ring heading, 10 px at scale 1, prints at 8 pt: 10 × 16/15 px.
        let ring = 10.0 * 16.0 / 15.0;
        let mut drawn = Vec::new();
        sizes(&page_one.root, &mut drawn);
        assert!(
            drawn.iter().any(|size| (size - ring).abs() < 1e-12),
            "{drawn:?}"
        );
        let points = ring * POINTS_PER_PX;
        assert!((points - 8.0).abs() < 1e-12, "{points}");
    }

    /// rg-q1q: a full continuation page holds exactly the columns that
    /// fit, each a piece's width and the gap apart.
    #[test]
    fn full_continuation_pages_hold_per_page_columns() {
        let fixture = long(1, 1, 400);
        for paper in [Paper::Letter, Paper::A4] {
            let sheets = fixture.sheets(paper, "T", None);
            let n = per_page(&sheets, fonts());
            let width = piece_width(&sheets, fonts());
            let pages = documents(&sheets);
            assert!(pages.len() >= 4, "{paper:?}");
            for page in pages.iter().skip(1).take(pages.len() - 2) {
                let columns: Vec<Rect> = children(page)
                    .iter()
                    .take(children(page).len() - 1)
                    .map(rect)
                    .collect();
                assert_eq!(columns.len(), n, "{paper:?}");
                for pair in columns.windows(2) {
                    let [a, b] = pair else { continue };
                    assert!(
                        (b.left - a.left - width - COLUMN_GAP).abs() < 1e-6,
                        "{paper:?}"
                    );
                }
            }
        }
    }

    /// rg-q1q: the footer's two runs never touch.
    #[test]
    fn footer_runs_are_disjoint() {
        let fixture = long(1, 1, 300);
        for paper in [Paper::Letter, Paper::A4] {
            for page in documents(&fixture.sheets(paper, LONG_TITLE, Some("A subtitle"))) {
                let Some(Node::Group(footer)) = children(&page).last() else {
                    panic!("a footer group")
                };
                let runs: Vec<Rect> = footer.children.iter().map(rect).collect();
                assert_eq!(runs.len(), 2);
                if let [a, b] = runs.as_slice() {
                    assert!(disjoint(*a, *b), "{a:?} {b:?}");
                }
            }
        }
    }
}
