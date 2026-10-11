//! Every ordering the output depends on (architecture §2, §3): FR-10's
//! quadrant order, FR-34's item order, and FR-18's numbers taken from it.
//!
//! The comparators read the keys `model.rs` computed once and never fold
//! (FR-76, architecture §6). Every one is total, so the sorts are unstable
//! and no order depends on sort stability.

// qual:allow(coupling, sdp) reason: "the pipeline runs csv → model → order → layout (architecture §2); depending on the previous stage is the design"

use std::cmp::Ordering;
use std::ops::Range;

use crate::model::{Catalogue, Item, Quadrant};
use crate::trim;

/// The `resolve` stage's output (architecture §2): every order the
/// output depends on, as indices into the [`Catalogue`], which is never
/// reordered.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Plotted {
    /// FR-10's order, as indices into [`Catalogue::quadrants`]. Layout,
    /// the legend and the palette read this list (OQ-17).
    pub quadrants: Vec<usize>,
    /// `rank[q]` is quadrant `q`'s position in [`Plotted::quadrants`]:
    /// FR-67's palette index, never [`Item::quadrant`] (architecture §5).
    pub rank: Vec<usize>,
    /// The plotted items in FR-34's order, as indices into
    /// [`Catalogue::items`]. The item at position `i` is number `i + 1`.
    pub items: Vec<usize>,
    /// FR-18: each item's number, aligned with [`Catalogue::items`];
    /// `None` for an unplotted item. FR-78's number column and FR-50's
    /// accessible names read it.
    pub numbers: Vec<Option<usize>>,
}

impl Plotted {
    /// FR-78's row order: plotted items in FR-34's order, then unplotted
    /// ones in CSV order, as indices into [`Catalogue::items`].
    #[must_use]
    pub fn table_rows(&self) -> Vec<usize> {
        let unplotted = self
            .numbers
            .iter()
            .enumerate()
            .filter(|(_, number)| number.is_none())
            .map(|(i, _)| i);
        self.items.iter().copied().chain(unplotted).collect()
    }

    /// The bands that hold a plotted item, in FR-34's order: FR-10's
    /// quadrant order, then declared ring order. FR-33 numbers `j` within
    /// each, and FR-89 checks each. Every item in [`Plotted::items`] has a
    /// ring ([`resolve`] plots only those), so the runs cover it exactly
    /// and `layout::geometry::place` stays aligned with it.
    #[must_use]
    pub fn runs(&self, catalogue: &Catalogue) -> Vec<Run> {
        let band = |i: usize| (catalogue.items[i].quadrant, catalogue.items[i].ring);
        let mut start = 0;
        self.items
            .chunk_by(|&a, &b| band(a) == band(b))
            .filter_map(|chunk| {
                let items = start..start + chunk.len();
                start = items.end;
                let (quadrant, ring) = band(chunk[0]);
                ring.map(|ring| Run {
                    quadrant,
                    ring,
                    items,
                })
            })
            .collect()
    }
}

/// One quadrant-and-ring band's plotted items: a run of
/// [`Plotted::items`], which FR-34 orders by quadrant and then ring.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Run {
    /// Index into [`Catalogue::quadrants`].
    pub quadrant: usize,
    /// Index into [`Catalogue::rings`], the declared order.
    pub ring: usize,
    /// The run's positions in [`Plotted::items`].
    pub items: Range<usize>,
}

/// FR-10, FR-18, FR-34: order the quadrants and the plotted items, and
/// number the items from 1.
///
/// # Panics
///
/// If an item's `quadrant` is out of range for the catalogue's
/// quadrants, as for an item from another catalogue.
#[must_use]
pub fn resolve(catalogue: &Catalogue) -> Plotted {
    let quadrants = quadrant_order(&catalogue.quadrants);
    let rank = ranks(&quadrants);
    let items = item_order(catalogue, &rank);
    let numbers = numbers(&items, catalogue.items.len());
    Plotted {
        quadrants,
        rank,
        items,
        numbers,
    }
}

/// FR-10: case-insensitive alphabetical by FR-76's key, in code point
/// order (D-7, D-53).
///
/// FR-9 groups quadrants by key, so keys are unique in a built
/// `Catalogue` and the label never decides; it breaks a tie only in a
/// hand-built list.
#[must_use]
pub fn compare_quadrants(a: &Quadrant, b: &Quadrant) -> Ordering {
    a.key.cmp(&b.key).then_with(|| a.label.cmp(&b.label))
}

