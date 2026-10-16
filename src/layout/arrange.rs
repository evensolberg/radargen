//! The wide and narrow arrangements (FR-68–FR-71): one set of blocks,
//! placed two ways.
//!
//! An arrangement reads each block's size and renders it at a placement; it
//! never lays a block out again, so both arrangements hold the same items,
//! numbers and treatments by construction (architecture §4). The key is
//! the one block with variants: the arrangement picks its columns for the
//! width its rule spans, from widths measured once (rg-zb9).

use std::num::NonZeroUsize;

use crate::diagnostics::{RenderKind, Warning, Warnings};
use crate::layout::block::{Block, Placement};
use crate::layout::geometry::growth_steps;
use crate::layout::key::KeyBlock;
use crate::layout::legend::LegendBlock;
use crate::layout::radar::RadarBlock;
use crate::layout::title::TitleBlock;
use crate::model::Catalogue;
use crate::order::Plotted;
use crate::palette::{Neutral, Role};
use crate::style::{
    COLUMN_GAP, DOC_TOP, KEY_RULE_STROKE, KEY_TO_LEGEND, LEGEND_PANEL_INSET_X,
    LEGEND_PANEL_INSETS_X, LEGEND_WIDE_DROP, MARGIN, NARROW_MAX_WIDTH, NARROW_MIN_SCALE,
    NARROW_RADAR_COLUMN, NARROW_WIDTH, RADAR_INSET, RADAR_INSETS, RADAR_LABEL_PAD,
    wide_shows_on_laptops,
};
use crate::svg::{Document, Group, Node, Paint, Point, Semantics};
use crate::text::Fonts;

/// The four blocks, laid out once and placed by either arrangement
/// (FR-68). The pipeline builds one and arranges it twice.
#[derive(Debug, Clone, PartialEq)]
pub struct Blocks {
    /// The title block (FR-65).
    pub title: TitleBlock,
    /// The radar (FR-30–FR-36).
    pub radar: RadarBlock,
    /// The encoding key (FR-66).
    pub key: KeyBlock,
    /// The legend (FR-35).
    pub legend: LegendBlock,
}

impl Blocks {
    /// The four blocks for `catalogue` in `plotted`'s order, each laid out
    /// once (architecture §4), the radar grown to fit its markers (D-86).
    /// `title` and `subtitle` are FR-65's values.
    #[must_use]
    pub fn new(
        catalogue: &Catalogue,
        plotted: &Plotted,
        title: Option<&str>,
        subtitle: Option<&str>,
        fonts: &Fonts,
    ) -> Self {
        let mut blocks = Self {
            title: TitleBlock::new(title, subtitle, fonts),
            radar: RadarBlock::new(catalogue, plotted, fonts),
            key: KeyBlock::new(fonts),
            legend: LegendBlock::new(catalogue, plotted, fonts),
        };
        blocks.grow_within(blocks.radar.needed_growth());
        blocks
    }

    /// D-86: grows the radar by the largest step up to `needed` at which the
    /// wide arrangement still shows on a `WIDE_MIN_VIEWPORT_PX` screen. The
    /// steps end at 1, so when none shows the radar is left ungrown.
    ///
    /// Growth widens the wide document, which raises FR-69's breakpoint, so
    /// the bound is measured on the placed document: labels and the legend
    /// add width that does not grow.
    fn grow_within(&mut self, needed: f64) {
        for g in growth_steps().rev().filter(|&g| g <= needed) {
            self.radar.set_growth(g);
            let at = wide(self);
            if wide_shows_on_laptops(at.width, at.radar_right) {
                return;
            }
        }
    }
}

/// Which way the blocks are placed (FR-68).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arrangement {
    /// The legend beside the radar, everything at scale 1.
    Wide,
    /// The legend below the key, the radar scaled to fit (FR-70, D-71).
    Narrow,
}

/// The rule drawn along the key's top: the arrangement's, not the key's
/// (FR-66).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rule {
    /// Where the rule starts, at the left margin.
    pub from: Point,
    /// Where it ends.
    pub to: Point,
}

/// Where everything goes, worked out from the blocks' sizes alone.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placements {
    /// The document's width in user units; its viewBox starts at 0.
    pub width: f64,
    /// The document's height in user units.
    pub height: f64,
    /// The title block, at scale 1.
    pub title: Placement,
    /// The radar, at 1 in wide and its narrow scale in narrow.
    pub radar: Placement,
    /// The key's top rule.
    pub rule: Rule,
    /// The key, at scale 1.
    pub key: Placement,
    /// The key's columns: as many as fit along its rule (rg-zb9).
    pub key_columns: NonZeroUsize,
    /// The legend panel, at scale 1.
    pub legend: Placement,
    /// The legend panel's width at scale 1: its own in wide, where it sets
    /// the document's width, and the column's in narrow (rg-zch).
    pub legend_width: f64,
    /// Where the radar block ends on the right, at its scale; in wide,
    /// where HTML cuts the arrangement between radar and legend (D-85).
    pub radar_right: f64,
}

/// `blocks` drawn in `arrangement`, in document order: title, radar, the
/// key's rule, key, legend. This is reading order: every radar marker
/// precedes every legend row (FR-68).
#[must_use]
pub fn arrange(arrangement: Arrangement, blocks: &Blocks) -> Document {
    let at = placements(arrangement, blocks);
    let children = vec![
        blocks.title.render(at.title),
        blocks.radar.render(at.radar),
        rule_node(at.rule),
        blocks.key.grid(at.key_columns).render(at.key),
        blocks.legend.spanning(at.legend_width).render(at.legend),
    ];
    Document {
        width: at.width,
        height: at.height,
        root: Node::Group(Group::new(Point::ORIGIN, children)),
    }
}

