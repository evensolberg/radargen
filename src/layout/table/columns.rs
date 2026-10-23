//! The table's columns (FR-112): which it draws, and how wide each is.
//!
//! The number column fits its widest number. A label column fits its
//! widest text up to a cap, and never comes below its widest unbreakable
//! word, so a word is never broken in it. A prose column gets at least
//! [`TABLE_PROSE_MIN`], or its content when that is less, and the prose
//! shares what is left by how much each holds. When the minimums do not
//! fit, the cap comes down before the table is called cramped.

use crate::columns::ColumnKind;
use crate::layout::arrange::fit_scale;
use crate::layout::title::cut;
use crate::model::NOT_ON_THE_RADAR;
use crate::published::{Cell, CellKind, NUMBER_HEADING, Published, PublishedColumn};
use crate::style::{
    TABLE_HEAD, TABLE_LABEL_CAP, TABLE_LABEL_FLOOR, TABLE_NAME, TABLE_PAD_X, TABLE_PROSE_MIN,
    TABLE_TEXT, TextStyle,
};
use crate::text::Fonts;

use super::{CONTINUED_MARK, CONTINUES, count};

/// A cell's padding, both sides.
pub(super) const PADS_X: f64 = 2.0 * TABLE_PAD_X;

/// The most lines a heading takes; a longer one is cut with an ellipsis,
/// so a header can never fill a page (final review, Rust 1).
const HEADING_LINES: usize = 3;

/// One drawn column: its kind (`None` for the number column), where it
/// starts, how wide it is, padding included, and its heading's lines.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Column {
    pub(super) kind: Option<CellKind>,
    /// Its index among the published columns; `None` for the number.
    pub(super) source: Option<usize>,
    pub(super) x: f64,
    pub(super) width: f64,
    pub(super) heading: Vec<String>,
}

/// The columns as drawn, how far the cap came down, and whether the prose
/// is narrower than it should be even so.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Sized {
    pub(super) columns: Vec<Column>,
    pub(super) cap: f64,
    pub(super) cramped: bool,
}

/// Whether `column` is folded into the muted line under the name, not
/// drawn (the maintainer's ruling after the final review, 2026-10-23):
/// qualifier, status and the dates are short, and as columns they took
/// the width the prose needs.
pub(super) const fn folded(column: ColumnKind) -> bool {
    matches!(
        column,
        ColumnKind::Qualifier
            | ColumnKind::Status
            | ColumnKind::LastAffirmed
            | ColumnKind::ExitDate
    )
}

/// The style a cell of `kind` is drawn in.
pub(super) const fn style_of(kind: CellKind) -> TextStyle {
    match kind {
        CellKind::Name => TABLE_NAME,
        CellKind::Label | CellKind::Prose | CellKind::Link => TABLE_TEXT,
    }
}

/// A heading's pieces: its words, each broken after any underscore, with
/// whether a space comes before each (the maintainer's ruling,
/// 2026-10-23: a heading never breaks mid-word).
pub(super) fn pieces(heading: &str) -> Vec<(String, bool)> {
    heading
        .split_whitespace()
        .enumerate()
        .flat_map(|(i, word)| {
            word.split_inclusive('_')
                .enumerate()
                .map(move |(j, piece)| (piece.to_owned(), i > 0 && j == 0))
        })
        .collect()
}

/// `heading` in lines no wider than `width`, broken only between its
/// pieces, at most [`HEADING_LINES`] of them; a piece wider than `width`
/// stands alone.
pub(super) fn heading_lines(heading: &str, width: f64, fonts: &Fonts) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line = String::new();
    for (piece, space) in pieces(heading) {
        let candidate = if line.is_empty() {
            piece.clone()
        } else if space {
            format!("{line} {piece}")
        } else {
            format!("{line}{piece}")
        };
        if line.is_empty() || TABLE_HEAD.measure(&candidate, fonts) <= width {
            line = candidate;
        } else {
            lines.push(std::mem::replace(&mut line, piece));
        }
    }
    lines.push(line);
    if lines.len() > HEADING_LINES {
        let rest = lines.split_off(HEADING_LINES - 1).join(" ");
        lines.push(cut(TABLE_HEAD, &rest, width, fonts));
    }
    lines
}

