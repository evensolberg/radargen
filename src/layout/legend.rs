//! The legend's layout (FR-35, FR-46, FR-74, FR-79), as data.
//!
//! The "Legend" heading, then per quadrant in FR-10 order its swatch and
//! heading, per ring with plotted items its heading, and one row per
//! plotted item: measured and wrapped once, at scale 1, content-relative,
//! in reading order.

use std::num::NonZeroU8;

use crate::model::{Catalogue, Item, Qualifier, Status};
use crate::order::{Plotted, Run};
use crate::palette::Neutral;
use crate::style::{
    BADGE_RADIUS, INDENT_ITEM, INDENT_QUADRANT, INDENT_RING, LEGEND_EMPTY_GROUP_GAP,
    LEGEND_FIRST_BASELINE, LEGEND_HEADING, LEGEND_HEADING_BASELINE, LEGEND_HEADING_TO_GROUP,
    LEGEND_META, LEGEND_NAME, LEGEND_NAME_X, LEGEND_QUADRANT, LEGEND_QUADRANT_TO_RING, LEGEND_RING,
    LEGEND_RING_TO_ROW, LEGEND_ROW_MIN_HEIGHT, LEGEND_ROW_PAD, LEGEND_ROW_TO_QUADRANT,
    LEGEND_ROW_TO_RING, LEGEND_TEXT_MIN_HEIGHT, LEGEND_WIDTH_MAX, LINE_HEIGHT, META_GAP,
    SWATCH_RADIUS, SWATCH_RAISE, TextStyle,
};
use crate::svg::{Anchor, Control, ItemKey, Point, TextRun};
use crate::text::{Fonts, TEXT_DESCENT, wrap};
use crate::trim;

/// The legend's heading text.
const TITLE: &str = "Legend";
/// Between a row's qualifier and status (FR-74).
const META_SEPARATOR: &str = " \u{b7} ";
/// The column a row's name and meta wrap in: D-56's 380 less the name's
/// indent.
const NAME_COLUMN: f64 = LEGEND_WIDTH_MAX - LEGEND_NAME_X;
/// The first quadrant heading's baseline.
const FIRST_GROUP: f64 = LEGEND_HEADING_BASELINE + LEGEND_HEADING_TO_GROUP;
/// A badge's centre, from the content's left edge.
const BADGE_X: f64 = INDENT_ITEM + BADGE_RADIUS;

/// One tier of the legend's headings: where it starts, how it looks, and
/// the column it wraps in (its indent less than D-56's 380, so a hanging
/// indent).
#[derive(Debug, Clone, Copy)]
struct Tier {
    indent: f64,
    style: TextStyle,
    ink: Neutral,
    level: NonZeroU8,
    column: f64,
}

/// "Legend": a peer of the key's heading, below the document title.
const TITLE_TIER: Tier = Tier {
    indent: 0.0,
    style: LEGEND_HEADING,
    ink: Neutral::TextPrimary,
    level: NonZeroU8::new(2).expect("2 is not zero"),
    column: LEGEND_WIDTH_MAX,
};

/// FR-35's first level: a quadrant.
const QUADRANT_TIER: Tier = Tier {
    indent: INDENT_QUADRANT,
    style: LEGEND_QUADRANT,
    ink: Neutral::TextPrimary,
    level: NonZeroU8::new(3).expect("3 is not zero"),
    column: LEGEND_WIDTH_MAX - INDENT_QUADRANT,
};

/// FR-35's second level: a ring within its quadrant.
const RING_TIER: Tier = Tier {
    indent: INDENT_RING,
    style: LEGEND_RING,
    ink: Neutral::TextMuted,
    level: NonZeroU8::new(4).expect("4 is not zero"),
    column: LEGEND_WIDTH_MAX - INDENT_RING,
};

/// The legend laid out at scale 1, relative to its content's top-left (the
/// panel's inset is added when it is drawn).
#[derive(Debug, Clone, PartialEq)]
pub struct Layout {
    /// The widest line's right edge; at most `LEGEND_WIDTH_MAX`, since
    /// everything wraps there.
    pub width: f64,
    /// The bottom of the last entry.
    pub height: f64,
    /// Every heading, swatch and row in reading order, which is document
    /// and Tab order.
    pub entries: Vec<Entry>,
}

