//! Test-only: whether a piece of work grows linearly with its input.
//!
//! A wall-clock limit measures how busy the machine is as much as how the
//! code scales: a test that takes two seconds idle can take seven under
//! load (rg-the). Comparing two sizes in the same run cancels the load out,
//! since both are slowed alike, and it measures what the tests are for.

use std::hint::black_box;
use std::time::{Duration, Instant};

/// How much larger the second input is than the first.
const FACTOR: usize = 8;

/// The largest time ratio between the two sizes that still passes. Linear
/// work takes about `FACTOR` times as long, quadratic about `FACTOR`²; the
/// bound sits between them, with room for noise either way.
const BOUND: f64 = 24.0;

/// Each size is timed this many times and the fastest run kept, so a
/// preempted run does not decide the result.
const RUNS: usize = 5;

/// Asserts that `work` on `make(n * FACTOR)` takes less than `BOUND` times
/// as long as on `make(n)`. Only `work` is timed. `n` should make the
/// smaller run take some milliseconds, so timer resolution does not
/// dominate it.
///
/// The two sizes' runs are interleaved, small then large, so a burst of
/// load from tests running beside this one slows both sizes' runs rather
/// than all of one size's.
pub fn assert_linear<T>(n: usize, make: impl Fn(usize) -> T, work: impl Fn(&T)) {
    let time = |input: &T| -> Duration {
        let start = Instant::now();
        work(black_box(input));
        start.elapsed()
    };
    let (small_input, large_input) = (make(n), make(n * FACTOR));
    let (mut small, mut large) = (Duration::MAX, Duration::MAX);
    for _ in 0..RUNS {
        small = small.min(time(&small_input));
        large = large.min(time(&large_input));
    }
    let ratio = large.as_secs_f64() / small.as_secs_f64().max(f64::MIN_POSITIVE);
    assert!(
        ratio < BOUND,
        "{FACTOR} times the input took {ratio:.1} times as long ({small:?} to {large:?}); linear is about {FACTOR}"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn linear(n: usize) {
        black_box((0..n).map(black_box).sum::<usize>());
    }

    fn quadratic(n: usize) {
        for i in 0..n {
            black_box((0..n).map(|j| black_box(i ^ j)).sum::<usize>());
        }
    }

    #[test]
    fn linear_work_passes() {
        assert_linear(1 << 16, |n| n, |&n| linear(n));
    }

    #[test]
    #[should_panic(expected = "times the input took")]
    fn quadratic_work_fails() {
        assert_linear(400, |n| n, |&n| quadratic(n));
    }
}
