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
        .take(4)
        .map(|t| lines_of(t).join(" "))
        .collect();
    assert_eq!(headings, ["#", "name", "Sector ring", "description"]);
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
        .filter_map(|p| p.cells.get(2).map(|c| c.lines.join(" ")))
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
        assert_inside_the_page(&block);
        assert_lines_inside_their_columns(&block);
    }
}

/// Every drawn node of `block` within the page's width; a rule's stroke
/// reaches half its width past its ends.
fn assert_inside_the_page(block: &TableBlock) {
    let (width, _) = letter();
    let reach = TABLE_RULE_HEAVY / 2.0 + 1e-9;
    let rects = all_pages(block)
        .iter()
        .flatten()
        .filter_map(|node| bounds(node, fonts()))
        .collect::<Vec<_>>();
    for rect in rects {
        assert!(
            rect.left >= -reach && rect.right <= width + reach,
            "{rect:?}"
        );
    }
}

/// Every cell's lines of `block` within its column's text width.
fn assert_lines_inside_their_columns(block: &TableBlock) {
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

/// A long run of CJK with no spaces breaks inside the column, every line
/// within it.
#[test]
fn a_long_cjk_run_breaks_inside_its_column() {
    let csv = format!(
        "name,sector,ring,description\nA,Tools,Adopt,{}\n",
        "\u{4e2d}".repeat(400)
    );
    let block = block(&csv, Table::Radar);
    let column = block.columns.get(3).expect("description");
    let cell = block
        .parts
        .first()
        .and_then(|p| p.cells.get(2))
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
    // The table's first child is the header row (PR 21c-1).
    let Some(Node::Group(table)) = first.get(1) else {
        panic!("the table")
    };
    let Some(Node::Group(header)) = table.children.first() else {
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
    // A repeat is page furniture to a tagged PDF (PR 21c-1).
    assert_eq!(heading(&second), Semantics::Pagination);
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

/// The first table whose second page opens on a change of `key`, built
/// from `head` rows then `tail` rows for each count of `head` rows.
fn opening_on(key: fn(&Part) -> String, row: fn(usize, bool) -> String) -> TableBlock {
    (1..80)
        .map(|k| {
            let rows: String = (0..k + 40).map(|i| row(i, i < k)).collect();
            block(&format!("name,sector,ring,description\n{rows}"), Table::All)
        })
        .find(|block| {
            let start = block.pages().get(1).map(|p| p.parts.start);
            start.is_some_and(|s| {
                let at = |i: usize| block.parts.get(i).map(key);
                s > 0 && at(s - 1) != at(s)
            })
        })
        .expect("a count whose page break falls on the change")
}

/// The rules around a page that opens on a change: the page before ends
/// on a plain rule, and the page opens under the header row's heavy rule
/// with no second heavy rule.
fn assert_opens_cleanly(block: &TableBlock) {
    let pages = all_pages(block);
    let before = rules(pages.first().expect("page 1"));
    assert_eq!(before.last().map(|(_, w)| *w), Some(TABLE_RULE));
    let after = rules(pages.get(1).expect("page 2"));
    let heavy = after.iter().filter(|(_, w)| *w == TABLE_RULE_HEAVY).count();
    let sectors: std::collections::BTreeSet<&str> = block
        .parts
        .get(block.pages().get(1).expect("page 2").parts.clone())
        .unwrap_or_default()
        .iter()
        .map(|p| p.sector.as_str())
        .collect();
    // The header's, and one more only where the page itself changes.
    assert!(heavy <= sectors.len(), "{after:?}");
}

/// rg-p82 (QA 9): a change of sector that falls at a page top leaves no
/// heavy rule at the foot of the page before, and the page names the
/// new sector.
#[test]
fn a_sector_change_at_a_page_top() {
    let block = opening_on(
        |p| p.sector.clone(),
        |i, head| {
            let sector = if head { "Alpha" } else { "Beta" };
            format!("i{i:03},{sector},Adopt,\"a\nb\nc\"\n")
        },
    );
    assert_opens_cleanly(&block);
    let page = texts(all_pages(&block).get(1).expect("page 2"));
    let heading = page.get(1).map(lines_of).unwrap_or_default().join(" ");
    assert!(heading.contains("Beta"), "{heading}");
}

/// rg-p82 (QA 9): the start of the unplotted rows at a page top: the
/// page before ends on a plain rule, and the page holds no numbers.
#[test]
fn the_unplotted_boundary_at_a_page_top() {
    let block = opening_on(
        |p| p.number.is_some().to_string(),
        |i, head| {
            let ring = if head { "Adopt" } else { "Elsewhere" };
            format!("i{i:03},Tools,{ring},\"a\nb\nc\"\n")
        },
    );
    assert_opens_cleanly(&block);
    let second = block.pages().get(1).expect("page 2");
    let parts = block.parts.get(second.parts.clone()).unwrap_or_default();
    assert!(parts.iter().all(|p| p.number.is_none()));
    let page = texts(all_pages(&block).get(1).expect("page 2"));
    let heading = page.get(1).map(lines_of).unwrap_or_default().join(" ");
    assert_eq!(heading, "(continued): Tools", "no number range");
}

/// Every group's semantics under `node`, depth first, with the depth.
fn outline(node: &Node, depth: usize, out: &mut Vec<(usize, Semantics)>) {
    if let Node::Group(group) = node {
        out.push((depth, group.semantics.clone()));
        for child in &group.children {
            outline(child, depth + 1, out);
        }
    }
}

/// PR 21c-1: a table page is the table, its header row of column headers
/// on the first page only, each row keyed by its published row with the
/// name as its row header; a split row's later parts repeat their number
/// and name as furniture, and the marks are furniture everywhere.
#[test]
fn the_table_is_structured_for_tagging() {
    let tall = (0..3_000)
        .map(|i| format!("w{i}"))
        .collect::<Vec<_>>()
        .join(" ");
    let csv =
        format!("name,sector,ring,description\nAlpha,Tools,Adopt,d\nBeta,Tools,Adopt,{tall}\n");
    let block = block(&csv, Table::Radar);
    let pages = all_pages(&block);
    assert!(pages.len() >= 2);
    for (k, page) in pages.iter().enumerate() {
        let mut groups = Vec::new();
        for node in page {
            outline(node, 0, &mut groups);
        }
        let tables = groups
            .iter()
            .filter(|(_, s)| *s == Semantics::Structure(Structure::Table))
            .count();
        assert_eq!(tables, 1, "page {k}");
        let headers = groups
            .iter()
            .filter(|(_, s)| *s == Semantics::Structure(Structure::Header(Scope::Column)))
            .count();
        let header_rows = groups
            .iter()
            .filter(|(_, s)| *s == Semantics::Structure(Structure::HeaderRow))
            .count();
        if k == 0 {
            assert_eq!((header_rows, headers), (1, block.columns.len()));
        } else {
            assert_eq!(
                (header_rows, headers),
                (0, 0),
                "page {k} repeats as furniture"
            );
            assert!(groups.iter().any(|(_, s)| *s == Semantics::Pagination));
        }
        // Every row has as many cells as there are columns.
        for (at, (depth, semantics)) in groups.iter().enumerate() {
            if !matches!(semantics, Semantics::Structure(Structure::Row(_))) {
                continue;
            }
            let cells = groups
                .iter()
                .skip(at + 1)
                .take_while(|(d, _)| d > depth)
                .filter(|(d, s)| {
                    *d == depth + 1
                        && matches!(
                            s,
                            Semantics::Structure(
                                Structure::Cell { .. } | Structure::Header(Scope::Row)
                            )
                        )
                })
                .count();
            assert_eq!(cells, block.columns.len(), "page {k}");
        }
    }
    let rows: Vec<usize> = block.parts.iter().map(|p| p.row).collect();
    assert!(
        rows.windows(2).any(|w| matches!(w, [a, b] if a == b)),
        "Beta is split"
    );
}

/// PR 21c-1: FR-106's one row spans every column.
#[test]
fn nothing_plotted_spans_as_one_cell() {
    let block = block("name,sector,ring,description\n", Table::Radar);
    let page = all_pages(&block).into_iter().next().expect("a page");
    let mut groups = Vec::new();
    for node in &page {
        outline(node, 0, &mut groups);
    }
    let span = NonZeroUsize::new(block.columns.len());
    assert!(
        groups
            .iter()
            .any(|(_, s)| *s == Semantics::Structure(Structure::Cell { column: 0, span }))
    );
}

/// Review of 21c-1: a mark is furniture because the split made it, not
/// because a value reads like one; a qualifier of "(continued)" on a row
/// that never splits stays content.
#[test]
fn a_value_that_reads_like_a_mark_stays_content() {
    let csv = "name,sector,ring,description,qualifier\nAlpha,Tools,Adopt,d,(continued)\n";
    let block = block(csv, Table::Radar);
    let page = all_pages(&block).into_iter().next().expect("a page");
    let mut groups = Vec::new();
    for node in &page {
        outline(node, 0, &mut groups);
    }
    assert!(
        groups.iter().all(|(_, s)| *s != Semantics::Pagination),
        "{groups:?}"
    );
}

/// Every row group under `nodes`, in document order.
fn row_groups(nodes: &[Node]) -> Vec<&crate::svg::Group> {
    fn walk<'a>(node: &'a Node, out: &mut Vec<&'a crate::svg::Group>) {
        if let Node::Group(group) = node {
            if matches!(group.semantics, Semantics::Structure(Structure::Row(_))) {
                out.push(group);
            }
            group.children.iter().for_each(|child| walk(child, out));
        }
    }
    let mut out = Vec::new();
    for node in nodes {
        walk(node, &mut out);
    }
    out
}

/// Review of 21c-1: a split row renders as one key on every part; its
/// cells run column by column with the name as the row header; on later
/// parts the number and name cells hold furniture only, and the marks are
/// furniture on the first part too.
#[test]
fn a_split_row_repeats_only_as_furniture() {
    let tall = (0..3_000)
        .map(|i| format!("w{i}"))
        .collect::<Vec<_>>()
        .join(" ");
    let csv =
        format!("name,sector,ring,description\nAlpha,Tools,Adopt,d\nBeta,Tools,Adopt,{tall}\n");
    let block = block(&csv, Table::Radar);
    let pages = all_pages(&block);
    let rows: Vec<&crate::svg::Group> = pages.iter().flat_map(|p| row_groups(p)).collect();
    let beta: Vec<&&crate::svg::Group> = rows
        .iter()
        .filter(|g| g.semantics == Semantics::Structure(Structure::Row(1)))
        .collect();
    assert!(beta.len() >= 2, "{}", beta.len());
    let name = block
        .columns
        .iter()
        .position(|c| c.kind == Some(CellKind::Name))
        .expect("a name column");
    for row in &rows {
        let cells: Vec<Semantics> = row
            .children
            .iter()
            .map(|c| match c {
                Node::Group(g) => g.semantics.clone(),
                _ => Semantics::None,
            })
            .collect();
        let expected: Vec<Semantics> = (0..block.columns.len())
            .map(|column| {
                Semantics::Structure(if column == name {
                    Structure::Header(Scope::Row)
                } else {
                    Structure::Cell { column, span: None }
                })
            })
            .collect();
        assert_eq!(cells, expected);
    }
    let furniture_only = |cell: &Node| match cell {
        Node::Group(g) => g
            .children
            .iter()
            .all(|c| matches!(c, Node::Group(inner) if inner.semantics == Semantics::Pagination)),
        _ => false,
    };
    for later in beta.iter().skip(1) {
        let number = later.children.first().expect("the number cell");
        let header = later.children.get(name).expect("the name cell");
        assert!(furniture_only(number) && furniture_only(header));
    }
    let first_name = beta
        .first()
        .and_then(|row| row.children.get(name))
        .expect("Beta's first name cell");
    let Node::Group(first_name) = first_name else {
        panic!("a cell")
    };
    assert!(
        first_name
            .children
            .iter()
            .any(|c| matches!(c, Node::Group(g) if g.semantics == Semantics::Pagination))
    );
    assert!(!furniture_only(
        beta.first()
            .and_then(|r| r.children.first())
            .expect("a number")
    ));
}

/// 21c-2 review: a part of a split row reads its lines as the cell's words
/// do, so a token broken mid-word inside a part stays one word (D-88).
#[test]
fn a_parts_words_keep_a_broken_token_whole() {
    let token = "x".repeat(300);
    let words: Vec<String> = (0..1_500).map(|i| format!("w{i}")).collect();
    let csv = format!(
        "name,sector,ring,description\nBeta,Tools,Adopt,{} {token} {}\n",
        words.join(" "),
        words.join(" ")
    );
    let block = block(&csv, Table::Radar);
    let parts: Vec<&Part> = block.parts.iter().collect();
    assert!(parts.len() >= 2);
    let holding = parts
        .iter()
        .filter_map(|p| p.cells.get(2))
        .find(|cell| cell.lines.iter().any(|l| l.starts_with("xx")))
        .expect("a part holds the token");
    assert!(
        holding.words.split(' ').any(|w| w.len() >= token.len() / 2),
        "the token stays whole: {:?}",
        holding
            .words
            .split(' ')
            .filter(|w| w.starts_with('x'))
            .collect::<Vec<_>>()
    );
    // Every part's words, rejoined, are the cell's.
    let all: Vec<&str> = parts
        .iter()
        .filter_map(|p| p.cells.get(2))
        .map(|c| c.words.as_str())
        .collect();
    assert!(all.concat().contains(&token));
}
