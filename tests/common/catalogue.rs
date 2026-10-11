//! Shared by the whole-file tests: CSV text to a `Catalogue`.

use radargen::csv::parse::parse;
use radargen::diagnostics::Warnings;
use radargen::model::{Catalogue, build_catalogue};
use radargen::rings::parse_rings;

/// Parse `csv` (asserting no row errors), read the `--rings` value, and
/// build the catalogue. Warnings are dropped: a test that asserts on them
/// builds its own.
// qual:allow(dry, duplicate) reason: "src/order.rs unit tests cannot reach tests/common; the same helper by necessity"
pub fn catalogue(csv: &str, rings: Option<&str>) -> Catalogue {
    let mut w = Warnings::default();
    let (table, errs) = parse(csv, &mut w).expect("parses");
    assert!(errs.is_empty(), "{errs:?}");
    let rings = parse_rings(rings).expect("valid rings");
    build_catalogue(&table, &rings, &mut w).expect("builds")
}
