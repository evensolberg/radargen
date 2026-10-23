//! The table's pages and rows: splitting, pagination, headings and rules.

use super::*;

#[test]
fn pages_never_split_a_row() {
    let block = block(&long_csv(300), Table::Radar);
    let mut seen = Vec::new();
    for page in block.pages() {
        let height: f64 = block
            .parts
            .get(page.parts.clone())
            .unwrap_or_default()
            .iter()
            .map(|p| p.height)
            .sum();
        assert!(height <= block.room + 1e-9, "{height} > {}", block.room);
        seen.extend(
            block
                .parts
                .get(page.parts.clone())
                .unwrap_or_default()
                .iter()
                .map(|p| p.row),
        );
    }
    assert_eq!(seen, (0..300).collect::<Vec<_>>());
    assert!(block.pages().len() > 3);
    assert_eq!(block.tall_rows().len(), 0);
}

#[test]
fn a_row_exactly_filling_the_room_is_whole() {
    let lines = 20;
    let description: Vec<String> = (0..lines).map(|i| format!("line {i}")).collect();
    let csv = format!(
        "name,sector,ring,description\nA,Tools,Adopt,\"{}\"\n",
        description.join("\n")
    );
    let shape = shape(&csv);
    let table = published(&shape.0, &shape.1, Table::Radar);
    let probe = TableBlock::new(&table, letter().0, 10_000.0, fonts());
    let row = probe.parts.first().map_or(0.0, |p| p.height);
    let head = 10_000.0 - probe.room;
    let exact = TableBlock::new(&table, letter().0, head + row, fonts());
    assert_eq!(exact.parts.len(), 1);
    assert_eq!(exact.tall_rows().len(), 0);
    let short = TableBlock::new(&table, letter().0, head + row - 1e-9, fonts());
    assert_eq!(short.parts.len(), 2);
    // A row exactly filling a page, then another: the next opens page 2.
    let two = format!("{csv}B,Tools,Adopt,d\n");
    let shape = self::shape(&two);
    let table = published(&shape.0, &shape.1, Table::Radar);
    let pair = TableBlock::new(&table, letter().0, head + row, fonts());
    assert_eq!(pair.parts.len(), 2);
    assert_eq!(pair.pages().len(), 2);
}

#[test]
fn later_pages_head_with_sector_and_numbers() {
    let block = block(&long_csv(300), Table::Radar);
    let pages = all_pages(&block);
    let second = block.pages().get(1).expect("two pages");
    let numbers: Vec<usize> = block
        .parts
        .get(second.parts.clone())
        .unwrap_or_default()
        .iter()
        .filter_map(|p| p.number)
        .collect();
    let sector = &block.parts.get(second.parts.start).expect("a row").sector;
    let page = texts(pages.get(1).expect("page 2"));
    assert_eq!(
        page.first().map(lines_of),
        Some(vec![RADAR_HEADING.to_owned()])
    );
    assert_eq!(
        page.get(1).map(lines_of),
        Some(vec![format!(
            "(continued): {sector}, {}\u{2013}{}",
            numbers.first().expect("numbers"),
            numbers.last().expect("numbers")
        )])
    );
    let headings: Vec<String> = page
        .iter()
        .skip(2)
        .take(5)
        .map(|t| lines_of(t).join(" "))
        .collect();
    assert_eq!(headings, ["#", "name", "Sector", "ring", "description"]);
    let first = texts(pages.first().expect("page 1"));
    assert_eq!(first.get(1).map(lines_of), Some(vec!["#".to_owned()]));
}

#[test]
fn a_page_of_unplotted_rows_has_no_number_range() {
    let rows = (0..80)
        .map(|i| format!("gone {i:02},Tools,Elsewhere,\"a\nb\nc\"\n"))
        .collect::<Vec<_>>()
        .concat();
    let block = block(&format!("name,sector,ring,description\n{rows}"), Table::All);
    let pages = all_pages(&block);
    let page = texts(pages.get(1).expect("two pages"));
    assert_eq!(
        page.get(1).map(lines_of),
        Some(vec!["(continued): Tools".to_owned()])
    );
}

