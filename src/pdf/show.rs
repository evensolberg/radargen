//! A text node as PDF text (FR-110). Written in Task 7.

use pdf_writer::Content;

use super::content::Canvas;
use crate::svg::Node;

/// `node`'s lines and tail as PDF text.
#[expect(
    clippy::missing_const_for_fn,
    reason = "an empty stub until Task 7 writes the text, which is not const"
)]
pub(super) fn text(_out: &mut Content, _node: &Node, _canvas: &Canvas<'_>) {}
