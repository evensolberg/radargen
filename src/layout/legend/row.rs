//! One legend row's composition (FR-74, FR-94): badge, wrapped name and
//! meta, framed for its target size. The walk that places rows is the
//! `layout` submodule's.

use crate::model::{Item, Qualifier, Status};
use crate::palette::Neutral;
use crate::palette::Role;
use crate::style::{
    LEGEND_BADGE_X, LEGEND_FIRST_BASELINE, LEGEND_META, LEGEND_NAME, LEGEND_NAME_COLUMN,
    LEGEND_NAME_X, LEGEND_ROW_PAD, LEGEND_TEXT_MIN_HEIGHT, LINE_HEIGHT, META_GAP,
};
use crate::svg::{Anchor, Control, ItemKey, ListKey, Paint, Point, Tail, TextRun};
use crate::text::{Fonts, words};

/// Between a row's qualifier and status (FR-74).
const META_SEPARATOR: &str = " \u{b7} ";
/// Between a row's name and its inline meta: a no-break space (see `tail`).
const SEPARATOR: &str = "\u{a0}";

/// A plotted item and its FR-34 number, as the walk finds them.
#[derive(Debug, Clone, Copy)]
pub(super) struct Numbered<'a> {
    pub(super) item: &'a Item,
    /// FR-34's number: 1-based, in legend order.
    pub(super) number: usize,
}

/// FR-74's meta as a row draws it.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum MetaLine {
    /// Continuing the name's last line, inside the name's own text, so a
    /// reader's text spacing pushes it along rather than into the name
    /// (D-78).
    Inline(Tail),
    /// On its own line below the name.
    OwnLine(TextRun),
}

/// One plotted item's row: badge, wrapped name and meta (FR-74).
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Row {
    /// The row's top edge; rows abut.
    pub(super) top: f64,
    /// The row's height, at least the minimum it was laid out with:
    /// `LEGEND_ROW_MIN_HEIGHT` (FR-94), or `PAPER_ROW_MIN_HEIGHT` on paper.
    pub(super) height: f64,
    /// The badge's centre.
    pub(super) badge: Point,
    /// The sector's FR-10 rank: its series index, unreduced (FR-67).
    pub(super) series: usize,
    /// The ring's list in a tagged PDF: catalogue sector and ring
    /// (FR-114), so a ring broken across columns or pages is one list.
    pub(super) list: ListKey,
    /// Whether the badge is hollow (FR-20).
    pub(super) hollow: bool,
    /// FR-34's number.
    pub(super) number: usize,
    /// The name, wrapped at the name column (FR-46, FR-79).
    pub(super) name: TextRun,
    /// The qualifier and status, inline after the name or on their own
    /// line; `None` when the row shows neither.
    pub(super) meta: Option<MetaLine>,
    /// The row's key (its CSV line), FR-75's accessible name, no tooltip.
    pub(super) control: Control,
}

impl Row {
    /// A numbered item as a row from `top`, at least `min_height` tall,
    /// with its sector's `series`, the `ring_list` it belongs to (FR-114)
    /// and FR-75's accessible name `label`.
    pub(super) fn new(
        numbered: Numbered<'_>,
        (series, ring_list): (usize, ListKey),
        label: String,
        top: f64,
        min_height: f64,
        fonts: &Fonts,
    ) -> Self {
        let Numbered { item, number } = numbered;
        let name = LEGEND_NAME.wrap(&item.name, LEGEND_NAME_COLUMN, fonts);
        let meta = meta_text(item.qualifier, item.status).map(|text| fitted(text, &name, fonts));
        let frame = frame(top, line_count(&name, meta.as_ref()), min_height);
        let name = name_run(&item.name, name, frame.first_baseline);
        let last = name.last_baseline();
        Self {
            top,
            height: frame.height,
            badge: frame.badge,
            series,
            list: ring_list,
            hollow: item.hollow(),
            number,
            meta: meta.map(|meta| meta.line(last)),
            name,
            control: Control {
                key: ItemKey(item.line),
                label,
                tooltip: None,
            },
        }
    }

    /// The row's bottom edge, where the next row starts.
    pub(super) fn bottom(&self) -> f64 {
        self.top + self.height
    }

    /// The furthest right the row's text reaches at its layout width, an
    /// inline meta included after the name's layout width (D-78).
    pub(super) fn right_edge(&self, fonts: &Fonts) -> f64 {
        let meta = self.meta.as_ref().map_or(0.0, |meta| match meta {
            MetaLine::Inline(tail) => {
                self.name.origin.x + inline_end(&self.name.lines, tail, fonts)
            }
            MetaLine::OwnLine(run) => run.right_edge(fonts),
        });
        self.name.right_edge(fonts).max(meta)
    }
}

