//! Writing rendered output to disk (architecture §3).

pub mod html;
pub mod png;
pub mod write;

pub use write::write_atomic;

/// The generator string every output carries: `major.minor`, never the
/// patch, so a patch release does not change a byte of output (NFR-12,
/// D-52).
pub const OUTPUT_VERSION: &str = concat!(
    "radargen ",
    env!("CARGO_PKG_VERSION_MAJOR"),
    ".",
    env!("CARGO_PKG_VERSION_MINOR")
);

/// D-75: what names an untitled output: the page's tab and hidden heading,
/// the exported SVG's `<title>`.
pub const UNTITLED: &str = "Technology Radar";
