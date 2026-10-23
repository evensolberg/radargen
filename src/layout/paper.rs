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
use crate::layout::key::KeyBlock;
use crate::layout::legend::LegendBlock;
use crate::layout::radar::RadarBlock;
use crate::layout::title::Wrap;
use crate::palette::Neutral;
use crate::style::{
    COLUMN_GAP, FOOTER, FOOTER_BASELINE, LEGEND_WIDE_DROP, LINE_HEIGHT, NARROW_MIN_SCALE,
    PAPER_LEGEND_SCALE, PAPER_MARGIN, POINTS_PER_PX, RADAR_LABEL_PAD,
};
use crate::svg::{Anchor, Document, Group, Node, Point, Scale, TextRun};
use crate::text::{Fonts, words};

/// Between the footer's title and subtitle.
const FOOTER_SEPARATOR: &str = " \u{b7} ";
/// Where a footer's title is cut.
const ELLIPSIS: char = '\u{2026}';

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
}

/// A paper's landscape `points`, width first, as a page in px.
#[must_use]
pub fn page_size(points: (f64, f64)) -> (f64, f64) {
    (points.0 / POINTS_PER_PX, points.1 / POINTS_PER_PX)
}

/// The pages' contents, before their footers.
#[derive(Debug, Clone, PartialEq)]
pub struct Bodies {
    /// Each page's nodes, page 1 first, in reading order.
    pub pages: Vec<Vec<Node>>,
    /// Whether the radar, at its smallest, still runs past the first
    /// page's margins: labels never shrink, so a long enough sector name
    /// can be wider than the page.
    pub radar_too_wide: bool,
}

/// Where page 1's parts go.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct FirstPage {
    /// The title and subtitle.
    pub title: Placement,
    /// The radar, at the scale that fits.
    pub radar: Placement,
    /// The rule over the key.
    pub rule: Rule,
    /// The key, at scale 1.
    pub key: Placement,
    /// The key's columns.
    pub key_columns: NonZeroUsize,
    /// The legend's first piece, at `PAPER_LEGEND_SCALE`; `None` when the
    /// legend is empty or starts on page 2.
    pub legend: Option<Placement>,
    /// Whether the radar fits at the scale it has.
    pub fits: bool,
}

/// The page's content box: its margins inset, in px.
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
            height: h - PAPER_MARGIN - PAPER_MARGIN,
        }
    }
}

/// The title block for `sheets`, wrapped across the content box.
fn title(sheets: &Sheets<'_>, fonts: &Fonts) -> Wrap {
    Wrap::new(
        Some(sheets.title),
        sheets.subtitle,
        Content::of(sheets).width,
        fonts,
    )
}

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
    let least = NARROW_MIN_SCALE / sheets.radar.growth();
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

