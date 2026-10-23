//! The table block's tests (FR-112).

use super::columns::{PADS_X, heading_lines};
use super::rows::Part;
use super::*;
use crate::layout::block::fonts;
use crate::layout::test_support::{PRD_EXAMPLE, bounds, plotted};
use crate::model::Catalogue;
use crate::model::NOT_ON_THE_RADAR;
use crate::options::Table;
use crate::order::Plotted;
use crate::palette::Neutral;
use crate::palette::Role;
use crate::published::RADAR_HEADING;
use crate::published::published;
use crate::published::{CellKind, NOTHING_PLOTTED};
use crate::style::{PAPER_FOOT_ROOM, PAPER_MARGIN};
use crate::style::{
    TABLE_HEAD, TABLE_LABEL_CAP, TABLE_LABEL_FLOOR, TABLE_NAME, TABLE_NAME_CAP, TABLE_PROSE_MIN,
    TABLE_RULE, TABLE_RULE_HEAVY, TABLE_TEXT,
};
use crate::style::{TABLE_HEADING, TABLE_LEADING, TABLE_PAD_X, TABLE_PAD_Y};
use crate::svg::{Anchor, Node, Point, Semantics};
use crate::svg::{Paint, TextPaint};

const HOSTILE: &str = include_str!("../../../../tests/fixtures/table-hostile.csv");

/// Letter's content box, in px.
fn letter() -> (f64, f64) {
    (
        1056.0 - PAPER_MARGIN - PAPER_MARGIN,
        816.0 - PAPER_MARGIN - PAPER_FOOT_ROOM,
    )
}

fn shape(csv: &str) -> (Catalogue, Plotted) {
    plotted(csv, None)
}

/// `csv`'s table under `table`, on Letter's content box.
fn block_of(shape: &(Catalogue, Plotted), table: Table) -> TableBlock {
    let (width, height) = letter();
    TableBlock::new(
        &published(&shape.0, &shape.1, table),
        width,
        height,
        fonts(),
    )
}

fn block(csv: &str, table: Table) -> TableBlock {
    block_of(&shape(csv), table)
}

/// The width of the column headed `heading`.
fn width_of(block: &TableBlock, heading: &str) -> f64 {
    block
        .columns
        .iter()
        .find(|c| c.heading.join(" ") == heading)
        .map_or(f64::NAN, |c| c.width)
}

/// Every text node under `nodes`, with its offset applied.
fn texts(nodes: &[Node]) -> Vec<Node> {
    fn walk(node: &Node, at: Point, out: &mut Vec<Node>) {
        match node {
            Node::Group(group) => {
                let at = Point {
                    x: at.x + group.offset.x,
                    y: at.y + group.offset.y,
                };
                for child in &group.children {
                    walk(child, at, out);
                }
            }
            Node::Text { origin, .. } => {
                let mut text = node.clone();
                if let Node::Text { origin: o, .. } = &mut text {
                    *o = Point {
                        x: at.x + origin.x,
                        y: at.y + origin.y,
                    };
                }
                out.push(text);
            }
            _ => {}
        }
    }
    let mut out = Vec::new();
    for node in nodes {
        walk(node, Point::ORIGIN, &mut out);
    }
    out
}

/// Every line node under `nodes`, as (y, width).
fn rules(nodes: &[Node]) -> Vec<(f64, f64)> {
    fn walk(node: &Node, out: &mut Vec<(f64, f64)>) {
        match node {
            Node::Group(group) => group.children.iter().for_each(|c| walk(c, out)),
            Node::Line {
                from, stroke_width, ..
            } => out.push((from.y, *stroke_width)),
            _ => {}
        }
    }
    let mut out = Vec::new();
    for node in nodes {
        walk(node, &mut out);
    }
    out
}

fn words_of(node: &Node) -> String {
    match node {
        Node::Text { words, .. } => words.clone(),
        _ => String::new(),
    }
}

fn lines_of(node: &Node) -> Vec<String> {
    match node {
        Node::Text { lines, .. } => lines.clone(),
        _ => Vec::new(),
    }
}

fn all_pages(block: &TableBlock) -> Vec<Vec<Node>> {
    block
        .pages()
        .iter()
        .map(|page| block.render(page, Point::ORIGIN, fonts()))
        .collect()
}

/// `n` rows of three-paragraph prose in two sectors.
fn long_csv(n: usize) -> String {
    (0..n).fold("name,sector,ring,description\n".to_owned(), |csv, i| {
        let sector = if i < n / 2 { "Tools" } else { "Platforms" };
        csv + &format!(
            "item {i:03},{sector},Adopt,\"One sentence of prose about item {i}.\n\nA second paragraph, a little longer, about why it sits where it does on the radar.\n\nA third.\"\n"
        )
    })
}

