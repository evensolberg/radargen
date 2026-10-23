//! Drawing the table's pages (FR-112): the heading, the header row, the
//! rows and their rules, in reading order, each row a group of cell groups
//! so a tagged PDF can tell rows, cells and artifacts apart.

use std::num::NonZeroU8;

use crate::palette::{Neutral, Role};
use crate::published::{CellKind, NUMBER_SPOKEN};
use crate::style::{
    CONTINUED_MARK, LINE_HEIGHT, TABLE_ASCENT, TABLE_HEAD, TABLE_HEADING, TABLE_HEADING_BASELINE,
    TABLE_HEADING_TO_HEAD, TABLE_LEADING, TABLE_PAD_X, TABLE_PAD_Y, TABLE_RULE, TABLE_RULE_HEAVY,
    TABLE_TEXT, TextStyle,
};
use crate::svg::{Anchor, Group, Node, Paint, Point, Scale, Semantics, TextRun};
use crate::text::{Fonts, words};

use super::columns::{Column, style_of};
use super::rows::{Lines, Part};
use super::{TableBlock, TablePage, count};

impl TableBlock {
    /// `page` drawn with its top-left at `origin`, in reading order: the
    /// heading, the header row, then each row, its cells left to right and
    /// its rule after it.
    #[must_use]
    pub fn render(&self, page: &TablePage, origin: Point, fonts: &Fonts) -> Vec<Node> {
        let parts = self.parts.get(page.parts.clone()).unwrap_or_default();
        let mut out = vec![
            self.heading_node(page, parts, origin, fonts),
            self.header_node(origin),
        ];
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

    /// The header row: a group per column, each holding its heading, the
    /// number column's read as [`NUMBER_SPOKEN`].
    fn header_node(&self, origin: Point) -> Node {
        let baseline =
            origin.y + TABLE_HEADING_BASELINE + TABLE_HEADING_TO_HEAD + TABLE_PAD_Y + TABLE_ASCENT;
        let cells = self
            .columns
            .iter()
            .map(|column| {
                let (x, anchor) = column_text_x(column, origin.x);
                let spoken = if column.kind.is_none() {
                    NUMBER_SPOKEN.to_owned()
                } else {
                    words(&column.heading.join(" "))
                };
                let text = run(
                    Point { x, y: baseline },
                    (TABLE_HEAD, Neutral::TextMuted, anchor),
                    Lines {
                        lines: column.heading.clone(),
                        words: spoken,
                    },
                );
                Node::Group(Group::new(Point::ORIGIN, vec![text]))
            })
            .collect();
        Node::Group(Group::new(Point::ORIGIN, cells))
    }

    /// The heading: on the first page a level-2 heading; on a later page a
    /// repeat, its words followed by a run naming the page's sector and
    /// item numbers, which a tagged PDF marks as an artifact.
    fn heading_node(&self, page: &TablePage, parts: &[Part], origin: Point, fonts: &Fonts) -> Node {
        let baseline = origin.y + TABLE_HEADING_BASELINE;
        let style = (TABLE_HEADING, Neutral::TextPrimary, Anchor::Start);
        let mut children = vec![run(
            Point {
                x: origin.x,
                y: baseline,
            },
            style,
            Lines {
                lines: vec![self.heading.to_owned()],
                words: self.heading.to_owned(),
            },
        )];
        if page.first {
            return Node::Group(
                Group::new(Point::ORIGIN, children)
                    .with(Semantics::Heading(NonZeroU8::MIN.saturating_add(1))),
            );
        }
        let numbers: Vec<usize> = parts.iter().filter_map(|p| p.number).collect();
        let sector = parts.first().map_or("", |p| p.sector.as_str());
        let range = match (numbers.first(), numbers.last()) {
            (Some(a), Some(b)) if a == b => format!(", {a}"),
            (Some(a), Some(b)) => format!(", {a}\u{2013}{b}"),
            _ => String::new(),
        };
        let text = format!("{CONTINUED_MARK}: {sector}{range}");
        let x = origin.x
            + TABLE_HEADING.advance(self.heading, fonts)
            + TABLE_HEADING.advance(" ", fonts);
        children.push(run(Point { x, y: baseline }, style, Lines::of_text(&text)));
        Node::Group(Group::new(Point::ORIGIN, children))
    }

    /// One part's row: a group per cell, each holding its texts.
    fn row_node(&self, part: &Part, left: f64, top: f64) -> Node {
        let baseline = top + TABLE_PAD_Y + TABLE_ASCENT;
        let at = |x: f64, line: usize| Point {
            x,
            y: line_y(baseline, line),
        };
        let cell = |texts: Vec<Node>| Node::Group(Group::new(Point::ORIGIN, texts));
        if part.spanning {
            let text = part.cells.first().cloned().unwrap_or_default();
            let style = (TABLE_TEXT, Neutral::TextPrimary, Anchor::Start);
            return Node::Group(Group::new(
                Point::ORIGIN,
                vec![cell(vec![run(at(left + TABLE_PAD_X, 0), style, text)])],
            ));
        }
        let mut cells = Vec::new();
        if let Some(column) = self.columns.first() {
            let (x, anchor) = column_text_x(column, left);
            let number = part.number.map(|n| n.to_string()).unwrap_or_default();
            cells.push(cell(vec![run(
                at(x, 0),
                (TABLE_TEXT, Neutral::TextPrimary, anchor),
                Lines::of_text(&number),
            )]));
        }
        for (c, (lines, column)) in part
            .cells
            .iter()
            .zip(self.columns.iter().skip(1))
            .enumerate()
        {
            let kind = column.kind.unwrap_or(CellKind::Label);
            let (x, anchor) = column_text_x(column, left);
            let mut texts = vec![run(
                at(x, 0),
                (style_of(kind), Neutral::TextPrimary, anchor),
                lines.clone(),
            )];
            if let Some((_, ring)) = part.beneath.as_ref().filter(|(b, _)| *b == c) {
                texts.push(run(
                    at(x, lines.lines.len()),
                    (TABLE_TEXT, Neutral::TextMuted, Anchor::Start),
                    ring.clone(),
                ));
            }
            if kind == CellKind::Name {
                let mut line = lines.lines.len();
                for under in &part.under {
                    texts.push(run(
                        at(x, line),
                        (TABLE_TEXT, Neutral::TextMuted, Anchor::Start),
                        under.clone(),
                    ));
                    line += under.lines.len();
                }
            }
            cells.push(cell(texts));
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

impl Lines {
    /// One line of `text`.
    fn of_text(text: &str) -> Self {
        Self {
            lines: vec![text.to_owned()],
            words: words(text),
        }
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

/// `lines` as a text node at scale 1, `origin` its first baseline, in a
/// style, ink and anchor.
fn run(origin: Point, (style, ink, anchor): (TextStyle, Neutral, Anchor), lines: Lines) -> Node {
    TextRun {
        origin,
        style,
        ink,
        anchor,
        lines: lines.lines,
        words: lines.words,
        leading: if style == TABLE_HEADING {
            LINE_HEIGHT
        } else {
            TABLE_LEADING
        },
    }
    .node(Scale::uniform(1.0))
}
