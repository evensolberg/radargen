//! FR-100's column split (D-90): where a static legend's columns break.
//!
//! Breaks fall only between quadrant groups. Of the possible sets, the one
//! whose tallest column is shortest is taken; among equals, the one whose
//! first break is earliest, then its second, and so on (NFR-4).

use std::ops::Range;

/// The first group of each column, for `groups` quadrant groups in at most
/// `columns` columns, where `height` is a column's drawn height when it
/// holds a range of groups.
///
/// With fewer groups than columns there is one column per group; with no
/// groups, no columns. Every column holds at least one group, so the result
/// starts at 0 and ascends strictly.
// qual:allow(complexity, error_handling) reason: "the expect states an invariant: an optimal split always continues"
pub(super) fn split(
    groups: usize,
    columns: usize,
    height: impl Fn(Range<usize>) -> f64,
) -> Vec<usize> {
    let count = columns.min(groups);
    if count == 0 {
        return Vec::new();
    }
    // `best[r][i]`: the shortest tallest column for groups `i..` in `r + 1`
    // columns, each holding at least one group; infinite where too few
    // groups are left.
    let mut best = vec![(0..groups).map(|i| height(i..groups)).collect::<Vec<_>>()];
    for _ in 1..count {
        let fewer = best.last().map_or(&[][..], Vec::as_slice);
        let row = (0..groups)
            .map(|i| {
                (i + 1..groups)
                    .map(|j| height(i..j).max(at(fewer, j)))
                    .fold(f64::INFINITY, f64::min)
            })
            .collect();
        best.push(row);
    }
    let tallest = at(best.last().map_or(&[][..], Vec::as_slice), 0);
    // The earliest break that still allows the rest within `tallest`, at
    // each step: the lexicographically first optimal set (FR-100's tie).
    let mut starts = vec![0];
    let mut i = 0;
    for rest in best.iter().take(count - 1).rev() {
        i = (i + 1..groups)
            .find(|&j| height(i..j) <= tallest && at(rest, j) <= tallest)
            .expect("an optimal split continues from every step");
        starts.push(i);
    }
    starts
}

/// `row`'s value for groups `i..`: infinite past the last group, where no
/// column can start.
fn at(row: &[f64], i: usize) -> f64 {
    row.get(i).copied().unwrap_or(f64::INFINITY)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A column's height as the sum of its groups' `heights`.
    fn summed(heights: &[f64]) -> impl Fn(Range<usize>) -> f64 + '_ {
        |range| heights.get(range).unwrap_or_default().iter().sum()
    }

    fn of(heights: &[f64], columns: usize) -> Vec<usize> {
        split(heights.len(), columns, summed(heights))
    }

    #[test]
    fn the_shortest_tallest_column_is_taken() {
        // [1 2 3 | 4] is 6 tall; [1 2 | 3 4] is 7; greedy halving would err.
        assert_eq!(of(&[1.0, 2.0, 3.0, 4.0], 2), [0, 3]);
        // [1 2 | 3 | 4] is 4 tall; [1 | 2 3 | 4] is 5.
        assert_eq!(of(&[1.0, 2.0, 3.0, 4.0], 3), [0, 2, 3]);
        // One dominant group stands alone.
        assert_eq!(of(&[5.0, 1.0, 1.0, 1.0, 1.0, 1.0], 2), [0, 1]);
    }

    #[test]
    fn among_equals_the_earliest_break_wins() {
        // [3 1 | 1 1 3] and [3 1 1 | 1 3] are both 5 tall.
        assert_eq!(of(&[3.0, 1.0, 1.0, 1.0, 3.0], 2), [0, 2]);
        // Equal groups in three columns of four: the first break decides
        // only after the tallest column; here every split ties at 2.
        assert_eq!(of(&[1.0; 4], 3), [0, 1, 2]);
    }

    #[test]
    fn more_columns_than_groups_gives_one_per_group() {
        assert_eq!(of(&[1.0, 9.0, 1.0], 4), [0, 1, 2]);
    }

    #[test]
    fn one_group_is_one_column() {
        assert_eq!(of(&[7.0], 4), [0]);
        assert_eq!(of(&[7.0, 1.0], 1), [0]);
    }

    #[test]
    fn no_groups_is_no_columns() {
        assert_eq!(of(&[], 3), Vec::<usize>::new());
    }
}