/// FR-10's order, as indices into `quadrants`.
///
/// This list is the one place the order is computed, so an explicit
/// `--quadrants` later (OQ-17) replaces this function, not its readers.
/// The index ends the comparison, so the order is total even for exact
/// duplicates.
#[must_use]
pub fn quadrant_order(quadrants: &[Quadrant]) -> Vec<usize> {
    let mut order: Vec<usize> = (0..quadrants.len()).collect();
    order
        .sort_unstable_by(|&a, &b| compare_quadrants(&quadrants[a], &quadrants[b]).then(a.cmp(&b)));
    order
}

/// FR-34's name steps: FR-76's key, then the trimmed name ordinally, so
/// names differing only in case do not fall through to row order, and a
/// stray space cannot move a name (D-63).
///
/// D-53, and OQ-9 (b)'s home (architecture §8): both steps compare by
/// Unicode code point, which is `str::cmp`, UTF-8 byte order. That
/// differs from UTF-16 code-unit order, the prototype's `<`, for a
/// character above U+FFFF against one in U+E000–U+FFFF.
#[must_use]
pub fn compare_names(a: &Item, b: &Item) -> Ordering {
    a.name_key
        .cmp(&b.name_key)
        .then_with(|| trim(&a.name).cmp(trim(&b.name)))
}

/// FR-34: the quadrant's FR-10 position (`rank`, from [`quadrant_order`]),
/// then the ring in declared order, which is the legend's (FR-35, D-33),
/// then [`compare_names`], then CSV line, which no two rows share.
///
/// An unplotted item (`ring` is `None`) sorts before its quadrant's
/// plotted ones; [`resolve`] sorts only plotted items, so that only keeps
/// this total.
///
/// # Panics
///
/// If either item's `quadrant` is out of range for `rank`, as for an
/// item from another catalogue.
#[must_use]
pub fn compare_items(rank: &[usize], a: &Item, b: &Item) -> Ordering {
    rank[a.quadrant]
        .cmp(&rank[b.quadrant])
        .then_with(|| a.ring.cmp(&b.ring))
        .then_with(|| compare_names(a, b))
        .then_with(|| a.line.cmp(&b.line))
}

/// The inverse of `order`: `rank[q]` is quadrant `q`'s position in it,
/// which is FR-67's palette index (architecture §5).
fn ranks(order: &[usize]) -> Vec<usize> {
    let mut rank = vec![0; order.len()];
    for (position, &q) in order.iter().enumerate() {
        rank[q] = position;
    }
    rank
}

/// The plotted items in FR-34's order, as indices into
/// `catalogue.items`. Unplotted items get no number (FR-18), so they are
/// not here.
///
/// The index ends the comparison, so the order is total even for
/// hand-built items equal on every key including `line`.
fn item_order(catalogue: &Catalogue, rank: &[usize]) -> Vec<usize> {
    let mut order: Vec<usize> = catalogue
        .items
        .iter()
        .enumerate()
        .filter(|(_, item)| item.ring.is_some())
        .map(|(i, _)| i)
        .collect();
    order.sort_unstable_by(|&a, &b| {
        compare_items(rank, &catalogue.items[a], &catalogue.items[b]).then(a.cmp(&b))
    });
    order
}

