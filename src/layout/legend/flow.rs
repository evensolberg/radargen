//! The legend broken into pieces for paper (FR-111): single columns of
//! bounded height, breaking only between units, each piece that starts
//! inside a sector or ring repeating that heading, marked continued.

use std::ops::Range;

use super::layout::{
    Entry, GroupStart, LEGEND_TITLE, Layout, RING_TIER, SECTOR_TIER, TITLE_TIER, Tier, heading,
    height, swatch,
};
use super::panel_size;
use crate::layout::paginate::paginate;
use crate::style::{
    CONTINUED, LEGEND_FIRST_GROUP, LEGEND_HEADING_BASELINE, LEGEND_RING_TO_ROW,
    LEGEND_SECTOR_TO_RING, SWATCH_RAISE,
};
use crate::text::Fonts;

/// The content width every piece of `layout` gets: its own, widened to
/// fit any heading a piece may repeat with [`CONTINUED`] after it.
pub(super) fn widest(layout: &Layout, fonts: &Fonts) -> f64 {
    let continued = |tier: Tier, words: &str| {
        heading(tier, &format!("{words}{CONTINUED}"), 0.0, fonts).right_edge(fonts)
    };
    layout
        .entries
        .iter()
        .filter_map(|entry| match entry {
            Entry::Heading { run, level, .. } => [SECTOR_TIER, RING_TIER]
                .into_iter()
                .find(|tier| tier.level == *level)
                .map(|tier| continued(tier, &run.words)),
            _ => None,
        })
        .chain([continued(TITLE_TIER, LEGEND_TITLE)])
        .fold(layout.width, f64::max)
}

/// `layout` cut into pieces `width` wide: the first at most `first` tall
/// as a panel, each later one at most `rest`. A piece `k` for which
/// `opens_page(k)` holds opens with *Legend (continued)*.
///
/// Entries are placed in units (see [`units`]); a unit that does not fit
/// closes the piece once it holds a row, and goes in anyway when it does
/// not, so a row taller than a piece stands alone rather than splitting.
pub(super) fn pieces(
    layout: &Layout,
    width: f64,
    first: f64,
    rest: f64,
    opens_page: &dyn Fn(usize) -> bool,
    fonts: &Fonts,
) -> Vec<Layout> {
    let entries = &layout.entries;
    let units = units(entries);
    let entry_range = |range: Range<usize>| {
        let start = units.get(range.start).map_or(0, |unit| unit.start);
        let end = units.get(range.end - 1).map_or(start, |unit| unit.end);
        start..end
    };
    // The walk up to each piece's first entry, carried from piece to
    // piece: `open` is asked in order, its starts increasing.
    let mut under = Under::default();
    let mut walked = 0;
    let mut open = |k: usize, start: usize| {
        let first_entry = units.get(start).map_or(entries.len(), |unit| unit.start);
        for entry in entries.get(walked..first_entry).unwrap_or_default() {
            under.update(entry);
        }
        walked = first_entry;
        if k == 0 {
            Opening::default()
        } else {
            opening(entries.get(first_entry), &under, opens_page(k), fonts)
        }
    };
    let fits = |opened: &Opening, k: usize, range: Range<usize>| {
        let limit = if k == 0 { first } else { rest };
        let end = entry_range(range).end;
        let bottom = entries.get(end - 1).map_or(0.0, Entry::bottom) + opened.dy;
        panel_size(width, bottom).height <= limit
    };
    paginate(units.len(), &mut open, &fits)
        .into_iter()
        .map(|piece| {
            let carried = entry_range(piece.units.clone());
            built(layout, width, piece.opening, carried)
        })
        .collect()
}

/// One piece of `layout`, `width` wide: its opening, then the entries
/// `carried` shifted by the opening's `dy`, with the groups they start.
fn built(layout: &Layout, width: f64, opening: Opening, carried: Range<usize>) -> Layout {
    let Opening {
        mut placed,
        dy,
        mut groups,
    } = opening;
    let Range { start, end } = carried;
    let base = placed.len();
    let entries = layout.entries.get(start..end).unwrap_or_default();
    placed.extend(entries.iter().map(|entry| entry.shifted(dy)));
    groups.extend(
        layout
            .groups
            .iter()
            .filter(|group| (start..end).contains(&group.entry))
            .map(|group| GroupStart {
                entry: group.entry - start + base,
                baseline: group.baseline + dy,
            }),
    );
    Layout {
        width,
        height: height(&placed),
        entries: placed,
        groups,
    }
}

