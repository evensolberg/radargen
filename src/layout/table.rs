//! The full PDF's table on paper (FR-112): the published table measured
//! for a page, split into whole rows per page, each page drawn under its
//! heading and the header row.

use std::num::NonZeroU8;
use std::ops::Range;

use crate::layout::arrange::fit_scale;
use crate::layout::paginate::paginate;
use crate::model::NOT_ON_THE_RADAR;
use crate::palette::{Neutral, Role};
use crate::published::{Cell, CellKind, NOTHING_PLOTTED, NUMBER_HEADING, Published, PublishedRow};
use crate::style::{
    CONTINUED, LINE_HEIGHT, TABLE_ASCENT, TABLE_HEAD, TABLE_HEADING, TABLE_HEADING_BASELINE,
    TABLE_HEADING_TO_HEAD, TABLE_LABEL_CAP, TABLE_LABEL_FLOOR, TABLE_LEADING, TABLE_NAME,
    TABLE_PAD_X, TABLE_PAD_Y, TABLE_PROSE_MIN, TABLE_RULE, TABLE_RULE_HEAVY, TABLE_TEXT, TextStyle,
};
use crate::svg::{Anchor, Group, Node, Paint, Point, Scale, Semantics, TextRun};
use crate::text::{Fonts, words};

/// Under the name of a row split across pages, on every part but its last.
pub const CONTINUES: &str = "continues on the next page";

/// A cell's padding, both sides.
const PADS_X: f64 = 2.0 * TABLE_PAD_X;
/// A cell's padding, above and below.
const PADS_Y: f64 = 2.0 * TABLE_PAD_Y;

/// [`CONTINUED`]'s word, which a split row's name never breaks.
const CONTINUED_WORD: &str = "(continued)";

/// One column: its kind (`None` for the number column), where it starts,
/// how wide it is, padding included, and its heading's lines.
#[derive(Debug, Clone, PartialEq)]
struct Column {
    kind: Option<CellKind>,
    x: f64,
    width: f64,
    heading: Vec<String>,
}

/// A cell's lines as drawn, and its words for a reader (D-88).
#[derive(Debug, Clone, PartialEq, Eq)]
struct Lines {
    lines: Vec<String>,
    words: String,
}

/// A row as placed on a page: the whole row, or one part of a row taller
/// than a page.
#[derive(Debug, Clone, PartialEq)]
struct Part {
    /// Its row's index in the published rows.
    row: usize,
    /// FR-18's number, repeated on every part.
    number: Option<usize>,
    /// The sector label, for a page's heading and the sector rule.
    sector: String,
    /// One cell per published column, the number column aside.
    cells: Vec<Lines>,
    /// A muted line under the name: the unplotted flag, or that the row
    /// continues on the next page.
    marker: Option<&'static str>,
    /// Whether the unplotted flag is drawn under the name as well.
    flagged: bool,
    /// Its height, `2 · TABLE_PAD_Y + n · TABLE_LEADING`.
    height: f64,
    /// Whether a rule closes it: every part but a split row's earlier ones.
    ruled: bool,
    /// The one cell of an empty table, spanning every column (FR-106).
    spanning: bool,
}

impl Part {
    /// Its name cell's lines below the name: the flag, then the marker.
    fn markers(&self) -> Vec<&'static str> {
        self.flagged
            .then_some(NOT_ON_THE_RADAR)
            .into_iter()
            .chain(self.marker)
            .collect()
    }
}

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

/// `count` as an `f64`, exact for any count a table holds.
fn count(count: usize) -> f64 {
    f64::from(u32::try_from(count).unwrap_or(u32::MAX))
}

/// `paragraphs` wrapped at `width` in `style`; an empty paragraph is an
/// empty line, so blank lines stay (FR-97).
fn wrap(style: TextStyle, paragraphs: &[&str], width: f64, fonts: &Fonts) -> Vec<String> {
    paragraphs
        .iter()
        .flat_map(|paragraph| {
            let lines = style.wrap(paragraph, width, fonts);
            if lines.is_empty() {
                vec![String::new()]
            } else {
                lines
            }
        })
        .collect()
}

