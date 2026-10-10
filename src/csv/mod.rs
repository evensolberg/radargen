//! The CSV stage (architecture §3): bytes to text, then text to records.
//!
//! Nothing here interprets a column, except that `NoHeader` names FR-5's
//! required ones so its message is actionable; meaning is `model.rs` (PR 06).

pub mod decode;
pub mod parse;