/// Page 1's placements: the radar beside the legend's first piece when
/// it fits there, else across the whole page with the legend starting on
/// page 2, else at its floor, marked as not fitting.
pub(crate) fn first_page(sheets: &Sheets<'_>, fonts: &Fonts) -> FirstPage {
    let has_legend = !sheets.legend.size(1.0).is_empty();
    let beside = has_legend && largest(sheets, column(sheets, true, fonts)).is_some();
    let col = column(sheets, beside, fonts);
    let found = largest(sheets, col);
    let s = found.unwrap_or_else(|| NARROW_MIN_SCALE / sheets.radar.growth());
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
        title: Placement::at(Point {
            x: PAPER_MARGIN,
            y: PAPER_MARGIN,
        }),
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
        key: Placement::at(Point {
            x: x + RADAR_LABEL_PAD,
            y: key_y,
        }),
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

/// Every page's content, before footers (FR-111).
#[must_use]
pub fn bodies(sheets: &Sheets<'_>, fonts: &Fonts) -> Bodies {
    let first = first_page(sheets, fonts);
    let content = Content::of(sheets);
    let n = per_page(sheets, fonts);
    let t = title(sheets, fonts).size(1.0).height;
    let rest = content.height / PAPER_LEGEND_SCALE;
    let pieces = match first.legend {
        Some(_) => {
            let room = (content.height - t - LEGEND_WIDE_DROP).max(0.0) / PAPER_LEGEND_SCALE;
            sheets
                .legend
                .flowed(room, rest, &|k| k >= 1 && (k - 1) % n == 0, fonts)
        }
        None => sheets.legend.flowed(rest, rest, &|k| k % n == 0, fonts),
    };
    let mut pieces = pieces.into_iter();
    let mut page_one = vec![
        title(sheets, fonts).render(first.title),
        sheets.radar.render(first.radar),
        rule_node(first.rule),
        sheets.key.grid(first.key_columns).render(first.key),
    ];
    if let Some(at) = first.legend
        && let Some(piece) = pieces.next()
    {
        page_one.push(piece.render(at));
    }
    let rest: Vec<LegendBlock> = pieces.collect();
    let width = piece_width(sheets, fonts);
    let mut pages = vec![page_one];
    pages.extend(
        rest.chunks(n)
            .map(|chunk| continuation(chunk, width, sheets)),
    );
    Bodies {
        pages,
        radar_too_wide: !first.fits,
    }
}

/// A continuation page: `pieces` side by side, `width` each, the group
/// centred across the content box, hung from the top margin.
fn continuation(pieces: &[LegendBlock], width: f64, sheets: &Sheets<'_>) -> Vec<Node> {
    let content = Content::of(sheets);
    let mut span = 0.0;
    for (i, _) in pieces.iter().enumerate() {
        span += if i == 0 { width } else { COLUMN_GAP + width };
    }
    let mut x = PAPER_MARGIN + (content.width - span) / 2.0;
    pieces
        .iter()
        .map(|piece| {
            let node = piece.render(Placement {
                origin: Point { x, y: PAPER_MARGIN },
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
/// cut to fit, and *Page n of m* at the right. The footer is one group, so a tagged PDF can mark it as a
/// pagination artifact.
#[must_use]
pub fn footed(bodies: Vec<Vec<Node>>, sheets: &Sheets<'_>, fonts: &Fonts) -> Vec<Document> {
    let (width, height) = sheets.size;
    let count = bodies.len();
    bodies
        .into_iter()
        .enumerate()
        .map(|(i, mut children)| {
            children.push(footer(sheets, i + 1, count, fonts));
            Document {
                width,
                height,
                root: Node::Group(Group::new(Point::ORIGIN, children)),
            }
        })
        .collect()
}

/// Page `page` of `count`'s footer.
fn footer(sheets: &Sheets<'_>, page: usize, count: usize, fonts: &Fonts) -> Node {
    let (width, height) = sheets.size;
    let baseline = height - FOOTER_BASELINE;
    let numbered = format!("Page {page} of {count}");
    let named = sheets.subtitle.map_or_else(
        || sheets.title.to_owned(),
        |subtitle| format!("{}{FOOTER_SEPARATOR}{subtitle}", sheets.title),
    );
    let room = Content::of(sheets).width - FOOTER.measure(&numbered, fonts) - COLUMN_GAP;
    let named = cut(&named, room, fonts);
    let run = |text: &str, x: f64, anchor: Anchor| {
        TextRun {
            origin: Point { x, y: baseline },
            style: FOOTER,
            ink: Neutral::TextMuted,
            anchor,
            lines: vec![text.to_owned()],
            words: words(text),
            leading: LINE_HEIGHT,
        }
        .node(Scale::uniform(1.0))
    };
    Node::Group(Group::new(
        Point::ORIGIN,
        vec![
            run(&named, PAPER_MARGIN, Anchor::Start),
            // Right-aligned by its layout width (D-78): the document has no end anchor.
            run(
                &numbered,
                width - PAPER_MARGIN - FOOTER.measure(&numbered, fonts),
                Anchor::Start,
            ),
        ],
    ))
}

/// `text` as it fits `room` in the footer's style: whole, or with words
/// dropped from the end and an ellipsis, or, when even its first word is
/// too wide, cut by characters with an ellipsis.
fn cut(text: &str, room: f64, fonts: &Fonts) -> String {
    let fits = |candidate: &str| FOOTER.measure(candidate, fonts) <= room;
    if fits(text) {
        return text.to_owned();
    }
    let words: Vec<&str> = text.split_whitespace().collect();
    for n in (1..words.len()).rev() {
        let candidate = format!("{}{ELLIPSIS}", words.get(..n).unwrap_or_default().join(" "));
        if fits(&candidate) {
            return candidate;
        }
    }
    let mut kept = String::new();
    for ch in text.chars() {
        let candidate = format!("{kept}{ch}{ELLIPSIS}");
        if !fits(&candidate) {
            break;
        }
        kept.push(ch);
    }
    format!("{kept}{ELLIPSIS}")
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
    use crate::options::Paper;
    use crate::style::{PAPER_LEGEND_SCALE, PAPER_MARGIN};
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
            }
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
        let ring_list: Vec<String> = (0..rings).map(|r| format!("R{r}")).collect();
        let rows: String = (0..sectors)
            .flat_map(|s| (0..rings).map(move |r| (s, r)))
            .flat_map(|(s, r)| (0..per_ring).map(move |i| format!("S{s}R{r}-{i:03},S{s},R{r},d\n")))
            .collect();
        let (catalogue, order) = plotted(&format!("{HEAD}{rows}"), Some(&ring_list.join(",")));
        let (prd_catalogue, prd_order) = plotted(PRD_EXAMPLE, None);
        Fixture {
            blocks: Blocks::new(&prd_catalogue, &prd_order, None, None, fonts()),
            legend: LegendBlock::on_paper(&catalogue, &order, fonts()),
        }
    }

    fn documents(sheets: &Sheets<'_>) -> Vec<Document> {
        footed(bodies(sheets, fonts()).pages, sheets, fonts())
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
    fn continuation_pages_hold_their_columns_centred() {
        let fixture = long(1, 1, 300);
        for paper in [Paper::Letter, Paper::A4] {
            let pages = documents(&fixture.sheets(paper, "T", None));
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
                let left = cols.first().expect("a column").left;
                let right = cols.last().expect("a column").right;
                assert!(
                    ((left - PAPER_MARGIN) - (page.width - PAPER_MARGIN - right)).abs() < 1.0,
                    "centred"
                );
                for col in &cols {
                    assert!((col.top - PAPER_MARGIN).abs() < 1e-6, "{col:?}");
                    assert!(col.bottom <= page.height - PAPER_MARGIN + 1e-6);
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
            assert!(
                footer.left >= PAPER_MARGIN - 1e-6
                    && footer.right <= page.width - PAPER_MARGIN + 1e-6
            );
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

    #[test]
    fn the_legend_draws_at_sixteen_fifteenths() {
        let fixture = prd();
        let first = first_page(&fixture.sheets(Paper::Letter, "T", None), fonts());
        assert_eq!(
            first.legend.map(|at| at.scale.to_bits()),
            Some(PAPER_LEGEND_SCALE.to_bits())
        );
        assert_eq!(first.key.scale.to_bits(), 1.0_f64.to_bits());
    }
}
