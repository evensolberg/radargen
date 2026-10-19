//! Shared by the whole-file tests: CSV text to a `Catalogue`.

use radargen::csv::parse::parse;
use radargen::diagnostics::{OffRadar, Warnings};
use radargen::model::{Catalogue, build_catalogue};
use radargen::rings::{default_rings, parse_ring_list};
use radargen::settings::Source;

/// Parse `csv` (asserting no row errors), read the `--rings` value, and
/// build the catalogue. Warnings are dropped: a test that asserts on them
/// builds its own.
pub fn catalogue(csv: &str, rings: Option<&str>) -> Catalogue {
    let mut w = Warnings::default();
    let (table, errs) = parse(csv, &mut w).expect("parses");
    assert!(errs.is_empty(), "{errs:?}");
    let rings = rings
        .map_or_else(|| Ok(default_rings()), |r| parse_ring_list(r, Source::Flag))
        .expect("valid rings");
    build_catalogue(&table, &rings, OffRadar::Listed, &mut w).expect("builds")
}
