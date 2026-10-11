//! Shared by the rasterising tests: a `usvg` that knows both shipped faces.

use radargen::text::{FAMILY, REGULAR_TTF, SEMIBOLD_TTF};
use std::sync::Arc;

/// `usvg` options whose font database holds Inter Regular and `SemiBold`,
/// with Inter as the default family.
pub fn usvg_options() -> usvg::Options<'static> {
    let mut db = fontdb::Database::new();
    db.load_font_data(REGULAR_TTF.to_vec());
    db.load_font_data(SEMIBOLD_TTF.to_vec());
    usvg::Options {
        fontdb: Arc::new(db),
        font_family: FAMILY.to_string(),
        ..usvg::Options::default()
    }
}