/// FR-74: whether an inline meta ending at `end` fits within `column`.
/// Inclusive: exactly filling the column fits.
#[must_use]
fn fits_inline(end: f64, column: f64) -> bool {
    end <= column
}

/// Where an inline meta ends, from the name's origin, at the layout width:
/// the last line's layout width, the gap, and the tail's (D-78).
fn inline_end(name: &[String], tail: &Tail, fonts: &Fonts) -> f64 {
    let last = name.last().map_or("", String::as_str);
    LEGEND_NAME.measure(last, fonts) + tail.layout_width(fonts)
}

/// The meta as a tail: a no-break space, so copied text reads two words,
/// then the meta, its gap set so the meta's first glyph lands `META_GAP`
/// after the name's drawn end. No-break, because `resvg` drops a leading
/// U+0020 in a tspan, glyph and advance, where a browser keeps it; both
/// keep U+00A0, so the PNG and the page agree (SC-7).
fn tail(text: &str, fonts: &Fonts) -> Tail {
    let gap = META_GAP - LEGEND_META.advance(SEPARATOR, fonts);
    debug_assert!(gap >= 0.0, "the space fits in the meta gap");
    Tail {
        gap,
        style: LEGEND_META,
        ink: Paint::Role(Role::Neutral(Neutral::TextMuted)),
        text: [SEPARATOR, text].concat(),
    }
}

/// FR-74's meta: the qualifier and status as the row shows them, omitting
/// an absent value and `NoChange`, which is the plain marker (FR-40).
fn meta_text(qualifier: Option<Qualifier>, status: Option<Status>) -> Option<String> {
    let parts: Vec<&str> = [
        qualifier.map(Qualifier::label),
        status.and_then(Status::shown).map(Status::label),
    ]
    .into_iter()
    .flatten()
    .collect();
    (!parts.is_empty()).then(|| parts.join(META_SEPARATOR))
}

/// A row's meta, fitted against its name's last line: inline when it fits
/// there at the layout width, so a reader's text spacing still fits (FR-74,
/// D-78); otherwise on its own line.
enum Fitted {
    Inline(Tail),
    OwnLine(String),
}

impl Fitted {
    /// The meta as drawn: the name's tail, or its own line below the name's
    /// last baseline `last`.
    fn line(self, last: f64) -> MetaLine {
        match self {
            Self::Inline(tail) => MetaLine::Inline(tail),
            Self::OwnLine(text) => MetaLine::OwnLine(TextRun {
                origin: Point {
                    x: LEGEND_NAME_X,
                    y: last + LINE_HEIGHT,
                },
                style: LEGEND_META,
                ink: Neutral::TextMuted,
                anchor: Anchor::Start,
                words: text.clone(),
                lines: vec![text],
                leading: LINE_HEIGHT,
            }),
        }
    }
}

/// `text` fitted against the last of `name`'s lines.
fn fitted(text: String, name: &[String], fonts: &Fonts) -> Fitted {
    let tail = tail(&text, fonts);
    if fits_inline(inline_end(name, &tail, fonts), LEGEND_NAME_COLUMN) {
        Fitted::Inline(tail)
    } else {
        Fitted::OwnLine(text)
    }
}

/// The row's text lines: the name's, and the meta's own line if it has one.
fn line_count(name: &[String], meta: Option<&Fitted>) -> usize {
    name.len() + usize::from(matches!(meta, Some(Fitted::OwnLine(_))))
}

/// Where a row's parts sit.
struct Frame {
    height: f64,
    first_baseline: f64,
    badge: Point,
}

/// A row from `top` holding `lines` lines of text: its height, floored at
/// `min_height` (FR-94, FR-111); its text block centred in it; the badge at its middle.
fn frame(top: f64, lines: usize, min_height: f64) -> Frame {
    #[expect(
        clippy::cast_precision_loss,
        clippy::as_conversions,
        reason = "a line count is far below 2^53, so the conversion is exact"
    )]
    let text = lines as f64 * LINE_HEIGHT;
    let height = min_height.max(text + LEGEND_ROW_PAD);
    let block = LEGEND_TEXT_MIN_HEIGHT.max(text);
    let block_top = f64::midpoint(top, top + height - block);
    Frame {
        height,
        first_baseline: block_top + LEGEND_FIRST_BASELINE,
        badge: Point {
            x: LEGEND_BADGE_X,
            y: f64::midpoint(top, top + height),
        },
    }
}

