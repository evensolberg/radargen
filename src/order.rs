//! Every ordering the output depends on (architecture §2, §3): FR-10's
//! quadrant order, FR-34's item order, and FR-18's numbers taken from it.
//!
//! The comparators read the keys `model.rs` computed once and never fold
//! (FR-76, architecture §6). Every one is total, so the sorts are unstable
//! and no order depends on sort stability.

use std::cmp::Ordering;

use crate::model::Quadrant;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::csv::parse::parse;
    use crate::diagnostics::Warnings;
    use crate::model::{Catalogue, build_catalogue};
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
}
