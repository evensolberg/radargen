//! The PDF export's writer (FR-110, D-103): the shared document drawn as
//! one vector page, its text real text in an embedded Inter subset.

pub mod content;
pub mod font;
pub mod geometry;
mod halo;
mod show;

use crate::num::{MAX_MAGNITUDE, round_thousandths};

/// PDF glyph space units per em: a font's widths, and a TJ adjustment,
/// are in thousandths of an em.
pub const EM: f64 = 1000.0;

/// The page in its own units (FR-110): what the `MediaBox`, the
/// `/UserUnit` and the content's flip all read, so they cannot disagree.
#[derive(Debug, Clone, Copy)]
pub struct Sheet {
    /// 1, or the smallest whole number that brings both sides under the
    /// longest side viewers accept.
    pub unit: f64,
    /// The content-to-page transform's scale: points per document unit,
    /// 0.75 × FR-57's width over the document's, divided by the page's
    /// `/UserUnit`. A paper size (PR 21) adds an offset beside it, not a
    /// second transform.
    pub scale: f64,
    /// The `MediaBox`'s width in the page's own units, a whole number: the
    /// drawing's, rounded up; what the ground covers.
    pub width: f64,
    /// The `MediaBox`'s height, likewise: what the flip translates by, so
    /// the drawing hangs from the page's top edge.
    pub height: f64,
}

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
        v.is_finite() && v.abs() < MAX_MAGNITUDE,
        "pdf::real: {v} is not a drawable coordinate"
    );
    // pdf-writer writes an integral float, -0.0 included, as an integer,
    // so `-0` never reaches the file.
    narrow(round_thousandths(v))
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

/// Test helper: a content stream's operators, decoded by lopdf.
#[cfg(test)]
#[must_use]
pub fn decoded(stream: &[u8]) -> Vec<lopdf::content::Operation> {
    lopdf::content::Content::decode(stream)
        .expect("decodes")
        .operations
}

/// Test helper: the operators of a PDF file's first page.
#[cfg(test)]
#[must_use]
pub fn operations(pdf: &[u8]) -> Vec<lopdf::content::Operation> {
    let doc = lopdf::Document::load_mem(pdf).expect("parses");
    let page = *doc.get_pages().values().next().expect("a page");
    decoded(&doc.get_page_content(page))
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
