//! The table's rows (FR-112): each cell wrapped to its column, the folded
//! columns as a muted line under the name, and a row taller than a page
//! split into parts of whole lines.

use crate::layout::title::cut;
use crate::model::NOT_ON_THE_RADAR;
use crate::published::{Cell, CellKind, Published, PublishedRow};
use crate::style::{TABLE_LEADING, TABLE_NAME, TABLE_PAD_Y, TABLE_TEXT, TextStyle};
use crate::text::{Fonts, words};

use super::columns::{Column, PADS_X, folded, style_of};
use super::{CONTINUED_MARK, CONTINUES, count};

/// A cell's padding, above and below.
const PADS_Y: f64 = 2.0 * TABLE_PAD_Y;

/// The most lines a part holds, so a room no float can satisfy still
/// ends (final review, Rust 3).
const MOST_LINES: usize = 10_000;

/// Lines as drawn, and their words for a reader (D-88).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(super) struct Lines {
    pub(super) lines: Vec<String>,
    pub(super) words: String,
}

impl Lines {
    /// `paragraphs` wrapped at `width` in `style`; an empty paragraph is
    /// an empty line, so blank lines stay (FR-97).
    fn wrapped(style: TextStyle, paragraphs: &[&str], width: f64, fonts: &Fonts) -> Self {
        let lines = paragraphs
            .iter()
            .flat_map(|paragraph| {
                let lines = style.wrap(paragraph, width, fonts);
                if lines.is_empty() {
                    vec![String::new()]
                } else {
                    lines
                }
            })
            .collect();
        Self {
            lines,
            words: words(&paragraphs.join(" ")),
        }
    }

    /// `lines` with their words, as the lines read.
    fn of(lines: Vec<String>) -> Self {
        let words = words(&lines.join(" "));
        Self { lines, words }
    }
}

/// A row as placed on a page: the whole row, or one part of a row taller
/// than a page.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Part {
    /// Its row's index in the published rows.
    pub(super) row: usize,
    /// FR-18's number, repeated on every part.
    pub(super) number: Option<usize>,
    /// The sector label, for a page's heading and the sector rule.
    pub(super) sector: String,
    /// One cell per drawn column, the number column aside.
    pub(super) cells: Vec<Lines>,
    /// Muted runs under the name, each its own text: the folded columns,
    /// the unplotted flag, a later part's *(continued)*, and
    /// [`CONTINUES`] on every part but a split row's last.
    pub(super) under: Vec<Lines>,
    /// Its height, `2 · TABLE_PAD_Y + n · TABLE_LEADING`.
    pub(super) height: f64,
    /// Whether a rule closes it: every part but a split row's earlier ones.
    pub(super) ruled: bool,
    /// The one cell of an empty table, spanning every column (FR-106).
    pub(super) spanning: bool,
}

/// A part's height for `lines` lines in its tallest cell.
#[expect(
    clippy::suboptimal_flops,
    reason = "NFR-4: plain * and + are bit-identical on every target; mul_add falls back to a libm fma where the CPU lacks one"
)]
pub(super) fn height_of(lines: usize) -> f64 {
    PADS_Y + count(lines) * TABLE_LEADING
}

/// The most lines that fit `room` in one part, at least one.
fn lines_within(room: f64) -> usize {
    (1..MOST_LINES)
        .find(|&k| height_of(k + 1) > room)
        .unwrap_or(MOST_LINES)
}

/// The folded columns' values for `row`, as one line's text: qualifier
/// and status as written, each date after its heading, joined by ` · `.
fn meta(published: &Published<'_>, row: &PublishedRow<'_>) -> String {
    published
        .columns
        .iter()
        .zip(&row.cells)
        .filter(|(column, _)| folded(column.column))
        .filter_map(|(column, cell)| {
            let text = cell.paragraphs().join(" ");
            let text = crate::trim(&text).to_owned();
            if text.is_empty() {
                return None;
            }
            Some(match column.column {
                crate::columns::ColumnKind::LastAffirmed | crate::columns::ColumnKind::ExitDate => {
                    format!("{} {text}", column.heading)
                }
                _ => text,
            })
        })
        .collect::<Vec<_>>()
        .join(" \u{b7} ")
}

/// A row as drawn, before any split: its drawn cells, and the muted runs
/// under its name.
pub(super) struct Full {
    pub(super) cells: Vec<Lines>,
    pub(super) under: Vec<Lines>,
    /// The name column's index among the drawn cells.
    pub(super) name: Option<usize>,
    /// The name column's text width, which the marks under it wrap to.
    pub(super) width: f64,
}

