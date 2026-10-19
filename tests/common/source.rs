//! A `--config` file as a value's source (FR-91), as the tests name one.

use std::sync::Arc;

use radargen::settings::Source;

/// A value from the `--config` file named `file`.
pub fn config(file: &str) -> Source {
    Source::Config(Arc::from(file))
}