/// The entries in runs placed whole: every heading and swatch glued to
/// what follows it, up to and including the row it leads to. Every sector
/// in the legend has a row (rg-i4a), so the entries end on one, and every
/// unit holds a row, and so does every piece.
fn units(entries: &[Entry]) -> Vec<Range<usize>> {
    let mut out = Vec::new();
    let mut start = 0;
    for (i, entry) in entries.iter().enumerate() {
        if matches!(entry, Entry::Row(_)) {
            out.push(start..i + 1);
            start = i + 1;
        }
    }
    out
}

/// The sector and ring the walk is inside: what a piece starting here
/// repeats.
#[derive(Debug, Default)]
struct Under {
    /// The sector's series and heading.
    sector: Option<(usize, String)>,
    /// The ring's heading.
    ring: Option<String>,
}

impl Under {
    /// The walk past `entry`.
    fn update(&mut self, entry: &Entry) {
        match entry {
            Entry::Swatch { series, .. } => {
                self.sector = Some((*series, String::new()));
                self.ring = None;
            }
            Entry::Heading { run, level, .. } if *level == SECTOR_TIER.level => {
                if let Some((_, words)) = &mut self.sector {
                    words.clone_from(&run.words);
                }
            }
            Entry::Heading { run, level, .. } if *level == RING_TIER.level => {
                self.ring = Some(run.words.clone());
            }
            _ => {}
        }
    }
}

/// What a piece after the first opens with, the shift its carried
/// entries take, and the group its continued sector starts.
#[derive(Debug, Default)]
struct Opening {
    placed: Vec<Entry>,
    dy: f64,
    groups: Vec<GroupStart>,
}

/// The opening of a piece whose first carried entry is `first`, inside
/// `under`: *Legend (continued)* when it opens a page; the sector's swatch
/// and heading, continued, unless `first` starts a sector; the ring's
/// heading, continued, when `first` is a row. The carried entries move by
/// one `dy` so the first sits where a fresh legend's spacing puts it.
fn opening(first: Option<&Entry>, under: &Under, page: bool, fonts: &Fonts) -> Opening {
    let mut out = Opening::default();
    let mut baseline = LEGEND_HEADING_BASELINE;
    if page {
        let title = format!("{LEGEND_TITLE}{CONTINUED}");
        out.placed.push(Entry::continued(
            heading(TITLE_TIER, &title, baseline, fonts),
            TITLE_TIER,
        ));
        baseline = LEGEND_FIRST_GROUP;
    }
    if !inside_sector(first) {
        // The carried swatch's heading takes the baseline; the swatch sits
        // `SWATCH_RAISE` above it in both places, so one shift moves both.
        out.dy = first.map_or(0.0, |entry| baseline - heading_baseline(entry));
        return out;
    }
    let Some((series, words)) = &under.sector else {
        return out;
    };
    out.groups.push(GroupStart {
        entry: out.placed.len(),
        baseline,
    });
    out.placed.push(swatch(*series, baseline));
    let sector = heading(SECTOR_TIER, &format!("{words}{CONTINUED}"), baseline, fonts);
    let ring_baseline = sector.last_baseline() + LEGEND_SECTOR_TO_RING;
    out.placed.push(Entry::continued(sector, SECTOR_TIER));
    match first {
        Some(Entry::Row(row)) => {
            let words = under.ring.as_deref().unwrap_or_default();
            let ring = heading(
                RING_TIER,
                &format!("{words}{CONTINUED}"),
                ring_baseline,
                fonts,
            );
            out.dy = ring.last_baseline() + LEGEND_RING_TO_ROW - row.top;
            out.placed.push(Entry::continued(ring, RING_TIER));
        }
        Some(entry) => out.dy = ring_baseline - entry.top(),
        None => {}
    }
    out
}

/// Whether `first` carries on inside the sector the walk is in: a ring
/// heading or a row, not a swatch or a sector or legend heading.
fn inside_sector(first: Option<&Entry>) -> bool {
    match first {
        Some(Entry::Row(_)) => true,
        Some(Entry::Heading { level, .. }) => *level == RING_TIER.level,
        _ => false,
    }
}

/// The baseline of the sector heading a carried `entry` starts: a
/// swatch's is its heading's, `SWATCH_RAISE` below its centre.
fn heading_baseline(entry: &Entry) -> f64 {
    match entry {
        Entry::Swatch { centre, .. } => centre.y + SWATCH_RAISE,
        _ => entry.top(),
    }
}