/// A row's name, wrapped into `lines`, from `baseline`.
fn name_run(name: &str, lines: Vec<String>, baseline: f64) -> TextRun {
    TextRun {
        origin: Point {
            x: LEGEND_NAME_X,
            y: baseline,
        },
        style: LEGEND_NAME,
        ink: Neutral::TextPrimary,
        anchor: Anchor::Start,
        lines,
        words: words(name),
        leading: LINE_HEIGHT,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::block::{close, fonts};
    use crate::layout::test_support::{
        HEAD, HEAD_META, NAME_FULL_LINE, NAME_THREE_LINES, NAME_TWO_LINES, near, plotted,
    };
    use crate::palette::Role;
    use crate::style::LEGEND_ROW_MIN_HEIGHT;
    use crate::svg::{Paint, Scale, Tail};

    /// Each of `csv`'s items as a row from the top, so positions read
    /// relative to the row; the walk's placement is the layout's test.
    fn rows(csv: &str) -> Vec<Row> {
        let (catalogue, _) = plotted(csv, None);
        (1..)
            .zip(&catalogue.items)
            .map(|(number, item)| {
                Row::new(
                    Numbered { item, number },
                    (0, ListKey::Key),
                    String::new(),
                    0.0,
                    LEGEND_ROW_MIN_HEIGHT,
                    fonts(),
                )
            })
            .collect()
    }

    fn only(csv: &str) -> Row {
        let rows = rows(csv);
        let count = rows.len();
        let Ok([row]) = <[Row; 1]>::try_from(rows) else {
            panic!("expected one row, got {count}");
        };
        row
    }

    /// mockup.html l.733–735: block = max(17, n · 15), row = block + 7,
    /// floored at 36 (FR-94, D-71); the block centred in the row and the
    /// first baseline 9 below its top: (36 − 17)/2 + 9, (37 − 30)/2 + 9,
    /// (52 − 45)/2 + 9.
    #[test]
    fn rows_of_one_two_and_three_lines() {
        let (two, three) = (NAME_TWO_LINES, NAME_THREE_LINES);
        let rows = rows(&format!(
            "{HEAD}A,Q,Adopt,d\n{two},Q,Adopt,d\n{three},Q,Adopt,d\n"
        ));
        let got: Vec<(usize, f64, f64)> = rows
            .iter()
            .map(|r| (r.name.lines.len(), r.height, r.name.origin.y))
            .collect();
        let want = [(1, 36.0, 18.5), (2, 37.0, 12.5), (3, 52.0, 12.5)];
        assert_eq!(got.len(), want.len(), "{got:?}");
        for (g, w) in got.iter().zip(want) {
            assert!(g.0 == w.0 && close(g.1, w.1) && close(g.2, w.2), "{got:?}");
        }
        for row in &rows {
            assert!(close(row.name.leading, 15.0));
            assert!(close(row.bottom(), row.height));
        }
    }

    /// FR-46, FR-79. 312 = 380 − 68 (D-56; mockup.html l.711).
    #[test]
    fn long_names_wrap_and_long_tokens_break() {
        let fits = "Continuous delivery pipelines for";
        let wraps = "Continuous delivery pipelines for managed infrastructure";
        let word = "Unbreakable".repeat(5);
        let rows = rows(&format!(
            "{HEAD}{fits},Q,Adopt,d\n{wraps},Q,Trial,d\n{word},Q,Assess,d\n"
        ));
        let names: Vec<&[String]> = rows.iter().map(|r| r.name.lines.as_slice()).collect();
        let [whole, wrapped, broken] = names.as_slice() else {
            panic!("three rows: {names:?}");
        };
        assert_eq!(broken.len(), 2, "{broken:?}");
        assert_eq!(broken.concat(), word, "nothing dropped");
        assert_eq!(*wrapped, [fits, "managed infrastructure"]);
        assert_eq!(*whole, [fits]);
        for line in names.iter().flat_map(|lines| lines.iter()) {
            assert!(LEGEND_NAME.measure(line, fonts()) <= 312.0, "{line}");
        }
    }

    /// FR-74: inclusive — exactly filling the column still fits.
    #[test]
    fn fits_inline_is_inclusive() {
        assert!(fits_inline(312.0, 312.0));
        assert!(!fits_inline(312.0 + 1e-9, 312.0));
        assert!(fits_inline(0.0, 0.0));
    }

    /// The inline meta's tail: a space, then the meta, `dx` such that the
    /// meta's first glyph lands `META_GAP` after the name's drawn end.
    fn expected_tail(text: &str) -> Tail {
        Tail {
            gap: META_GAP - LEGEND_META.advance(SEPARATOR, fonts()),
            style: LEGEND_META,
            ink: Paint::Role(Role::Neutral(Neutral::TextMuted)),
            text: [SEPARATOR, text].concat(),
        }
    }

    /// mockup.html l.725–729, l.749–751: meta on the name's last baseline,
    /// 8 after its end. Inter Regular "A" advances 1413/2048 em, 8.624… at
    /// 12.5, so the meta starts at 68 + 8.624… + 8.
    #[test]
    fn meta_rides_inline_when_it_fits() {
        let row = only(&format!("{HEAD_META}A,Q,Adopt,d,default,new\n"));
        let Some(MetaLine::Inline(tail)) = &row.meta else {
            panic!("an inline meta: {row:?}");
        };
        assert_eq!(*tail, expected_tail("Default \u{b7} New"));
        // Drawn where it always was: the space's advance after the tail's
        // start, which is the name's drawn end plus the gap.
        let node = row.name.node_with_tail(tail, Scale::uniform(1.0));
        let first_glyph =
            node.tail_x(fonts()).expect("a tail") + LEGEND_META.advance(SEPARATOR, fonts());
        assert!(close(first_glyph, 84.624_267_578_125), "{first_glyph}");
        assert!(close(row.height, 36.0));
        assert!(!row.hollow, "Default is solid (FR-20)");

        let row = only(&format!("{HEAD_META}A,Q,Adopt,d,conditional,\n"));
        assert!(row.hollow, "Conditional is hollow (FR-20)");
        assert_eq!(
            row.meta,
            Some(MetaLine::Inline(expected_tail("Conditional")))
        );
    }

    /// The row's right edge holds the inline meta at its layout width,
    /// after the name's layout width: where it ends under 1.4.12 (D-78).
    #[test]
    fn the_right_edge_holds_an_inline_meta() {
        let row = only(&format!("{HEAD_META}A,Q,Adopt,d,default,new\n"));
        let tail = expected_tail("Default \u{b7} New");
        let want = 68.0
            + LEGEND_NAME.measure("A", fonts())
            + tail.gap
            + LEGEND_META.measure(&tail.text, fonts());
        assert!(
            close(row.right_edge(fonts()), want),
            "{} vs {want}",
            row.right_edge(fonts())
        );
        let plain = only(&format!("{HEAD}Kafka,Q,Adopt,d\n"));
        assert!(close(
            plain.right_edge(fonts()),
            68.0 + LEGEND_NAME.measure("Kafka", fonts())
        ));
    }

    /// FR-74: the name's layout width, 8, and the meta's layout width pass
    /// 312, so the meta takes its own line: two lines, a 37-unit row, a
    /// 30-unit block from 3.5.
    #[test]
    fn meta_moves_to_its_own_line_when_it_does_not_fit() {
        let name = NAME_FULL_LINE;
        let row = only(&format!("{HEAD_META}{name},Q,Adopt,d,default,new\n"));
        assert_eq!(row.name.lines, [name]);
        let Some(MetaLine::OwnLine(meta)) = &row.meta else {
            panic!("a meta on its own line: {row:?}");
        };
        assert_eq!(
            meta.lines,
            ["Default \u{b7} New"],
            "no leading space on its own line"
        );
        assert!(near(row.name.origin, Point { x: 68.0, y: 12.5 }), "{row:?}");
        assert!(near(meta.origin, Point { x: 68.0, y: 27.5 }), "{meta:?}");
        assert!(
            close(row.height, 37.0) && close(row.badge.y, 18.5),
            "{row:?}"
        );
    }

    #[test]
    fn meta_text_joins_what_the_row_shows() {
        use Qualifier::{Conditional, Default};
        use Status::{MovedIn, New, NoChange};
        let cases = [
            (Some(Default), Some(New), Some("Default \u{b7} New")),
            (Some(Conditional), None, Some("Conditional")),
            (None, Some(MovedIn), Some("Moved In")),
            (Some(Default), Some(NoChange), Some("Default")),
            (None, Some(NoChange), None),
            (None, None, None),
        ];
        for (qualifier, status, want) in cases {
            assert_eq!(
                meta_text(qualifier, status).as_deref(),
                want,
                "{qualifier:?} {status:?}"
            );
        }
    }
}