/// The wide arrangement as HTML draws it (D-85): four documents.
///
/// The radar can then grow to fill the window while the title, key and
/// legend keep their natural size. Each is the wide document's own drawing,
/// cut at the radar's right edge and at the key's top rule.
#[derive(Debug, Clone, PartialEq)]
pub struct WideParts {
    /// The title block, across the whole width.
    pub title: Document,
    /// The radar, from the left edge to its block's right edge.
    pub radar: Document,
    /// The key and its rule, under the radar and as wide.
    pub key: Document,
    /// The legend panel, from the radar's right edge to the document's.
    pub legend: Document,
}

/// `blocks` in the wide arrangement, cut into [`WideParts`]. The parts
/// draw what [`arrange`] draws, each translated to its own origin.
#[must_use]
pub fn wide_parts(blocks: &Blocks) -> WideParts {
    let at = placements(Arrangement::Wide, blocks);
    let top = at.radar.origin.y;
    let radar_right = at.radar_right;
    // The cut runs half a rule above the rule, so its stroke is the key's.
    let cut = at.rule.from.y - KEY_RULE_STROKE / 2.0;
    let key = blocks.key.grid(at.key_columns);
    let legend = blocks.legend.spanning(at.legend_width);
    let part = |width: f64, height: f64, origin: Point, children: Vec<Node>| Document {
        width,
        height,
        root: Node::Group(Group::new(
            Point {
                x: -origin.x,
                y: -origin.y,
            },
            children,
        )),
    };
    WideParts {
        title: part(
            at.width,
            top,
            Point::ORIGIN,
            vec![blocks.title.render(at.title)],
        ),
        radar: part(
            radar_right,
            cut - top,
            Point { x: 0.0, y: top },
            vec![blocks.radar.render(at.radar)],
        ),
        key: part(
            radar_right,
            at.key.origin.y + key.size(1.0).height + MARGIN - cut,
            Point { x: 0.0, y: cut },
            vec![rule_node(at.rule), key.render(at.key)],
        ),
        legend: part(
            at.width - radar_right,
            at.legend.origin.y + legend.size(1.0).height + MARGIN - top,
            Point {
                x: radar_right,
                y: top,
            },
            vec![legend.render(at.legend)],
        ),
    }
}

/// FR-95: one warning when the narrow arrangement is wider than FR-94's
/// targets are sized for (D-71), naming the label that sets the radar's
/// width (D-73).
pub fn warn_narrow(blocks: &Blocks, warnings: &mut Warnings) {
    let at = placements(Arrangement::Narrow, blocks);
    if at.width <= NARROW_MAX_WIDTH {
        return;
    }
    if let Some(binding) = blocks.radar.binding_label(at.radar.scale) {
        warnings.push(Warning::Render(RenderKind::NarrowWidened {
            binding,
            width: whole_units(at.width),
        }));
    }
}

/// `width` rounded up to a whole unit, as FR-95 reports it.
const fn whole_units(width: f64) -> u32 {
    crate::num::saturating_u32(width.ceil())
}

/// The key's rule: decorative, since the key's heading marks the boundary.
fn rule_node(rule: Rule) -> Node {
    let line = Node::Line {
        from: rule.from,
        to: rule.to,
        stroke: Paint::Role(Role::Neutral(Neutral::StrokeDivider)),
        stroke_width: KEY_RULE_STROKE,
    };
    Node::Group(Group::new(Point::ORIGIN, vec![line]).with(Semantics::Decorative))
}

/// Where `blocks` go in `arrangement`.
#[must_use]
pub fn placements(arrangement: Arrangement, blocks: &Blocks) -> Placements {
    match arrangement {
        Arrangement::Wide => wide(blocks),
        // D-70's floor is on the radar's drawn size, so a grown radar may
        // shrink further: it draws on a phone as an ungrown one would (D-86).
        Arrangement::Narrow => narrow(
            blocks,
            narrow_scale(
                &blocks.radar,
                NARROW_RADAR_COLUMN,
                NARROW_MIN_SCALE / blocks.radar.growth(),
            ),
        ),
    }
}

/// The legend beside the radar, its content `COLUMN_GAP` after the radar's
/// widest label ink; the key under the radar, its rule as wide as the
/// radar's ink.
fn wide(blocks: &Blocks) -> Placements {
    let top = DOC_TOP + blocks.title.size(1.0).height;
    let radar = blocks.radar.size(1.0);
    let radar_right = RADAR_INSET + radar.width;
    let ink_right = radar_right - RADAR_LABEL_PAD;
    let key_y = top + radar.height;
    let key_columns = blocks.key.columns_within(ink_right - MARGIN);
    let legend = blocks.legend.size(1.0);
    let (legend_at, width) = if legend.is_empty() {
        (
            Point {
                x: radar_right,
                y: top,
            },
            radar_right + RADAR_INSET,
        )
    } else {
        let content_x = ink_right + COLUMN_GAP;
        (
            Point {
                x: content_x - LEGEND_PANEL_INSET_X,
                y: top + LEGEND_WIDE_DROP,
            },
            content_x + legend.width - LEGEND_PANEL_INSETS_X + MARGIN,
        )
    };
    Placements {
        width,
        height: (key_y + blocks.key.grid(key_columns).size(1.0).height)
            .max(legend_at.y + legend.height)
            + MARGIN,
        title: Placement::at(Point {
            x: MARGIN,
            y: DOC_TOP,
        }),
        radar: Placement::at(Point {
            x: RADAR_INSET,
            y: top,
        }),
        rule: Rule {
            from: Point {
                x: MARGIN,
                y: key_y,
            },
            to: Point {
                x: ink_right,
                y: key_y,
            },
        },
        key: Placement::at(Point {
            x: MARGIN,
            y: key_y,
        }),
        key_columns,
        legend: Placement::at(legend_at),
        legend_width: legend.width,
        radar_right,
    }
}