/// A heading's pieces: its words, each broken after any underscore, with
/// whether a space comes before each (the maintainer's ruling,
/// 2026-10-23: a heading never breaks mid-word).
fn pieces(heading: &str) -> Vec<(String, bool)> {
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
/// pieces; a piece wider than `width` stands alone.
fn heading_lines(heading: &str, width: f64, fonts: &Fonts) -> Vec<String> {
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
    lines
}

/// The style a cell of `kind` is drawn in.
const fn style_of(kind: CellKind) -> TextStyle {
    match kind {
        CellKind::Name => TABLE_NAME,
        CellKind::Label | CellKind::Prose | CellKind::Link => TABLE_TEXT,
    }
}

/// The most lines that fit `room` in one part, at least one.
fn lines_within(room: f64) -> usize {
    let mut k = 1;
    loop {
        if height_of(k + 1) > room {
            return k;
        }
        k += 1;
    }
}

/// A part's height for `lines` lines in its tallest cell.
#[expect(
    clippy::suboptimal_flops,
    reason = "NFR-4: plain * and + are bit-identical on every target; mul_add falls back to a libm fma where the CPU lacks one"
)]
fn height_of(lines: usize) -> f64 {
    PADS_Y + count(lines) * TABLE_LEADING
}

/// Column `c`'s measure: for a label, its natural width (its widest text,
/// padded) and its least (its heading's widest piece, and in the name
/// column the flag of an unplotted row and, when a row is split, the mark
/// of its later parts, all unbroken); for prose, its weight, its total
/// characters. The pair says whether any row is unplotted, and any split.
fn measure(
    published: &Published<'_>,
    c: usize,
    (kind, heading): (CellKind, &str),
    (unplotted, split): (bool, bool),
    fonts: &Fonts,
) -> (CellKind, f64, f64) {
    let cells = published.rows.iter().filter_map(|row| row.cells.get(c));
    if kind == CellKind::Prose {
        let chars: usize = cells
            .flat_map(Cell::paragraphs)
            .map(|p| p.chars().count())
            .sum();
        return (kind, count(chars.max(1)), 0.0);
    }
    let widest = cells
        .flat_map(Cell::paragraphs)
        .map(|text| style_of(kind).measure(&words(text), fonts))
        .fold(0.0, f64::max);
    let piece = pieces(heading)
        .iter()
        .map(|(piece, _)| TABLE_HEAD.measure(piece, fonts))
        .fold(0.0, f64::max);
    let flag = if kind == CellKind::Name && unplotted {
        TABLE_TEXT.measure(NOT_ON_THE_RADAR, fonts)
    } else {
        0.0
    };
    let mark = if kind == CellKind::Name && split {
        TABLE_NAME.measure(CONTINUED_WORD, fonts)
    } else {
        0.0
    };
    let least = piece.max(flag).max(mark) + PADS_X;
    (kind, widest.max(piece) + PADS_X, least)
}