/// One laid-out piece of the legend.
#[derive(Debug, Clone, PartialEq)]
pub enum Entry {
    /// "Legend", a quadrant or a ring.
    Heading {
        /// The wrapped heading; later lines hang at its indent.
        run: TextRun,
        /// Its heading level: 2 for "Legend", 3 a quadrant, 4 a ring.
        level: NonZeroU8,
    },
    /// FR-35's quadrant-colour swatch, beside its heading's first line.
    Swatch {
        /// The swatch's centre.
        centre: Point,
        /// The quadrant's FR-10 rank: its series index, unreduced (FR-67).
        series: usize,
    },
    /// One plotted item.
    Row(Row),
}

/// One plotted item's row: badge, wrapped name and meta (FR-74).
#[derive(Debug, Clone, PartialEq)]
pub struct Row {
    /// The row's top edge; rows abut.
    pub top: f64,
    /// The row's height, at least `LEGEND_ROW_MIN_HEIGHT` (FR-94).
    pub height: f64,
    /// The badge's centre.
    pub badge: Point,
    /// The quadrant's FR-10 rank: its series index, unreduced (FR-67).
    pub series: usize,
    /// Whether the badge is hollow (FR-20).
    pub hollow: bool,
    /// FR-34's number.
    pub number: usize,
    /// The name, wrapped at the name column (FR-46, FR-79).
    pub name: TextRun,
    /// The qualifier and status, inline after the name or on their own
    /// line; `None` when the row shows neither.
    pub meta: Option<TextRun>,
    /// The row's key (its CSV line), FR-75's accessible name, no tooltip.
    pub control: Control,
}

/// FR-74: whether the meta fits after the name's last line, `META_GAP`
/// after it, within `column`. Inclusive: exactly filling the column fits.
#[must_use]
pub fn fits_inline(last_end: f64, meta_width: f64, column: f64) -> bool {
    last_end + META_GAP + meta_width <= column
}

/// The legend for `catalogue`: empty when it has no quadrants (FR-42).
///
/// # Panics
///
/// If `plotted` was not built from `catalogue`, so an item, quadrant or
/// ring index is out of range for it.
#[must_use]
pub fn layout(catalogue: &Catalogue, plotted: &Plotted, fonts: &Fonts) -> Layout {
    let context = Context {
        catalogue,
        plotted,
        runs: plotted.runs(catalogue),
        fonts,
    };
    let title = Entry::Heading {
        run: heading(TITLE_TIER, TITLE, LEGEND_HEADING_BASELINE, fonts),
        level: TITLE_TIER.level,
    };
    let entries = titled(title, context.groups());
    Layout {
        width: width(&entries, fonts),
        height: height(&entries),
        entries,
    }
}

/// What every step of the walk reads.
struct Context<'a> {
    catalogue: &'a Catalogue,
    plotted: &'a Plotted,
    /// FR-34's bands: declared ring order, empty rings absent.
    runs: Vec<Run>,
    fonts: &'a Fonts,
}

