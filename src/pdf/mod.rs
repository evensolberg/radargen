//! The PDF export's writer (FR-110, D-103): the shared document drawn as
//! one vector page, its text real text in an embedded Inter subset.

pub mod content;
pub mod font;
pub mod geometry;
mod show;

/// The one path a number takes into the PDF (NFR-17): rounded to
/// thousandths, half away from zero, as [`fmt_coord`] rounds it, then
/// narrowed to the `f32` pdf-writer writes. The rounding absorbs a trig
/// residue that differs between platforms (NFR-4); the narrowing is
/// exact for the thousandths of any coordinate below 8192 and within a
/// thousandth below 16384, and deterministic beyond.
///
/// # Panics
///
/// On NaN, infinity or a magnitude of 10¹² or more, as [`fmt_coord`]
/// does: each can only come from a layout bug.
///
/// [`fmt_coord`]: crate::num::fmt_coord
#[must_use]
pub fn real(v: f64) -> f32 {
    assert!(
        v.is_finite() && v.abs() < 1e12,
        "pdf::real: {v} is not a drawable coordinate"
    );
    // pdf-writer writes an integral float, -0.0 included, as an integer,
    // so `-0` never reaches the file.
    narrow((v * 1000.0).round() / 1000.0)
}

/// The page's px-to-pt scale (FR-110), narrowed unrounded. It is a
/// quotient of two layout numbers with no trigonometry in it, so it is
/// already the same on every platform; rounding it to thousandths, as
/// [`real`] rounds a coordinate, would mis-size the drawing against the
/// `MediaBox` by up to 0.3% at `--width 320` and leave a sliver of paper at
/// the page's edge.
///
/// # Panics
///
/// On a scale that is not finite and positive: a layout bug.
#[must_use]
pub fn scale(v: f64) -> f32 {
    assert!(
        v.is_finite() && v > 0.0,
        "pdf::scale: {v} is not a page scale"
    );
    narrow(v)
}

/// `f64` to the `f32` pdf-writer writes: the crate's one narrowing.
#[expect(
    clippy::cast_possible_truncation,
    clippy::as_conversions,
    reason = "f32 has no From<f64>; pdf-writer writes f32, and every caller has made the value deterministic first"
)]
const fn narrow(v: f64) -> f32 {
    v as f32
}

#[cfg(test)]
mod tests {
    use super::scale;

    /// The page scale keeps its precision: not rounded to thousandths.
    #[test]
    fn scale_is_narrowed_unrounded() {
        assert_eq!(scale(0.18987), 0.18987_f32);
    }

    /// A zero scale is a layout bug and panics.
    #[test]
    #[should_panic(expected = "pdf::scale")]
    fn scale_rejects_zero() {
        let _ = scale(0.0);
    }
}