/// The column widths: the number column its content; each label column
/// its widest text, up to a cap the prose may lower; each prose column at
/// least [`TABLE_PROSE_MIN`], sharing the rest by how much it holds.
/// Returns the columns, the cap and whether the table is cramped.
fn columns(
    published: &Published<'_>,
    width: f64,
    split: bool,
    fonts: &Fonts,
) -> (Vec<Column>, f64, bool) {
    let number = published
        .rows
        .iter()
        .filter_map(|row| row.number)
        .map(|n| TABLE_TEXT.measure(&n.to_string(), fonts))
        .fold(TABLE_HEAD.measure(NUMBER_HEADING, fonts), f64::max)
        + PADS_X;
    let unplotted = published.rows.iter().any(|row| row.number.is_none());
    // Per column: its natural width and least width (labels), or its
    // weight (prose).
    let measured: Vec<(CellKind, f64, f64)> = published
        .columns
        .iter()
        .enumerate()
        .map(|(c, &(kind, heading))| {
            measure(published, c, (kind, heading), (unplotted, split), fonts)
        })
        .collect();
    let label = |natural: f64, least: f64, cap: f64| natural.min(cap).max(least);
    let room = |cap: f64| {
        width
            - number
            - measured
                .iter()
                .filter(|(kind, ..)| *kind != CellKind::Prose)
                .map(|&(_, natural, least)| label(natural, least, cap))
                .sum::<f64>()
    };
    let prose = measured
        .iter()
        .filter(|(kind, ..)| *kind == CellKind::Prose)
        .count();
    let need = count(prose) * TABLE_PROSE_MIN;
    let fits = |s: f64| room(TABLE_LABEL_CAP * s) >= need;
    let floor = TABLE_LABEL_FLOOR / TABLE_LABEL_CAP;
    let cramped = !fits(floor);
    let cap = if fits(1.0) {
        TABLE_LABEL_CAP
    } else if cramped {
        TABLE_LABEL_FLOOR
    } else {
        TABLE_LABEL_CAP * fit_scale(floor, fits)
    };
    let left = room(cap);
    let weights: f64 = measured
        .iter()
        .filter(|(kind, ..)| *kind == CellKind::Prose)
        .map(|&(_, weight, _)| weight)
        .sum();
    let share = |weight: f64| {
        if cramped {
            // Too narrow either way: an equal share, never so narrow that
            // a line holds nothing.
            (left / count(prose.max(1))).max(PADS_X + TABLE_TEXT.size)
        } else {
            TABLE_PROSE_MIN + (left - need) * weight / weights
        }
    };
    let mut x = 0.0;
    let mut place = |kind: Option<CellKind>, w: f64, heading: Vec<String>| {
        let column = Column {
            kind,
            x,
            width: w,
            heading,
        };
        x += w;
        column
    };
    let mut out = vec![place(None, number, vec![NUMBER_HEADING.to_owned()])];
    for (&(kind, natural, least), (_, heading)) in measured.iter().zip(&published.columns) {
        let w = if kind == CellKind::Prose {
            share(natural)
        } else {
            label(natural, least, cap)
        };
        let lines = heading_lines(heading, w - PADS_X, fonts);
        out.push(place(Some(kind), w, lines));
    }
    (out, cap, cramped)
}

/// A whole row's cells, wrapped to their columns.
fn cells(row: &PublishedRow<'_>, columns: &[Column], fonts: &Fonts) -> Vec<Lines> {
    row.cells
        .iter()
        .zip(columns.iter().skip(1))
        .map(|(cell, column)| {
            let kind = column.kind.unwrap_or(CellKind::Label);
            let paragraphs = cell.paragraphs();
            Lines {
                lines: wrap(style_of(kind), &paragraphs, column.width - PADS_X, fonts),
                words: words(&paragraphs.join(" ")),
            }
        })
        .collect()
}

/// A split row's name on its later parts: its first line, marked
/// [`CONTINUED`], wrapped to the name column (`name`, among the cells).
fn continued_name(
    full: &[Lines],
    name: Option<usize>,
    columns: &[Column],
    fonts: &Fonts,
) -> Vec<String> {
    let first_name = name
        .and_then(|c| full.get(c))
        .and_then(|cell| cell.lines.first())
        .cloned()
        .unwrap_or_default();
    let name_width = name
        .and_then(|c| columns.get(c + 1))
        .map_or(0.0, |c| c.width - PADS_X);
    wrap(
        TABLE_NAME,
        &[&format!("{first_name}{CONTINUED}")],
        name_width,
        fonts,
    )
}

