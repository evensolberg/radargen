//! The legend's layout (FR-35, FR-46, FR-74, FR-79), as data.
//!
//! The "Legend" heading, then per quadrant in FR-10 order its swatch and
//! heading, per ring with plotted items its heading, and one row per
//! plotted item: measured and wrapped once, at scale 1, content-relative,
//! in reading order.

use std::num::NonZeroU8;

use crate::model::Catalogue;
use crate::order::{Plotted, Run};
use crate::palette::Neutral;
use crate::style::{
    INDENT_QUADRANT, INDENT_RING, LEGEND_EMPTY_GROUP_GAP, LEGEND_FIRST_GROUP, LEGEND_HEADING,
    LEGEND_HEADING_BASELINE, LEGEND_QUADRANT, LEGEND_QUADRANT_TO_RING, LEGEND_RING,
    LEGEND_RING_TO_ROW, LEGEND_ROW_TO_QUADRANT, LEGEND_ROW_TO_RING, LEGEND_WIDTH_MAX, LINE_HEIGHT,
    SWATCH_RADIUS, SWATCH_RAISE, TextStyle,
};
use crate::svg::{Anchor, Point, TextRun};
use crate::text::Fonts;

use super::row::{Numbered, Row};

/// The legend's heading text.
const LEGEND_TITLE: &str = "Legend";

/// One tier of the legend's headings: where it starts and how it looks. It
/// wraps at D-56's 380 less its indent, so later lines hang.
#[derive(Debug, Clone, Copy)]
struct Tier {
    indent: f64,
    style: TextStyle,
    ink: Neutral,
    level: NonZeroU8,
}

/// "Legend": a peer of the key's heading, below the document title.
const TITLE_TIER: Tier = Tier {
    indent: 0.0,
    style: LEGEND_HEADING,
    ink: Neutral::TextPrimary,
    level: NonZeroU8::new(2).expect("2 is not zero"),
};

/// FR-35's first level: a quadrant.
const QUADRANT_TIER: Tier = Tier {
    indent: INDENT_QUADRANT,
    style: LEGEND_QUADRANT,
    ink: Neutral::TextPrimary,
    level: NonZeroU8::new(3).expect("3 is not zero"),
};

/// FR-35's second level: a ring within its quadrant.
const RING_TIER: Tier = Tier {
    indent: INDENT_RING,
    style: LEGEND_RING,
    ink: Neutral::TextMuted,
    level: NonZeroU8::new(4).expect("4 is not zero"),
};

/// The legend laid out at scale 1, relative to its content's top-left (the
/// panel's inset is added when it is drawn).
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Layout {
    /// The widest line's right edge; at most `LEGEND_WIDTH_MAX`, since
    /// everything wraps there.
    pub(super) width: f64,
    /// The bottom of the last entry.
    pub(super) height: f64,
    /// Every heading, swatch and row in reading order, which is document
    /// and Tab order.
    pub(super) entries: Vec<Entry>,
}

/// One laid-out piece of the legend.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum Entry {
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