impl Context<'_> {
    /// Every quadrant's group in FR-10 order, each starting where the last
    /// left off. The baseline after the last group is not needed: the
    /// height is the last entry's bottom.
    fn groups(&self) -> Vec<Entry> {
        let mut entries = Vec::new();
        self.plotted
            .quadrants
            .iter()
            .fold(FIRST_GROUP, |baseline, &q| {
                self.quadrant(q, baseline, &mut entries)
            });
        entries
    }

    /// Quadrant `q`'s swatch, heading, rings and rows from `baseline`;
    /// returns the next quadrant's baseline.
    fn quadrant(&self, q: usize, baseline: f64, entries: &mut Vec<Entry>) -> f64 {
        let run = heading(QUADRANT_TIER, self.quadrant_label(q), baseline, self.fonts);
        let last = last_baseline(&run);
        entries.push(swatch(self.series(q), baseline));
        entries.push(Entry::Heading {
            run,
            level: QUADRANT_TIER.level,
        });
        self.rings(q, last, entries)
    }

    /// Quadrant `q`'s rings below its heading's last baseline `last`;
    /// returns the next quadrant's baseline.
    fn rings(&self, q: usize, last: f64, entries: &mut Vec<Entry>) -> f64 {
        let bottom = self
            .runs
            .iter()
            .filter(|run| run.quadrant == q)
            .fold(None, |bottom, run| {
                Some(self.ring(run, ring_baseline(last, bottom), entries))
            });
        next_quadrant(last, bottom)
    }

    /// One ring's heading at `baseline` and its rows; returns the last
    /// row's bottom.
    fn ring(&self, run: &Run, baseline: f64, entries: &mut Vec<Entry>) -> f64 {
        let heading = heading(RING_TIER, self.ring_label(run.ring), baseline, self.fonts);
        let top = first_row_top(last_baseline(&heading));
        entries.push(Entry::Heading {
            run: heading,
            level: RING_TIER.level,
        });
        run.items.clone().fold(top, |top, position| {
            let row = self.row(position, top);
            let bottom = row.bottom();
            entries.push(Entry::Row(row));
            bottom
        })
    }

    /// The item at `position` in FR-34's order, as a row from `top`.
    fn row(&self, position: usize, top: f64) -> Row {
        let (item, number) = self.item(position);
        let name = wrapped(trim(&item.name), LEGEND_NAME, NAME_COLUMN, self.fonts);
        let meta =
            meta_text(item.qualifier, item.status).map(|text| measured(text, &name, self.fonts));
        let frame = frame(top, line_count(&name, meta.as_ref()));
        let name = name_run(name, frame.first_baseline);
        let last = last_baseline(&name);
        Row {
            top,
            height: frame.height,
            badge: frame.badge,
            series: self.series(item.quadrant),
            hollow: item.hollow(),
            number,
            meta: meta.map(|meta| meta_run(meta, last)),
            name,
            control: Control {
                key: ItemKey(item.line),
                label: self.catalogue.accessible_name(item, Some(number)),
                tooltip: None,
            },
        }
    }

    /// The item at `position` in FR-34's order, and its number.
    // qual:allow(complexity, error_handling) reason: "the documented # Panics of `layout`: a Plotted from another catalogue is a caller bug"
    fn item(&self, position: usize) -> (&Item, usize) {
        let item = self
            .plotted
            .items
            .get(position)
            .and_then(|&index| self.catalogue.items.get(index))
            .expect("FR-34's order indexes the catalogue's items");
        (item, position + 1)
    }

    /// Quadrant `q`'s FR-10 rank: its series index (FR-67).
    fn series(&self, q: usize) -> usize {
        *self
            .plotted
            .rank
            .get(q)
            .expect("rank covers every quadrant of the catalogue")
    }

    fn quadrant_label(&self, q: usize) -> &str {
        &self
            .catalogue
            .quadrants
            .get(q)
            .expect("FR-10's order indexes the catalogue's quadrants")
            .label
    }

    fn ring_label(&self, ring: usize) -> &str {
        self.catalogue
            .rings
            .get(ring)
            .expect("a run's ring is within the catalogue's rings")
    }
}

/// `text` as a heading in `tier`, its first baseline at `baseline`,
/// wrapped greedily at the tier's column (FR-79).
fn heading(tier: Tier, text: &str, baseline: f64, fonts: &Fonts) -> TextRun {
    TextRun {
        origin: Point {
            x: tier.indent,
            y: baseline,
        },
        style: tier.style,
        ink: tier.ink,
        anchor: Anchor::Start,
        lines: wrapped(text, tier.style, tier.column, fonts),
        leading: LINE_HEIGHT,
    }
}

/// `text` wrapped at `column` in `style` (FR-79).
fn wrapped(text: &str, style: TextStyle, column: f64, fonts: &Fonts) -> Vec<String> {
    wrap(
        text,
        column,
        fonts.face(style.weight),
        style.size,
        style.letter_spacing,
    )
}

/// `text`'s advance in `style`.
fn measure(text: &str, style: TextStyle, fonts: &Fonts) -> f64 {
    fonts
        .face(style.weight)
        .measure(text, style.size, style.letter_spacing)
}