#[test]
fn a_row_taller_than_a_page_splits_at_lines_marked_twice() {
    let tall = (0..3_000)
        .map(|i| format!("w{i}"))
        .collect::<Vec<_>>()
        .join(" ");
    let csv = format!(
        "name,sector,ring,description\nAlpha,Tools,Adopt,d\nBeta,Tools,Adopt,{tall}\nGamma,Tools,Adopt,d\n"
    );
    let block = block(&csv, Table::Radar);
    assert_eq!(block.tall_rows(), [(3, "Beta".to_owned())]);
    let parts: Vec<&Part> = block.parts.iter().filter(|p| p.row == 1).collect();
    assert!(parts.len() >= 2);
    for part in &parts {
        assert!(part.height <= block.room + 1e-9);
    }
    assert_eq!(block.parts.iter().filter(|p| p.row != 1).count(), 2);
    let marks = |part: &Part| {
        part.under
            .iter()
            .map(|u| u.words.clone())
            .collect::<Vec<_>>()
    };
    let (first, last) = (
        parts.first().expect("a part"),
        parts.last().expect("a part"),
    );
    assert_eq!(marks(first), [CONTINUES]);
    assert!(!first.ruled && last.ruled);
    for (k, later) in parts.iter().enumerate().skip(1) {
        assert_eq!(later.cells.first().map(|c| c.words.as_str()), Some("Beta"));
        let expected: &[&str] = if k + 1 == parts.len() {
            &[CONTINUED_MARK]
        } else {
            &[CONTINUED_MARK, CONTINUES]
        };
        assert_eq!(marks(later), expected, "part {k}");
        assert_eq!(later.number, first.number);
    }
    // The first part draws without a closing rule.
    let page = block
        .pages()
        .iter()
        .find(|p| {
            p.parts
                .contains(&block.parts.iter().position(|p| p.row == 1).expect("Beta"))
        })
        .expect("Beta's page");
    let nodes = block.render(page, Point::ORIGIN, fonts());
    let tops: Vec<f64> = rules(&nodes).iter().map(|(y, _)| *y).collect();
    let beta_bottom = block.head_height
        + block
            .parts
            .get(page.parts.clone())
            .unwrap_or_default()
            .iter()
            .take_while(|p| p.row != 1)
            .map(|p| p.height)
            .sum::<f64>()
        + first.height;
    assert!(
        tops.iter().all(|y| (y - beta_bottom).abs() > 1e-9),
        "{tops:?}"
    );
    let words: Vec<String> = parts
        .iter()
        .filter_map(|p| p.cells.get(3).map(|c| c.lines.join(" ")))
        .collect();
    assert_eq!(words.join(" "), tall);
}

#[test]
fn two_tall_rows_are_listed_in_order_and_a_tall_name_terminates() {
    let tall = "word ".repeat(3_000);
    let name = "name ".repeat(1_500);
    let csv =
        format!("name,sector,ring,description\nAlpha,Tools,Adopt,{tall}\nb{name},Tools,Adopt,d\n");
    let block = block(&csv, Table::Radar);
    let lines: Vec<usize> = block.tall_rows().iter().map(|(line, _)| *line).collect();
    assert_eq!(lines, [2, 3]);
    // The long name cannot split; its row is drawn whole, past the room.
    assert_eq!(block.parts.iter().filter(|p| p.row == 1).count(), 1);
    assert!(
        block
            .parts
            .iter()
            .filter(|p| p.row == 0)
            .all(|p| p.height <= block.room + 1e-9)
    );
}

#[test]
fn rules_mark_sectors_and_the_unplotted_boundary() {
    let csv = "name,sector,ring,description\nA,Tools,Adopt,d\nB,Tools,Adopt,d\nC,Platforms,Adopt,d\nD,Platforms,Gone,d\n";
    let block = block(csv, Table::All);
    let nodes = block.render(
        block.pages().first().expect("a page"),
        Point::ORIGIN,
        fonts(),
    );
    let heavy = rules(&nodes)
        .iter()
        .filter(|(_, w)| w.to_bits() == TABLE_RULE_HEAVY.to_bits())
        .count();
    assert_eq!(heavy, 3, "header, sector change, unplotted boundary");
    let only = block_of(
        &shape("name,sector,ring,description\nA,Tools,Gone,d\nB,Tools,Gone,d\n"),
        Table::All,
    );
    let nodes = only.render(
        only.pages().first().expect("a page"),
        Point::ORIGIN,
        fonts(),
    );
    let heavy = rules(&nodes)
        .iter()
        .filter(|(_, w)| w.to_bits() == TABLE_RULE_HEAVY.to_bits())
        .count();
    assert_eq!(heavy, 1, "the header's alone");
}

#[test]
fn nothing_plotted_spans_every_column() {
    let block = block(
        "name,sector,ring,description\nA,Tools,Gone,d\n",
        Table::Radar,
    );
    let nodes = block.render(
        block.pages().first().expect("a page"),
        Point::ORIGIN,
        fonts(),
    );
    let said = texts(&nodes)
        .iter()
        .filter(|t| lines_of(t) == [NOTHING_PLOTTED])
        .count();
    assert_eq!(said, 1);
    let headers_only = block_of(&shape("name,sector,ring,description\n"), Table::All);
    assert_eq!(headers_only.pages().len(), 1);
    assert_eq!(headers_only.parts.len(), 0);
}

