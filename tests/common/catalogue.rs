//! Shared by the whole-file tests: CSV text to a `Catalogue`.

use radargen::csv::parse::parse;
use radargen::diagnostics::Warnings;
use radargen::model::{Catalogue, build_catalogue};
use radargen::rings::parse_rings;

/// Parse `csv` (asserting no row errors), read the `--rings` value, and
/// build the catalogue. Warnings are dropped: a test that asserts on them
/// builds its own.
pub fn catalogue(csv: &str, rings: Option<&str>) -> Catalogue {
    let rings = parse_rings(rings).expect("valid rings");
    let mut warnings = Warnings::default();
    let (table, row_errors) = parse(csv, &mut warnings).expect("parses");
    assert!(row_errors.is_empty(), "{row_errors:?}");
    build_catalogue(&table, &rings, &mut warnings).expect("builds")
}
