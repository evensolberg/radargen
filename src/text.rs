//! Text measurement and FR-79 wrapping, over FR-85's two faces.
//!
//! Retires the risk that the generator cannot measure rendered text without a
//! browser. `mockup.html` sizes its canvas with `measureCtx.measureText(...)`,
//! a Canvas API with no Rust equivalent; FR-46 and FR-79 need the same numbers
//! from real font metrics, and SC-7 requires the wrap points baked into the
//! HTML to match the ones resvg rasterises.

use harfrust::{Direction, FontRef, ShapeOptions, ShaperData, UnicodeBuffer};

/// FR-85's family name, as `fontdb` registers both vendored faces.
pub const FAMILY: &str = "Inter";

/// Inter Regular, subset to the scripts Inter draws (`scripts/subset-inter.sh`, D-68).
pub const REGULAR_TTF: &[u8] = include_bytes!("../assets/Inter-Regular.ttf");
/// Inter `SemiBold`, same subset, from the same run as [`REGULAR_TTF`].
pub const SEMIBOLD_TTF: &[u8] = include_bytes!("../assets/Inter-SemiBold.ttf");
/// Inter Regular as WOFF2, from the same subset run: what the HTML inlines
/// (NFR-5), so the page draws with the face the layout measured.
pub const REGULAR_WOFF2: &[u8] = include_bytes!("../assets/Inter-Regular.woff2");
/// Inter `SemiBold` as WOFF2, beside [`REGULAR_WOFF2`].
pub const SEMIBOLD_WOFF2: &[u8] = include_bytes!("../assets/Inter-SemiBold.woff2");

/// Inter's ascent as a fraction of the em: the `hhea` ascender, 1984, over
/// 2048 units per em. Both vendored faces agree.
pub const TEXT_ASCENT: f64 = 1984.0 / 2048.0;
/// Inter's descent as a fraction of the em: the negated `hhea` descender,
/// 494, over 2048 units per em. The line gap is 0 in both faces.
pub const TEXT_DESCENT: f64 = 494.0 / 2048.0;
/// How far below a centre a number's baseline sits, per unit of font size.
///
/// The em box's centre (CSS `dominant-baseline: central`), which for Inter is
/// also half the cap height, because 1984 - 494 = 1490 = capHeight, so lining
/// digits sit centred on a marker or badge.
pub const NUMBER_CENTRING: f64 = f64::midpoint(TEXT_ASCENT, -TEXT_DESCENT);

/// FR-85 pins exactly two weights; FR-35 separates the legend's levels with
/// them plus size, letter-spacing and indentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Weight {
    /// Inter Regular, 400.
    Regular,
    /// Inter `SemiBold`, 600.
    SemiBold,
}

impl Weight {
    /// The CSS `font-weight` the emitters write for this face.
    #[must_use]
    pub const fn css(self) -> u16 {
        match self {
            Self::Regular => 400,
            Self::SemiBold => 600,
        }
    }
}

/// A vendored face the shaper cannot read: NFR-15's font load failure.
#[derive(Debug, thiserror::Error)]
#[error("the embedded {face} font cannot be read: {reason}")]
pub struct FontError {
    face: &'static str,
    reason: String,
}

/// Both faces, parsed once at startup and passed down.
///
/// Every `Block::size` measures, and re-parsing per call would put a font
/// parse in the inner loop of layout (architecture §3). The faces are the
/// binary's own bytes, so no lifetime parameter: every `&Fonts` in a layout
/// signature would carry one that could only ever be `'static`.
pub struct Fonts {
    regular: FontMetrics<'static>,
    semibold: FontMetrics<'static>,
}

impl Fonts {
    /// The faces compiled into the binary. A parse failure is NFR-15's
    /// "font load failure" and is fatal to the caller.
    ///
    /// # Errors
    ///
    /// A vendored face that `harfrust` cannot read.
    pub fn embedded() -> Result<Self, FontError> {
        Ok(Self {
            regular: FontMetrics::load(REGULAR_TTF, "Inter Regular")?,
            semibold: FontMetrics::load(SEMIBOLD_TTF, "Inter SemiBold")?,
        })
    }

