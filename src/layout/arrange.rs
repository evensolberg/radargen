//! The wide and narrow arrangements (FR-68–FR-71): one set of blocks,
//! placed two ways.
//!
//! An arrangement reads each block's size and renders it at a placement; it
//! never lays a block out again, so both arrangements hold the same items,
//! numbers and treatments by construction (architecture §4).

use crate::diagnostics::{RenderKind, Warning, Warnings};
use crate::layout::block::{Block, Placement};
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
    /// once (architecture §4). `title` and `subtitle` are FR-65's values.
    #[must_use]
    pub fn new(
        catalogue: &Catalogue,
        plotted: &Plotted,
        title: Option<&str>,
        subtitle: Option<&str>,
        fonts: &Fonts,
    ) -> Self {
        Self {
            title: TitleBlock::new(title, subtitle, fonts),
            radar: RadarBlock::new(catalogue, plotted, fonts),
            key: KeyBlock::new(fonts),
            legend: LegendBlock::new(catalogue, plotted, fonts),
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
    /// The legend panel, at scale 1.
    pub legend: Placement,
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
        blocks.key.render(at.key),
        blocks.legend.render(at.legend),
    ];
    Document {
        width: at.width,
        height: at.height,
        root: Node::Group(Group::new(Point::ORIGIN, children)),
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
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::as_conversions,
    reason = "f64 has no TryFrom to an integer; a document width is positive, and `as` saturates at u32::MAX for any width past it"
)]
const fn whole_units(width: f64) -> u32 {
    width.ceil() as u32
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
        Arrangement::Narrow => narrow(blocks, narrow_scale(&blocks.radar, NARROW_RADAR_COLUMN)),
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
        height: (key_y + blocks.key.size(1.0).height).max(legend_at.y + legend.height) + MARGIN,
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
        legend: Placement::at(legend_at),
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
    let key_bottom = key_y + blocks.key.size(1.0).height;
    let legend = blocks.legend.size(1.0);
    let legend_y = if legend.is_empty() {
        key_bottom
    } else {
        key_bottom + KEY_TO_LEGEND
    };
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
                x: width - MARGIN,
                y: key_y,
            },
        },
        key: Placement::at(Point {
            x: MARGIN,
            y: key_y,
        }),
        legend: Placement::at(Point {
            x: MARGIN - LEGEND_PANEL_INSET_X,
            y: legend_y,
        }),
    }
}

/// Halvings of [`NARROW_MIN_SCALE`, 1]: past f64 resolution after about 54;
/// the count is fixed so every machine gets the same bits (NFR-4).
const BISECTIONS: u32 = 60;

