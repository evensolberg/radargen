//! One legend row's composition (FR-74, FR-94): badge, wrapped name and
//! meta, framed for its target size. The walk that places rows is the
//! `layout` submodule's.

use crate::model::{Item, Qualifier, Status};
use crate::palette::Neutral;
use crate::style::{
    LEGEND_BADGE_X, LEGEND_FIRST_BASELINE, LEGEND_META, LEGEND_NAME, LEGEND_NAME_COLUMN,
    LEGEND_NAME_X, LEGEND_ROW_MIN_HEIGHT, LEGEND_ROW_PAD, LEGEND_TEXT_MIN_HEIGHT, LINE_HEIGHT,
    META_GAP,
};
use crate::svg::{Anchor, Control, ItemKey, Point, TextRun};
use crate::text::Fonts;
use crate::trim;

use super::{measure, wrapped};

/// Between a row's qualifier and status (FR-74).
const META_SEPARATOR: &str = " \u{b7} ";

/// A plotted item and its FR-34 number, as the walk finds them.
#[derive(Debug, Clone, Copy)]
pub(super) struct Numbered<'a> {
    pub(super) item: &'a Item,
    /// FR-34's number: 1-based, in legend order.
    pub(super) number: usize,
}

/// One plotted item's row: badge, wrapped name and meta (FR-74).
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Row {
    /// The row's top edge; rows abut.
    pub(super) top: f64,
    /// The row's height, at least `LEGEND_ROW_MIN_HEIGHT` (FR-94).
    pub(super) height: f64,
    /// The badge's centre.
    pub(super) badge: Point,
    /// The quadrant's FR-10 rank: its series index, unreduced (FR-67).
    pub(super) series: usize,
    /// Whether the badge is hollow (FR-20).
    pub(super) hollow: bool,
    /// FR-34's number.
    pub(super) number: usize,
    /// The name, wrapped at the name column (FR-46, FR-79).
    pub(super) name: TextRun,
    /// The qualifier and status, inline after the name or on their own
    /// line; `None` when the row shows neither.
    pub(super) meta: Option<TextRun>,
    /// The row's key (its CSV line), FR-75's accessible name, no tooltip.
    pub(super) control: Control,
}