/// `row` wrapped to `columns` (the number column first).
pub(super) fn full(
    published: &Published<'_>,
    row: &PublishedRow<'_>,
    columns: &[Column],
    fonts: &Fonts,
) -> Full {
    let drawn = columns.iter().skip(1);
    let cells = drawn
        .clone()
        .map(|column| {
            let kind = column.kind.unwrap_or(CellKind::Label);
            let cell = column.source.and_then(|c| row.cells.get(c));
            let paragraphs = cell.map(Cell::paragraphs).unwrap_or_default();
            Lines::wrapped(style_of(kind), &paragraphs, column.width - PADS_X, fonts)
        })
        .collect();
    let name = drawn.clone().position(|c| c.kind == Some(CellKind::Name));
    let width = name
        .and_then(|c| columns.get(c + 1))
        .map_or(0.0, |c| c.width - PADS_X);
    let meta = meta(published, row);
    let under = [
        (!meta.is_empty()).then(|| Lines::wrapped(TABLE_TEXT, &[&meta], width, fonts)),
        row.number
            .is_none()
            .then(|| Lines::wrapped(TABLE_TEXT, &[NOT_ON_THE_RADAR], width, fonts)),
    ]
    .into_iter()
    .flatten()
    .collect();
    Full {
        cells,
        under,
        name,
        width,
    }
}

/// How many lines a part's tallest cell holds.
fn tallest(cells: &[Lines], name: Option<usize>, under: &[Lines]) -> usize {
    let below: usize = under.iter().map(|u| u.lines.len()).sum();
    cells
        .iter()
        .enumerate()
        .map(|(c, cell)| cell.lines.len() + if Some(c) == name { below } else { 0 })
        .max()
        .unwrap_or(1)
        .max(1)
}

/// `row`'s parts for a page `room` tall: the whole row when it fits, else
/// parts of whole lines. The name, its folded line and flag stay whole on
/// the first part; each later part repeats the name (cut to one line when
/// it is long) marked *(continued)*, and every part but the last ends
/// [`CONTINUES`]. Each part takes at least one line of every cell that has
/// lines left, so the split always ends; a name too tall for a page
/// overflows rather than splits.
pub(super) fn parts(
    index: usize,
    row: &PublishedRow<'_>,
    full: Full,
    room: f64,
    fonts: &Fonts,
) -> Vec<Part> {
    let Full {
        cells,
        under,
        name,
        width,
    } = full;
    let part = |cells: Vec<Lines>, under: Vec<Lines>, ruled: bool| Part {
        row: index,
        number: row.number,
        sector: row.sector.to_owned(),
        height: height_of(tallest(&cells, name, &under)),
        cells,
        under,
        ruled,
        spanning: false,
    };
    if height_of(tallest(&cells, name, &under)) <= room {
        return vec![part(cells, under, true)];
    }
    let most = lines_within(room);
    let name_lines = name
        .and_then(|c| cells.get(c))
        .map(|c| c.lines.clone())
        .unwrap_or_default();
    let mark =
        |continues: bool| continues.then(|| Lines::wrapped(TABLE_TEXT, &[CONTINUES], width, fonts));
    // A later part's name, the same on every part: cut once.
    let later_name = Lines::of(short(&name_lines, width, fonts));
    let mut taken = vec![0; cells.len()];
    let mut out = Vec::new();
    loop {
        let rest = |c: usize| {
            cells
                .get(c)
                .map_or(0, |cell| cell.lines.len())
                .saturating_sub(taken.get(c).copied().unwrap_or(0))
        };
        let last = (0..cells.len())
            .filter(|&c| Some(c) != name)
            .all(|c| rest(c) <= most);
        let first = out.is_empty();
        let head = if first {
            name.and_then(|c| cells.get(c)).cloned().unwrap_or_default()
        } else {
            later_name.clone()
        };
        let drawn = next_part(&cells, (name, head), &mut taken, most);
        let mut below = if first {
            under.clone()
        } else {
            vec![Lines::of(vec![CONTINUED_MARK.to_owned()])]
        };
        below.extend(mark(!last));
        out.push(part(drawn, below, last));
        if last {
            return out;
        }
    }
}

/// The next part's cells: up to `most` lines more of every cell but the
/// name, whose place takes `head`; `taken` counts the lines used so far.
fn next_part(
    cells: &[Lines],
    (name, head): (Option<usize>, Lines),
    taken: &mut [usize],
    most: usize,
) -> Vec<Lines> {
    let mut head = Some(head);
    cells
        .iter()
        .zip(taken.iter_mut())
        .enumerate()
        .map(|(c, (cell, used))| {
            if Some(c) == name {
                return head.take().unwrap_or_default();
            }
            let to = (*used + most).min(cell.lines.len());
            let lines = cell.lines.get(*used..to).unwrap_or_default().to_vec();
            *used = to;
            Lines::of(lines)
        })
        .collect()
}

/// A split row's name on its later parts: whole when it takes a line or
/// two, else its words cut to one line with an ellipsis.
fn short(lines: &[String], width: f64, fonts: &Fonts) -> Vec<String> {
    if lines.len() <= 2 {
        return lines.to_vec();
    }
    vec![cut(TABLE_NAME, &lines.join(" "), width, fonts)]
}