/// The legend for `catalogue`: empty when it has no quadrants (FR-42).
///
/// # Panics
///
/// If `plotted` was not built from `catalogue`, so an item, quadrant or
/// ring index is out of range for it.
#[must_use]
pub(super) fn layout(catalogue: &Catalogue, plotted: &Plotted, fonts: &Fonts) -> Layout {
    let context = Context {
        catalogue,
        plotted,
        runs: plotted.runs(catalogue),
        fonts,
    };
    let title = Entry::Heading {
        run: heading(TITLE_TIER, LEGEND_TITLE, LEGEND_HEADING_BASELINE, fonts),
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
        let mut baseline = LEGEND_FIRST_GROUP;
        for &q in &self.plotted.quadrants {
            baseline = self.quadrant(q, baseline, &mut entries);
        }
        entries
    }

    /// Quadrant `q`'s swatch, heading, rings and rows from `baseline`;
    /// returns the next quadrant's baseline.
    fn quadrant(&self, q: usize, baseline: f64, entries: &mut Vec<Entry>) -> f64 {
        let run = heading(QUADRANT_TIER, self.quadrant_label(q), baseline, self.fonts);
        let last = run.last_baseline();
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
        let mut bottom = None;
        for run in self.runs.iter().filter(|run| run.quadrant == q) {
            bottom = Some(self.ring(run, ring_baseline(last, bottom), entries));
        }
        next_quadrant(last, bottom)
    }

    /// One ring's heading at `baseline` and its rows; returns the last
    /// row's bottom.
    fn ring(&self, run: &Run, baseline: f64, entries: &mut Vec<Entry>) -> f64 {
        let heading = heading(RING_TIER, self.ring_label(run.ring), baseline, self.fonts);
        let top = first_row_top(heading.last_baseline());
        entries.push(Entry::Heading {
            run: heading,
            level: RING_TIER.level,
        });
        let mut bottom = top;
        for position in run.items.clone() {
            let row = self.row(position, bottom);
            bottom = row.bottom();
            entries.push(Entry::Row(row));
        }
        bottom
    }

    /// The item at `position` in FR-34's order, as a row from `top`.
    fn row(&self, position: usize, top: f64) -> Row {
        let numbered = self.item(position);
        let Numbered { item, number } = numbered;
        Row::new(
            numbered,
            self.series(item.quadrant),
            self.catalogue.accessible_name(item, Some(number)),
            top,
            self.fonts,
        )
    }

    /// The item at `position` in FR-34's order, and its number.
    // qual:allow(complexity, error_handling) reason: "the documented # Panics of `layout`: a Plotted from another catalogue is a caller bug"
    fn item(&self, position: usize) -> Numbered<'_> {
        let item = self
            .plotted
            .items
            .get(position)
            .and_then(|&index| self.catalogue.items.get(index))
            .expect("FR-34's order indexes the catalogue's items");
        Numbered {
            item,
            number: position + 1,
        }
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
        lines: tier.style.wrap(text, LEGEND_WIDTH_MAX - tier.indent, fonts),
        leading: LINE_HEIGHT,
    }
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

impl Entry {
    /// The entry's lowest painted extent: a row's bottom, a heading's last
    /// descent, a swatch's lowest point.
    fn bottom(&self) -> f64 {
        match self {
            Self::Heading { run, .. } => run.bottom(),
            Self::Swatch { centre, .. } => centre.y + SWATCH_RADIUS,
            Self::Row(row) => row.bottom(),
        }
    }

    /// The furthest right the entry's text reaches at its layout width
    /// (D-78); zero for a swatch.
    fn right_edge(&self, fonts: &Fonts) -> f64 {
        match self {
            Self::Heading { run, .. } => run.right_edge(fonts),
            Self::Swatch { .. } => 0.0,
            Self::Row(row) => row.right_edge(fonts),
        }
    }
}

/// The content height: the bottom of the last entry, whatever it is.
fn height(entries: &[Entry]) -> f64 {
    entries.last().map_or(0.0, Entry::bottom)
}