#[test]
fn hostile_cells_render() {
    let csv = format!("{HOSTILE}中文,Tools,Adopt,中文 text,,,,,\n");
    let block = block(&csv, Table::All);
    let drawn: Vec<String> = all_pages(&block)
        .iter()
        .flat_map(|p| texts(p))
        .map(|t| words_of(&t))
        .collect();
    for expected in ["Ærø (Q3) \\ test", "中文", "中文 text", "a<b & \"c\""] {
        assert!(drawn.iter().any(|w| w == expected), "{expected}");
    }
}

#[test]
fn everything_inside_its_column() {
    for (csv, table) in [
        (long_csv(300), Table::Radar),
        (HOSTILE.to_owned(), Table::All),
    ] {
        let block = block(&csv, table);
        let (width, _) = letter();
        for page in all_pages(&block) {
            for node in &page {
                let Some(rect) = bounds(node, fonts()) else {
                    continue;
                };
                // A rule's stroke reaches half its width past its ends.
                let reach = TABLE_RULE_HEAVY / 2.0 + 1e-9;
                assert!(
                    rect.left >= -reach && rect.right <= width + reach,
                    "{rect:?}"
                );
            }
        }
        for part in &block.parts {
            for (cell, column) in part.cells.iter().zip(block.columns.iter().skip(1)) {
                let style = if column.kind == Some(CellKind::Name) {
                    TABLE_NAME
                } else {
                    TABLE_TEXT
                };
                for line in &cell.lines {
                    assert!(
                        style.measure(line, fonts()) <= column.width - PADS_X + 1e-9,
                        "{line}"
                    );
                }
            }
        }
    }
}

#[test]
fn the_table_lays_out_in_linear_time() {
    crate::test_timing::assert_linear(
        150,
        |n| shape(&long_csv(n)),
        |shape| {
            let _ = block_of(shape, Table::Radar);
        },
    );
}

/// Final review (Rust 1): a heading of a hundred words and a row taller
/// than a page end: the heading is cut to three lines, and the split
/// takes lines every part.
#[test]
fn a_tall_heading_and_a_split_row_end() {
    let heading: Vec<String> = (0..100).map(|i| format!("word{i}")).collect();
    let tall = "lorem ".repeat(3_000);
    let csv = format!(
        "name,sector,ring,description,\"{}\"\nAlpha,Tools,Adopt,\"{tall}\",x\nBeta,Tools,Adopt,short,y\n",
        heading.join(" ")
    );
    let block = block(&csv, Table::All);
    assert!(block.columns.iter().all(|c| c.heading.len() <= 3));
    assert!(
        block
            .columns
            .iter()
            .any(|c| c.heading.last().is_some_and(|l| l.ends_with('\u{2026}')))
    );
    assert!(block.parts.len() < 200, "{}", block.parts.len());
    assert_eq!(block.tall_rows().len(), 1);
}

/// The maintainer's ruling (2026-10-23): qualifier, status and the dates
/// are a muted line under the name, not columns.
#[test]
fn short_columns_fold_under_the_name() {
    let csv = "name,sector,ring,description,qualifier,status,last_affirmed,exit_date\nAlpha,Tools,Adopt,d,conditional,New,2026-06-01,\nBeta,Tools,Adopt,d,,,,\n";
    let block = block(csv, Table::Radar);
    let headings: Vec<String> = block.columns.iter().map(|c| c.heading.join(" ")).collect();
    assert_eq!(headings, ["#", "name", "Sector", "ring", "description"]);
    let alpha = block.parts.first().expect("Alpha");
    let under: Vec<&str> = alpha.under.iter().map(|u| u.words.as_str()).collect();
    assert_eq!(
        under,
        ["conditional \u{b7} New \u{b7} last_affirmed 2026-06-01"]
    );
    assert_eq!(
        block.parts.get(1).map(|p| p.under.len()),
        Some(0),
        "nothing to fold"
    );
    let nodes = block.render(
        block.pages().first().expect("a page"),
        Point::ORIGIN,
        fonts(),
    );
    let muted = TextPaint::Fill(Paint::Role(Role::Neutral(Neutral::TextMuted)));
    assert!(texts(&nodes).iter().any(|t| matches!(t, Node::Text { ink, words, .. } if *ink == muted && words.starts_with("conditional"))));
}