#[cfg(test)]
mod tests {
    use super::super::LegendBlock;
    use super::super::layout::Entry;
    use crate::layout::block::{Block, Placement, fonts};
    use crate::layout::test_support::{HEAD, assert_inside, long, plotted};
    use crate::model::Catalogue;
    use crate::order::Plotted;
    use crate::style::{LEGEND_FIRST_GROUP, LEGEND_HEADING_BASELINE};
    use crate::svg::Point;

    /// As `long`, with a sector whose one row is in an undeclared ring
    /// (FR-17) after sector `after`: the legend leaves it out (rg-i4a).
    fn with_empty(sectors: usize, after: usize, per_ring: usize) -> (Catalogue, Plotted) {
        let rows: String = (0..sectors)
            .flat_map(|s| {
                let empty = (s == after).then(|| format!("gone,S{s}e,Elsewhere,d\n"));
                (0..per_ring)
                    .map(move |i| format!("S{s}R0-{i:03},S{s},R0,d\n"))
                    .chain(empty)
            })
            .collect();
        plotted(&format!("{HEAD}{rows}"), Some("R0"))
    }

    fn legend(shape: (Catalogue, Plotted)) -> LegendBlock {
        let (catalogue, order) = shape;
        LegendBlock::new(&catalogue, &order, fonts())
    }

    /// RG-021B: the paginator closes a piece once it holds a unit, which
    /// is right only because every unit holds a row.
    #[test]
    fn every_unit_holds_a_row() {
        for shape in [
            long(1, 1, 30),
            long(4, 3, 5),
            with_empty(3, 1, 4),
            with_empty(3, 2, 4),
        ] {
            let block = legend(shape);
            let entries = &block.layout.entries;
            let units = super::units(entries);
            assert_ne!(units.len(), 0);
            for unit in units {
                let held = entries.get(unit.clone()).unwrap_or_default();
                assert!(held.iter().any(|e| matches!(e, Entry::Row(_))), "{unit:?}");
            }
        }
    }

    fn never(_: usize) -> bool {
        false
    }

    fn pieces(block: &LegendBlock, first: f64, rest: f64) -> Vec<LegendBlock> {
        block.flowed(first, rest, &never, fonts())
    }

    /// Every row's name, in entry order.
    fn names(block: &LegendBlock) -> Vec<String> {
        block
            .layout
            .entries
            .iter()
            .filter_map(|entry| match entry {
                Entry::Row(row) => Some(row.name.words.clone()),
                _ => None,
            })
            .collect()
    }

    /// The headings before a piece's first row: level, words, continued.
    fn openings(block: &LegendBlock) -> Vec<(u8, String, bool)> {
        block
            .layout
            .entries
            .iter()
            .take_while(|entry| !matches!(entry, Entry::Row(_)))
            .filter_map(|entry| match entry {
                Entry::Heading {
                    run,
                    level,
                    continued,
                } => Some((level.get(), run.words.clone(), *continued)),
                _ => None,
            })
            .collect()
    }

    /// Every heading's (level, first baseline).
    fn baselines(block: &LegendBlock) -> Vec<(u8, f64)> {
        block
            .layout
            .entries
            .iter()
            .filter_map(|entry| match entry {
                Entry::Heading { run, level, .. } => Some((level.get(), run.origin.y)),
                _ => None,
            })
            .collect()
    }

    /// Each entry's top: a row's top, a heading's first baseline, a
    /// swatch's centre.
    fn tops(block: &LegendBlock) -> Vec<f64> {
        block.layout.entries.iter().map(Entry::top).collect()
    }

    fn shapes() -> Vec<LegendBlock> {
        vec![
            legend(long(1, 1, 300)),
            legend(long(4, 3, 30)),
            legend(long(40, 1, 6)),
        ]
    }

    fn height(block: &LegendBlock) -> f64 {
        block.size(1.0).height
    }

    #[test]
    fn a_legend_that_fits_is_one_piece_with_its_entries() {
        let block = legend(long(2, 2, 3));
        let h = height(&block);
        let one = pieces(&block, h, h);
        assert_eq!(one.len(), 1);
        let piece = one.first().expect("a piece");
        assert_eq!(piece.layout.entries, block.layout.entries);
        assert_eq!(height(piece).to_bits(), h.to_bits());
        assert_eq!(pieces(&block, h - 0.01, h - 0.01).len(), 2);
    }

    #[test]
    fn every_row_appears_once_in_order() {
        for block in shapes() {
            let all: Vec<String> = pieces(&block, 600.0, 680.0)
                .iter()
                .flat_map(names)
                .collect();
            assert_eq!(all, names(&block));
        }
    }

