//! The full PDF's table on paper (FR-112): the published table measured
//! for a page, split into whole rows per page, each page drawn under its
//! heading and the header row.
//!
//! [`columns`] decides which columns are drawn and how wide; [`rows`] wraps
//! each row, folds the short columns under the name and splits a row taller
//! than a page; this module cuts the rows into pages and draws them.

mod columns;
mod draw;
mod rows;

use std::ops::Range;

use crate::layout::geometry::real as count;
use crate::layout::paginate::paginate;
use crate::published::{Cell, NOTHING_PLOTTED, Published};
use crate::style::{CONTINUED_MARK, TABLE_HEADING_BASELINE, TABLE_HEADING_TO_HEAD};
use crate::text::{Fonts, words};

use columns::{Column, Sized, sized};
use rows::{Lines, Part, full, height_of, parts};

/// Under the name of a row split across pages, on every part but its last.
pub const CONTINUES: &str = "continues on the next page";

/// One table page: the parts it holds, and whether it is the first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TablePage {
    parts: Range<usize>,
    first: bool,
}

/// The table, measured for a page (FR-112).
#[derive(Debug, Clone, PartialEq)]
pub struct TableBlock {
    heading: &'static str,
    width: f64,
    columns: Vec<Column>,
    head_height: f64,
    room: f64,
    cap: f64,
    cramped: bool,
    parts: Vec<Part>,
    pages: Vec<TablePage>,
    tall: Vec<(usize, String)>,
}

/// The parts laid out for one sizing: the columns, the header row's
/// height and the room it leaves, and every row's parts.
struct Laid {
    sized: Sized,
    head_height: f64,
    room: f64,
    parts: Vec<Part>,
}

/// `published` laid out across `width` on a page `page_height` tall.
fn laid(
    published: &Published<'_>,
    (width, page_height): (f64, f64),
    split: bool,
    fonts: &Fonts,
) -> Laid {
    let sized = sized(published, width, split, fonts);
    let header_lines = sized
        .columns
        .iter()
        .map(|c| c.heading.len())
        .max()
        .unwrap_or(1);
    let head_height = TABLE_HEADING_BASELINE + TABLE_HEADING_TO_HEAD + height_of(header_lines);
    let room = page_height - head_height;
    let parts = published
        .rows
        .iter()
        .enumerate()
        .flat_map(|(i, row)| {
            parts(
                i,
                row,
                full(published, row, &sized.columns, fonts),
                room,
                fonts,
            )
        })
        .collect();
    Laid {
        sized,
        head_height,
        room,
        parts,
    }
}

/// Each row taller than a page, by its CSV line and name, in one pass: a
/// row split across pages (its parts are adjacent), or one drawn whole
/// past the room because its name alone is that tall.
fn tall(published: &Published<'_>, parts: &[Part], room: f64) -> Vec<(usize, String)> {
    parts
        .chunk_by(|a, b| a.row == b.row)
        .filter(|chunk| chunk.len() > 1 || chunk.iter().any(|part| part.height > room))
        .filter_map(|chunk| chunk.first().and_then(|part| published.rows.get(part.row)))
        .map(|row| {
            let name = row
                .cells
                .iter()
                .find_map(|cell| match cell {
                    Cell::Name { name, .. } => Some(words(name)),
                    _ => None,
                })
                .unwrap_or_default();
            (row.line, name)
        })
        .collect()
}

/// The parts cut into pages of `room`; an empty table is one page with its
/// header row alone.
fn pages(parts: &[Part], room: f64) -> Vec<TablePage> {
    let mut tops = vec![0.0];
    for part in parts {
        tops.push(tops.last().copied().unwrap_or(0.0) + part.height);
    }
    let fits = |(): &(), _: usize, units: Range<usize>| {
        let top = tops.get(units.start).copied().unwrap_or(0.0);
        let bottom = tops.get(units.end).copied().unwrap_or(top);
        bottom - top <= room
    };
    let mut pages: Vec<TablePage> = paginate(parts.len(), &mut |_, _| (), &fits)
        .into_iter()
        .enumerate()
        .map(|(k, piece)| TablePage {
            parts: piece.units,
            first: k == 0,
        })
        .collect();
    if pages.is_empty() {
        pages.push(TablePage {
            parts: 0..0,
            first: true,
        });
    }
    pages
}

/// FR-106's one row when nothing is plotted, spanning every column.
fn nothing_plotted() -> Part {
    Part {
        row: 0,
        number: None,
        sector: String::new(),
        cells: vec![Lines {
            lines: vec![NOTHING_PLOTTED.to_owned()],
            words: NOTHING_PLOTTED.to_owned(),
        }],
        under: Vec::new(),
        beneath: None,
        height: height_of(1),
        ruled: true,
        spanning: true,
    }
}

impl TableBlock {
    /// `published` measured for a page `width` wide and `page_height` tall:
    /// its columns sized, its cells wrapped, rows taller than a page split
    /// and the rows cut into pages, in one pass.
    #[must_use]
    pub fn new(published: &Published<'_>, width: f64, page_height: f64, fonts: &Fonts) -> Self {
        // Sized as if no row splits; when one does, sized again so the name
        // column holds its later parts' marks unbroken. The second sizing
        // reserves the marks, so whatever it splits is marked whole.
        let page = (width, page_height);
        let mut laid_out = laid(published, page, false, fonts);
        if laid_out.parts.iter().any(|part| !part.ruled) {
            // Only when the marks widen the name column does a row wrap
            // differently, so only then are the rows laid out again.
            let marked = sized(published, width, true, fonts);
            if marked.columns != laid_out.sized.columns {
                laid_out = laid(published, page, true, fonts);
            }
        }
        let Laid {
            sized,
            head_height,
            room,
            mut parts,
        } = laid_out;
        let tall = tall(published, &parts, room);
        if published.says_empty {
            parts.push(nothing_plotted());
        }
        Self {
            heading: published.heading,
            width,
            pages: pages(&parts, room),
            columns: sized.columns,
            head_height,
            room,
            cap: sized.cap,
            cramped: sized.cramped,
            parts,
            tall,
        }
    }

    /// The table's pages, in order.
    #[must_use]
    pub fn pages(&self) -> &[TablePage] {
        &self.pages
    }

    /// The drawn columns, the number column included, when the prose is
    /// narrower than it should be even with the label columns at their
    /// floor; `None` when it is not.
    #[must_use]
    pub fn cramped(&self) -> Option<usize> {
        self.cramped.then_some(self.columns.len())
    }

    /// Each row split across pages: its CSV line and name, in order.
    #[must_use]
    pub fn tall_rows(&self) -> &[(usize, String)] {
        &self.tall
    }
}

#[cfg(test)]
mod tests;