/// Final review (UI 1): a word in a name or label column is never broken,
/// on the PRD example, the hostile CSV or a crowded `all` table.
#[test]
fn label_columns_never_break_a_word() {
    let others = (0..6)
        .map(|i| format!(",o{i}"))
        .collect::<Vec<_>>()
        .concat();
    let prose = ",Prose that runs on for a while.".repeat(6);
    let crowded = format!(
        "name,sector,ring,description{others}\nKubernetes Operators,Platforms,Assess,d{prose}\n"
    );
    for (csv, table) in [
        (PRD_EXAMPLE, Table::Radar),
        (HOSTILE, Table::All),
        (crowded.as_str(), Table::All),
    ] {
        let block = block(csv, table);
        for part in &block.parts {
            for (cell, column) in part.cells.iter().zip(block.columns.iter().skip(1)) {
                if matches!(column.kind, Some(CellKind::Name | CellKind::Label)) {
                    assert_eq!(
                        words(&cell.lines.join(" ")),
                        cell.words,
                        "{:?}",
                        column.heading
                    );
                }
            }
        }
    }
}

/// A long run of CJK with no spaces breaks inside the column, every line
/// within it.
#[test]
fn a_long_cjk_run_breaks_inside_its_column() {
    let csv = format!(
        "name,sector,ring,description\nA,Tools,Adopt,{}\n",
        "\u{4e2d}".repeat(400)
    );
    let block = block(&csv, Table::Radar);
    let column = block.columns.get(4).expect("description");
    let cell = block
        .parts
        .first()
        .and_then(|p| p.cells.get(3))
        .expect("a cell");
    assert!(cell.lines.len() > 1);
    for line in &cell.lines {
        assert!(TABLE_TEXT.measure(line, fonts()) <= column.width - PADS_X + 1e-9);
    }
}

/// Every page fits its room on A4 as on Letter.
#[test]
fn pages_never_split_a_row_on_a4() {
    let a4 = (
        1_122.519_685_039_370_1 - PAPER_MARGIN - PAPER_MARGIN,
        793.700_787_401_574_8 - PAPER_MARGIN - PAPER_FOOT_ROOM,
    );
    let shape = shape(&long_csv(200));
    let block = TableBlock::new(
        &published(&shape.0, &shape.1, Table::Radar),
        a4.0,
        a4.1,
        fonts(),
    );
    for page in block.pages() {
        let height: f64 = block
            .parts
            .get(page.parts.clone())
            .unwrap_or_default()
            .iter()
            .map(|p| p.height)
            .sum();
        assert!(height <= block.room + 1e-9);
    }
    assert_eq!(block.tall_rows().len(), 0);
}

/// Cramped, the prose shares the room by how much each column holds.
#[test]
fn cramped_prose_shares_by_content() {
    let others = (0..10)
        .map(|i| format!(",o{i}"))
        .collect::<Vec<_>>()
        .concat();
    let long = format!(
        ",{}",
        "Much more prose in this column than in any other. ".repeat(6)
    );
    let short = ",Two short sentences here. And one more.".repeat(9);
    let csv = format!("name,sector,ring,description{others}\nAlpha,Tools,Adopt,d{long}{short}\n");
    let block = block(&csv, Table::All);
    assert!(block.cramped().is_some());
    assert!(width_of(&block, "o0") > width_of(&block, "o1"));
}

/// Final review (architecture 3, 11): the header row nests row, cell,
/// text as body rows do, the number heading read as a word; the heading
/// is a heading on the first page only, a repeat after.
#[test]
fn headers_nest_and_headings_repeat_plainly() {
    let block = block(&long_csv(300), Table::Radar);
    let first = block.render(
        block.pages().first().expect("a page"),
        Point::ORIGIN,
        fonts(),
    );
    let Some(Node::Group(header)) = first.get(1) else {
        panic!("a header row")
    };
    assert_eq!(header.children.len(), block.columns.len());
    assert!(
        header
            .children
            .iter()
            .all(|c| matches!(c, Node::Group(g) if g.children.len() == 1))
    );
    assert!(
        texts(&first)
            .iter()
            .any(|t| words_of(t) == crate::published::NUMBER_SPOKEN)
    );
    let heading = |nodes: &[Node]| match nodes.first() {
        Some(Node::Group(g)) => g.semantics.clone(),
        _ => Semantics::None,
    };
    assert!(matches!(heading(&first), Semantics::Heading(_)));
    let second = block.render(
        block.pages().get(1).expect("page 2"),
        Point::ORIGIN,
        fonts(),
    );
    assert_eq!(heading(&second), Semantics::None);
}

/// FR-106's empty table: its one line starts at the first column's
/// padding and runs across the table.
#[test]
fn nothing_plotted_starts_at_the_padding() {
    let block = block(
        "name,sector,ring,description\nA,Tools,Gone,d\n",
        Table::Radar,
    );
    let nodes = block.render(
        block.pages().first().expect("a page"),
        Point::ORIGIN,
        fonts(),
    );
    let said = texts(&nodes)
        .into_iter()
        .find(|t| lines_of(t) == [NOTHING_PLOTTED])
        .expect("the line");
    let Node::Text { origin, .. } = said else {
        panic!("text")
    };
    assert!((origin.x - TABLE_PAD_X).abs() < 1e-9);
}