/// FR-18: number `order`'s items from 1, in a list of `len` aligned with
/// `Catalogue::items`; every other item is `None`.
fn numbers(order: &[usize], len: usize) -> Vec<Option<usize>> {
    let mut numbers = vec![None; len];
    for (position, &i) in order.iter().enumerate() {
        numbers[i] = Some(position + 1);
    }
    numbers
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::csv::parse::parse;
    use crate::diagnostics::Warnings;
    use crate::model::build_catalogue;
    use crate::rings::parse_rings;

    /// A `name,quadrant,ring,description` CSV, one row per
    /// `(name, quadrant, ring)`. Data rows start on line 2.
    fn csv(rows: &[(&str, &str, &str)]) -> String {
        use std::fmt::Write as _;
        let mut text = String::from("name,quadrant,ring,description\n");
        for (name, quadrant, ring) in rows {
            writeln!(text, "{name},{quadrant},{ring},d").expect("writes to a String");
        }
        text
    }

    fn catalogue_with(csv: &str, rings: Option<&str>) -> Catalogue {
        let mut w = Warnings::default();
        let (table, errs) = parse(csv, &mut w).expect("parses");
        assert!(errs.is_empty(), "{errs:?}");
        let rings = parse_rings(rings).expect("valid rings");
        build_catalogue(&table, &rings, &mut w).expect("builds")
    }

    /// Under the default rings: Assess, Trial, Adopt, Retire.
    fn catalogue(csv: &str) -> Catalogue {
        catalogue_with(csv, None)
    }

    fn labels<'a>(cat: &'a Catalogue, order: &[usize]) -> Vec<&'a str> {
        order
            .iter()
            .map(|&q| cat.quadrants[q].label.as_str())
            .collect()
    }

    /// FR-10: case-insensitive, so `beta` sits between `Alpha` and `Gamma`
    /// although byte order would put it last.
    #[test]
    fn quadrants_order_case_insensitively() {
        let cat = catalogue(&csv(&[
            ("A", "beta", "Adopt"),
            ("B", "Gamma", "Adopt"),
            ("C", "Alpha", "Adopt"),
        ]));
        let order = quadrant_order(&cat.quadrants);
        assert_eq!(labels(&cat, &order), ["Alpha", "beta", "Gamma"]);
        assert_eq!(order, [2, 0, 1]);
    }

    /// D-7: the order comes from the names alone, not from row order.
    #[test]
    fn quadrant_order_does_not_depend_on_row_order() {
        let a = catalogue(&csv(&[
            ("A", "Tool", "Adopt"),
            ("B", "Platform", "Adopt"),
            ("C", "Practice", "Adopt"),
        ]));
        let b = catalogue(&csv(&[
            ("C", "Practice", "Adopt"),
            ("A", "Tool", "Adopt"),
            ("B", "Platform", "Adopt"),
        ]));
        let expected = ["Platform", "Practice", "Tool"];
        assert_eq!(labels(&a, &quadrant_order(&a.quadrants)), expected);
        assert_eq!(labels(&b, &quadrant_order(&b.quadrants)), expected);
    }

    /// D-53: code point order, so `é` (U+00E9) sorts after `z` (U+007A).
    /// No locale puts `Écosse` before `Zulu` here.
    #[test]
    fn quadrants_order_by_code_point_above_ascii() {
        let cat = catalogue(&csv(&[("A", "Écosse", "Adopt"), ("B", "Zulu", "Adopt")]));
        assert_eq!(
            labels(&cat, &quadrant_order(&cat.quadrants)),
            ["Zulu", "Écosse"]
        );
    }

    /// FR-42: a quadrant whose rows are all unplotted keeps its place, so
    /// its legend group renders empty rather than vanishing.
    #[test]
    fn a_quadrant_of_unplotted_rows_is_still_ordered() {
        let cat = catalogue(&csv(&[
            ("A", "Tool", "Adopt"),
            ("B", "Platform", "Rejected"),
        ]));
        assert_eq!(
            labels(&cat, &quadrant_order(&cat.quadrants)),
            ["Platform", "Tool"]
        );
    }

    /// Architecture §6: total even on a hand-built list with a repeated
    /// key, which FR-9's grouping never produces. The label breaks the
    /// key tie, then the index breaks an exact duplicate.
    #[test]
    fn the_quadrant_order_is_total() {
        let q = |key: &str, label: &str| Quadrant {
            key: key.to_string(),
            label: label.to_string(),
        };
        let quadrants = [
            q("tool", "Tool"),
            q("tool", "TOOL"),
            q("alpha", "Alpha"),
            q("tool", "Tool"),
        ];
        assert_eq!(quadrant_order(&quadrants), [2, 1, 0, 3]);
        assert_eq!(
            compare_quadrants(&quadrants[0], &quadrants[3]),
            Ordering::Equal
        );
        assert_eq!(
            compare_quadrants(&quadrants[1], &quadrants[0]),
            Ordering::Less
        );
    }

    fn rank_of(cat: &Catalogue) -> Vec<usize> {
        resolve(cat).rank
    }

    fn names_in_order(cat: &Catalogue) -> Vec<&str> {
        resolve(cat)
            .items
            .iter()
            .map(|&i| cat.items[i].name.as_str())
            .collect()
    }

    /// FR-34, FR-35, D-33: quadrant (FR-10), then ring in the declared
    /// order — here not alphabetical — then name. An unplotted row has no
    /// number (FR-18), so it is not in the order. Three quadrants in
    /// first-appearance order Tool, Delivery, Platform give FR-10 order
    /// `[1, 2, 0]` and rank `[2, 0, 1]`, which differ, so passing the
    /// order where the rank belongs would fail here.
    #[test]
    fn items_order_by_quadrant_then_declared_ring_then_name() {
        let cat = catalogue_with(
            &csv(&[
                ("Zed", "Tool", "Hold"),
                ("Ops", "Delivery", "Adopt"),
                ("Beta", "Tool", "Adopt"),
                ("Alpha", "Tool", "Adopt"),
                ("Kube", "Platform", "Trial"),
                ("Old", "Platform", "Rejected"),
            ]),
            Some("Hold,Assess,Trial,Adopt"),
        );
        assert_eq!(
            names_in_order(&cat),
            ["Ops", "Kube", "Zed", "Alpha", "Beta"]
        );
    }

    /// FR-34's case-insensitive step decides before the ordinal one: real
    /// radars hold `gRPC`, `npm` and `k6`, and code point order alone
    /// would put every lowercase-initial name after `Z`.
    #[test]
    fn names_order_case_insensitively_before_ordinally() {
        let cat = catalogue(&csv(&[
            ("beta", "T", "Adopt"),
            ("Gamma", "T", "Adopt"),
            ("Alpha", "T", "Adopt"),
        ]));
        assert_eq!(names_in_order(&cat), ["Alpha", "beta", "Gamma"]);
    }

    /// FR-34: case-insensitively first, then ordinally, so names differing
    /// only in case do not fall through to row order.
    #[test]
    fn names_order_case_insensitively_then_by_case() {
        let cat = catalogue(&csv(&[
            ("kafka", "T", "Adopt"),
            ("Kafka", "T", "Adopt"),
            ("Airflow", "T", "Adopt"),
        ]));
        assert_eq!(names_in_order(&cat), ["Airflow", "Kafka", "kafka"]);
    }

    /// D-63: the ordinal step compares trimmed names. ` Kafka` and `Kafka`
    /// are then equal and fall to CSV line, where a raw comparison would
    /// put the spaced one first; and a leading space cannot move
    /// `Zookeeper` ahead of `Airflow`. The second case is decided by the
    /// name key, and the third only by the trimmed ordinal step.
    #[test]
    fn names_compare_trimmed_so_a_space_cannot_move_a_name() {
        let cat = catalogue(&csv(&[("Kafka", "T", "Adopt"), (" Kafka", "T", "Adopt")]));
        assert_eq!(names_in_order(&cat), ["Kafka", " Kafka"]);
        let cat = catalogue(&csv(&[
            (" Zookeeper", "T", "Adopt"),
            ("Airflow", "T", "Adopt"),
        ]));
        assert_eq!(names_in_order(&cat), ["Airflow", " Zookeeper"]);
        let cat = catalogue(&csv(&[(" kafka", "T", "Adopt"), ("Kafka", "T", "Adopt")]));
        assert_eq!(names_in_order(&cat), ["Kafka", " kafka"]);
    }

    /// D-53: code point order, which is UTF-8 byte order. U+FF21 (FULLWIDTH
    /// A) sorts before U+1F600; UTF-16 code-unit order, the prototype's,
    /// would invert them, since U+1F600 starts with the surrogate U+D83D.
    #[test]
    fn names_order_by_code_point_not_utf16() {
        let cat = catalogue(&csv(&[
            ("\u{1F600}", "T", "Adopt"),
            ("\u{FF21}", "T", "Adopt"),
        ]));
        assert_eq!(names_in_order(&cat), ["\u{FF21}", "\u{1F600}"]);
    }

    /// Architecture §6: FR-34's chain is total. Over items that collide on
    /// every key but the line — the same name in three spellings, across
    /// quadrants and rings, one unplotted — no two distinct items compare
    /// equal, and the order is antisymmetric and transitive.
    #[test]
    fn the_item_comparator_is_total() {
        let cat = catalogue(&csv(&[
            ("Kafka", "Tool", "Adopt"),
            (" Kafka", "Tool", "Adopt"),
            ("kafka", "Tool", "Adopt"),
            ("Kafka", "Tool", "Adopt"),
            ("Kafka", "tool", "Trial"),
            ("Kafka", "Platform", "Adopt"),
            ("Kafka", "Platform", "Rejected"),
            ("Airflow", "Platform", "Adopt"),
        ]));
        let rank = rank_of(&cat);
        let cmp = |a: usize, b: usize| compare_items(&rank, &cat.items[a], &cat.items[b]);
        let n = cat.items.len();
        for a in 0..n {
            for b in 0..n {
                assert_eq!(cmp(a, b) == Ordering::Equal, a == b, "{a} vs {b}");
                assert_eq!(cmp(a, b), cmp(b, a).reverse(), "{a} vs {b}");
                for c in 0..n {
                    if cmp(a, b).is_lt() && cmp(b, c).is_lt() {
                        assert!(cmp(a, c).is_lt(), "{a} < {b} < {c}");
                    }
                }
            }
        }
    }

    /// FR-18: numbers run 1, 2, … over plotted items only, in FR-34's
    /// order, with no gap where an unplotted row sits.
    #[test]
    fn numbers_are_contiguous_over_plotted_items() {
        let cat = catalogue(&csv(&[
            ("B", "Tool", "Adopt"),
            ("Old", "Tool", "Rejected"),
            ("A", "Tool", "Adopt"),
        ]));
        let plotted = resolve(&cat);
        assert_eq!(plotted.items, [2, 0]);
        assert_eq!(plotted.numbers, [Some(2), None, Some(1)]);
    }

    /// FR-18, FR-42: with nothing plotted there are no numbers, and every
    /// quadrant is still ordered for its empty legend group.
    #[test]
    fn every_row_unplotted_numbers_nothing() {
        let cat = catalogue(&csv(&[
            ("A", "Tool", "Gone"),
            ("B", "Platform", "Rejected"),
        ]));
        let plotted = resolve(&cat);
        assert_eq!(plotted.items, Vec::<usize>::new());
        assert_eq!(plotted.numbers, [None, None]);
        assert_eq!(labels(&cat, &plotted.quadrants), ["Platform", "Tool"]);
    }

    /// FR-78: plotted rows first in FR-34's order, then unplotted rows in
    /// CSV order, not interleaved.
    #[test]
    fn table_rows_put_plotted_first_then_unplotted_in_csv_order() {
        let cat = catalogue(&csv(&[
            ("Old2", "Tool", "Gone"),
            ("B", "Tool", "Adopt"),
            ("Old1", "Tool", "Rejected"),
            ("A", "Tool", "Adopt"),
        ]));
        assert_eq!(resolve(&cat).table_rows(), [3, 1, 0, 2]);
    }

    /// The rank inverts FR-10's order: `rank[q]` is quadrant `q`'s place,
    /// FR-67's palette index (architecture §5).
    #[test]
    fn rank_inverts_the_quadrant_order() {
        let cat = catalogue(&csv(&[
            ("A", "beta", "Adopt"),
            ("B", "Gamma", "Adopt"),
            ("C", "Alpha", "Adopt"),
        ]));
        let plotted = resolve(&cat);
        assert_eq!(plotted.quadrants, [2, 0, 1]);
        assert_eq!(plotted.rank, [1, 2, 0]);
    }

    /// An empty catalogue resolves to nothing, without panicking.
    #[test]
    fn an_empty_catalogue_resolves_to_nothing() {
        let plotted = resolve(&catalogue("name,quadrant,ring,description\n"));
        assert_eq!(plotted, Plotted::default());
    }

    /// The bands are runs of FR-34's order: one per quadrant and ring that
    /// holds a plotted item, in FR-10 then declared-ring order.
    #[test]
    fn runs_group_plotted_items_by_quadrant_and_ring() {
        let cat = catalogue(&csv(&[
            ("A", "Tool", "Adopt"),
            ("B", "Platform", "Trial"),
            ("C", "Tool", "Adopt"),
            ("D", "Tool", "Assess"),
            ("Old", "Tool", "Rejected"),
        ]));
        let plotted = resolve(&cat);
        let runs: Vec<_> = plotted
            .runs(&cat)
            .iter()
            .map(|r| {
                (
                    cat.quadrants[r.quadrant].label.as_str(),
                    cat.rings[r.ring].as_str(),
                    r.items.clone(),
                )
            })
            .collect();
        assert_eq!(
            runs,
            [
                ("Platform", "Trial", 0..1),
                ("Tool", "Assess", 1..2),
                ("Tool", "Adopt", 2..4)
            ]
        );
    }
}
