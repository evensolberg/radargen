//! CSV fixtures and the catalogue builder every block's tests share.

use crate::csv::parse::parse;
use crate::diagnostics::Warnings;
use crate::model::{Catalogue, build_catalogue};
use crate::order::{Plotted, resolve};
use crate::rings::parse_rings;

/// The PRD's three-quadrant example (§12), shared by every block's tests.
pub const PRD_EXAMPLE: &str = include_str!("../../../tests/fixtures/three-quadrant.csv");

/// The four required columns.
pub const HEAD: &str = "name,quadrant,ring,description\n";

/// The required columns with FR-20's qualifier and FR-26's status.
pub const HEAD_META: &str = "name,quadrant,ring,description,qualifier,status\n";

/// A name that wraps to two lines in a legend row's 312-unit name column.
pub const NAME_TWO_LINES: &str =
    "Bounded trial of continuous delivery pipelines across managed estates and environments";

/// A name that wraps to three lines in a legend row's name column.
pub const NAME_THREE_LINES: &str = "Bounded trial of continuous delivery pipelines across managed estates and environments with a shared policy layer";

/// A one-line name too wide for "Default \u{b7} New" to ride after it, so a
/// legend row's meta takes its own line.
pub const NAME_FULL_LINE: &str = "Bounded trial of continuous delivery pipelines";

/// One item per quadrant name, all in `ring`.
pub fn csv_of(quadrants: &[&str], ring: &str) -> String {
    quadrants
        .iter()
        .enumerate()
        .fold(HEAD.to_owned(), |text, (i, q)| {
            format!("{text}item {i},{q},{ring},d\n")
        })
}

/// A catalogue and its FR-10 order from CSV text and an optional `--rings`
/// value.
pub fn plotted(csv: &str, rings: Option<&str>) -> (Catalogue, Plotted) {
    let mut warnings = Warnings::default();
    let (table, errors) = parse(csv, &mut warnings).expect("parses");
    assert!(errors.is_empty(), "{errors:?}");
    let rings = parse_rings(rings).expect("valid rings");
    let catalogue = build_catalogue(&table, &rings, &mut warnings).expect("builds");
    let order = resolve(&catalogue);
    (catalogue, order)
}

/// A catalogue with no columns, quadrants, rings or items (FR-42).
pub const fn empty_catalogue() -> Catalogue {
    Catalogue {
        columns: Vec::new(),
        quadrants: Vec::new(),
        rings: Vec::new(),
        items: Vec::new(),
    }
}

// Redundant for the compiler (the module is already test-only), but it is
// how rustqual tells this module's own tests from the helpers it checks.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plotted_builds_the_catalogue_and_its_order() {
        let csv = "name,quadrant,ring,description\nA,Tools,Adopt,d\nB,Tools,Trial,d\n";
        let (catalogue, order) = plotted(csv, None);
        assert_eq!(catalogue.items.len(), 2);
        assert_eq!(order.items.len(), 2);
        assert_eq!(order.quadrants, [0]);
    }
}