    /// The metrics for one weight's face.
    #[must_use]
    pub const fn face(&self, w: Weight) -> &FontMetrics<'static> {
        match w {
            Weight::Regular => &self.regular,
            Weight::SemiBold => &self.semibold,
        }
    }
}

/// One face, ready to shape. Measurement goes through `harfrust`, the shaper
/// `resvg` 0.48 lays text out with, so FR-79's wrap points agree with the
/// rasterised output by construction (FR-84, D-67).
pub struct FontMetrics<'a> {
    font: FontRef<'a>,
    data: ShaperData,
}

impl<'a> FontMetrics<'a> {
    /// Parse a font face for shaping; `face` names it in the error
    /// (NFR-15), since the binary embeds more than one.
    ///
    /// # Errors
    ///
    /// [`FontError`] if `data` is not a font `harfrust` can read.
    pub fn load(data: &'a [u8], face: &'static str) -> Result<Self, FontError> {
        let font = FontRef::from_index(data, 0).map_err(|e| FontError {
            face,
            reason: e.to_string(),
        })?;
        let data = ShaperData::new(&font);
        Ok(Self { font, data })
    }

    /// Advance width of `text` in pixels at `size_px`, with `letter_spacing_px`
    /// applied between clusters, as resvg applies it.
    ///
    /// Shaped, not summed from raw `hmtx` advances: the naive sum ignores
    /// GPOS kerning, which the rasteriser applies (D-42). The buffer is set
    /// up as `usvg` sets it up — left to right (A-4 keeps input out of
    /// right-to-left scripts), segment properties guessed, default options —
    /// so the two shape identically.
    #[must_use]
    pub fn measure(&self, text: &str, size_px: f64, letter_spacing_px: f64) -> f64 {
        if text.is_empty() {
            return 0.0;
        }
        // A built shaper borrows the font and its data, so it is built per
        // call rather than stored; `usvg` does the same.
        let shaper = self.data.shaper(&self.font).build();
        let mut buffer = UnicodeBuffer::new();
        buffer.push_str(text);
        buffer.set_direction(Direction::LeftToRight);
        buffer.guess_segment_properties();
        let output = shaper.shape(buffer, ShapeOptions::new());
        let units: i32 = output.glyph_positions().iter().map(|p| p.x_advance).sum();
        // FR-84: one gap between clusters, none after the last, as resvg
        // applies letter-spacing. A combining mark that stays a separate glyph
        // shares its base's cluster and earns no gap of its own.
        let boundaries = output
            .glyph_infos()
            .windows(2)
            .filter(|pair| matches!(pair, [a, b] if a.cluster != b.cluster))
            .count();
        #[expect(
            clippy::cast_precision_loss,
            clippy::as_conversions,
            reason = "usize has no lossless conversion to f64, and f64 holds every integer up to 2^53 exactly; a cluster count is far below that"
        )]
        let gaps = boundaries as f64;
        let upem = f64::from(shaper.units_per_em());
        #[expect(
            clippy::suboptimal_flops,
            reason = "NFR-4: plain * and + are bit-identical on every target; mul_add falls back to a libm fma where the CPU lacks one"
        )]
        let width = f64::from(units) * size_px / upem + gaps * letter_spacing_px;
        width
    }
}

/// FR-79: greedy first-fit wrap against a fixed maximum column width.
///
/// Words are taken in order and placed while they fit; the first that does not
/// starts a new line. A single token wider than the column is broken mid-token
/// at the last character that fits — FR-46 forbids truncating it, and letting
/// it overflow would push it outside the `viewBox`.
#[must_use]
pub fn wrap(
    text: &str,
    max_width: f64,
    m: &FontMetrics,
    size_px: f64,
    letter_spacing_px: f64,
) -> Vec<String> {
    let column = Column {
        max_width,
        metrics: m,
        size_px,
        letter_spacing_px,
    };
    let mut lines = Vec::new();
    let last = text.split_whitespace().fold(String::new(), |line, word| {
        column.place(word, line, &mut lines)
    });
    if !last.is_empty() {
        lines.push(last);
    }
    lines
}