/// The widest of `widths`; zero for none.
fn widest(widths: impl Iterator<Item = f64>) -> f64 {
    widths.fold(0.0, f64::max)
}

/// One column's measure before sizing: a label's natural and least
/// widths, or prose's content width and weight.
#[derive(Debug, Clone, Copy)]
struct Measure {
    kind: CellKind,
    /// Its widest text, unwrapped, padded.
    natural: f64,
    /// A label's widest unbreakable word, padded; prose's own minimum.
    least: f64,
    /// Prose: its total characters, at least one.
    weight: f64,
}

/// Published column `c`'s measure. `meta` is the widest unbreakable word
/// of the lines folded under the name; `split` says a row splits, so the
/// name column holds its marks unbroken.
fn measure(
    published: &Published<'_>,
    (c, column): (usize, &PublishedColumn<'_>),
    (unplotted, split, meta): (bool, bool, f64),
    fonts: &Fonts,
) -> Measure {
    let style = style_of(column.cell);
    let texts: Vec<&str> = published
        .rows
        .iter()
        .filter_map(|row| row.cells.get(c))
        .flat_map(Cell::paragraphs)
        .collect();
    let piece = widest(
        pieces(column.heading)
            .iter()
            .map(|(piece, _)| TABLE_HEAD.measure(piece, fonts)),
    );
    let natural = widest(texts.iter().map(|text| style.measure(text, fonts))).max(piece) + PADS_X;
    if column.cell == CellKind::Prose {
        let chars: usize = texts.iter().map(|text| text.chars().count()).sum();
        return Measure {
            kind: column.cell,
            natural,
            least: natural.min(TABLE_PROSE_MIN),
            weight: count(chars.max(1)),
        };
    }
    // An address may break where it must (FR-112); every other label
    // keeps its words whole.
    let word = if column.cell == CellKind::Link {
        0.0
    } else {
        widest(
            texts
                .iter()
                .flat_map(|text| text.split_whitespace())
                .map(|w| style.measure(w, fonts)),
        )
    };
    let name = column.cell == CellKind::Name;
    let marks = [
        (unplotted, TABLE_TEXT.measure(NOT_ON_THE_RADAR, fonts)),
        (split, TABLE_TEXT.measure(CONTINUED_MARK, fonts)),
        (
            split,
            widest(
                CONTINUES
                    .split_whitespace()
                    .map(|w| TABLE_TEXT.measure(w, fonts)),
            ),
        ),
        (true, meta),
    ];
    let mark = widest(marks.iter().filter(|(on, _)| name && *on).map(|&(_, w)| w));
    Measure {
        kind: column.cell,
        natural,
        least: word.max(piece).max(mark) + PADS_X,
        weight: 0.0,
    }
}

/// The widest unbreakable word of the lines folded under the names.
fn meta_word(published: &Published<'_>, fonts: &Fonts) -> f64 {
    let folded: Vec<usize> = published
        .columns
        .iter()
        .enumerate()
        .filter(|(_, column)| folded(column.column))
        .map(|(c, _)| c)
        .collect();
    widest(
        published
            .rows
            .iter()
            .flat_map(|row| folded.iter().filter_map(|&c| row.cells.get(c)))
            .flat_map(Cell::paragraphs)
            .flat_map(str::split_whitespace)
            .map(|word| TABLE_TEXT.measure(word, fonts)),
    )
}

/// The prose widths, given `room` and each prose column's measure: each
/// its least, then the rest by weight, none past its content while
/// another can take it; cramped, `room` by weight alone.
fn prose_widths(prose: &[Measure], room: f64, cramped: bool) -> Vec<f64> {
    let weight = |p: &[Measure], on: &[bool]| -> f64 {
        p.iter()
            .zip(on)
            .filter(|(_, on)| **on)
            .map(|(m, _)| m.weight)
            .sum()
    };
    if cramped {
        let all: f64 = prose.iter().map(|m| m.weight).sum();
        return prose
            .iter()
            .map(|m| (room * m.weight / all).max(PADS_X + TABLE_TEXT.size))
            .collect();
    }
    let mut widths: Vec<f64> = prose.iter().map(|m| m.least).collect();
    let mut open: Vec<bool> = prose.iter().map(|m| m.natural > m.least).collect();
    // Each round fills at least one column to its content or spends the
    // rest, so it ends within one round per column.
    for _ in 0..=prose.len() {
        let left = room - widths.iter().sum::<f64>();
        let shares = weight(prose, &open);
        if left <= 0.0 || shares <= 0.0 {
            break;
        }
        for ((width, m), on) in widths.iter_mut().zip(prose).zip(open.iter_mut()) {
            if *on {
                *width = (*width + left * m.weight / shares).min(m.natural);
                *on = *width < m.natural;
            }
        }
    }
    // Content narrower than the page: the slack goes by weight to all.
    let left = room - widths.iter().sum::<f64>();
    let all: f64 = prose.iter().map(|m| m.weight).sum();
    if left > 0.0 && all > 0.0 {
        for (width, m) in widths.iter_mut().zip(prose) {
            *width += left * m.weight / all;
        }
    }
    widths
}

/// The number column's width: its widest number or heading, padded.
fn number_width(published: &Published<'_>, fonts: &Fonts) -> f64 {
    widest(
        published
            .rows
            .iter()
            .filter_map(|row| row.number)
            .map(|n| TABLE_TEXT.measure(&n.to_string(), fonts)),
    )
    .max(TABLE_HEAD.measure(NUMBER_HEADING, fonts))
        + PADS_X
}

/// A label column's width under `cap`: its natural width, capped, never
/// below its least.
const fn label(m: &Measure, cap: f64) -> f64 {
    m.natural.min(cap).max(m.least)
}

/// The drawn columns laid side by side: the number column, then each of
/// `drawn` at its width, prose from `prose` in order.
fn placed(
    published: &Published<'_>,
    (number, drawn): (f64, &[(usize, Measure)]),
    cap: f64,
    prose: Vec<f64>,
    fonts: &Fonts,
) -> Vec<Column> {
    let mut prose = prose.into_iter();
    let mut x = 0.0;
    let mut place =
        |kind: Option<CellKind>, source: Option<usize>, w: f64, heading: Vec<String>| {
            let column = Column {
                kind,
                source,
                x,
                width: w,
                heading,
            };
            x += w;
            column
        };
    let mut columns = vec![place(None, None, number, vec![NUMBER_HEADING.to_owned()])];
    for (c, m) in drawn {
        let w = if m.kind == CellKind::Prose {
            prose.next().unwrap_or(m.least)
        } else {
            label(m, cap)
        };
        let heading = published
            .columns
            .get(*c)
            .map_or("", |column| column.heading);
        columns.push(place(
            Some(m.kind),
            Some(*c),
            w,
            heading_lines(heading, w - PADS_X, fonts),
        ));
    }
    columns
}

/// The drawn columns of `published` across `width`.
pub(super) fn sized(published: &Published<'_>, width: f64, split: bool, fonts: &Fonts) -> Sized {
    let number = number_width(published, fonts);
    let unplotted = published.rows.iter().any(|row| row.number.is_none());
    let meta = meta_word(published, fonts);
    let drawn: Vec<(usize, Measure)> = published
        .columns
        .iter()
        .enumerate()
        .filter(|(_, column)| !folded(column.column))
        .map(|(c, column)| {
            (
                c,
                measure(published, (c, column), (unplotted, split, meta), fonts),
            )
        })
        .collect();
    let labels = |cap: f64| -> f64 {
        drawn
            .iter()
            .filter(|(_, m)| m.kind != CellKind::Prose)
            .map(|(_, m)| label(m, cap))
            .sum()
    };
    let prose: Vec<Measure> = drawn
        .iter()
        .filter(|(_, m)| m.kind == CellKind::Prose)
        .map(|&(_, m)| m)
        .collect();
    let need: f64 = prose.iter().map(|m| m.least).sum();
    let fits = |s: f64| width - number - labels(TABLE_LABEL_CAP * s) >= need;
    let floor = TABLE_LABEL_FLOOR / TABLE_LABEL_CAP;
    let cramped = !fits(floor);
    let cap = match (fits(1.0), cramped) {
        (true, _) => TABLE_LABEL_CAP,
        (false, true) => TABLE_LABEL_FLOOR,
        (false, false) => TABLE_LABEL_CAP * fit_scale(floor, fits),
    };
    let widths = prose_widths(&prose, width - number - labels(cap), cramped);
    Sized {
        columns: placed(published, (number, &drawn), cap, widths, fonts),
        cap,
        cramped,
    }
}