/// The legend's heading before its groups, or nothing with no groups:
/// with no quadrants the legend is omitted (FR-42).
fn titled(title: Entry, groups: Vec<Entry>) -> Vec<Entry> {
    if groups.is_empty() {
        return groups;
    }
    std::iter::once(title).chain(groups).collect()
}

/// A quadrant's swatch beside its heading's first baseline.
fn swatch(series: usize, baseline: f64) -> Entry {
    Entry::Swatch {
        centre: Point {
            x: SWATCH_RADIUS,
            y: baseline - SWATCH_RAISE,
        },
        series,
    }
}

/// The next ring's baseline: below the quadrant heading's last baseline
/// for the first ring, else below the previous ring's last row.
fn ring_baseline(heading_last: f64, previous_bottom: Option<f64>) -> f64 {
    previous_bottom.map_or(heading_last + LEGEND_QUADRANT_TO_RING, |bottom| {
        bottom + LEGEND_ROW_TO_RING
    })
}

/// The next quadrant's baseline: below the last row, or, for a quadrant
/// with no rows, below its heading.
fn next_quadrant(heading_last: f64, last_bottom: Option<f64>) -> f64 {
    last_bottom.map_or(heading_last + LEGEND_EMPTY_GROUP_GAP, |bottom| {
        bottom + LEGEND_ROW_TO_QUADRANT
    })
}

/// A ring's first row's top, below its heading's last baseline.
fn first_row_top(heading_last: f64) -> f64 {
    heading_last + LEGEND_RING_TO_ROW
}

/// The baseline of `run`'s last line.
fn last_baseline(run: &TextRun) -> f64 {
    #[expect(
        clippy::cast_precision_loss,
        clippy::as_conversions,
        reason = "a line count is far below 2^53, so the conversion is exact"
    )]
    let gaps = run.lines.len().saturating_sub(1) as f64;
    #[expect(
        clippy::suboptimal_flops,
        reason = "NFR-4: plain * and + are bit-identical on every target; mul_add falls back to a libm fma where the CPU lacks one"
    )]
    let last = run.origin.y + gaps * run.leading;
    last
}

/// FR-74's meta: the qualifier and status as the row shows them, omitting
/// an absent value and `NoChange`, which is the plain marker (FR-40).
fn meta_text(qualifier: Option<Qualifier>, status: Option<Status>) -> Option<String> {
    let parts: Vec<&str> = [
        qualifier.map(Qualifier::label),
        status.filter(|s| *s != Status::NoChange).map(Status::label),
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
        inline: fits_inline(last_end, width, NAME_COLUMN),
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
            x: BADGE_X,
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

impl Row {
    /// The row's bottom edge, where the next row starts.
    fn bottom(&self) -> f64 {
        self.top + self.height
    }
}

impl Entry {
    /// The entry's lowest painted extent: a row's bottom, a heading's last
    /// descent, a swatch's lowest point.
    fn bottom(&self) -> f64 {
        match self {
            Self::Heading { run, .. } => {
                #[expect(
                    clippy::suboptimal_flops,
                    reason = "NFR-4: plain * and + are bit-identical on every target; mul_add falls back to a libm fma where the CPU lacks one"
                )]
                let descent = last_baseline(run) + TEXT_DESCENT * run.style.size;
                descent
            }
            Self::Swatch { centre, .. } => centre.y + SWATCH_RADIUS,
            Self::Row(row) => row.bottom(),
        }
    }

    /// The entry's text.
    fn runs(&self) -> Vec<&TextRun> {
        match self {
            Self::Heading { run, .. } => vec![run],
            Self::Swatch { .. } => Vec::new(),
            Self::Row(row) => std::iter::once(&row.name).chain(&row.meta).collect(),
        }
    }
}

/// The content height: the bottom of the last entry, whatever it is.
fn height(entries: &[Entry]) -> f64 {
    entries.last().map_or(0.0, Entry::bottom)
}