impl Row {
    /// A numbered item as a row from `top`, with its quadrant's `series`
    /// and FR-75's accessible name `label`.
    pub(super) fn new(
        numbered: Numbered<'_>,
        series: usize,
        label: String,
        top: f64,
        fonts: &Fonts,
    ) -> Self {
        let Numbered { item, number } = numbered;
        let name = wrapped(trim(&item.name), LEGEND_NAME, LEGEND_NAME_COLUMN, fonts);
        let meta = meta_text(item.qualifier, item.status).map(|text| measured(text, &name, fonts));
        let frame = frame(top, line_count(&name, meta.as_ref()));
        let name = name_run(name, frame.first_baseline);
        let last = name.last_baseline();
        Self {
            top,
            height: frame.height,
            badge: frame.badge,
            series,
            hollow: item.hollow(),
            number,
            meta: meta.map(|meta| meta_run(meta, last)),
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
}

/// FR-74: whether the meta fits after the name's last line, `META_GAP`
/// after it, within `column`. Inclusive: exactly filling the column fits.
#[must_use]
fn fits_inline(last_end: f64, meta_width: f64, column: f64) -> bool {
    last_end + META_GAP + meta_width <= column
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

/// A row's meta, measured against its name's last line.
struct Meta {
    text: String,
    /// The name's last line's advance.
    last_end: f64,
    /// Whether it rides on that line (FR-74).
    inline: bool,
}

/// `text` measured against the last of `name`'s lines.
fn measured(text: String, name: &[String], fonts: &Fonts) -> Meta {
    let last_end = measure(name.last().map_or("", String::as_str), LEGEND_NAME, fonts);
    let width = measure(&text, LEGEND_META, fonts);
    Meta {
        inline: fits_inline(last_end, width, LEGEND_NAME_COLUMN),
        last_end,
        text,
    }
}

/// The row's text lines: the name's, and the meta's own line if it has one.
fn line_count(name: &[String], meta: Option<&Meta>) -> usize {
    name.len() + usize::from(meta.is_some_and(|meta| !meta.inline))
}

/// Where a row's parts sit.
struct Frame {
    height: f64,
    first_baseline: f64,
    badge: Point,
}

/// A row from `top` holding `lines` lines of text: its height, floored for
/// FR-94; its text block centred in it; the badge at its middle.
fn frame(top: f64, lines: usize) -> Frame {
    #[expect(
        clippy::cast_precision_loss,
        clippy::as_conversions,
        reason = "a line count is far below 2^53, so the conversion is exact"
    )]
    let text = lines as f64 * LINE_HEIGHT;
    let height = LEGEND_ROW_MIN_HEIGHT.max(text + LEGEND_ROW_PAD);
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

/// A row's wrapped name from `baseline`.
const fn name_run(lines: Vec<String>, baseline: f64) -> TextRun {
    TextRun {
        origin: Point {
            x: LEGEND_NAME_X,
            y: baseline,
        },
        style: LEGEND_NAME,
        ink: Neutral::TextPrimary,
        anchor: Anchor::Start,
        lines,
        leading: LINE_HEIGHT,
    }
}

/// A row's meta: after the name's last line, on its baseline `last`, or on
/// its own line below it.
fn meta_run(meta: Meta, last: f64) -> TextRun {
    let origin = if meta.inline {
        Point {
            x: LEGEND_NAME_X + meta.last_end + META_GAP,
            y: last,
        }
    } else {
        Point {
            x: LEGEND_NAME_X,
            y: last + LINE_HEIGHT,
        }
    };
    TextRun {
        origin,
        style: LEGEND_META,
        ink: Neutral::TextMuted,
        anchor: Anchor::Start,
        lines: vec![meta.text],
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

    /// Each of `csv`'s items as a row from the top, so positions read
    /// relative to the row; the walk's placement is the layout's test.
    fn rows(csv: &str) -> Vec<Row> {
        let (catalogue, _) = plotted(csv, None);
        (1..)
            .zip(&catalogue.items)
            .map(|(number, item)| {
                Row::new(Numbered { item, number }, 0, String::new(), 0.0, fonts())
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
        let fits = "Continuous delivery pipelines for managed";
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
        assert_eq!(*wrapped, [fits, "infrastructure"]);
        assert_eq!(*whole, [fits]);
        for line in names.iter().flat_map(|lines| lines.iter()) {
            assert!(measure(line, LEGEND_NAME, fonts()) <= 312.0, "{line}");
        }
    }

    /// FR-74: inclusive — exactly filling the column still fits.
    #[test]
    fn fits_inline_is_inclusive() {
        assert!(fits_inline(300.0, 4.0, 312.0));
        assert!(!fits_inline(300.0, 4.0 + 1e-9, 312.0));
        assert!(fits_inline(0.0, 0.0, 8.0));
        assert!(!fits_inline(0.0, 0.0, 7.999));
        assert!(fits_inline(100.0, 50.0, 312.0));
        assert!(!fits_inline(250.0, 60.0, 312.0));
    }

    /// mockup.html l.725–729, l.749–751: meta on the name's last baseline,
    /// 8 after its end. Inter Regular "A" advances 1413/2048 em, 8.624… at
    /// 12.5, so the meta starts at 68 + 8.624… + 8.
    #[test]
    fn meta_rides_inline_when_it_fits() {
        let row = only(&format!("{HEAD_META}A,Q,Adopt,d,default,new\n"));
        let meta = row.meta.as_ref().expect("meta");
        assert_eq!(meta.lines, ["Default \u{b7} New"]);
        let at = Point {
            x: 84.624_267_578_125,
            y: 18.5,
        };
        assert!(near(meta.origin, at), "{meta:?}");
        assert!(close(row.height, 36.0));
        assert!(!row.hollow, "Default is solid (FR-20)");

        let row = only(&format!("{HEAD_META}A,Q,Adopt,d,conditional,\n"));
        assert!(row.hollow, "Conditional is hollow (FR-20)");
        let meta = row.meta.as_ref().expect("meta");
        assert_eq!(meta.lines, ["Conditional"]);
    }

    /// FR-74: 270.07 + 8 + 67.02 > 312, so the meta takes its own line:
    /// two lines, a 37-unit row, a 30-unit block from 3.5.
    #[test]
    fn meta_moves_to_its_own_line_when_it_does_not_fit() {
        let name = NAME_FULL_LINE;
        let row = only(&format!("{HEAD_META}{name},Q,Adopt,d,default,new\n"));
        assert_eq!(row.name.lines, [name]);
        let meta = row.meta.as_ref().expect("meta");
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