    #[test]
    fn the_first_limit_is_the_first_pieces() {
        let block = legend(long(1, 1, 300));
        let all = pieces(&block, 200.0, 680.0);
        let (first, rest) = all.split_first().expect("pieces");
        assert!(height(first) <= 200.0);
        assert!(rest.iter().all(|p| height(p) <= 680.0));
        assert!(rest.first().is_some_and(|p| height(p) > 200.0));
    }

    #[test]
    fn pieces_lie_inside_their_panels() {
        for block in shapes() {
            for piece in pieces(&block, 600.0, 680.0) {
                assert_inside(
                    &piece.render(Placement::at(Point::ORIGIN)),
                    piece.size(1.0),
                    fonts(),
                );
                let tops = tops(&piece);
                assert!(
                    tops.windows(2).all(|w| matches!(w, [a, b] if a <= b)),
                    "{tops:?}"
                );
            }
        }
    }

    /// The height of `block`'s first `entries` entries as a panel.
    fn through(block: &LegendBlock, entries: usize) -> f64 {
        let bottom = block
            .layout
            .entries
            .get(entries - 1)
            .map_or(0.0, Entry::bottom);
        super::super::panel_size(block.layout.width, bottom).height
    }

    /// The index of the entry whose heading or row reads `words`.
    fn index_of_opt(block: &LegendBlock, words: &str) -> Option<usize> {
        block.layout.entries.iter().position(|entry| match entry {
            Entry::Heading { run, .. } => run.words == words,
            Entry::Row(row) => row.name.words == words,
            Entry::Swatch { .. } => false,
        })
    }

    fn index_of(block: &LegendBlock, words: &str) -> usize {
        index_of_opt(block, words).expect("the entry")
    }

    #[test]
    fn breaks_open_with_the_right_headings() {
        // Two sectors of two rings of three rows each.
        let block = legend(long(2, 2, 3));
        let at = |words: &str| through(&block, index_of(&block, words) + 1);
        let opening = |first: f64| {
            let all = pieces(&block, first, 10_000.0);
            openings(all.get(1).expect("a second piece"))
        };
        // Mid-ring: the first piece ends after S0R0's second row.
        assert_eq!(
            opening(at("S0R0-001")),
            [
                (3, "S0 (continued)".to_owned(), true),
                (4, "R0 (continued)".to_owned(), true)
            ]
        );
        // At a ring boundary: after S0R0's last row.
        assert_eq!(
            opening(at("S0R0-002")),
            [
                (3, "S0 (continued)".to_owned(), true),
                (4, "R1".to_owned(), false)
            ]
        );
        // At a sector boundary: after S0's last row.
        assert_eq!(
            opening(at("S0R1-002")),
            [(3, "S1".to_owned(), false), (4, "R0".to_owned(), false)]
        );
    }

    #[test]
    fn a_continued_sector_has_its_swatch() {
        let block = legend(long(2, 1, 40));
        let all = pieces(&block, 300.0, 300.0);
        for piece in all.iter().skip(1) {
            let first = piece.layout.entries.first().expect("an entry");
            assert!(matches!(first, Entry::Swatch { .. }), "{first:?}");
        }
    }

    #[test]
    fn a_continued_heading_sits_where_a_fresh_one_does() {
        let block = legend(long(1, 1, 60));
        let plain = pieces(&block, 300.0, 300.0);
        let second = plain.get(1).expect("a second piece");
        let sector = baselines(second).into_iter().find(|&(level, _)| level == 3);
        assert_eq!(sector, Some((3, LEGEND_HEADING_BASELINE)));
        let paged = block.flowed(300.0, 300.0, &|k| k == 1, fonts());
        let second = paged.get(1).expect("a second piece");
        let heads = baselines(second);
        assert_eq!(heads.first(), Some(&(2, LEGEND_HEADING_BASELINE)));
        assert_eq!(heads.get(1), Some(&(3, LEGEND_FIRST_GROUP)));
    }

    /// The gap from a block's first ring heading to the row after it.
    fn ring_to_row(block: &LegendBlock) -> f64 {
        let entries = &block.layout.entries;
        let ring = entries
            .iter()
            .position(|entry| matches!(entry, Entry::Heading { level, .. } if level.get() == 4))
            .expect("a ring heading");
        match (entries.get(ring), entries.get(ring + 1)) {
            (Some(Entry::Heading { run, .. }), Some(Entry::Row(row))) => {
                row.top - run.last_baseline()
            }
            other => panic!("a ring heading then a row: {other:?}"),
        }
    }