#[test]
fn label_columns_take_their_content_and_prose_the_rest() {
    let block = block(PRD_EXAMPLE, Table::Radar);
    let (width, _) = letter();
    let sum: f64 = block.columns.iter().map(|c| c.width).sum();
    assert!((sum - width).abs() < 1e-9, "{sum} against {width}");
    assert!(width_of(&block, "name") <= TABLE_NAME_CAP + 1e-9);
    assert!(width_of(&block, "description") >= TABLE_PROSE_MIN);
    assert_eq!(block.cramped(), None);
}

#[test]
fn prose_shares_by_content() {
    let long = "word ".repeat(60);
    let short = "word ".repeat(20);
    let csv = format!("name,sector,ring,description,rationale\nA,Tools,Adopt,{long},{short}\n");
    let block = block(&csv, Table::Radar);
    let (description, rationale) = (
        width_of(&block, "description"),
        width_of(&block, "rationale"),
    );
    assert!(description > rationale, "{description} {rationale}");
    assert!(rationale >= TABLE_PROSE_MIN);
}

#[test]
fn a_wide_label_is_capped_and_wraps() {
    let name = "A name of sixty characters that cannot fit one label column";
    let block = block(
        &format!("name,sector,ring,description\n{name},Tools,Adopt,d\n"),
        Table::Radar,
    );
    assert!((width_of(&block, "name") - TABLE_NAME_CAP).abs() < 1e-9);
    let part = block.parts.first().expect("a row");
    assert!(part.cells.first().is_some_and(|c| c.lines.len() >= 2));
}

#[test]
fn a_label_column_is_its_widest_text() {
    let csv = "name,sector,ring,description\nA,Data & Analytics,Adopt,d\n";
    let block = block(csv, Table::Radar);
    let widest = TABLE_TEXT.measure("Data & Analytics", fonts()) + PADS_X;
    assert!((width_of(&block, "Sector ring") - widest).abs() < 1e-9);
}

/// The maintainer's ruling (2026-10-23): a heading breaks at a space
/// or after an underscore, and its column is never narrower than its
/// longest piece, so no heading breaks mid-word.
#[test]
fn headings_break_after_underscores_never_mid_word() {
    let block = block(PRD_EXAMPLE, Table::Radar);
    for column in &block.columns {
        for line in &column.heading {
            assert!(
                TABLE_HEAD.measure(line, fonts()) <= column.width - PADS_X + 1e-9,
                "{line}"
            );
        }
    }
    assert_eq!(
        heading_lines("last_affirmed", 1.0, fonts()),
        ["last_", "affirmed"]
    );
    assert_eq!(heading_lines("a b_c", 1.0, fonts()), ["a", "b_", "c"]);
    assert_eq!(
        heading_lines("last_affirmed", 500.0, fonts()),
        ["last_affirmed"]
    );
}

#[test]
fn the_prd_example_is_not_cramped() {
    assert_eq!(block(PRD_EXAMPLE, Table::Radar).cramped(), None);
}

#[test]
fn the_cap_comes_down_before_cramping() {
    let rows = (0..3)
        .map(|i| {
            let prose = "Some words of prose about this item that run to a few lines. ".repeat(4);
            format!("A long name for item {i} that wraps in its column,Platforms,Adopt,{prose},{prose},{prose},{prose},{prose}\n")
        })
        .collect::<Vec<_>>()
        .concat();
    let csv = format!("name,sector,ring,description,rationale,context,notes,more\n{rows}");
    let block = block(&csv, Table::All);
    assert_eq!(block.cramped(), None);
    assert!(
        block.cap > TABLE_LABEL_FLOOR && block.cap < TABLE_LABEL_CAP,
        "{}",
        block.cap
    );
    for heading in ["description", "rationale", "context", "notes", "more"] {
        assert!(
            width_of(&block, heading) >= TABLE_PROSE_MIN - 1e-6,
            "{heading}"
        );
    }
}

