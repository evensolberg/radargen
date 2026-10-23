//! The table's columns: what folds under the name, what stacks under the
//! sector, and how wide the labels grow.

use super::*;

/// The maintainer's ruling (2026-10-23): qualifier, status and the dates
/// are a muted line under the name, not columns.
#[test]
fn short_columns_fold_under_the_name() {
    let csv = "name,sector,ring,description,qualifier,status,last_affirmed,exit_date\nAlpha,Tools,Adopt,d,conditional,New,2026-06-01,\nBeta,Tools,Adopt,d,,,,\n";
    let block = block(csv, Table::Radar);
    let headings: Vec<String> = block.columns.iter().map(|c| c.heading.join(" ")).collect();
    assert_eq!(headings, ["#", "name", "Sector ring", "description"]);
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

/// Final review after simplify: a folded date's heading is a word under
/// the name too, so short names never break it.
#[test]
fn a_folded_dates_heading_is_never_broken() {
    let csv = "name,sector,ring,description,last_affirmed\nGo,Tools,Adopt,d,2026-06-01\nC,Tools,Trial,d,2026-06-02\n";
    let block = block(csv, Table::Radar);
    for part in &block.parts {
        for under in &part.under {
            assert_eq!(
                words(&under.lines.join(" ")),
                under.words,
                "{:?}",
                under.lines
            );
        }
    }
}

/// The maintainer (2026-10-23): sector and ring share one column, the
/// ring muted under the sector, and the header reads both.
#[test]
fn sector_and_ring_stack_in_one_column() {
    let csv = "name,sector,ring,description\nAlpha,Tools,Adopt,d\n";
    let block = block(csv, Table::Radar);
    let headings: Vec<String> = block.columns.iter().map(|c| c.heading.join(" ")).collect();
    assert_eq!(headings, ["#", "name", "Sector ring", "description"]);
    let nodes = block.render(
        block.pages().first().expect("a page"),
        Point::ORIGIN,
        fonts(),
    );
    let muted = TextPaint::Fill(Paint::Role(Role::Neutral(Neutral::TextMuted)));
    let primary = TextPaint::Fill(Paint::Role(Role::Neutral(Neutral::TextPrimary)));
    let drawn = texts(&nodes);
    let sector = drawn
        .iter()
        .find(|t| words_of(t) == "Tools")
        .expect("the sector");
    let ring = drawn
        .iter()
        .find(|t| words_of(t) == "Adopt")
        .expect("the ring");
    assert!(matches!(sector, Node::Text { ink, .. } if *ink == primary));
    assert!(matches!(ring, Node::Text { ink, .. } if *ink == muted));
    let (Node::Text { origin: s, .. }, Node::Text { origin: r, .. }) = (sector, ring) else {
        panic!("texts")
    };
    assert!(
        (s.x - r.x).abs() < 1e-9 && r.y > s.y,
        "the ring under the sector"
    );
}

/// The maintainer (2026-10-23): the domain folds under the name with the
/// short columns, first.
#[test]
fn the_domain_folds_first_under_the_name() {
    let csv = "name,sector,ring,description,status,domain\nAlpha,Tools,Adopt,d,New,database\n";
    let block = block(csv, Table::Radar);
    assert!(
        block
            .columns
            .iter()
            .all(|c| c.heading.join(" ") != "domain")
    );
    let under: Vec<&str> = block
        .parts
        .first()
        .expect("Alpha")
        .under
        .iter()
        .map(|u| u.words.as_str())
        .collect();
    assert_eq!(under, ["database \u{b7} New"]);
}

/// The maintainer (2026-10-23): the line folded under the name widens the
/// name column, up to the name cap, so it wraps no more than it must.
#[test]
fn the_folded_line_widens_the_name_column() {
    let short = "name,sector,ring,description,status\nGo,Tools,Adopt,d,Moved Out Again\n";
    let block_short = block(short, Table::Radar);
    let line = TABLE_TEXT.measure("Moved Out Again", fonts());
    assert!((width_of(&block_short, "name") - (line + PADS_X)).abs() < 1e-9);
    let long = "name,sector,ring,description,status,domain\nGo,Tools,Adopt,d,New,Container Orchestration and Scheduling\n";
    let block_long = block(long, Table::Radar);
    assert!((width_of(&block_long, "name") - TABLE_NAME_CAP).abs() < 1e-9);
}

/// Review of the stacked column: a split row draws its ring once, on its
/// first part, and that part's height counts it.
#[test]
fn a_split_row_draws_its_ring_once() {
    let tall = (0..3_000)
        .map(|i| format!("w{i}"))
        .collect::<Vec<_>>()
        .join(" ");
    let csv = format!("name,sector,ring,description\nBeta,Tools,Adopt,{tall}\n");
    let block = block(&csv, Table::Radar);
    let parts = &block.parts;
    assert!(parts.len() >= 2);
    let rings: Vec<Option<&str>> = parts
        .iter()
        .map(|p| p.beneath.as_ref().map(|(_, l)| l.words.as_str()))
        .collect();
    assert_eq!(rings.first(), Some(&Some("Adopt")));
    assert!(rings.iter().skip(1).all(Option::is_none), "{rings:?}");
    let first = parts.first().expect("a part");
    let sector = first.cells.get(1).map_or(0, |c| c.lines.len());
    assert!(first.height >= height_of(sector + 1) - 1e-9);
}

/// Review of the name cap: a label other than the name keeps the 168 cap.
#[test]
fn a_wide_sector_is_capped_at_the_label_cap() {
    let sector = "A sector named at length so it cannot fit one label column";
    let csv = format!("name,sector,ring,description\nA,{sector},Adopt,d\n");
    let block = block(&csv, Table::Radar);
    assert!((width_of(&block, "Sector ring") - TABLE_LABEL_CAP).abs() < 1e-9);
}
