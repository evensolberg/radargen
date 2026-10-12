//! The legend (FR-35, FR-46, FR-50, FR-74, FR-75, FR-79, FR-94). Its
//! layout, as data, lives in the `layout` submodule.

mod layout;

pub use layout::{Entry, Layout, Row, fits_inline, layout};
