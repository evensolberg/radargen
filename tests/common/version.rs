//! Goldens hold [`PLACEHOLDER`] where an output names its version, so a
//! minor bump changes none of them. The version itself is checked against
//! [`OUTPUT_VERSION`] where each output writes it (`serialise/page.rs`,
//! `serialise/export.rs`).

use radargen::emit::OUTPUT_VERSION;

/// What a golden holds in place of [`OUTPUT_VERSION`].
pub const PLACEHOLDER: &str = "radargen {version}";

/// `output` as its golden holds it.
pub fn versionless(output: &str) -> String {
    output.replace(OUTPUT_VERSION, PLACEHOLDER)
}