/// Everything stacked: title, the radar at `scale`, key, legend. The
/// document is `NARROW_WIDTH` wide, or as wide as the radar and its insets
/// when it does not fit its column (D-71).
fn narrow(blocks: &Blocks, scale: f64) -> Placements {
    let top = DOC_TOP + blocks.title.size(1.0).height;
    let radar = blocks.radar.size(scale);
    let width = NARROW_WIDTH.max(radar.width + RADAR_INSETS);
    let key_y = top + radar.height;
    let rule_end = width - MARGIN;
    let key_columns = blocks.key.columns_within(rule_end - MARGIN);
    let key_bottom = key_y + blocks.key.grid(key_columns).size(1.0).height;
    let legend = blocks.legend.size(1.0);
    let legend_y = if legend.is_empty() {
        key_bottom
    } else {
        key_bottom + KEY_TO_LEGEND
    };
    // The panel starts its inset before the rule and ends as far past it.
    let legend_x = MARGIN - LEGEND_PANEL_INSET_X;
    Placements {
        width,
        height: legend_y + legend.height + MARGIN,
        title: Placement::at(Point {
            x: MARGIN,
            y: DOC_TOP,
        }),
        radar: Placement {
            origin: Point {
                x: RADAR_INSET,
                y: top,
            },
            scale,
        },
        rule: Rule {
            from: Point {
                x: MARGIN,
                y: key_y,
            },
            to: Point {
                x: rule_end,
                y: key_y,
            },
        },
        key: Placement::at(Point {
            x: MARGIN,
            y: key_y,
        }),
        key_columns,
        legend: Placement::at(Point {
            x: legend_x,
            y: legend_y,
        }),
        // Twice the inset, as two subtractions: NFR-4 keeps mul_add out.
        legend_width: width - legend_x - legend_x,
        radar_right: RADAR_INSET + radar.width,
    }
}

/// Halvings of [least, 1]: past f64 resolution after about 54;
/// the count is fixed so every machine gets the same bits (NFR-4).
const BISECTIONS: u32 = 60;

