//! Text measurement and FR-79 wrapping, over FR-85's two faces.
//!
//! Retires the risk that the generator cannot measure rendered text without a
//! browser. `mockup.html` sizes its canvas with `measureCtx.measureText(...)`,
//! a Canvas API with no Rust equivalent; FR-46 and FR-79 need the same numbers
//! from real font metrics, and SC-7 requires the wrap points baked into the
//! HTML to match the ones resvg rasterises.

use rustybuzz::{Face as ShaperFace, UnicodeBuffer};

/// FR-85's family name, as `fontdb` registers both vendored faces.
pub const FAMILY: &str = "Inter";

/// Inter Regular, Latin and Latin Extended subset (`scripts/subset-inter.sh`).
pub const REGULAR_TTF: &[u8] = include_bytes!("../assets/Inter-Regular.ttf");
/// Inter `SemiBold`, same subset, from the same run as [`REGULAR_TTF`].
pub const SEMIBOLD_TTF: &[u8] = include_bytes!("../assets/Inter-SemiBold.ttf");

/// FR-85 pins exactly two weights; FR-35 separates the legend's levels with
/// them plus size, letter-spacing and indentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Weight {
    Regular,
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
    /// A vendored face that `ttf-parser` cannot parse.
    pub fn embedded() -> Result<Self, ttf_parser::FaceParsingError> {
        Ok(Self {
            regular: FontMetrics::load(REGULAR_TTF)?,
            semibold: FontMetrics::load(SEMIBOLD_TTF)?,
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

pub struct FontMetrics<'a> {
    face: ShaperFace<'a>,
}

impl<'a> FontMetrics<'a> {
    /// Parse a font face for shaping.
    ///
    /// # Errors
    ///
    /// [`ttf_parser::FaceParsingError::MalformedFont`] if `data` is not a
    /// font `rustybuzz` can read.
    pub fn load(data: &'a [u8]) -> Result<Self, ttf_parser::FaceParsingError> {
        // Shape rather than sum raw `hmtx` advances: the naive sum ignores
        // GPOS kerning, which the rasteriser's shaper applies, and FR-79's
        // wrap points must agree with the rasterised output (D-42). Since
        // resvg 0.48 that shaper is harfrust, not rustybuzz, so agreement now
        // rests on `tests/agrees_with_resvg.rs` alone; do not loosen its
        // tolerance. Crumb rg-mbv moves this measurement to harfrust.
        let face =
            ShaperFace::from_slice(data, 0).ok_or(ttf_parser::FaceParsingError::MalformedFont)?;
        Ok(Self { face })
    }

    /// Advance width of `text` in pixels at `size_px`, with `letter_spacing_px`
    /// applied between glyphs.
    #[must_use]
    pub fn measure(&self, text: &str, size_px: f64, letter_spacing_px: f64) -> f64 {
        if text.is_empty() {
            return 0.0;
        }
        let mut buf = UnicodeBuffer::new();
        buf.push_str(text);
        let shaped = rustybuzz::shape(&self.face, &[], buf);
        let units: i32 = shaped.glyph_positions().iter().map(|p| p.x_advance).sum();
        #[allow(
            clippy::cast_precision_loss,
            reason = "f64 holds every integer up to 2^53 exactly, and a glyph count is far below that"
        )]
        let gaps = shaped.len().saturating_sub(1) as f64;
        let upem = f64::from(self.face.units_per_em());
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
    fn break_token<'w>(&self, token: &'w str, lines: &mut Vec<String>) -> &'w str {
        let mut rest = token;
        while self.overflows(rest) {
            let cut = self.fitting_prefix(rest);
            // Only an empty `rest` has no first character, and it overflows
            // only a negative width: stop rather than loop.
            if cut == 0 {
                break;
            }
            lines.push(rest[..cut].to_string());
            rest = &rest[cut..];
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
            .take_while(|&end| !self.overflows(&token[..end]))
            .last()
            .unwrap_or(first)
    }
}
