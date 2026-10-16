//! Shared by the rasterising tests: a `usvg` that knows both shipped faces.

/// `usvg` options whose font database holds Inter Regular and `SemiBold`,
/// with Inter as the default family: the export's own, so the oracles
/// rasterise as the PNG does.
pub use radargen::emit::png::usvg_options;