/// A row's parts: the whole row when it fits `room`, else parts of whole
/// lines, each fitting `room` and taking at least one line, the first
/// marked [`CONTINUES`] and each later one naming the item
/// [`CONTINUED`].
fn parts(
    index: usize,
    row: &PublishedRow<'_>,
    columns: &[Column],
    room: f64,
    fonts: &Fonts,
) -> Vec<Part> {
    let full = cells(row, columns, fonts);
    let name = columns
        .iter()
        .skip(1)
        .position(|c| c.kind == Some(CellKind::Name));
    let flagged = row.number.is_none();
    let flag = usize::from(flagged);
    let tallest = |cells: &[Lines], extra: usize| {
        cells
            .iter()
            .enumerate()
            .map(|(c, cell)| cell.lines.len() + if Some(c) == name { extra } else { 0 })
            .max()
            .unwrap_or(1)
            .max(1)
    };
    let part = |cells: Vec<Lines>, marker: Option<&'static str>, ruled: bool| {
        let extra = flag + usize::from(marker.is_some());
        Part {
            row: index,
            number: row.number,
            sector: row.sector.to_owned(),
            height: height_of(tallest(&cells, extra)),
            cells,
            marker,
            flagged,
            ruled,
            spanning: false,
        }
    };
    if height_of(tallest(&full, flag)) <= room {
        return vec![part(full, None, true)];
    }
    let most = lines_within(room);
    let continued = continued_name(&full, name, columns, fonts);
    let mut taken = vec![0; full.len()];
    let mut out = Vec::new();
    loop {
        let header = |c: usize| {
            if Some(c) == name && !out.is_empty() {
                continued.len()
            } else {
                0
            }
        };
        let remaining = |c: usize, taken: &[usize]| {
            full.get(c)
                .map_or(0, |cell| cell.lines.len())
                .saturating_sub(taken.get(c).copied().unwrap_or(0))
        };
        let reserve = |c: usize| if Some(c) == name { flag } else { 0 };
        let last = (0..full.len()).all(|c| header(c) + remaining(c, &taken) + reserve(c) <= most);
        let marker = (!last).then_some(CONTINUES);
        let mut take: Vec<usize> = (0..full.len())
            .map(|c| {
                let busy = header(c)
                    + reserve(c)
                    + if Some(c) == name {
                        usize::from(!last)
                    } else {
                        0
                    };
                remaining(c, &taken).min(most.saturating_sub(busy))
            })
            .collect();
        if take.iter().all(|&t| t == 0) {
            // Nothing fits beside the headers: the longest cell gives a
            // line anyway, so the split always ends.
            let longest = (0..full.len())
                .max_by_key(|&c| remaining(c, &taken))
                .unwrap_or(0);
            if let Some(t) = take.get_mut(longest) {
                *t = 1;
            }
        }
        let cells: Vec<Lines> = full
            .iter()
            .enumerate()
            .map(|(c, cell)| {
                let from = taken.get(c).copied().unwrap_or(0);
                let to = from + take.get(c).copied().unwrap_or(0);
                let mut lines: Vec<String> = if header(c) > 0 {
                    continued.clone()
                } else {
                    Vec::new()
                };
                lines.extend(cell.lines.get(from..to).unwrap_or_default().iter().cloned());
                let words = words(&lines.join(" "));
                Lines { lines, words }
            })
            .collect();
        for (t, more) in taken.iter_mut().zip(&take) {
            *t += more;
        }
        out.push(part(cells, marker, last));
        if last {
            return out;
        }
    }
}