/// The largest scale in [`NARROW_MIN_SCALE`, 1] at which `radar` is at most
/// `column` wide, or `NARROW_MIN_SCALE` when none is (D-71).
///
/// Only scales that passed the test are returned, so the result fits when
/// any scale does; it is the largest because the radar's size is
/// non-decreasing in the scale. A radar block is at least
/// `MIN_RADAR_WIDTH` (660) wide at 1, wider than any column, so it never
/// takes the first return; only blocks that fit at natural size do.
fn narrow_scale(radar: &impl Block, column: f64) -> f64 {
    let fits = |scale: f64| radar.size(scale).width <= column;
    if fits(1.0) {
        return 1.0;
    }
    if !fits(NARROW_MIN_SCALE) {
        return NARROW_MIN_SCALE;
    }
    let (mut lo, mut hi) = (NARROW_MIN_SCALE, 1.0);
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
    use crate::layout::test_support::{
        HEAD, NAME_THREE_LINES, PRD_EXAMPLE, Rect, assert_finite, assert_inside, bounds, csv_of,
        groups, near, plotted,
    };
    use crate::model::Catalogue;
    use crate::order::Plotted;
    use crate::svg::ItemKey;

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

    /// Thoughtworks' four quadrants: 386.75 wide at 0.3, inside the column.
    fn thoughtworks() -> String {
        quadrants(&["Techniques", "Tools", "Platforms", "Languages & Frameworks"])
    }

    /// 437.8 wide at 0.3: the document widens, within the headroom.
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
        assert!(close(narrow_scale(&block, 400.0), 0.4));
        for column in [300.0, 333.3, 399.999] {
            let scale = narrow_scale(&block, column);
            assert!(block.size(scale).width <= column, "{column}: {scale}");
        }
    }

    #[test]
    fn narrow_scale_boundaries() {
        // Fits at natural size, inclusively: never enlarged.
        assert!(close(narrow_scale(&Linear(400.0), 400.0), 1.0));
        assert!(close(narrow_scale(&Linear(300.0), 400.0), 1.0));
        // Fits exactly at the floor, and misses it by one ulp.
        let wide = Linear(2000.0);
        let at_floor = wide.size(NARROW_MIN_SCALE).width;
        assert!(close(narrow_scale(&wide, at_floor), NARROW_MIN_SCALE));
        let below = f64::from_bits(at_floor.to_bits() - 1);
        assert!(close(narrow_scale(&wide, below), NARROW_MIN_SCALE));
    }

    /// NFR-4: the search runs to the last ulp, the largest double with
    /// 1000 · s ≤ 400, on every machine; a bisection count cut short of
    /// about 54 would stop below it.
    #[test]
    fn narrow_scale_is_bit_stable() {
        let scale = narrow_scale(&Linear(1000.0), 400.0);
        assert_eq!(scale.to_bits(), GOLDEN_LINEAR_BITS, "{scale}");
    }

    /// 0.4 exactly, as recorded on the first run; see `narrow_scale_is_bit_stable`.
    const GOLDEN_LINEAR_BITS: u64 = 0x3FD9_9999_9999_999A;

    #[test]
    fn narrow_scale_of_the_radar() {
        // Zero quadrants: the 660 · s floor sets the width.
        let floor = radar(HEAD);
        let scale = narrow_scale(&floor, NARROW_RADAR_COLUMN);
        assert!(close(scale, 400.0 / 660.0), "{scale}");
        // The PRD example's labels set it: it fills the column, and a hair
        // more scale does not fit (valid off a plateau, as here).
        let prd = radar(PRD_EXAMPLE);
        let scale = narrow_scale(&prd, NARROW_RADAR_COLUMN);
        assert!(close(prd.size(scale).width, 400.0), "{scale}");
        assert!(prd.size(scale + 1e-6).width > 400.0, "{scale}");
    }

    #[test]
    fn wide_places_the_legend_beside_the_radar() {
        let b = blocks(PRD_EXAMPLE, TITLED);
        assert!(close(b.title.size(1.0).height, 53.0));
        assert!(close(b.radar.size(1.0).width, 660.0) && close(b.key.size(1.0).height, 146.0));
        let p = placements(Arrangement::Wide, &b);
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
        // The key is the lower: 723 + 146 + 20.
        assert!(close(p.height, 889.0), "{}", p.height);
        assert!(69.0 + legend.height + 20.0 < 889.0, "{legend:?}");
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
    }

    /// FR-94's premise (D-71): realistic radars keep the 420 width.
    #[test]
    fn ordinary_radars_keep_the_narrow_width() {
        for (name, b) in fixtures() {
            // The fixtures with deliberately long quadrant names widen.
            if matches!(name, "widened" | "beyond" | "wide radar") {
                continue;
            }
            let p = placements(Arrangement::Narrow, &b);
            assert!(close(p.width, NARROW_WIDTH), "{name}: {}", p.width);
        }
    }

    #[test]
    fn narrow_widens_when_the_radar_cannot_fit() {
        let widened = blocks(&widened(), TITLED);
        let p = placements(Arrangement::Narrow, &widened);
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

        for p in [p, q] {
            assert!(near(p.title.origin, Point { x: 20.0, y: 10.0 }), "{p:?}");
            assert!(
                close(p.key.origin.x, 20.0) && close(p.legend.origin.x, 8.0),
                "{p:?}"
            );
            assert!(close(p.rule.to.x, p.width - 20.0), "{p:?}");
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
            assert_eq!(binding, Binding::Ring(ring.clone()), "{csv}");
        }
    }

    #[test]
    fn fr95_is_quiet_within_the_headroom() {
        for csv in [widened(), thoughtworks(), PRD_EXAMPLE.to_owned()] {
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
        assert!(bottom(p.legend, &b.legend) > bottom(p.key, &b.key), "{p:?}");
        assert!(
            close(p.height, 10.0 + 53.0 + 6.0 + legend.height + 20.0),
            "{}",
            p.height
        );
        let key_right = p.key.origin.x + b.key.size(1.0).width;
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
            assert!(close(p.height, bottom(p.key, &b.key) + 20.0), "{p:?}");
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
            for width in [b.title.size(1.0).width, b.key.size(1.0).width] {
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
            assert_eq!(*key, b.key.render(p.key));
            assert_eq!(*legend, b.legend.render(p.legend));
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