#[test]
fn too_many_columns_cramp() {
    let others = (0..10)
        .map(|i| format!(",o{i}"))
        .collect::<Vec<_>>()
        .concat();
    let prose = ",Prose that runs on for a while in every column.".repeat(10);
    let csv = format!(
        "name,sector,ring,description{others}\nAlpha,Tools,Adopt,d{prose}\nGone,Tools,Elsewhere,d{prose}\n"
    );
    let block = block(&csv, Table::All);
    assert_eq!(block.cramped(), Some(block.columns.len()));
    let flag = TABLE_TEXT.measure(NOT_ON_THE_RADAR, fonts()) + PADS_X;
    for (drawn, column) in block.columns.iter().enumerate() {
        if column.kind == Some(CellKind::Label) || column.kind == Some(CellKind::Link) {
            // At the floor, or at its widest unbreakable word: never less.
            let source = drawn.saturating_sub(1);
            let word = block
                .parts
                .iter()
                .flat_map(|p| {
                    // The ring stacked under the sector counts too.
                    let ring = p.beneath.as_ref().filter(|(b, _)| *b == source);
                    p.cells.get(source).into_iter().chain(ring.map(|(_, l)| l))
                })
                .flat_map(|c| c.lines.iter())
                .flat_map(|l| l.split_whitespace())
                .map(|w| TABLE_TEXT.measure(w, fonts()))
                .chain(
                    column
                        .heading
                        .iter()
                        .map(|h| TABLE_HEAD.measure(h, fonts())),
                )
                .fold(0.0, f64::max)
                + PADS_X;
            assert!(
                column.width <= TABLE_LABEL_FLOOR.max(word) + 1e-9,
                "{:?}",
                column.heading
            );
        }
        if column.kind == Some(CellKind::Name) {
            assert!(column.width <= flag.max(TABLE_LABEL_FLOOR) + 1e-9);
        }
        if column.kind == Some(CellKind::Prose) {
            assert!(column.width >= PADS_X + TABLE_TEXT.size - 1e-9);
        }
    }
    assert_ne!(all_pages(&block).len(), 0);
}

/// Review of the name cap: the name's wider cap comes down to the same
/// floor as every other label's, so it never cramps a table sooner.
#[test]
fn the_name_cap_comes_down_to_the_floor() {
    let others = (0..10)
        .map(|i| format!(",o{i}"))
        .collect::<Vec<_>>()
        .concat();
    let prose = ",Prose that runs on for a while in every column.".repeat(10);
    let csv = format!(
        "name,sector,ring,description{others}\nAlpha with a name of some length,Tools,Adopt,d{prose}\n"
    );
    let block = block(&csv, Table::All);
    assert!(block.cramped().is_some());
    assert!(width_of(&block, "name") <= TABLE_LABEL_FLOOR + 1e-9);
}

#[test]
fn the_flag_fits_its_name_column() {
    let block = block(
        "name,sector,ring,description\nAbc,Tools,Elsewhere,d\n",
        Table::All,
    );
    let flag = TABLE_TEXT.measure(NOT_ON_THE_RADAR, fonts()) + PADS_X;
    assert!(width_of(&block, "name") >= flag - 1e-9);
}

#[test]
fn table_styles_are_fr112s() {
    let block = block(HOSTILE, Table::All);
    let nodes = block.render(
        block.pages().first().expect("a page"),
        Point::ORIGIN,
        fonts(),
    );
    assert_text_styles(&block, &nodes);
    let mut drawn = Vec::new();
    for node in &nodes {
        paints(node, &mut drawn);
    }
    assert_ne!(drawn.len(), 0);
    for (fill, stroke, width) in drawn {
        assert_eq!(fill, Paint::None, "no fills");
        assert_eq!(stroke, Paint::Role(Role::Neutral(Neutral::StrokeDivider)));
        assert!(
            width.to_bits() == TABLE_RULE.to_bits()
                || width.to_bits() == TABLE_RULE_HEAVY.to_bits()
        );
    }
}

/// Every shape's fill, stroke and stroke width under `node`.
fn paints(node: &Node, out: &mut Vec<(Paint, Paint, f64)>) {
    match node {
        Node::Group(group) => group.children.iter().for_each(|c| paints(c, out)),
        Node::Line {
            stroke,
            stroke_width,
            ..
        } => out.push((Paint::None, *stroke, *stroke_width)),
        Node::Rect {
            fill,
            stroke,
            stroke_width,
            ..
        }
        | Node::Circle {
            fill,
            stroke,
            stroke_width,
            ..
        }
        | Node::Path {
            fill,
            stroke,
            stroke_width,
            ..
        } => out.push((*fill, *stroke, *stroke_width)),
        _ => {}
    }
}