impl TableBlock {
    /// `published` measured for a page `width` wide and `page_height` tall:
    /// its columns sized, its cells wrapped, rows taller than a page split
    /// and the rows cut into pages, in one pass.
    #[must_use]
    pub fn new(published: &Published<'_>, width: f64, page_height: f64, fonts: &Fonts) -> Self {
        // Sized as if no row splits; when one does, sized again so the name
        // column holds its later parts' mark unbroken. The second sizing
        // reserves the mark, so whatever it splits is marked whole.
        let laid = |split: bool| {
            let (columns, cap, cramped) = columns(published, width, split, fonts);
            let header_lines = columns.iter().map(|c| c.heading.len()).max().unwrap_or(1);
            let head_height =
                TABLE_HEADING_BASELINE + TABLE_HEADING_TO_HEAD + height_of(header_lines);
            let room = page_height - head_height;
            let parts: Vec<Part> = published
                .rows
                .iter()
                .enumerate()
                .flat_map(|(i, row)| parts(i, row, &columns, room, fonts))
                .collect();
            (columns, cap, cramped, head_height, room, parts)
        };
        let mut laid_out = laid(false);
        if laid_out.5.iter().any(|part| !part.ruled) {
            laid_out = laid(true);
        }
        let (columns, cap, cramped, head_height, room, mut parts) = laid_out;
        let tall = published
            .rows
            .iter()
            .enumerate()
            .filter(|(i, _)| parts.iter().filter(|p| p.row == *i).count() > 1)
            .map(|(_, row)| {
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
            .collect();
        if published.says_empty {
            parts.push(Part {
                row: 0,
                number: None,
                sector: String::new(),
                cells: vec![Lines {
                    lines: vec![NOTHING_PLOTTED.to_owned()],
                    words: NOTHING_PLOTTED.to_owned(),
                }],
                marker: None,
                flagged: false,
                height: height_of(1),
                ruled: true,
                spanning: true,
            });
        }
        let mut tops = vec![0.0];
        for part in &parts {
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
        Self {
            heading: published.heading,
            width,
            columns,
            head_height,
            room,
            cap,
            cramped,
            parts,
            pages,
            tall,
        }
    }

    /// The table's pages, in order.
    #[must_use]
    pub fn pages(&self) -> &[TablePage] {
        &self.pages
    }

    /// Whether the prose columns are narrower than [`TABLE_PROSE_MIN`]
    /// even with the label columns at their floor.
    #[must_use]
    pub const fn cramped(&self) -> bool {
        self.cramped
    }

    /// Each row split across pages: its CSV line and name, in order.
    #[must_use]
    pub fn tall_rows(&self) -> &[(usize, String)] {
        &self.tall
    }

    /// `page` drawn with its top-left at `origin`, in reading order: the
    /// heading, the header row, then each row, its cells left to right and
    /// its rule after it.
    #[must_use]
    pub fn render(&self, page: &TablePage, origin: Point, fonts: &Fonts) -> Vec<Node> {
        let parts = self.parts.get(page.parts.clone()).unwrap_or_default();
        let mut out = vec![self.heading_node(page, parts, origin, fonts)];
        let header_top = origin.y + TABLE_HEADING_BASELINE + TABLE_HEADING_TO_HEAD;
        let header_cells = self
            .columns
            .iter()
            .map(|column| {
                let (x, anchor) = column_text_x(column, origin.x);
                run(
                    Point {
                        x,
                        y: header_top + TABLE_PAD_Y + TABLE_ASCENT,
                    },
                    TABLE_HEAD,
                    Neutral::TextMuted,
                    anchor,
                    column.heading.clone(),
                    words(&column.heading.join(" ")),
                )
            })
            .collect();
        out.push(Node::Group(Group::new(Point::ORIGIN, header_cells)));
        let mut y = origin.y + self.head_height;
        out.push(self.rule(origin.x, y, TABLE_RULE_HEAVY));
        for (i, part) in parts.iter().enumerate() {
            out.push(self.row_node(part, origin.x, y));
            let bottom = y + part.height;
            if part.ruled {
                let heavy = parts.get(i + 1).is_some_and(|next| {
                    let new_sector = next.sector != part.sector;
                    let boundary = part.number.is_some() && next.number.is_none();
                    new_sector || boundary
                });
                let width = if heavy { TABLE_RULE_HEAVY } else { TABLE_RULE };
                out.push(self.rule(origin.x, bottom, width));
            }
            y = bottom;
        }
        out
    }

    /// The heading, a level-2 heading; on a later page, followed by a run
    /// naming the page's sector and item numbers.
    fn heading_node(&self, page: &TablePage, parts: &[Part], origin: Point, fonts: &Fonts) -> Node {
        let baseline = origin.y + TABLE_HEADING_BASELINE;
        let mut children = vec![run(
            Point {
                x: origin.x,
                y: baseline,
            },
            TABLE_HEADING,
            Neutral::TextPrimary,
            Anchor::Start,
            vec![self.heading.to_owned()],
            self.heading.to_owned(),
        )];
        if !page.first {
            let numbers: Vec<usize> = parts.iter().filter_map(|p| p.number).collect();
            let sector = parts.first().map_or("", |p| p.sector.as_str());
            let range = match (numbers.first(), numbers.last()) {
                (Some(a), Some(b)) if a == b => format!(", {a}"),
                (Some(a), Some(b)) => format!(", {a}\u{2013}{b}"),
                _ => String::new(),
            };
            let continued = CONTINUED.strip_prefix(' ').unwrap_or(CONTINUED);
            let text = format!("{continued}: {sector}{range}");
            let x = origin.x
                + TABLE_HEADING.advance(self.heading, fonts)
                + TABLE_HEADING.advance(" ", fonts);
            children.push(run(
                Point { x, y: baseline },
                TABLE_HEADING,
                Neutral::TextPrimary,
                Anchor::Start,
                vec![text.clone()],
                text,
            ));
        }
        Node::Group(
            Group::new(Point::ORIGIN, children)
                .with(Semantics::Heading(NonZeroU8::MIN.saturating_add(1))),
        )
    }

    /// One part's row: a group per cell, each holding its texts.
    fn row_node(&self, part: &Part, left: f64, top: f64) -> Node {
        let baseline = top + TABLE_PAD_Y + TABLE_ASCENT;
        let at = |x: f64, line: usize| Point {
            x,
            y: line_y(baseline, line),
        };
        if part.spanning {
            let cell = part.cells.first().cloned().unwrap_or(Lines {
                lines: Vec::new(),
                words: String::new(),
            });
            let text = run(
                at(left + TABLE_PAD_X, 0),
                TABLE_TEXT,
                Neutral::TextPrimary,
                Anchor::Start,
                cell.lines,
                cell.words,
            );
            return Node::Group(Group::new(
                Point::ORIGIN,
                vec![Node::Group(Group::new(Point::ORIGIN, vec![text]))],
            ));
        }
        let mut cells = Vec::new();
        if let Some(column) = self.columns.first() {
            let number = part.number.map(|n| n.to_string()).unwrap_or_default();
            let (x, anchor) = column_text_x(column, left);
            cells.push(Node::Group(Group::new(
                Point::ORIGIN,
                vec![run(
                    at(x, 0),
                    TABLE_TEXT,
                    Neutral::TextPrimary,
                    anchor,
                    vec![number.clone()],
                    number,
                )],
            )));
        }
        for (cell, column) in part.cells.iter().zip(self.columns.iter().skip(1)) {
            let kind = column.kind.unwrap_or(CellKind::Label);
            let (x, anchor) = column_text_x(column, left);
            let mut texts = vec![run(
                at(x, 0),
                style_of(kind),
                Neutral::TextPrimary,
                anchor,
                cell.lines.clone(),
                cell.words.clone(),
            )];
            if kind == CellKind::Name {
                for (k, marker) in part.markers().into_iter().enumerate() {
                    texts.push(run(
                        at(x, cell.lines.len() + k),
                        TABLE_TEXT,
                        Neutral::TextMuted,
                        Anchor::Start,
                        vec![marker.to_owned()],
                        marker.to_owned(),
                    ));
                }
            }
            cells.push(Node::Group(Group::new(Point::ORIGIN, texts)));
        }
        Node::Group(Group::new(Point::ORIGIN, cells))
    }

    /// A rule across the table at `y`, `width` thick, decorative.
    fn rule(&self, left: f64, y: f64, width: f64) -> Node {
        Node::Group(
            Group::new(
                Point::ORIGIN,
                vec![Node::Line {
                    from: Point { x: left, y },
                    to: Point {
                        x: left + self.width,
                        y,
                    },
                    stroke: Paint::Role(Role::Neutral(Neutral::StrokeDivider)),
                    stroke_width: width,
                }],
            )
            .with(Semantics::Decorative),
        )
    }
}

/// The baseline of line `line` (0 the first) below `baseline`.
#[expect(
    clippy::suboptimal_flops,
    reason = "NFR-4: plain * and + are bit-identical on every target; mul_add falls back to a libm fma where the CPU lacks one"
)]
fn line_y(baseline: f64, line: usize) -> f64 {
    baseline + count(line) * TABLE_LEADING
}

/// Where `column`'s text sits in a table whose left edge is `left`: the
/// number column's right-aligned at its padding, every other at its left
/// padding.
fn column_text_x(column: &Column, left: f64) -> (f64, Anchor) {
    if column.kind.is_none() {
        (left + column.x + column.width - TABLE_PAD_X, Anchor::End)
    } else {
        (left + column.x + TABLE_PAD_X, Anchor::Start)
    }
}

/// A text node at scale 1.
fn run(
    origin: Point,
    style: TextStyle,
    ink: Neutral,
    anchor: Anchor,
    lines: Vec<String>,
    words: String,
) -> Node {
    TextRun {
        origin,
        style,
        ink,
        anchor,
        lines,
        words,
        leading: if lines_leading(style) {
            TABLE_LEADING
        } else {
            LINE_HEIGHT
        },
    }
    .node(Scale::uniform(1.0))
}

/// Whether `style` is a cell's, drawn at the table's leading.
fn lines_leading(style: TextStyle) -> bool {
    style != TABLE_HEADING
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::block::fonts;
    use crate::layout::test_support::{PRD_EXAMPLE, bounds, plotted};
    use crate::model::Catalogue;
    use crate::options::Table;
    use crate::order::Plotted;
    use crate::palette::Role;
    use crate::published::RADAR_HEADING;
    use crate::published::published;
    use crate::style::{PAPER_FOOT_ROOM, PAPER_MARGIN};
    use crate::svg::{Paint, TextPaint};

    const HOSTILE: &str = include_str!("../../tests/fixtures/table-hostile.csv");

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
        assert!(width_of(&block, "name") < TABLE_LABEL_CAP);
        assert!(width_of(&block, "description") >= TABLE_PROSE_MIN);
        assert!(!block.cramped());
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
        assert!((width_of(&block, "name") - TABLE_LABEL_CAP).abs() < 1e-9);
        let part = block.parts.first().expect("a row");
        assert!(part.cells.first().is_some_and(|c| c.lines.len() >= 2));
    }