/// The content width: the widest laid-out line's right edge, an inline
/// meta included where it ends under a reader's text spacing (D-78).
fn width(entries: &[Entry], fonts: &Fonts) -> f64 {
    entries
        .iter()
        .map(|entry| entry.right_edge(fonts))
        .fold(0.0, f64::max)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::block::{close, fonts};
    use crate::layout::test_support::{
        HEAD, NAME_THREE_LINES, NAME_TWO_LINES, PRD_EXAMPLE, empty_catalogue, near, plotted,
    };
    use crate::style::LEGEND_NAME;
    use crate::svg::{ItemKey, Point, TextRun};

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

    /// mockup.html l.688–700 with the plan's rhythm: heading at 14, group
    /// at 14 + 22, swatch at its baseline − 4, ring at + 18; the row from
    /// its top (ring + 4), 36 tall (FR-94, D-71), its badge at the middle
    /// and a one-line text block of 17 centred in it, 9.5 down, so the
    /// first baseline is 9.5 + 9 down.
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
        assert!(close(row.top, 58.0) && close(row.height, 36.0), "{row:?}");
        assert!(near(row.badge, Point { x: 52.0, y: 76.0 }), "{row:?}");
        assert!(near(row.name.origin, Point { x: 68.0, y: 76.5 }), "{row:?}");
        assert_eq!(row.name.lines, ["Kubernetes"]);
    }

    /// mockup.html l.733–735: block = max(17, n · 15), row = block + 7, here
    /// floored at 36 (FR-94, D-71); the walk abuts a ring's rows. Each
    /// block is centred, so its first baseline is (h − b)/2 + 9 below the
    /// row's top: 9.5 + 9, 3.5 + 9, 3.5 + 9.
    #[test]
    fn rows_of_one_two_and_three_lines() {
        let (two, three) = (NAME_TWO_LINES, NAME_THREE_LINES);
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
            (1, 58.0, 36.0, 76.5),
            (2, 94.0, 37.0, 106.5),
            (3, 131.0, 52.0, 143.5),
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
        // Hold's one row spans 58–94, so Adopt's baseline is 94 + 12.
        let [hold, adopt] = rings.as_slice() else {
            panic!("two ring headings: {rings:?}");
        };
        assert!(close(hold.origin.y, 54.0), "{hold:?}");
        assert!(close(adopt.origin.y, 106.0), "{adopt:?}");
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
            let w = LEGEND_QUADRANT.measure(line, fonts());
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
        let exact = style.measure(text, fonts());
        assert_eq!(style.wrap(text, exact, fonts()), [text]);
        assert_eq!(style.wrap(text, exact - 1e-9, fonts()).len(), 2);
    }

    /// A heading wraps at its own column, `LEGEND_WIDTH_MAX` less its
    /// indent: the longest run of words that fits that column stays on one
    /// line, and one word more wraps.
    #[test]
    fn a_heading_wraps_at_the_column_its_indent_leaves() {
        let tier = QUADRANT_TIER;
        let column = LEGEND_WIDTH_MAX - tier.indent;
        let words = |n: usize| vec!["Wide"; n].join(" ");
        // The column holds at most ~30 four-letter words; 64 bounds the search.
        let fit = (1..64)
            .take_while(|&n| tier.style.measure(&words(n), fonts()) <= column)
            .last()
            .expect("one word fits the column");
        let lines = |n: usize| heading(tier, &words(n), 36.0, fonts()).lines.len();
        assert_eq!(lines(fit), 1, "{fit} words fit {column}");
        assert_eq!(lines(fit + 1), 2, "{} words do not", fit + 1);
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

        // Last in FR-10 order: Beta's row spans 58–94, so Zeta's baseline
        // is 94 + 20, and the content reaches its descent, 114 + 494/2048
        // × 12 (Inter's hhea descender).
        let l = lay(&format!("{HEAD}A,Zeta,Nowhere,d\nB,Beta,Adopt,d\n"), None);
        let zeta = *headings(&l, 3).last().expect("Zeta");
        assert_eq!(zeta.lines, ["Zeta"]);
        assert!(close(zeta.origin.y, 114.0), "{zeta:?}");
        assert!(close(l.height, 116.894_531_25), "{}", l.height);

        // One unplotted row between two plotted ones; the content ends at
        // the second row's bottom, 58 + 36 + 36.
        let l = lay(
            &format!("{HEAD}A,Q,Adopt,d\nB,Q,Nowhere,d\nC,Q,Adopt,d\n"),
            None,
        );
        let got: Vec<(usize, &str)> = rows(&l).iter().map(|r| (r.number, first_line(r))).collect();
        assert_eq!(got, [(1, "A"), (2, "C")]);
        assert!(close(l.height, 130.0), "{}", l.height);
    }

    /// The width is the widest line's right edge at its layout width: here
    /// the name "A" at 68, its advance 8.624… and D-78's allowance of 1.5
    /// for one character (see `row`'s `meta_rides_inline_when_it_fits`).
    #[test]
    fn widths() {
        let l = lay(&format!("{HEAD}A,Q,Adopt,d\n"), None);
        assert!(close(l.width, 78.124_267_578_125), "{}", l.width);

        let long = NAME_THREE_LINES;
        let l = lay(&format!("{HEAD}{long},Q,Adopt,d\n"), None);
        let widest = only(&rows(&l))
            .name
            .lines
            .iter()
            .map(|line| 68.0 + LEGEND_NAME.measure(line, fonts()))
            .fold(0.0, f64::max);
        assert!(l.width <= 380.0, "{}", l.width);
        assert!(close(l.width, widest), "{} vs {widest}", l.width);
    }

    #[test]
    fn zero_quadrants_lays_out_nothing() {
        let catalogue = empty_catalogue();
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
