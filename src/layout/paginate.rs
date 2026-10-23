//! Greedy pagination (FR-111, FR-112): units placed whole into pieces of
//! bounded size, the caller judging what fits. The legend's pieces and the
//! table's pages are both cut here, so a break means the same in each.

use std::ops::Range;

/// One piece: the units it holds, what its caller opened it with, and
/// whether its one unit is too big for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Piece<P> {
    /// The units placed in it, in order.
    pub units: Range<usize>,
    /// What [`paginate`]'s `open` returned for it.
    pub opening: P,
    /// Its only unit did not fit even an empty piece, so it runs past the
    /// piece's bounds rather than splitting.
    pub overflow: bool,
}

/// `count` units cut into pieces, greedily: each piece takes units while
/// `fits` says they fit, and a unit that does not fit an empty piece goes
/// in alone, marked as overflowing.
///
/// `open(k, start)` is asked once per piece, in order, as piece `k` starts
/// at unit `start`; `fits(&opening, k, start..end)` judges the units
/// `start..end` in it. The caller keeps its own geometry, so each judges a
/// fit by its own exact arithmetic. The pieces are contiguous and cover
/// every unit; none is empty.
pub fn paginate<P>(
    count: usize,
    open: &mut dyn FnMut(usize, usize) -> P,
    fits: &dyn Fn(&P, usize, Range<usize>) -> bool,
) -> Vec<Piece<P>> {
    let mut pieces = Vec::new();
    let mut start = 0;
    while start < count {
        let k = pieces.len();
        let opening = open(k, start);
        let mut end = start + 1;
        let overflow = !fits(&opening, k, start..end);
        if !overflow {
            while end < count && fits(&opening, k, start..end + 1) {
                end += 1;
            }
        }
        pieces.push(Piece {
            units: start..end,
            opening,
            overflow,
        });
        start = end;
    }
    pieces
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Abutting spans of these heights, from 0.
    fn spans(heights: &[f64]) -> Vec<(f64, f64)> {
        let mut top = 0.0;
        heights
            .iter()
            .map(|h| {
                let span = (top, top + h);
                top += h;
                span
            })
            .collect()
    }

    /// `spans` cut with a fixed `room` per piece `k`.
    fn cut(spans: &[(f64, f64)], room: &dyn Fn(usize) -> f64) -> Vec<Piece<f64>> {
        paginate(spans.len(), &mut |k, _| room(k), &|room, _, units| {
            let top = spans.get(units.start).map_or(0.0, |s| s.0);
            let bottom = spans.get(units.end - 1).map_or(0.0, |s| s.1);
            bottom - top <= *room
        })
    }

    /// Each piece's units as (start, end).
    fn ranges(pieces: &[Piece<f64>]) -> Vec<(usize, usize)> {
        pieces
            .iter()
            .map(|p| (p.units.start, p.units.end))
            .collect()
    }

    #[test]
    fn all_fit_in_one() {
        assert_eq!(ranges(&cut(&spans(&[10.0; 3]), &|_| 100.0)), [(0, 3)]);
    }

    #[test]
    fn breaks_before_overflow() {
        let pieces = cut(&spans(&[10.0, 20.0, 30.0]), &|_| 30.0);
        assert_eq!(ranges(&pieces), [(0, 2), (2, 3)]);
        assert!(pieces.iter().all(|p| !p.overflow));
    }

    #[test]
    fn a_tall_unit_stands_alone_and_overflows() {
        let pieces = cut(&spans(&[10.0, 500.0, 10.0]), &|_| 100.0);
        assert_eq!(ranges(&pieces), [(0, 1), (1, 2), (2, 3)]);
        let overflow: Vec<bool> = pieces.iter().map(|p| p.overflow).collect();
        assert_eq!(overflow, [false, true, false]);
    }

    #[test]
    fn open_is_called_once_per_piece_with_its_start() {
        let spans = spans(&[10.0; 4]);
        let mut calls = Vec::new();
        paginate(
            spans.len(),
            &mut |k, start| {
                calls.push((k, start));
                (k, start)
            },
            &|opened, k, units| {
                assert_eq!(*opened, (k, units.start), "the piece's own opening");
                units.len() <= 2
            },
        );
        assert_eq!(calls, [(0, 0), (1, 2)]);
    }

    #[test]
    fn the_opening_comes_back_with_its_piece() {
        let pieces = cut(&spans(&[10.0; 4]), &|k| if k == 0 { 15.0 } else { 100.0 });
        assert_eq!(ranges(&pieces), [(0, 1), (1, 4)]);
        let openings: Vec<f64> = pieces.iter().map(|p| p.opening).collect();
        assert_eq!(openings.len(), 2);
        assert!(
            openings
                .first()
                .is_some_and(|r| r.to_bits() == 15.0_f64.to_bits())
        );
    }

    #[test]
    fn nothing_in_nothing_out() {
        assert_eq!(cut(&[], &|_| 100.0).len(), 0);
    }

    #[test]
    fn gaps_between_spans_count() {
        let spans = [(0.0, 10.0), (40.0, 50.0)];
        assert_eq!(ranges(&cut(&spans, &|_| 45.0)), [(0, 1), (1, 2)]);
        assert_eq!(ranges(&cut(&spans, &|_| 50.0)), [(0, 2)]);
    }

    #[test]
    fn an_exact_fit_fits() {
        assert_eq!(ranges(&cut(&spans(&[10.0, 20.0]), &|_| 30.0)), [(0, 2)]);
    }
}