    #[test]
    fn a_label_column_is_its_widest_text() {
        let csv = "name,sector,ring,description\nA,Data & Analytics,Adopt,d\n";
        let block = block(csv, Table::Radar);
        let widest = TABLE_TEXT.measure("Data & Analytics", fonts()) + PADS_X;
        assert!((width_of(&block, "Sector") - widest).abs() < 1e-9);
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
        assert!(!block(PRD_EXAMPLE, Table::Radar).cramped());
    }

    #[test]
    fn the_cap_comes_down_before_cramping() {
        let rows = (0..3)
            .map(|i| format!("A long name for item {i} that wraps in its column,Platforms,Adopt,d,r,dom,n,m\n"))
            .collect::<Vec<_>>()
            .concat();
        let csv = format!("name,sector,ring,description,rationale,domain,notes,more\n{rows}");
        let block = block(&csv, Table::All);
        assert!(!block.cramped());
        assert!(
            block.cap > TABLE_LABEL_FLOOR && block.cap < TABLE_LABEL_CAP,
            "{}",
            block.cap
        );
        for heading in ["description", "rationale", "domain", "notes", "more"] {
            assert!(
                width_of(&block, heading) >= TABLE_PROSE_MIN - 1e-6,
                "{heading}"
            );
        }
    }

