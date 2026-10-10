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
        // rustybuzz is the shaper resvg itself uses. Measuring through it rather
        // than summing raw `hmtx` advances is what makes FR-79's wrap points
        // agree with the rasterised output by construction instead of by luck:
        // the naive sum ignores GPOS kerning, which the shaper applies (D-42).
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
pub fn wrap(
    text: &str,
    max_width: f64,
    m: &FontMetrics,
    size_px: f64,
    letter_spacing_px: f64,
) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    let mut line = String::new();

    for word in text.split_whitespace() {
        let candidate = if line.is_empty() {
            word.to_string()
        } else {
            format!("{line} {word}")
        };
        if m.measure(&candidate, size_px, letter_spacing_px) <= max_width {
            line = candidate;
            continue;
        }
        if !line.is_empty() {
            lines.push(std::mem::take(&mut line));
        }
        // The word now starts a line of its own. If it still does not fit,
        // break it at the last character that does.
        let mut rest = word;
        #[expect(
            clippy::while_float,
            reason = "the comparison is the point; each pass removes at least one character from `rest` or breaks, so it terminates"
        )]
        while m.measure(rest, size_px, letter_spacing_px) > max_width {
            let mut cut = 0;
            let mut fitted = String::new();
            for (idx, ch) in rest.char_indices() {
                let mut trial = fitted.clone();
                trial.push(ch);
                if m.measure(&trial, size_px, letter_spacing_px) > max_width {
                    break;
                }
                fitted = trial;
                cut = idx + ch.len_utf8();
            }
            if cut == 0 {
                // Column narrower than one glyph: emit that glyph rather than loop.
                cut = rest.chars().next().map_or(0, char::len_utf8);
                if cut == 0 {
                    break;
                }
            }
            lines.push(rest[..cut].to_string());
            rest = &rest[cut..];
        }
        line = rest.to_string();
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}
