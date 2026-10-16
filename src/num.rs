//! NFR-17's number formatting — the only path a float takes to output
//! (architecture §6). A coordinate cannot be emitted unrounded because there
//! is no other formatter.

/// Far larger than any coordinate layout produces, long labels included
/// (reaching it would take some 3 × 10¹⁰ characters of quadrant name,
/// rg-tzz), and small enough that `v * 1000` is an exact integer in `i64`.
const MAX_MAGNITUDE: f64 = 1e12;

/// `v`, already rounded by the caller, as a `u32`: saturating past
/// `u32::MAX`, and 0 for a negative value or NaN.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::as_conversions,
    reason = "f64 has no TryFrom to an integer; `as` saturates at both ends and maps NaN to 0, which every caller handles"
)]
#[must_use]
pub const fn saturating_u32(v: f64) -> u32 {
    v as u32
}

/// Format `v` to three decimal places, half away from zero, `.` separator, no
/// exponent, no locale; trailing zeros and a trailing `.` are dropped, and
/// negative zero prints as `0`.
///
/// Not `format!("{v:.3}")`: that rounds an exact binary tie (0.0625) half to
/// even, and NFR-17 says half away from zero. `f64::round` is half away from
/// zero, so the value is rounded as an integer count of thousandths and the
/// decimal point is placed by hand. It is the product `v * 1000` that is
/// rounded, so `1.0005` (in binary just below a tie) gives `1.001`, as written:
/// deterministic, since IEEE multiplication is correctly rounded everywhere.
///
/// # Panics
///
/// On NaN, infinity or a magnitude of 10¹² or more. None is an input error —
/// each can only come from a layout bug — so it is a panic naming the value
/// rather than a `Result` every emitter would have to thread.
#[must_use]
pub fn fmt_coord(v: f64) -> String {
    assert!(
        v.is_finite() && v.abs() < MAX_MAGNITUDE,
        "fmt_coord: {v} is not a drawable coordinate"
    );
    #[expect(
        clippy::cast_possible_truncation,
        clippy::as_conversions,
        reason = "f64 has no TryFrom to an integer; bounded by the assert above, so the product is an exact integer well inside i64"
    )]
    let thousandths = (v * 1000.0).round() as i64;
    let sign = if thousandths < 0 { "-" } else { "" };
    let abs = thousandths.unsigned_abs();
    let (whole, frac) = (abs / 1000, abs % 1000);
    if frac == 0 {
        format!("{sign}{whole}")
    } else {
        let digits = format!("{frac:03}");
        format!("{sign}{whole}.{}", digits.trim_end_matches('0'))
    }
}

#[cfg(test)]
mod tests {
    use super::fmt_coord;

    #[test]
    fn integers_print_without_a_fraction() {
        assert_eq!(fmt_coord(0.0), "0");
        assert_eq!(fmt_coord(260.0), "260");
        assert_eq!(fmt_coord(-20.0), "-20");
    }

    #[test]
    fn three_decimals_with_trailing_zeros_trimmed() {
        assert_eq!(fmt_coord(1.5), "1.5");
        assert_eq!(fmt_coord(0.1 + 0.2), "0.3");
        assert_eq!(fmt_coord(12_345.678_9), "12345.679");
        // Not -3.14159: clippy's deny-by-default `approx_constant` reads it as π.
        assert_eq!(fmt_coord(-7.654_32), "-7.654");
        // The product is rounded, not the binary value just below the tie.
        assert_eq!(fmt_coord(1.0005), "1.001");
    }

    /// NFR-17: half away from zero. `{:.3}` would give 0.062 here — these
    /// values are exact in binary, so this is a real tie, not float noise.
    #[test]
    fn exact_ties_round_away_from_zero() {
        assert_eq!(fmt_coord(0.0625), "0.063");
        assert_eq!(fmt_coord(-0.0625), "-0.063");
        assert_eq!(fmt_coord(1.0625), "1.063");
        assert_eq!(fmt_coord(0.1875), "0.188");
    }

    #[test]
    fn negative_zero_prints_as_zero() {
        assert_eq!(fmt_coord(-0.0), "0");
        assert_eq!(fmt_coord(-0.0004), "0");
    }

    /// NFR-17's reason for rounding at emission: sin/cos differ across
    /// architectures by ~1e-13, which the third decimal absorbs.
    #[test]
    fn cross_architecture_noise_is_absorbed() {
        // 1e-13, not a longer literal: the f64 spacing near 100 is ~1.4e-14,
        // so anything much smaller parses as exactly 100.0 and tests nothing.
        assert_eq!(fmt_coord(100.0 + 1e-13), "100");
        assert_eq!(fmt_coord(100.0 - 1e-13), "100");
        assert_eq!(fmt_coord(1e-7), "0");
    }

    #[test]
    fn never_an_exponent() {
        assert_eq!(fmt_coord(1e9), "1000000000");
        assert_eq!(fmt_coord(0.001), "0.001");
    }

    #[test]
    #[should_panic(expected = "not a drawable coordinate")]
    fn nan_is_a_bug_not_an_output() {
        let _ = fmt_coord(f64::NAN);
    }

    #[test]
    #[should_panic(expected = "not a drawable coordinate")]
    fn infinity_is_a_bug_not_an_output() {
        let _ = fmt_coord(f64::INFINITY);
    }

    #[test]
    #[should_panic(expected = "not a drawable coordinate")]
    fn absurd_magnitudes_are_a_bug_not_an_output() {
        let _ = fmt_coord(1e12);
    }

    #[test]
    #[should_panic(expected = "not a drawable coordinate")]
    fn negative_absurd_magnitudes_are_a_bug_too() {
        let _ = fmt_coord(-1e12);
    }

    #[test]
    fn the_largest_drawable_magnitude_prints_in_full() {
        assert_eq!(fmt_coord(999_999_999_999.0), "999999999999");
        assert_eq!(fmt_coord(-999_999_999_999.5), "-999999999999.5");
    }
}