/// FR-79's column: a maximum width, measured in one face at one size.
struct Column<'m, 'f> {
    max_width: f64,
    metrics: &'m FontMetrics<'f>,
    size_px: f64,
    letter_spacing_px: f64,
}

impl Column<'_, '_> {
    /// Whether `text` fits on a line, for greedy placement.
    fn fits(&self, text: &str) -> bool {
        self.width(text) <= self.max_width
    }

    /// Whether `text` is wider than the column, for the mid-token break.
    /// Not `!fits`: the two differ only when a width is NaN, where the
    /// break keeps words whole as the original loop did.
    fn overflows(&self, text: &str) -> bool {
        self.width(text) > self.max_width
    }

    fn width(&self, text: &str) -> f64 {
        self.metrics
            .measure(text, self.size_px, self.letter_spacing_px)
    }

    /// Greedy first-fit: `word` joins `line` if the two fit together;
    /// otherwise `line` is finished and `word` starts the next, broken first
    /// if it is wider than the column. Returns the line now being filled.
    fn place(&self, word: &str, line: String, lines: &mut Vec<String>) -> String {
        let joined = if line.is_empty() {
            word.to_string()
        } else {
            format!("{line} {word}")
        };
        if self.fits(&joined) {
            return joined;
        }
        if !line.is_empty() {
            lines.push(line);
        }
        self.break_token(word, lines).to_string()
    }

    /// FR-46: push the column-wide pieces of a token wider than the column,
    /// in order, and return the tail that fits. Nothing is dropped.
    // qual:allow(complexity, error_handling) reason: "fitting_prefix returns a char boundary, so split_at_checked cannot fail"
    fn break_token<'w>(&self, token: &'w str, lines: &mut Vec<String>) -> &'w str {
        let mut rest = token;
        while self.overflows(rest) {
            let cut = self.fitting_prefix(rest);
            // Only an empty `rest` has no first character, and it overflows
            // only a negative width: stop rather than loop.
            if cut == 0 {
                break;
            }
            let (head, tail) = rest.split_at_checked(cut).expect(
                "fitting_prefix returns a char_indices end or the first char's length, both char boundaries",
            );
            lines.push(head.to_string());
            rest = tail;
        }
        rest
    }

    /// The byte length of `token`'s longest prefix that fits — or of its
    /// first character when not even that fits, so a column narrower than
    /// one glyph still makes progress. Zero only for an empty token.
    fn fitting_prefix(&self, token: &str) -> usize {
        let first = token.chars().next().map_or(0, char::len_utf8);
        token
            .char_indices()
            .map(|(i, c)| i + c.len_utf8())
            .take_while(|&end| {
                token
                    .get(..end)
                    .is_some_and(|prefix| !self.overflows(prefix))
            })
            .last()
            .unwrap_or(first)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ttf_parser::Face;

    #[test]
    fn number_centring_is_half_inters_cap_height() {
        assert!((NUMBER_CENTRING - 1490.0 / 4096.0).abs() < 1e-12);
        let face = Face::parse(REGULAR_TTF, 0).expect("the vendored face parses");
        let cap = f64::from(face.capital_height().expect("Inter has a cap height"));
        assert!((NUMBER_CENTRING - cap / 4096.0).abs() < 1e-12);
    }

    #[test]
    fn text_metrics_match_the_vendored_faces() {
        for bytes in [REGULAR_TTF, SEMIBOLD_TTF] {
            let face = Face::parse(bytes, 0).expect("the vendored face parses");
            assert_eq!(face.units_per_em(), 2048);
            assert_eq!(face.ascender(), 1984);
            assert_eq!(face.descender(), -494);
            assert_eq!(face.line_gap(), 0);
        }
    }
}