    #[test]
    fn too_many_columns_cramp() {
        let mut lines = HOSTILE.lines();
        let head = format!("{},o1,o2,o3,o4", lines.next().expect("a header"));
        let rows = lines
            .map(|l| format!("{l},,,,\n"))
            .collect::<Vec<_>>()
            .concat();
        let block = block(&format!("{head}\n{rows}"), Table::All);
        assert!(block.cramped());
        let flag = TABLE_TEXT.measure(NOT_ON_THE_RADAR, fonts()) + PADS_X;
        for column in &block.columns {
            if column.kind == Some(CellKind::Label) || column.kind == Some(CellKind::Link) {
                let piece = column
                    .heading
                    .iter()
                    .map(|line| TABLE_HEAD.measure(line, fonts()))
                    .fold(0.0, f64::max)
                    + PADS_X;
                assert!(
                    column.width <= TABLE_LABEL_FLOOR.max(piece) + 1e-9,
                    "{:?}",
                    column.heading
                );
            }
            if column.kind == Some(CellKind::Name) {
                assert!(column.width <= flag.max(TABLE_LABEL_FLOOR) + 1e-9);
            }
        }
        assert_ne!(all_pages(&block).len(), 0);
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
        let block = block(HOSTILE, Table::All);
        let nodes = block.render(
            block.pages().first().expect("a page"),
            Point::ORIGIN,
            fonts(),
        );
        let muted = TextPaint::Fill(Paint::Role(Role::Neutral(Neutral::TextMuted)));
        let primary = TextPaint::Fill(Paint::Role(Role::Neutral(Neutral::TextPrimary)));
        let all = texts(&nodes);
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

    #[test]
    fn the_number_column_never_wraps_and_ends_at_its_padding() {
        let csv = (0..1_200).fold("name,sector,ring,description\n".to_owned(), |csv, i| {
            csv + &format!("item {i:04},Tools,Adopt,d\n")
        });
        let block = block(&csv, Table::Radar);
        let number = block.columns.first().expect("the number column");
        assert_eq!(number.kind, None);
        let right = number.x + number.width - TABLE_PAD_X;
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
                if lines.len() == 1 && lines.iter().all(|l| l.parse::<usize>().is_ok()) {
                    assert_eq!(*anchor, Anchor::End);
                    assert!((origin.x - right).abs() < 1e-9);
                }
            }
        }
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
            part.cells.get(3).map(|c| c.lines.clone()),
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
            .and_then(|p| p.cells.get(4))
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
        assert!((part.height - (PADS_Y + three)).abs() < 1e-9);
    }

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
        let first = parts.first().expect("a part");
        assert_eq!(first.marker, Some(CONTINUES));
        assert!(!first.ruled);
        let later = parts.get(1).expect("a second part");
        assert_eq!(
            later.cells.first().map(|c| c.words.clone()),
            Some(format!("Beta{CONTINUED}"))
        );
        assert!(
            later
                .cells
                .first()
                .is_some_and(|c| c.lines.iter().all(|l| !l.ends_with("(contin")))
        );
        assert_eq!(later.number, first.number);
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
        let csv = format!(
            "name,sector,ring,description\nAlpha,Tools,Adopt,{tall}\nb{name},Tools,Adopt,d\n"
        );
        let block = block(&csv, Table::Radar);
        let lines: Vec<usize> = block.tall_rows().iter().map(|(line, _)| *line).collect();
        assert_eq!(lines, [2, 3]);
        assert!(block.parts.iter().all(|p| p.height <= block.room + 1e-9));
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
}