    #[test]
    fn a_continued_row_sits_where_a_fresh_one_does() {
        let block = legend(long(1, 1, 60));
        let all = pieces(&block, 300.0, 300.0);
        let second = all.get(1).expect("a second piece");
        assert_eq!(ring_to_row(second).to_bits(), ring_to_row(&block).to_bits());
    }

    #[test]
    fn a_heading_is_never_last_except_the_legends_own() {
        let mut blocks = shapes();
        blocks.push(legend(with_empty(30, 10, 2)));
        blocks.push(legend(with_empty(30, 29, 2)));
        for block in blocks {
            let all = pieces(&block, 120.0, 120.0);
            for piece in all.iter().rev().skip(1) {
                let last = piece.layout.entries.last().expect("an entry");
                assert!(matches!(last, Entry::Row(_)), "{last:?}");
            }
        }
    }

    /// rg-q1q: with page openers, no piece is taller than its space,
    /// unless its one row alone is.
    #[test]
    fn no_piece_is_taller_than_its_space() {
        fn odd(k: usize) -> bool {
            k % 2 == 1
        }
        for shape in [
            long(1, 1, 300),
            long(4, 3, 80),
            long(40, 1, 6),
            with_empty(5, 2, 20),
        ] {
            let block = legend(shape);
            let (first, rest) = (200.0, 680.0);
            for (k, piece) in block.flowed(first, rest, &odd, fonts()).iter().enumerate() {
                let limit = if k == 0 { first } else { rest };
                assert!(
                    piece.size(1.0).height <= limit || names(piece).len() == 1,
                    "piece {k}: {}",
                    piece.size(1.0).height
                );
            }
        }
    }

    #[test]
    fn only_page_openers_say_legend_continued() {
        let block = legend(long(2, 1, 200));
        let all = block.flowed(600.0, 680.0, &|k| k % 2 == 1, fonts());
        assert!(all.len() > 4);
        for (k, piece) in all.iter().enumerate() {
            let first = openings(piece).into_iter().next();
            let legend = first
                .as_ref()
                .map(|(level, words, continued)| (*level, words.as_str(), *continued));
            match k {
                0 => assert_eq!(legend, Some((2, "Legend", false))),
                _ if k % 2 == 1 => assert_eq!(legend, Some((2, "Legend (continued)", true))),
                _ => assert_ne!(legend.map(|(level, _, _)| level), Some(2)),
            }
        }
    }

    #[test]
    fn every_piece_is_one_width() {
        let (catalogue, order) = plotted(
            &format!("{HEAD}x,A,Assess and keep reviewing,d\n"),
            Some("Assess and keep reviewing"),
        );
        let narrow = LegendBlock::new(&catalogue, &order, fonts());
        let width = narrow.paper_width(fonts());
        assert!(width > narrow.size(1.0).width);
        for block in [narrow, legend(long(4, 3, 30))] {
            let width = block.paper_width(fonts());
            for piece in pieces(&block, 200.0, 300.0) {
                assert_eq!(piece.size(1.0).width.to_bits(), width.to_bits());
            }
        }
    }

    #[test]
    fn an_empty_legend_has_no_pieces() {
        let (catalogue, order) = (
            crate::layout::test_support::empty_catalogue(),
            Plotted::default(),
        );
        let block = LegendBlock::new(&catalogue, &order, fonts());
        assert_eq!(pieces(&block, 600.0, 680.0), []);
    }

    #[test]
    fn a_row_taller_than_a_piece_stands_alone() {
        let block = legend(long(1, 1, 3));
        for (first, rest) in [(10.0, 10.0), (10.0, 680.0)] {
            let all = pieces(&block, first, rest);
            assert!(all.iter().all(|p| !names(p).is_empty()));
            assert_eq!(all.first().map(|p| names(p).len()), Some(1));
        }
    }

    #[test]
    fn paper_rows_are_tighter() {
        let (catalogue, order) = long(1, 1, 3);
        let screen = LegendBlock::new(&catalogue, &order, fonts());
        let paper = LegendBlock::on_paper(&catalogue, &order, fonts());
        assert_eq!(names(&paper), names(&screen));
        let row = |block: &LegendBlock| {
            block
                .layout
                .entries
                .iter()
                .find_map(|entry| match entry {
                    Entry::Row(row) => Some(row.height),
                    _ => None,
                })
                .expect("a row")
        };
        assert_eq!((row(&screen), row(&paper)), (36.0, 26.0));
    }
}
