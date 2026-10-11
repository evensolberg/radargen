//! Every ordering the output depends on (architecture §2, §3): FR-10's
//! quadrant order, FR-34's item order, and FR-18's numbers taken from it.
//!
//! The comparators read the keys `model.rs` computed once and never fold
//! (FR-76, architecture §6). Every one is total, so the sorts are unstable
//! and no order depends on sort stability.

use std::cmp::Ordering;

use crate::model::{Item, Quadrant};
use crate::trim;

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
/// plotted ones; `resolve` sorts only plotted items, so that only keeps
/// this total.
#[must_use]
pub fn compare_items(rank: &[usize], a: &Item, b: &Item) -> Ordering {
    rank[a.quadrant]
        .cmp(&rank[b.quadrant])
        .then_with(|| a.ring.cmp(&b.ring))
        .then_with(|| compare_names(a, b))
        .then_with(|| a.line.cmp(&b.line))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::csv::parse::parse;
    use crate::diagnostics::Warnings;
    use crate::model::{Catalogue, Item, build_catalogue};
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

    /// Each quadrant's FR-10 position. Task 3 replaces this with
    /// `resolve(cat).rank`.
    fn rank_of(cat: &Catalogue) -> Vec<usize> {
        let mut rank = vec![0; cat.quadrants.len()];
        for (position, &q) in quadrant_order(&cat.quadrants).iter().enumerate() {
            rank[q] = position;
        }
        rank
    }

    /// The plotted items' names in FR-34's order. Task 3 replaces the body
    /// with `resolve(cat).items`.
    fn names_in_order(cat: &Catalogue) -> Vec<&str> {
        let rank = rank_of(cat);
        let mut plotted: Vec<&Item> = cat.items.iter().filter(|i| i.ring.is_some()).collect();
        plotted.sort_unstable_by(|a, b| compare_items(&rank, a, b));
        plotted.iter().map(|i| i.name.as_str()).collect()
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
    /// `Zookeeper` ahead of `Airflow`.
    #[test]
    fn names_compare_trimmed_so_a_space_cannot_move_a_name() {
        let cat = catalogue(&csv(&[("Kafka", "T", "Adopt"), (" Kafka", "T", "Adopt")]));
        assert_eq!(names_in_order(&cat), ["Kafka", " Kafka"]);
        let cat = catalogue(&csv(&[
            (" Zookeeper", "T", "Adopt"),
            ("Airflow", "T", "Adopt"),
        ]));
        assert_eq!(names_in_order(&cat), ["Airflow", " Zookeeper"]);
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
}
