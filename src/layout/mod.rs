//! Geometry and, from PR 09, the blocks drawn from it (architecture §3, §4).

pub mod block;
pub mod geometry;
pub mod key;
pub mod legend;
pub mod marker;
pub mod radar;
pub mod title;

// `pub`, not `pub(crate)`: clippy::redundant_pub_crate flags `pub(crate)`
// on a module in a `pub` parent.
#[cfg(test)]
pub mod test_support;