/// The largest scale in [`least`, 1] at which `radar` is at most `column`
/// wide, or `least` when none is (D-71). `least` is `NARROW_MIN_SCALE` on
/// an ungrown radar, and less on a grown one (D-86).
///
/// Only scales that passed the test are returned, so the result fits when
/// any scale does; it is the largest because the radar's size is
/// non-decreasing in the scale. A radar block is at least
/// `MIN_RADAR_WIDTH` (660) wide at 1, wider than any column, so it never
/// takes the first return; only blocks that fit at natural size do.
fn narrow_scale(radar: &impl Block, column: f64, least: f64) -> f64 {
    let fits = |scale: f64| radar.size(scale).width <= column;
    if fits(1.0) {
        return 1.0;
    }
    if !fits(least) {
        return least;
    }
    let (mut lo, mut hi) = (least, 1.0);
    for _ in 0..BISECTIONS {
        let mid = f64::midpoint(lo, hi);
        if fits(mid) {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    lo
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostics::Binding;
    use crate::layout::block::{Size, close, fonts};
    use crate::layout::geometry::{growth, place};
    use crate::layout::test_support::{
        HEAD, NAME_THREE_LINES, PRD_EXAMPLE, Rect, assert_finite, assert_inside, bounds, csv_of,
        groups, near, plotted,
    };
    use crate::model::Catalogue;
    use crate::order::Plotted;
    use crate::rings::RingSource;
    use crate::style::{GROWTH_STEP, MAX_GROWTH};
    use crate::svg::{ItemKey, Semantics};

    /// The title and subtitle a fixture is built with.
    #[derive(Clone, Copy)]
    struct Titles<'a> {
        title: Option<&'a str>,
        subtitle: Option<&'a str>,
    }

    const UNTITLED: Titles = Titles {
        title: None,
        subtitle: None,
    };

    /// T = 4 + 18 + 17 + 14 = 53: `TITLE_TOP`, the title's first baseline,
    /// `SUBTITLE_AFTER_TITLE`, `TITLE_BOTTOM`.
    const TITLED: Titles = Titles {
        title: Some("Technology Radar"),
        subtitle: Some("Managed estates, 2026"),
    };

    fn blocks_of(catalogue: &Catalogue, plotted: &Plotted, titles: Titles) -> Blocks {
        Blocks::new(catalogue, plotted, titles.title, titles.subtitle, fonts())
    }

    fn blocks(csv: &str, titles: Titles) -> Blocks {
        let (catalogue, order) = plotted(csv, None);
        blocks_of(&catalogue, &order, titles)
    }

    /// One item per quadrant, all in Adopt.
    fn quadrants(names: &[&str]) -> String {
        csv_of(names, "Adopt")
    }

    /// Thoughtworks' four quadrants: 460.0 wide at 0.3 with D-78's spacing
    /// allowance, so the document widens within the headroom (386.75 before).
    fn thoughtworks() -> String {
        quadrants(&["Techniques", "Tools", "Platforms", "Languages & Frameworks"])
    }

    /// 521.6 wide at 0.3 with D-78's allowance: past 480, so FR-95 warns
    /// (437.8, within the headroom, before).
    fn widened() -> String {
        quadrants(&["Infrastructure Platforms", "Tools", "Practice"])
    }

    /// One 70-character quadrant name: wider than 460 at 0.3, past 480.
    fn beyond() -> String {
        quadrants(&[&"W".repeat(70)])
    }

    /// Four quadrants, one with a long name: over 700 wide at scale 1.
    fn wide_radar() -> String {
        quadrants(&["A", "B", "C", &"W".repeat(40)])
    }

    /// Thirty items in one quadrant: a legend taller than radar and key.
    fn tall() -> String {
        quadrants(&["Q"; 30])
    }

    fn nine() -> String {
        let names: Vec<String> = (0..9).map(|i| format!("Q{i}")).collect();
        quadrants(&names.iter().map(String::as_str).collect::<Vec<_>>())
    }

    /// Every fixture the document-level properties are checked over.
    fn fixtures() -> Vec<(&'static str, Blocks)> {
        let long_title = "Technology radar ".repeat(12);
        vec![
            ("prd", blocks(PRD_EXAMPLE, TITLED)),
            ("untitled", blocks(PRD_EXAMPLE, UNTITLED)),
            (
                "long title",
                blocks(
                    PRD_EXAMPLE,
                    Titles {
                        title: Some(&long_title),
                        subtitle: None,
                    },
                ),
            ),
            ("one row", blocks(&quadrants(&["Q"]), TITLED)),
            (
                "long names",
                blocks(&format!("{HEAD}{NAME_THREE_LINES},Q,Adopt,d\n"), TITLED),
            ),
            ("nine", blocks(&nine(), TITLED)),
            ("thoughtworks", blocks(&thoughtworks(), TITLED)),
            ("widened", blocks(&widened(), TITLED)),
            ("beyond", blocks(&beyond(), TITLED)),
            ("wide radar", blocks(&wide_radar(), TITLED)),
            ("tall", blocks(&tall(), TITLED)),
            ("empty legend", blocks(HEAD, UNTITLED)),
            // D-86: grown to 1.25, to 2, held to 1.25 by the laptop bound,
            // and grown with names that hold the narrow radar at its floor.
            ("grown", blocks(&innermost("Q", 3), TITLED)),
            ("grown twice", blocks(&innermost("Q", 12), TITLED)),
            (
                "grown bound",
                blocks(&innermost(&"W".repeat(30), 5), TITLED),
            ),
            (
                "grown floor",
                blocks(&innermost("Languages & Frameworks ", 5), TITLED),
            ),
        ]
    }

    /// Where a placed block ends below.
    fn bottom(at: Placement, block: &impl Block) -> f64 {
        at.origin.y + block.size(at.scale).height
    }

    fn radar_size(b: &Blocks, p: &Placements) -> Size {
        b.radar.size(p.radar.scale)
    }

    /// A block `w · s` square at scale `s`: the bisection without a radar.
    struct Linear(f64);

    impl Block for Linear {
        fn size(&self, scale: f64) -> Size {
            Size {
                width: self.0 * scale,
                height: self.0 * scale,
            }
        }

        fn render(&self, at: Placement) -> Node {
            Node::Group(Group::new(at.origin, Vec::new()))
        }
    }

    fn radar(csv: &str) -> RadarBlock {
        let (catalogue, order) = plotted(csv, None);
        RadarBlock::new(&catalogue, &order, fonts())
    }

    #[test]
    fn narrow_scale_finds_the_largest_fit() {
        let block = Linear(1000.0);
        assert!(close(narrow_scale(&block, 400.0, NARROW_MIN_SCALE), 0.4));
        for column in [300.0, 333.3, 399.999] {
            let scale = narrow_scale(&block, column, NARROW_MIN_SCALE);
            assert!(block.size(scale).width <= column, "{column}: {scale}");
        }
    }

    /// `count` items in the first quadrant's innermost band and one in each
    /// other's, the quadrants named `stem` and a digit. Four quadrants'
    /// innermost band holds two at natural size (FR-89), so three overflow.
    fn innermost(stem: &str, count: usize) -> String {
        let names: Vec<String> = std::iter::repeat_n(0, count)
            .chain(1..=3)
            .map(|q| format!("{stem}{q}"))
            .collect();
        let names: Vec<&str> = names.iter().map(String::as_str).collect();
        csv_of(&names, "Retire")
    }

    /// Whether `b`'s wide arrangement shows on a laptop (D-86).
    fn shows(b: &Blocks) -> bool {
        let at = placements(Arrangement::Wide, b);
        wide_shows_on_laptops(at.width, at.radar_right)
    }

    /// The growth `csv`'s markers need, before the laptop bound (D-86).
    fn needed(csv: &str) -> f64 {
        let (catalogue, order) = plotted(csv, None);
        growth(&place(&catalogue, &order))
    }

    /// rg-06b: a radar whose markers already clear does not grow, so its
    /// documents are what they were.
    #[test]
    fn a_radar_that_fits_does_not_grow() {
        for csv in [PRD_EXAMPLE.to_owned(), thoughtworks(), widened()] {
            assert!(close(blocks(&csv, UNTITLED).radar.growth(), 1.0), "{csv}");
        }
    }

    /// D-86: an overfull band grows the radar by the least step that clears
    /// every pair of markers.
    #[test]
    fn an_overfull_band_grows_the_radar() {
        let want = needed(&innermost("Q", 3));
        assert!(want > 1.0, "the fixture overflows at natural size");
        assert!(close(
            blocks(&innermost("Q", 3), UNTITLED).radar.growth(),
            want
        ));
    }

    /// D-86: the radar grows no further than keeps the wide arrangement
    /// showing on a `WIDE_MIN_VIEWPORT_PX` screen, and stops short of what
    /// it needs only where one more step would hide it. Ordinary names
    /// reach 2× within the bound; quadrant names of 30 × "W" hold it to
    /// 1.25.
    #[test]
    fn growth_keeps_the_wide_arrangement_on_laptops() {
        let mut capped = 0;
        let long = "W".repeat(30);
        for (stem, count) in ["Q", long.as_str()]
            .into_iter()
            .flat_map(|stem| (3..=12).map(move |count| (stem, count)))
        {
            let csv = innermost(stem, count);
            let b = blocks(&csv, UNTITLED);
            let g = b.radar.growth();
            let want = needed(&csv);
            assert!(g <= want && g <= MAX_GROWTH, "{stem} {count}: {g}");
            assert!(shows(&b), "{stem} {count}: {g}");
            if g < want {
                capped += 1;
                let mut next = b.clone();
                next.radar.set_growth(g + GROWTH_STEP);
                assert!(!shows(&next), "{stem} {count}: {g} stopped short");
            }
        }
        assert!(capped > 0, "some fixture is held by the laptop bound");
        // D-86's figures: ordinary names reach 2×, 30 × "W" stop at 1.25.
        assert!(close(
            blocks(&innermost("Q", 12), UNTITLED).radar.growth(),
            MAX_GROWTH
        ));
        assert!(close(
            blocks(&innermost(&long, 12), UNTITLED).radar.growth(),
            1.25
        ));
    }

    /// D-86: the narrow arrangement draws a grown radar at the size it would
    /// draw an ungrown one, so phones see the same document.
    #[test]
    fn narrow_draws_a_grown_radar_as_an_ungrown_one() {
        // Short names fit the column above the floor; Thoughtworks-length
        // ones hold an ungrown radar at it, so the floor's own growth is
        // tested too.
        for stem in ["Q", "Languages & Frameworks "] {
            let grown = blocks(&innermost(stem, 5), UNTITLED);
            let g = grown.radar.growth();
            assert!(g > 1.0, "{stem}");
            let mut flat = grown.clone();
            flat.radar.set_growth(1.0);
            let (a, b) = (
                placements(Arrangement::Narrow, &grown),
                placements(Arrangement::Narrow, &flat),
            );
            let drawn = a.radar.scale * g;
            assert!(
                (drawn - b.radar.scale).abs() < 1e-9,
                "{stem}: {a:?} vs {b:?}"
            );
            assert!(
                (a.width - b.width).abs() < 1e-6 && (a.height - b.height).abs() < 1e-6,
                "{stem}: {a:?} vs {b:?}"
            );
        }
        let floored = blocks(&innermost("Languages & Frameworks ", 5), UNTITLED);
        let at = placements(Arrangement::Narrow, &floored);
        assert!(
            close(at.radar.scale, NARROW_MIN_SCALE / floored.radar.growth()),
            "the long names reach the floor: {at:?}"
        );
    }

    #[test]
    fn narrow_scale_boundaries() {
        // Fits at natural size, inclusively: never enlarged.
        assert!(close(
            narrow_scale(&Linear(400.0), 400.0, NARROW_MIN_SCALE),
            1.0
        ));
        assert!(close(
            narrow_scale(&Linear(300.0), 400.0, NARROW_MIN_SCALE),
            1.0
        ));
        // Fits exactly at the floor, and misses it by one ulp.
        let wide = Linear(2000.0);
        let at_floor = wide.size(NARROW_MIN_SCALE).width;
        assert!(close(
            narrow_scale(&wide, at_floor, NARROW_MIN_SCALE),
            NARROW_MIN_SCALE
        ));
        let below = f64::from_bits(at_floor.to_bits() - 1);
        assert!(close(
            narrow_scale(&wide, below, NARROW_MIN_SCALE),
            NARROW_MIN_SCALE
        ));
    }

    /// NFR-4: the search runs to the last ulp, the largest double with
    /// 1000 · s ≤ 400, on every machine; a bisection count cut short of
    /// about 54 would stop below it.
    #[test]
    fn narrow_scale_is_bit_stable() {
        let scale = narrow_scale(&Linear(1000.0), 400.0, NARROW_MIN_SCALE);
        assert_eq!(scale.to_bits(), GOLDEN_LINEAR_BITS, "{scale}");
    }

    /// 0.4 exactly, as recorded on the first run; see `narrow_scale_is_bit_stable`.
    const GOLDEN_LINEAR_BITS: u64 = 0x3FD9_9999_9999_999A;

    #[test]
    fn narrow_scale_of_the_radar() {
        // Zero quadrants: the 660 · s floor sets the width.
        let floor = radar(HEAD);
        let scale = narrow_scale(&floor, NARROW_RADAR_COLUMN, NARROW_MIN_SCALE);
        assert!(close(scale, 400.0 / 660.0), "{scale}");
        // The PRD example's labels set it: it fills the column, and a hair
        // more scale does not fit (valid off a plateau, as here).
        let prd = radar(PRD_EXAMPLE);
        let scale = narrow_scale(&prd, NARROW_RADAR_COLUMN, NARROW_MIN_SCALE);
        assert!(close(prd.size(scale).width, 400.0), "{scale}");
        assert!(prd.size(scale + 1e-6).width > 400.0, "{scale}");
    }

    #[test]
    fn wide_places_the_legend_beside_the_radar() {
        let b = blocks(PRD_EXAMPLE, TITLED);
        assert!(close(b.title.size(1.0).height, 53.0));
        let p = placements(Arrangement::Wide, &b);
        let key = b.key.grid(p.key_columns);
        assert!(close(b.radar.size(1.0).width, 660.0) && close(key.size(1.0).height, 98.0));
        assert!(near(p.title.origin, Point { x: 20.0, y: 10.0 }), "{p:?}");
        assert!(near(p.radar.origin, Point { x: 10.0, y: 63.0 }), "{p:?}");
        assert!(near(p.key.origin, Point { x: 20.0, y: 723.0 }), "{p:?}");
        assert!(near(p.rule.from, Point { x: 20.0, y: 723.0 }), "{p:?}");
        assert!(near(p.rule.to, Point { x: 660.0, y: 723.0 }), "{p:?}");
        // The legend's content at the radar's ink (660) + 28 = 688.
        assert!(near(p.legend.origin, Point { x: 676.0, y: 69.0 }), "{p:?}");
        for at in [p.title, p.radar, p.key, p.legend] {
            assert!(close(at.scale, 1.0), "{at:?}");
        }
        let legend = b.legend.size(1.0);
        assert!(
            close(p.width, 688.0 + legend.width - 24.0 + 20.0),
            "{}",
            p.width
        );
        // The key is the lower: 723 + 98 + 20, its five entries in two
        // columns of three rows (rg-zb9).
        assert!(close(p.height, 841.0), "{}", p.height);
        assert!(69.0 + legend.height + 20.0 < 841.0, "{legend:?}");
        assert_eq!(p.key_columns.get(), 2);
        // The wide legend's panel is its own width: it sets the document's.
        assert!(close(p.legend_width, legend.width), "{p:?}");
    }

    /// rg-zb9: the key takes as many columns as fit along its rule, so it
    /// never reaches past the rule's end, in either arrangement.
    #[test]
    fn the_key_fits_along_its_rule() {
        for (name, b) in fixtures() {
            for arrangement in [Arrangement::Wide, Arrangement::Narrow] {
                let p = placements(arrangement, &b);
                let key = b.key.grid(p.key_columns).size(1.0);
                assert!(
                    p.key.origin.x + key.width <= p.rule.to.x,
                    "{name} {arrangement:?}: {key:?} past {:?}",
                    p.rule
                );
            }
            // At the authored width, one column; a narrow document widened
            // for its radar (D-71) may hold more.
            let narrow = placements(Arrangement::Narrow, &b);
            if close(narrow.width, 420.0) {
                assert_eq!(narrow.key_columns.get(), 1, "{name}");
            }
        }
    }

    #[test]
    fn narrow_stacks_and_scales_the_radar() {
        let b = blocks(PRD_EXAMPLE, TITLED);
        let p = placements(Arrangement::Narrow, &b);
        let radar = radar_size(&b, &p);
        assert!(close(p.width, 420.0), "{}", p.width);
        assert!(near(p.radar.origin, Point { x: 10.0, y: 63.0 }), "{p:?}");
        assert!(close(radar.width, 400.0), "{radar:?}");
        let key_y = 63.0 + radar.height;
        assert!(near(p.key.origin, Point { x: 20.0, y: key_y }), "{p:?}");
        let key_bottom = key_y + 146.0;
        assert!(
            near(
                p.legend.origin,
                Point {
                    x: 8.0,
                    y: key_bottom + 16.0
                }
            ),
            "{p:?}"
        );
        for at in [p.title, p.key, p.legend] {
            assert!(close(at.scale, 1.0), "{at:?}");
        }
        assert!(
            close(p.height, bottom(p.legend, &b.legend) + 20.0),
            "{}",
            p.height
        );
        assert!(near(p.rule.from, Point { x: 20.0, y: key_y }), "{p:?}");
        assert!(near(p.rule.to, Point { x: 400.0, y: key_y }), "{p:?}");
        // rg-zch: the panel spans the column, as far past the rule's end
        // as it starts before the rule's start.
        assert!(close(p.legend_width, 404.0), "{p:?}");
    }

    /// FR-94's premise (D-71): realistic radars keep the 420 width.
    #[test]
    fn ordinary_radars_keep_the_narrow_width() {
        for (name, b) in fixtures() {
            // The fixtures with long quadrant names widen; since D-78's
            // spacing allowance, so do Thoughtworks' four.
            if matches!(
                name,
                "thoughtworks"
                    | "widened"
                    | "beyond"
                    | "wide radar"
                    | "grown bound"
                    | "grown floor"
            ) {
                continue;
            }
            let p = placements(Arrangement::Narrow, &b);
            assert!(close(p.width, NARROW_WIDTH), "{name}: {}", p.width);
        }
    }

    #[test]
    fn narrow_widens_when_the_radar_cannot_fit() {
        let within = blocks(&thoughtworks(), TITLED);
        let p = placements(Arrangement::Narrow, &within);
        assert!(close(p.radar.scale, NARROW_MIN_SCALE), "{p:?}");
        assert!(
            p.width > NARROW_WIDTH && p.width <= NARROW_MAX_WIDTH,
            "{}",
            p.width
        );
        assert!(close(p.radar.origin.x, 10.0), "{p:?}");

        let beyond = blocks(&beyond(), TITLED);
        let q = placements(Arrangement::Narrow, &beyond);
        assert!(q.width > NARROW_MAX_WIDTH, "{}", q.width);
        let past = placements(Arrangement::Narrow, &blocks(&widened(), TITLED));
        assert!(past.width > NARROW_MAX_WIDTH, "{}", past.width);

        for p in [p, q] {
            assert!(near(p.title.origin, Point { x: 20.0, y: 10.0 }), "{p:?}");
            assert!(
                close(p.key.origin.x, 20.0) && close(p.legend.origin.x, 8.0),
                "{p:?}"
            );
            assert!(close(p.rule.to.x, p.width - 20.0), "{p:?}");
            // rg-zch: the panel follows the widened column.
            assert!(close(p.legend_width, p.width - 16.0), "{p:?}");
        }
    }

    /// FR-95's warnings for `b`.
    fn narrow_warnings(b: &Blocks) -> Vec<Warning> {
        let mut warnings = Warnings::default();
        warn_narrow(b, &mut warnings);
        warnings.as_slice().to_vec()
    }

    /// The one FR-95 warning `b` raises.
    fn narrowed(b: &Blocks) -> (Binding, u32) {
        match narrow_warnings(b).as_slice() {
            [Warning::Render(RenderKind::NarrowWidened { binding, width })] => {
                (binding.clone(), *width)
            }
            other => panic!("one FR-95 warning, got {other:?}"),
        }
    }

    /// `b` with `rings` declared.
    fn with_rings(csv: &str, rings: &str) -> Blocks {
        let (catalogue, order) = plotted(csv, Some(rings));
        blocks_of(&catalogue, &order, TITLED)
    }

    #[test]
    fn fr95_names_the_binding_quadrant() {
        let b = blocks(&beyond(), TITLED);
        let width = placements(Arrangement::Narrow, &b).width;
        let (binding, warned) = narrowed(&b);
        assert_eq!(binding, Binding::Quadrant("W".repeat(70)));
        assert_eq!(f64::from(warned), width.ceil(), "{width}");
    }

    /// D-73: a long ring name sets the width with a short quadrant, or with
    /// none at all.
    #[test]
    fn fr95_names_a_binding_ring() {
        let ring = "W".repeat(70);
        let rings = format!("{ring},Adopt");
        for csv in [quadrants(&["Tools"]), HEAD.to_owned()] {
            let (binding, _) = narrowed(&with_rings(&csv, &rings));
            assert_eq!(
                binding,
                Binding::Ring {
                    name: ring.clone(),
                    source: RingSource::Flag,
                },
                "{csv}"
            );
        }
    }

    #[test]
    fn fr95_is_quiet_within_the_headroom() {
        for csv in [thoughtworks(), PRD_EXAMPLE.to_owned()] {
            assert_eq!(narrow_warnings(&blocks(&csv, TITLED)), [], "{csv}");
        }
    }

    #[test]
    fn narrow_width_invariant() {
        for (name, b) in fixtures() {
            let p = placements(Arrangement::Narrow, &b);
            let radar = radar_size(&b, &p);
            if radar.width <= NARROW_RADAR_COLUMN {
                assert!(close(p.width, NARROW_WIDTH), "{name}: {}", p.width);
            } else {
                assert!(close(p.width, radar.width + 20.0), "{name}: {}", p.width);
            }
        }
    }

    #[test]
    fn wide_follows_a_wide_radar() {
        let b = blocks(&wide_radar(), TITLED);
        let p = placements(Arrangement::Wide, &b);
        let radar = radar_size(&b, &p);
        assert!(radar.width > 700.0, "{radar:?}");
        assert!(close(p.rule.to.x, radar.width), "{p:?}");
        assert!(close(p.legend.origin.x, radar.width + 28.0 - 12.0), "{p:?}");
    }

    #[test]
    fn a_tall_legend_sets_the_wide_height() {
        let b = blocks(&tall(), TITLED);
        let p = placements(Arrangement::Wide, &b);
        let legend = b.legend.size(1.0);
        assert!(
            bottom(p.legend, &b.legend) > bottom(p.key, &b.key.grid(p.key_columns)),
            "{p:?}"
        );
        assert!(
            close(p.height, 10.0 + 53.0 + 6.0 + legend.height + 20.0),
            "{}",
            p.height
        );
        let key_right = p.key.origin.x + b.key.grid(p.key_columns).size(1.0).width;
        assert!(key_right < p.legend.origin.x, "{key_right} vs {p:?}");
        assert!(
            p.rule.to.x + KEY_RULE_STROKE / 2.0 < p.legend.origin.x,
            "{p:?}"
        );
    }

    #[test]
    fn no_title_reserves_nothing() {
        let b = blocks(PRD_EXAMPLE, UNTITLED);
        for arrangement in [Arrangement::Wide, Arrangement::Narrow] {
            let p = placements(arrangement, &b);
            assert!(near(p.title.origin, Point { x: 20.0, y: 10.0 }), "{p:?}");
            assert!(close(p.radar.origin.y, 10.0), "{p:?}");
            assert!(bounds(&b.title.render(p.title), fonts()).is_none());
        }
        let p = placements(Arrangement::Wide, &b);
        assert!(close(p.legend.origin.y, 16.0), "{p:?}");

        let subtitled = blocks(
            PRD_EXAMPLE,
            Titles {
                title: None,
                subtitle: Some("Managed estates"),
            },
        );
        let p = placements(Arrangement::Wide, &subtitled);
        assert!(p.radar.origin.y > 10.0, "{p:?}");
    }

    #[test]
    fn an_empty_legend_takes_no_space() {
        let b = blocks(HEAD, UNTITLED);
        let wide = placements(Arrangement::Wide, &b);
        assert!(close(wide.width, 680.0), "{}", wide.width);
        for p in [wide, placements(Arrangement::Narrow, &b)] {
            assert!(
                close(p.height, bottom(p.key, &b.key.grid(p.key_columns)) + 20.0),
                "{p:?}"
            );
        }
        let narrow = placements(Arrangement::Narrow, &b);
        assert!(close(narrow.radar.scale, 400.0 / 660.0), "{narrow:?}");
    }

    /// FR-69's breakpoint is computed from the wide width (architecture §4):
    /// below it the narrow document must not be the wider of the two.
    #[test]
    fn the_wide_document_is_never_narrower() {
        for (name, b) in fixtures() {
            let wide = placements(Arrangement::Wide, &b);
            let narrow = placements(Arrangement::Narrow, &b);
            assert!(
                wide.width >= narrow.width,
                "{name}: {} < {}",
                wide.width,
                narrow.width
            );
            assert!(wide.width > NARROW_WIDTH, "{name}: {}", wide.width);
        }
    }

    #[test]
    fn columns_hold_their_blocks() {
        for (name, b) in fixtures() {
            let ink = b.radar.size(1.0).width - 20.0;
            let one = NonZeroUsize::MIN;
            for width in [b.title.size(1.0).width, b.key.grid(one).size(1.0).width] {
                assert!(width <= 380.0 && width <= ink, "{name}: {width}");
            }
            let legend = b.legend.size(1.0).width;
            assert!(
                legend - 24.0 <= 380.0 && 8.0 + legend <= 412.0,
                "{name}: {legend}"
            );
        }
    }

    const BOTH: [Arrangement; 2] = [Arrangement::Wide, Arrangement::Narrow];

    /// The root's five children, in document order.
    fn children(doc: &Document) -> [&Node; 5] {
        let Node::Group(root) = &doc.root else {
            panic!("the root is a group");
        };
        let [title, radar, rule, key, legend] = root.children.as_slice() else {
            panic!("five children: {:?}", root.children.len());
        };
        [title, radar, rule, key, legend]
    }

    #[test]
    fn blocks_render_at_their_placements() {
        let b = blocks(PRD_EXAMPLE, TITLED);
        for arrangement in BOTH {
            let doc = arrange(arrangement, &b);
            let p = placements(arrangement, &b);
            let [title, radar, rule, key, legend] = children(&doc);
            assert_eq!(*title, b.title.render(p.title));
            assert_eq!(*radar, b.radar.render(p.radar));
            assert_eq!(*key, b.key.grid(p.key_columns).render(p.key));
            assert_eq!(*legend, b.legend.spanning(p.legend_width).render(p.legend));
            let want = Node::Group(
                Group::new(
                    Point::ORIGIN,
                    vec![Node::Line {
                        from: p.rule.from,
                        to: p.rule.to,
                        stroke: Paint::Role(Role::Neutral(Neutral::StrokeDivider)),
                        stroke_width: KEY_RULE_STROKE,
                    }],
                )
                .with(Semantics::Decorative),
            );
            assert_eq!(*rule, want);
        }
    }

    /// Every control's key in tree order, and whether it is a legend row.
    fn controls(root: &Node) -> Vec<(ItemKey, bool)> {
        groups(root)
            .into_iter()
            .filter_map(|group| match &group.semantics {
                Semantics::Marker(control) => Some((control.key, false)),
                Semantics::LegendRow(control) => Some((control.key, true)),
                _ => None,
            })
            .collect()
    }

    /// FR-68: the same items in both, every marker before the first legend
    /// row; PR 14 hands focus across by this key.
    #[test]
    fn both_arrangements_hold_the_same_controls() {
        let b = blocks(PRD_EXAMPLE, TITLED);
        let found: Vec<Vec<(ItemKey, bool)>> = BOTH
            .iter()
            .map(|&arrangement| controls(&arrange(arrangement, &b).root))
            .collect();
        let [wide, narrow] = found.as_slice() else {
            panic!("two documents");
        };
        assert_eq!(wide, narrow);
        let first_row = wide.iter().position(|c| c.1).expect("legend rows");
        assert!(wide.iter().skip(first_row).all(|c| c.1), "{wide:?}");
        assert!(wide.iter().take(first_row).all(|c| !c.1), "{wide:?}");
    }

    #[test]
    fn the_document_is_the_placements_size() {
        let b = blocks(PRD_EXAMPLE, TITLED);
        for arrangement in BOTH {
            let doc = arrange(arrangement, &b);
            let p = placements(arrangement, &b);
            assert!(
                close(doc.width, p.width) && close(doc.height, p.height),
                "{p:?}"
            );
        }
    }

    #[test]
    fn every_node_is_inside_the_document() {
        for (name, b) in fixtures() {
            for arrangement in BOTH {
                let doc = arrange(arrangement, &b);
                let size = Size {
                    width: doc.width,
                    height: doc.height,
                };
                assert_finite(&doc.root);
                assert_inside(&doc.root, size, fonts());
                assert!(
                    doc.width > 0.0 && doc.height > 0.0,
                    "{name} {arrangement:?}"
                );
            }
        }
    }

    /// Every node in `node`'s tree that is not a group.
    fn leaves(node: &Node) -> usize {
        groups(node)
            .iter()
            .flat_map(|g| &g.children)
            .filter(|n| !matches!(n, Node::Group(_)))
            .count()
    }

    /// rg-swu (D-85): HTML's wide arrangement in four documents. The radar
    /// and the legend tile the wide document's width, the title spans it,
    /// the key sits under the radar, and together they draw every node the
    /// one document draws, each inside its own part.
    #[test]
    fn the_wide_parts_tile_the_wide_document() {
        for (name, b) in fixtures() {
            let whole = arrange(Arrangement::Wide, &b);
            let parts = wide_parts(&b);
            let WideParts {
                title,
                radar,
                key,
                legend,
            } = &parts;
            assert!(close(radar.width + legend.width, whole.width), "{name}");
            assert!(
                close(title.width, whole.width) && close(key.width, radar.width),
                "{name}"
            );
            let left = title.height + radar.height + key.height;
            let right = title.height + legend.height;
            assert!(
                close(left.max(right), whole.height),
                "{name}: {left}, {right} against {}",
                whole.height
            );
            for (part, doc) in [
                ("title", title),
                ("radar", radar),
                ("key", key),
                ("legend", legend),
            ] {
                let size = Size {
                    width: doc.width,
                    height: doc.height,
                };
                assert!(doc.width > 0.0 && doc.height > 0.0, "{name} {part}");
                assert_finite(&doc.root);
                assert_inside(&doc.root, size, fonts());
            }
            let drawn: usize = [title, radar, key, legend]
                .iter()
                .map(|d| leaves(&d.root))
                .sum();
            assert_eq!(drawn, leaves(&whole.root), "{name}");
        }
    }

    /// The markers are in the radar's part and the legend rows in the
    /// legend's, which the page writes after the key (FR-68's order).
    #[test]
    fn markers_and_rows_land_in_their_parts() {
        let parts = wide_parts(&blocks(PRD_EXAMPLE, TITLED));
        let has = |doc: &Document, want: fn(&Semantics) -> bool| {
            groups(&doc.root).iter().any(|g| want(&g.semantics))
        };
        let marker = |s: &Semantics| matches!(s, Semantics::Marker(_));
        let row = |s: &Semantics| matches!(s, Semantics::LegendRow(_));
        assert!(has(&parts.radar, marker) && !has(&parts.legend, marker));
        assert!(has(&parts.legend, row) && !has(&parts.radar, row));
    }

    /// `a` ends at or before `b` begins, give or take float noise.
    fn before(a: f64, b: f64) -> bool {
        a <= b + 1e-9
    }

    #[test]
    fn blocks_do_not_overlap() {
        for (name, b) in fixtures() {
            for arrangement in BOTH {
                let doc = arrange(arrangement, &b);
                let [title, radar, rule, key, legend] = children(&doc);
                let at = |node: &Node| bounds(node, fonts());
                let radar = at(radar).expect("the radar draws");
                let key = at(key).expect("the key draws");
                let rule: Rect = at(rule).expect("the rule draws");
                if let Some(title) = at(title) {
                    assert!(
                        before(title.bottom, radar.top),
                        "{name}: {title:?} {radar:?}"
                    );
                }
                assert!(before(radar.bottom, key.top), "{name}: {radar:?} {key:?}");
                let Some(legend) = at(legend) else {
                    continue;
                };
                match arrangement {
                    Arrangement::Wide => {
                        for right in [radar.right, key.right, rule.right] {
                            assert!(right < legend.left, "{name}: {right} vs {legend:?}");
                        }
                    }
                    Arrangement::Narrow => {
                        assert!(key.bottom < legend.top, "{name}: {key:?} {legend:?}");
                    }
                }
            }
        }
    }
}