/// The content width: the widest laid-out line's right edge.
fn width(entries: &[Entry], fonts: &Fonts) -> f64 {
    entries
        .iter()
        .flat_map(Entry::runs)
        .flat_map(|run| {
            run.lines
                .iter()
                .map(move |line| run.origin.x + measure(line, run.style, fonts))
        })
        .fold(0.0, f64::max)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::block::{close, fonts};
    use crate::layout::test_support::{PRD_EXAMPLE, near, plotted};
    use crate::model::{Qualifier, Status};
    use crate::svg::{ItemKey, Point, TextRun};

    /// The four required columns.
    const HEAD: &str = "name,quadrant,ring,description\n";
    /// With FR-20's qualifier and FR-26's status.
    const HEAD_META: &str = "name,quadrant,ring,description,qualifier,status\n";

    fn lay(csv: &str, rings: Option<&str>) -> Layout {
        let (catalogue, order) = plotted(csv, rings);
        layout(&catalogue, &order, fonts())
    }

    /// The headings of `level`, in order.
    fn headings(l: &Layout, level: u8) -> Vec<&TextRun> {
        l.entries
            .iter()
            .filter_map(|entry| match entry {
                Entry::Heading { run, level: found } if found.get() == level => Some(run),
                _ => None,
            })
            .collect()
    }

    fn texts(runs: &[&TextRun]) -> Vec<String> {
        runs.iter().map(|run| run.lines.join(" ")).collect()
    }

    fn rows(l: &Layout) -> Vec<&Row> {
        l.entries
            .iter()
            .filter_map(|entry| match entry {
                Entry::Row(row) => Some(row),
                _ => None,
            })
            .collect()
    }

    fn swatches(l: &Layout) -> Vec<(Point, usize)> {
        l.entries
            .iter()
            .filter_map(|entry| match entry {
                Entry::Swatch { centre, series } => Some((*centre, *series)),
                _ => None,
            })
            .collect()
    }

    fn only<T: Copy>(items: &[T]) -> T {
        let [one] = items else {
            panic!("expected exactly one, got {}", items.len());
        };
        *one
    }

    fn first_line(row: &Row) -> &str {
        row.name.lines.first().map_or("", String::as_str)
    }

    fn name_width(line: &str) -> f64 {
        measure(line, LEGEND_NAME, fonts())
    }

    /// mockup.html l.688–700 with the plan's rhythm: heading at 14, group
    /// at 14 + 22, swatch at its baseline − 4, ring at + 18; the row from
    /// its top (ring + 4), 32 tall, its badge at the middle and a one-line
    /// text block of 17 centred in it, first baseline 9 down.
    #[test]
    fn pins_for_the_prd_example() {
        let l = lay(PRD_EXAMPLE, None);
        let legend = only(&headings(&l, 2));
        assert_eq!(legend.lines, ["Legend"]);
        assert!(near(legend.origin, Point { x: 0.0, y: 14.0 }));
        let quadrants = headings(&l, 3);
        let first = quadrants.first().expect("a quadrant heading");
        assert!(near(first.origin, Point { x: 16.0, y: 36.0 }));
        let (swatch, series) = *swatches(&l).first().expect("a swatch");
        assert!(near(swatch, Point { x: 5.0, y: 32.0 }), "{swatch:?}");
        assert_eq!(series, 0);
        let rings = headings(&l, 4);
        let ring = rings.first().expect("a ring heading");
        assert!(near(ring.origin, Point { x: 24.0, y: 54.0 }));
        let row = *rows(&l).first().expect("a row");
        assert!(close(row.top, 58.0) && close(row.height, 32.0), "{row:?}");
        assert!(near(row.badge, Point { x: 52.0, y: 74.0 }), "{row:?}");
        assert!(near(row.name.origin, Point { x: 68.0, y: 74.5 }), "{row:?}");
        assert_eq!(row.name.lines, ["Kubernetes"]);
    }

    /// mockup.html l.733–735: block = max(17, n · 15), row = block + 7, here
    /// floored at 32 (FR-94); rows abut.
    #[test]
    fn rows_of_one_two_and_three_lines() {
        let two = "Bounded trial of continuous delivery pipelines across managed estates and environments";
        let three = "Bounded trial of continuous delivery pipelines across managed estates and environments with a shared policy layer";
        let l = lay(
            &format!("{HEAD}A,Q,Adopt,d\n{two},Q,Adopt,d\n{three},Q,Adopt,d\n"),
            None,
        );
        let rows = rows(&l);
        let got: Vec<(usize, f64, f64, f64)> = rows
            .iter()
            .map(|r| (r.name.lines.len(), r.top, r.height, r.name.origin.y))
            .collect();
        let want = [
            (1, 58.0, 32.0, 74.5),
            (2, 90.0, 37.0, 102.5),
            (3, 127.0, 52.0, 139.5),
        ];
        assert_eq!(got.len(), want.len(), "{got:?}");
        for (g, w) in got.iter().zip(want) {
            assert!(
                g.0 == w.0 && close(g.1, w.1) && close(g.2, w.2) && close(g.3, w.3),
                "{got:?}"
            );
        }
        for row in &rows {
            assert!(close(row.name.leading, 15.0));
        }
    }

    #[test]
    fn groups_follow_fr10_and_rings_follow_the_declaration() {
        let csv = format!("{HEAD}a,Tool,Adopt,d\nb,platform,Adopt,d\nc,Practice,Adopt,d\n");
        let l = lay(&csv, None);
        assert_eq!(texts(&headings(&l, 3)), ["platform", "Practice", "Tool"]);
        let series: Vec<usize> = swatches(&l).iter().map(|s| s.1).collect();
        assert_eq!(series, [0, 1, 2]);

        // Assess and Trial are declared but hold nothing: no heading.
        let csv = format!("{HEAD}a,Q,Adopt,d\nb,Q,Hold,d\n");
        let l = lay(&csv, Some("Hold,Assess,Trial,Adopt"));
        let rings = headings(&l, 4);
        assert_eq!(texts(&rings), ["Hold", "Adopt"]);
        // Hold's one row spans 58–90, so Adopt's baseline is 90 + 12.
        let [hold, adopt] = rings.as_slice() else {
            panic!("two ring headings: {rings:?}");
        };
        assert!(close(hold.origin.y, 54.0), "{hold:?}");
        assert!(close(adopt.origin.y, 102.0), "{adopt:?}");
    }

    /// FR-34's numbers, and the CSV line as the key: B's quoted description
    /// spans two lines, so A begins on line 4 and C on 5. Default rings put
    /// Trial before Adopt.
    #[test]
    fn rows_follow_fr34() {
        let csv = format!("{HEAD}B,Q,Adopt,\"two\nlines\"\nA,Q,Adopt,d\nC,Q,Trial,d\n");
        let (catalogue, order) = plotted(&csv, None);
        let l = layout(&catalogue, &order, fonts());
        let rows = rows(&l);
        let got: Vec<(usize, ItemKey, &str)> = rows
            .iter()
            .map(|r| (r.number, r.control.key, first_line(r)))
            .collect();
        assert_eq!(
            got,
            [
                (1, ItemKey(5), "C"),
                (2, ItemKey(4), "A"),
                (3, ItemKey(2), "B")
            ]
        );
        for row in rows {
            let item = catalogue
                .items
                .iter()
                .find(|item| ItemKey(item.line) == row.control.key)
                .expect("the key is a CSV line");
            assert_eq!(
                row.control.label,
                catalogue.accessible_name(item, Some(row.number))
            );
            assert_eq!(row.control.tooltip, None);
        }
    }

    /// FR-46, FR-79. 312 = 380 − 68 (D-56; mockup.html l.711).
    #[test]
    fn long_names_wrap_and_long_tokens_break() {
        let fits = "Continuous delivery pipelines for managed";
        let wraps = "Continuous delivery pipelines for managed infrastructure";
        let word = "Unbreakable".repeat(5);
        let l = lay(
            &format!("{HEAD}{fits},Q,Adopt,d\n{wraps},Q,Trial,d\n{word},Q,Assess,d\n"),
            None,
        );
        let names: Vec<&[String]> = rows(&l).iter().map(|r| r.name.lines.as_slice()).collect();
        let [broken, wrapped, whole] = names.as_slice() else {
            panic!("three rows: {names:?}");
        };
        assert_eq!(broken.len(), 2, "{broken:?}");
        assert_eq!(broken.concat(), word, "nothing dropped");
        assert_eq!(*wrapped, [fits, "infrastructure"]);
        assert_eq!(*whole, [fits]);
        for line in names.iter().flat_map(|lines| lines.iter()) {
            assert!(name_width(line) <= 312.0, "{line}");
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
        let l = lay(&format!("{HEAD_META}A,Q,Adopt,d,default,new\n"), None);
        let row = only(&rows(&l));
        let meta = row.meta.as_ref().expect("meta");
        assert_eq!(meta.lines, ["Default \u{b7} New"]);
        let at = Point {
            x: 84.624_267_578_125,
            y: 74.5,
        };
        assert!(near(meta.origin, at), "{meta:?}");
        assert!(close(row.height, 32.0));
        assert!(!row.hollow, "Default is solid (FR-20)");

        let l = lay(&format!("{HEAD_META}A,Q,Adopt,d,conditional,\n"), None);
        let row = only(&rows(&l));
        assert!(row.hollow, "Conditional is hollow (FR-20)");
        let meta = row.meta.as_ref().expect("meta");
        assert_eq!(meta.lines, ["Conditional"]);
    }

    /// FR-74: 270.07 + 8 + 67.02 > 312, so the meta takes its own line:
    /// two lines, a 37-unit row, a 30-unit block from 61.5.
    #[test]
    fn meta_moves_to_its_own_line_when_it_does_not_fit() {
        let name = "Bounded trial of continuous delivery pipelines";
        let l = lay(&format!("{HEAD_META}{name},Q,Adopt,d,default,new\n"), None);
        let row = only(&rows(&l));
        assert_eq!(row.name.lines, [name]);
        let meta = row.meta.as_ref().expect("meta");
        assert!(near(row.name.origin, Point { x: 68.0, y: 70.5 }), "{row:?}");
        assert!(near(meta.origin, Point { x: 68.0, y: 85.5 }), "{meta:?}");
        assert!(
            close(row.height, 37.0) && close(row.badge.y, 76.5),
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

    /// PRD v0.53 FR-79: a heading wraps at its own column (380 − 16) with a
    /// hanging indent; the swatch stays on the first line; the ring heading
    /// moves down by the extra line (54 + 15).
    #[test]
    fn headings_wrap_at_the_column() {
        let long = "Infrastructure platforms developer tooling and delivery work";
        assert_eq!(long.chars().count(), 60);
        let l = lay(&format!("{HEAD}a,{long},Adopt,d\n"), None);
        let heading = only(&headings(&l, 3));
        assert_eq!(heading.lines.len(), 2, "{heading:?}");
        assert!(near(heading.origin, Point { x: 16.0, y: 36.0 }));
        assert!(close(heading.leading, 15.0));
        for line in &heading.lines {
            let w = measure(line, LEGEND_QUADRANT, fonts());
            assert!(w <= 364.0, "{line}: {w}");
        }
        assert!(near(only(&swatches(&l)).0, Point { x: 5.0, y: 32.0 }));
        assert!(near(
            only(&headings(&l, 4)).origin,
            Point { x: 24.0, y: 69.0 }
        ));
        assert!(close(only(&rows(&l)).top, 73.0));
    }

    /// The inclusive edge of a heading's wrap: a column exactly its width
    /// keeps it on one line. (Exact fits at 364 are not constructible:
    /// advances are multiples of size / 2048.)
    #[test]
    fn a_heading_exactly_filling_its_column_does_not_wrap() {
        let text = "Infrastructure platforms";
        let style = QUADRANT_TIER.style;
        let exact = fonts()
            .face(style.weight)
            .measure(text, style.size, style.letter_spacing);
        let tier = Tier {
            column: exact,
            ..QUADRANT_TIER
        };
        assert_eq!(heading(tier, text, 36.0, fonts()).lines, [text]);
        let narrower = Tier {
            column: exact - 1e-9,
            ..QUADRANT_TIER
        };
        assert_eq!(heading(narrower, text, 36.0, fonts()).lines.len(), 2);
    }

    /// FR-17, FR-18, FR-35: a quadrant whose rows are all unplotted keeps
    /// its heading and swatch; the next group is 26 below (18 + the
    /// mockup's 8, l.755); numbers stay contiguous.
    #[test]
    fn empty_rings_and_unplotted_rows() {
        let l = lay(&format!("{HEAD}A,Alpha,Nowhere,d\nB,Beta,Adopt,d\n"), None);
        let quadrants = headings(&l, 3);
        assert_eq!(texts(&quadrants), ["Alpha", "Beta"]);
        let [alpha, beta] = quadrants.as_slice() else {
            panic!("two quadrants");
        };
        assert!(close(alpha.origin.y, 36.0) && close(beta.origin.y, 62.0));
        assert_eq!(swatches(&l).len(), 2);
        assert_eq!(texts(&headings(&l, 4)), ["Adopt"]);
        assert!(close(only(&rows(&l)).top, 84.0));

        // Last in FR-10 order: the content reaches the heading's descent,
        // 110 + 494/2048 × 12 (Inter's hhea descender).
        let l = lay(&format!("{HEAD}A,Zeta,Nowhere,d\nB,Beta,Adopt,d\n"), None);
        let zeta = *headings(&l, 3).last().expect("Zeta");
        assert_eq!(zeta.lines, ["Zeta"]);
        assert!(close(zeta.origin.y, 110.0), "{zeta:?}");
        assert!(close(l.height, 112.894_531_25), "{}", l.height);

        // One unplotted row between two plotted ones; the content ends at
        // the second row's bottom, 58 + 32 + 32.
        let l = lay(
            &format!("{HEAD}A,Q,Adopt,d\nB,Q,Nowhere,d\nC,Q,Adopt,d\n"),
            None,
        );
        let got: Vec<(usize, &str)> = rows(&l).iter().map(|r| (r.number, first_line(r))).collect();
        assert_eq!(got, [(1, "A"), (2, "C")]);
        assert!(close(l.height, 122.0), "{}", l.height);
    }

    /// The width is the widest line's right edge: here the name "A" at 68
    /// (see `meta_rides_inline_when_it_fits`).
    #[test]
    fn widths() {
        let l = lay(&format!("{HEAD}A,Q,Adopt,d\n"), None);
        assert!(close(l.width, 76.624_267_578_125), "{}", l.width);

        let long = "Bounded trial of continuous delivery pipelines across managed estates and environments with a shared policy layer";
        let l = lay(&format!("{HEAD}{long},Q,Adopt,d\n"), None);
        let widest = only(&rows(&l))
            .name
            .lines
            .iter()
            .map(|line| 68.0 + name_width(line))
            .fold(0.0, f64::max);
        assert!(l.width <= 380.0, "{}", l.width);
        assert!(close(l.width, widest), "{} vs {widest}", l.width);
    }

    #[test]
    fn zero_quadrants_lays_out_nothing() {
        let catalogue = crate::model::Catalogue {
            columns: Vec::new(),
            quadrants: Vec::new(),
            rings: Vec::new(),
            items: Vec::new(),
        };
        let l = layout(&catalogue, &crate::order::Plotted::default(), fonts());
        assert!(l.entries.is_empty(), "{l:?}");
        assert!(close(l.width, 0.0) && close(l.height, 0.0), "{l:?}");
    }

    /// FR-67: the series index is the FR-10 rank, unreduced; the palette
    /// cycles it.
    #[test]
    fn nine_quadrants_use_unreduced_series() {
        let csv: String = ('a'..='i')
            .map(|q| format!("{q},{q},Adopt,d\n"))
            .fold(HEAD.to_owned(), |all, row| all + &row);
        let l = lay(&csv, None);
        let series: Vec<usize> = swatches(&l).iter().map(|s| s.1).collect();
        assert_eq!(series, [0, 1, 2, 3, 4, 5, 6, 7, 8]);
        assert_eq!(rows(&l).last().map(|r| r.series), Some(8));
    }

    /// Escaping is the emitter's (FR-47): the layout carries names as
    /// written.
    #[test]
    fn names_carried_raw() {
        let csv = format!("{HEAD}\"A<&\"\">B\",Q,Adopt,d\n\u{396}\u{3c9}\u{3ae},Q,Adopt,d\n");
        let l = lay(&csv, None);
        let names: Vec<&str> = rows(&l).iter().map(|r| first_line(r)).collect();
        assert_eq!(names, ["A<&\">B", "\u{396}\u{3c9}\u{3ae}"]);
    }
}