/// FR-112's text styles in `nodes`: the heading, the header row, the
/// names, the prose and the muted marks.
fn assert_text_styles(block: &TableBlock, nodes: &[Node]) {
    let muted = TextPaint::Fill(Paint::Role(Role::Neutral(Neutral::TextMuted)));
    let primary = TextPaint::Fill(Paint::Role(Role::Neutral(Neutral::TextPrimary)));
    let all = texts(nodes);
    let heading = all.first().expect("the heading");
    assert!(
        matches!(heading, Node::Text { style, ink, .. } if *style == TABLE_HEADING && *ink == primary)
    );
    for text in &all {
        let Node::Text {
            style, ink, lines, ..
        } = text
        else {
            continue;
        };
        let line = lines.join(" ");
        if line == NOT_ON_THE_RADAR || line == CONTINUES {
            assert_eq!((*style, *ink), (TABLE_TEXT, muted), "{line}");
        }
        if block.columns.iter().any(|c| c.heading.join(" ") == line) {
            assert_eq!((*style, *ink), (TABLE_HEAD, muted), "{line}");
        }
        if line == "Gone" || line == "Refused link" {
            assert_eq!((*style, *ink), (TABLE_NAME, primary), "{line}");
        }
        if line == "Plain description." {
            assert_eq!((*style, *ink), (TABLE_TEXT, primary), "{line}");
        }
    }
}

#[test]
fn the_number_column_never_wraps_and_ends_at_its_padding() {
    let csv = (0..1_200).fold("name,sector,ring,description\n".to_owned(), |csv, i| {
        csv + &format!("item {i:04},Tools,Adopt,d\n")
    });
    let block = block(&csv, Table::Radar);
    let number = block.columns.first().expect("the number column");
    assert_eq!(number.kind, None);
    let right = number.x + number.width - TABLE_PAD_X;
    let mut numbers = 0;
    for page in all_pages(&block) {
        for text in texts(&page) {
            let Node::Text {
                origin,
                anchor,
                lines,
                ..
            } = &text
            else {
                continue;
            };
            let numeric = lines.first().is_some_and(|l| l.parse::<usize>().is_ok());
            if numeric {
                assert_eq!(lines.len(), 1, "a number never wraps");
                assert_eq!(*anchor, Anchor::End);
                assert!((origin.x - right).abs() < 1e-9);
                numbers += 1;
            }
        }
    }
    // Every row's number, so the checks above cannot pass by finding none.
    assert_eq!(numbers, 1_200);
}

#[test]
fn cr_lf_never_reaches_a_line() {
    let block = block(HOSTILE, Table::All);
    for part in &block.parts {
        for cell in &part.cells {
            assert!(
                cell.lines
                    .iter()
                    .all(|l| !l.contains('\r') && !l.contains('\n'))
            );
        }
    }
}

#[test]
fn an_empty_description_has_one_blank_line() {
    let block = block(
        "name,sector,ring,description\nA,Tools,Adopt,\n",
        Table::Radar,
    );
    let part = block.parts.first().expect("a row");
    assert_eq!(
        part.cells.get(2).map(|c| c.lines.clone()),
        Some(vec![String::new()])
    );
}

#[test]
fn a_refused_link_draws_its_text() {
    let block = block(HOSTILE, Table::Radar);
    let drawn: Vec<String> = all_pages(&block)
        .iter()
        .flat_map(|p| texts(p))
        .map(|t| words_of(&t))
        .collect();
    assert!(drawn.iter().any(|w| w == "javascript:x"));
    assert!(drawn.iter().any(|w| w == "https://example.com/a?b=1&c=2"));
}

#[test]
fn a_long_url_breaks_inside_the_link_column() {
    let url = format!("https://example.com/{}", "segment/".repeat(25));
    let block = block(
        &format!("name,sector,ring,description,link\nA,Tools,Adopt,d,{url}\n"),
        Table::Radar,
    );
    let column = block
        .columns
        .iter()
        .find(|c| c.kind == Some(CellKind::Link))
        .expect("link");
    let lines = &block
        .parts
        .first()
        .and_then(|p| p.cells.get(3))
        .expect("a link cell")
        .lines;
    assert!(lines.len() >= 2);
    for line in lines {
        assert!(TABLE_TEXT.measure(line, fonts()) <= column.width - PADS_X + 1e-9);
    }
}

#[test]
fn row_height_is_its_tallest_cell() {
    let csv = "name,sector,ring,description\nA,Tools,Adopt,\"one\ntwo\nthree\"\n";
    let block = block(csv, Table::Radar);
    let part = block.parts.first().expect("a row");
    let three = TABLE_LEADING + TABLE_LEADING + TABLE_LEADING;
    assert!((part.height - (TABLE_PAD_Y + TABLE_PAD_Y + three)).abs() < 1e-9);
}

mod folding;
mod pages;
