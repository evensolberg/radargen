//! Test-only helpers every block's tests share: CSV fixtures and the
//! catalogue builder in `fixtures`, the bounds walker and its assertions in
//! `bounds`.

mod bounds;
mod fixtures;

pub(crate) use bounds::{Rect, assert_finite, assert_inside, bounds, fill, groups, near, plus};
pub(crate) use fixtures::{
    HEAD, HEAD_META, NAME_FULL_LINE, NAME_THREE_LINES, NAME_TWO_LINES, PRD_EXAMPLE, csv_of,
    empty_catalogue, long, plotted,
};
