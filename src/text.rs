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

/// The glyph a face draws for a character it lacks: the missing-glyph box.
const NOTDEF: u32 = 0;

/// Unicode's `Default_Ignorable_Code_Point`: characters a renderer draws as
/// nothing when it cannot draw them, so lacking a glyph for one is not a
/// box. The format characters (Cf), then the rest of
/// `DerivedCoreProperties.txt`'s list, which is mostly fillers, variation
/// selectors and reserved ranges.
fn ignorable(c: char) -> bool {
    use unicode_properties::{GeneralCategory, UnicodeGeneralCategory as _};
    c.general_category() == GeneralCategory::Format
        || matches!(
            c,
            '\u{034f}'
                | '\u{115f}'..='\u{1160}'
                | '\u{17b4}'..='\u{17b5}'
                | '\u{180b}'..='\u{180f}'
                | '\u{2065}'
                | '\u{3164}'
                | '\u{fe00}'..='\u{fe0f}'
                | '\u{ffa0}'
                | '\u{fff0}'..='\u{fff8}'
                | '\u{e0000}'..='\u{e0fff}'
        )
}

/// FR-85 pins exactly two weights; FR-35 separates the legend's levels with
/// them plus size, letter-spacing and indentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
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

    /// The distinct characters of `text` that a PNG or SVG draws as boxes,
    /// in either weight (FR-104). The two faces map the same characters to
    /// the same glyphs (`both_faces_map_the_same_characters`), so Regular
    /// answers for both.
    #[must_use]
    pub fn missing(&self, text: &str) -> Vec<char> {
        self.regular.missing(text)
    }
}

/// One face, ready to shape. Measurement goes through `harfrust`, the shaper
/// `resvg` 0.48 lays text out with, so FR-79's wrap points agree with the
/// rasterised output by construction (FR-84, D-67).
///
/// The exported SVG places and outlines its glyphs from the same face (D-88).
pub struct FontMetrics<'a> {
    font: FontRef<'a>,
    data: ShaperData,
    face: &'static str,
    upem: f64,
}

/// One glyph as the shaper lays it out, in font units (FR-84).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShapedGlyph {
    /// The glyph's id in its face.
    pub id: u32,
    /// The character cluster it belongs to; a letter-spacing gap falls
    /// between clusters, never inside one.
    pub cluster: u32,
    /// How far the pen moves past it.
    pub x_advance: i32,
    /// How far it sits right of the pen.
    pub x_offset: i32,
    /// How far it sits above the baseline.
    pub y_offset: i32,
}

/// Where one glyph is drawn: user units from its line's start, y down.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placed {
    /// The glyph's id in its face.
    pub glyph: u32,
    /// From the line's start.
    pub x: f64,
    /// From the baseline, down.
    pub y: f64,
}

/// One step of a glyph's contours, in font units, y up (D-88).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Segment {
    /// Start a contour.
    MoveTo(f32, f32),
    /// A straight edge.
    LineTo(f32, f32),
    /// A quadratic curve: control point, end.
    QuadTo(f32, f32, f32, f32),
    /// A cubic curve: two control points, end.
    CurveTo(f32, f32, f32, f32, f32, f32),
    /// Close the contour.
    Close,
}

/// Collects skrifa's pen calls, so skrifa stays private to this module.
struct Segments(Vec<Segment>);

impl skrifa::outline::OutlinePen for Segments {
    fn move_to(&mut self, x: f32, y: f32) {
        self.0.push(Segment::MoveTo(x, y));
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.0.push(Segment::LineTo(x, y));
    }

    fn quad_to(&mut self, cx: f32, cy: f32, x: f32, y: f32) {
        self.0.push(Segment::QuadTo(cx, cy, x, y));
    }

    fn curve_to(&mut self, c1x: f32, c1y: f32, c2x: f32, c2y: f32, x: f32, y: f32) {
        self.0.push(Segment::CurveTo(c1x, c1y, c2x, c2y, x, y));
    }

    fn close(&mut self) {
        self.0.push(Segment::Close);
    }
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
        let upem = f64::from(data.shaper(&font).build().units_per_em());
        Ok(Self {
            font,
            data,
            face,
            upem,
        })
    }

    /// `text` shaped as `usvg` shapes it: left to right (A-4 keeps input out
    /// of right-to-left scripts), segment properties guessed, default
    /// options, so the two shape identically.
    #[must_use]
    pub fn shape(&self, text: &str) -> Vec<ShapedGlyph> {
        // A built shaper borrows the font and its data, so it is built per
        // call rather than stored; `usvg` does the same.
        let shaper = self.data.shaper(&self.font).build();
        let mut buffer = UnicodeBuffer::new();
        buffer.push_str(text);
        buffer.set_direction(Direction::LeftToRight);
        buffer.guess_segment_properties();
        let output = shaper.shape(buffer, ShapeOptions::new());
        output
            .glyph_infos()
            .iter()
            .zip(output.glyph_positions())
            .map(|(info, position)| ShapedGlyph {
                id: info.glyph_id,
                cluster: info.cluster,
                x_advance: position.x_advance,
                x_offset: position.x_offset,
                y_offset: position.y_offset,
            })
            .collect()
    }

    /// The distinct characters of `text` this face cannot draw, in order of
    /// first appearance: what a PNG or SVG draws as a missing-glyph box
    /// (FR-104, D-93).
    ///
    /// Shaped, not looked up in the cmap alone, because the shaper composes
    /// and decomposes: a decomposed `e` + U+0302 + U+0301 draws as the
    /// precomposed glyph, and a precomposed letter the subset lacks could
    /// draw as base and mark. Only a cluster holding `.notdef` lacks
    /// anything, and within it the characters the cmap does not map are the
    /// ones named, less the default-ignorable ones, which draw nothing by
    /// design. A cluster whose every character is mapped (the shaper chose
    /// `.notdef` anyway) names its first.
    #[must_use]
    pub fn missing(&self, text: &str) -> Vec<char> {
        use skrifa::MetadataProvider as _;
        let shaped = self.shape(text);
        let charmap = self.font.charmap();
        let at = |cluster: u32| usize::try_from(cluster).unwrap_or(text.len());
        let mut missing = Vec::new();
        for glyph in shaped.iter().filter(|g| g.id == NOTDEF) {
            // Left to right, clusters ascend; the next larger one ends this.
            let end = shaped
                .iter()
                .map(|g| g.cluster)
                .filter(|&c| c > glyph.cluster)
                .min()
                .map_or(text.len(), at);
            let cluster = text.get(at(glyph.cluster)..end).unwrap_or_default();
            let mut lacking: Vec<char> = cluster
                .chars()
                .filter(|&c| charmap.map(c).is_none() && !ignorable(c))
                .collect();
            if lacking.is_empty() {
                lacking.extend(cluster.chars().next());
            }
            for c in lacking {
                if !missing.contains(&c) {
                    missing.push(c);
                }
            }
        }
        missing
    }

    /// The face's units per em.
    #[must_use]
    pub const fn units_per_em(&self) -> f64 {
        self.upem
    }

    /// Advance width of `text` in pixels at `size_px`, with `letter_spacing_px`
    /// applied between clusters, as resvg applies it.
    ///
    /// Shaped, not summed from raw `hmtx` advances: the naive sum ignores
    /// GPOS kerning, which the rasteriser applies (D-42).
    #[must_use]
    pub fn measure(&self, text: &str, size_px: f64, letter_spacing_px: f64) -> f64 {
        let shaped = self.shape(text);
        let units: i32 = shaped.iter().map(|g| g.x_advance).sum();
        // FR-84: one gap between clusters, none after the last, as resvg
        // applies letter-spacing. A combining mark that stays a separate glyph
        // shares its base's cluster and earns no gap of its own.
        let boundaries = shaped
            .windows(2)
            .filter(|pair| matches!(pair, [a, b] if a.cluster != b.cluster))
            .count();
        #[expect(
            clippy::cast_precision_loss,
            clippy::as_conversions,
            reason = "usize has no lossless conversion to f64, and f64 holds every integer up to 2^53 exactly; a cluster count is far below that"
        )]
        let gaps = boundaries as f64;
        #[expect(
            clippy::suboptimal_flops,
            reason = "NFR-4: plain * and + are bit-identical on every target; mul_add falls back to a libm fma where the CPU lacks one"
        )]
        let width = f64::from(units) * size_px / self.upem + gaps * letter_spacing_px;
        width
    }

    /// Where each glyph of `text` is drawn at `size_px`, with
    /// `letter_spacing_px` between clusters as [`Self::measure`] counts it
    /// (D-88).
    #[must_use]
    #[expect(
        clippy::suboptimal_flops,
        reason = "NFR-4: plain * and + are bit-identical on every target; mul_add falls back to a libm fma where the CPU lacks one"
    )]
    pub fn place(&self, text: &str, size_px: f64, letter_spacing_px: f64) -> Vec<Placed> {
        let scale = size_px / self.upem;
        let mut pen = 0.0;
        let mut previous = None;
        self.shape(text)
            .into_iter()
            .map(|glyph| {
                if previous.is_some_and(|cluster| cluster != glyph.cluster) {
                    pen += letter_spacing_px;
                }
                previous = Some(glyph.cluster);
                let placed = Placed {
                    glyph: glyph.id,
                    x: pen + f64::from(glyph.x_offset) * scale,
                    y: -f64::from(glyph.y_offset) * scale,
                };
                pen += f64::from(glyph.x_advance) * scale;
                placed
            })
            .collect()
    }

    /// Glyph `glyph`'s contours in font units, y up, unhinted.
    ///
    /// # Errors
    ///
    /// [`FontError`] when the face has no such glyph or cannot draw it:
    /// the shaper produced an id its own face does not hold (NFR-15).
    pub fn outline(&self, glyph: u32) -> Result<Vec<Segment>, FontError> {
        use skrifa::MetadataProvider as _;
        use skrifa::instance::{LocationRef, Size};
        use skrifa::outline::DrawSettings;
        let fail = |reason: String| FontError {
            face: self.face,
            reason,
        };
        let outlines = self.font.outline_glyphs();
        let found = outlines
            .get(skrifa::GlyphId::new(glyph))
            .ok_or_else(|| fail(format!("glyph {glyph} is not in the face")))?;
        let mut segments = Segments(Vec::new());
        found
            .draw(
                DrawSettings::unhinted(Size::unscaled(), LocationRef::default()),
                &mut segments,
            )
            .map_err(|e| fail(format!("glyph {glyph} cannot be outlined: {e}")))?;
        Ok(segments.0)
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
    wrap_by(text, max_width, &|line| {
        m.measure(line, size_px, letter_spacing_px)
    })
}

/// [`wrap`] against any width function: the layout width, which adds room
/// for a reader's text spacing (D-78), wraps through this.
#[must_use]
pub fn wrap_by(text: &str, max_width: f64, width: &dyn Fn(&str) -> f64) -> Vec<String> {
    let column = Column { max_width, width };
    let mut lines = Vec::new();
    let last = text.split_whitespace().fold(String::new(), |line, word| {
        column.place(word, line, &mut lines)
    });
    if !last.is_empty() {
        lines.push(last);
    }
    lines
}

/// `text` as [`wrap_by`] reads it, on one line: its words, one space
/// apart. A wrapped text's name (D-88), since joining its lines would
/// put a space inside any token broken mid-word.
#[must_use]
pub fn words(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// FR-79's column: a maximum width, and how a line's width is measured.
struct Column<'w> {
    max_width: f64,
    width: &'w dyn Fn(&str) -> f64,
}

impl Column<'_> {
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
        (self.width)(text)
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
    use skrifa::instance::{LocationRef, Size};
    use skrifa::{FontRef, MetadataProvider};

    /// A vendored face's metrics in font units.
    fn metrics(bytes: &[u8]) -> skrifa::metrics::Metrics {
        let face = FontRef::new(bytes).expect("the vendored face parses");
        face.metrics(Size::unscaled(), LocationRef::default())
    }

    #[test]
    fn number_centring_is_half_inters_cap_height() {
        assert!((NUMBER_CENTRING - 1490.0 / 4096.0).abs() < 1e-12);
        let cap = f64::from(
            metrics(REGULAR_TTF)
                .cap_height
                .expect("Inter has a cap height"),
        );
        assert!((NUMBER_CENTRING - cap / 4096.0).abs() < 1e-12);
    }

    #[test]
    fn text_metrics_match_the_vendored_faces() {
        for bytes in [REGULAR_TTF, SEMIBOLD_TTF] {
            let m = metrics(bytes);
            assert_eq!(m.units_per_em, 2048);
            assert!((m.ascent - 1984.0).abs() < f32::EPSILON, "{}", m.ascent);
            assert!((m.descent + 494.0).abs() < f32::EPSILON, "{}", m.descent);
            assert!(m.leading.abs() < f32::EPSILON, "{}", m.leading);
        }
    }

    /// The corpus `measure_keeps_its_bits` pins: both weights, the sizes the
    /// layout uses, spacings binary cannot represent exactly, a no-break
    /// space, a combining mark, and kerned pairs.
    const PINNED: [(Weight, &str, f64, f64); 8] = [
        (Weight::Regular, "Kubernetes", 12.5, 0.0),
        (Weight::SemiBold, "PLATFORMS", 11.0, 0.8),
        (Weight::Regular, "AV Wa. To", 17.0, 0.0),
        (Weight::Regular, "\u{a0}· New", 10.5, 0.2),
        (Weight::SemiBold, "Technology Radar", 17.0, 0.0),
        (Weight::Regular, "Cafe\u{301} Zu\u{308}rich", 12.0, 0.3),
        (Weight::SemiBold, "12", 9.0, 0.0),
        (Weight::Regular, "Infrastructure as Code", 12.5, 0.1),
    ];

    /// The pre-rebuild `measure`'s bits for [`PINNED`], recorded at deb1852.
    const RECORDED: [u64; 8] = [
        4_634_466_971_641_970_688,
        4_634_866_547_911_399_834,
        4_635_472_646_824_263_680,
        4_629_752_025_263_898_624,
        4_639_433_018_988_036_096,
        4_634_562_869_671_755_776,
        4_621_488_714_344_824_832,
        4_638_939_875_997_070_131,
    ];

    /// D-88: rebuilding `measure` on `shape` moves no bit of any width, so
    /// no wrap point and no golden moves.
    #[test]
    fn measure_keeps_its_bits() {
        let fonts = Fonts::embedded().expect("loads");
        let got: Vec<u64> = PINNED
            .iter()
            .map(|&(w, text, size, spacing)| fonts.face(w).measure(text, size, spacing).to_bits())
            .collect();
        assert_eq!(got, RECORDED, "{got:?}");
    }

    /// Placing and measuring share one shaping pass: the pen ends where
    /// `measure` says the line does, to well under NFR-17's precision. Not
    /// bit-equal: `place` adds spacing gap by gap, `measure` all at once.
    #[test]
    #[expect(
        clippy::suboptimal_flops,
        reason = "NFR-4: plain * and + are bit-identical on every target; mul_add falls back to a libm fma where the CPU lacks one"
    )]
    fn place_ends_where_measure_does() {
        let fonts = Fonts::embedded().expect("loads");
        for &(weight, text, size, spacing) in &PINNED {
            let face = fonts.face(weight);
            let shaped = face.shape(text);
            let placed = face.place(text, size, spacing);
            let (last, glyph) = (
                placed.last().expect("glyphs"),
                shaped.last().expect("glyphs"),
            );
            let scale = size / face.units_per_em();
            let end =
                last.x - f64::from(glyph.x_offset) * scale + f64::from(glyph.x_advance) * scale;
            let measured = face.measure(text, size, spacing);
            assert!(
                (end - measured).abs() < 1e-9,
                "{text:?} at {size}: {end} vs {measured}"
            );
        }
    }

    /// D-88, rg-sw0: a glyph the shaper raises is drawn above the baseline.
    /// A capital with two stacked accents is one of the cases the Inter
    /// subset shapes with a vertical offset (the upper accent goes up), while
    /// the other tests' corpora happen to shape flat. A flipped sign in
    /// `place` therefore moves the accent below the letter.
    #[test]
    fn place_raises_a_glyph_the_shaper_raises() {
        let fonts = Fonts::embedded().expect("loads");
        let text = "A\u{300}\u{301}";
        let size = 20.0;
        for weight in [Weight::Regular, Weight::SemiBold] {
            let face = fonts.face(weight);
            let shaped = face.shape(text);
            let placed = face.place(text, size, 0.0);
            assert!(
                shaped.iter().any(|g| g.y_offset > 0),
                "{weight:?}: the subset still raises the upper accent: {shaped:?}"
            );
            assert_eq!(
                placed.len(),
                shaped.len(),
                "{weight:?}: one placement per glyph"
            );
            let scale = size / face.units_per_em();
            for (glyph, at) in shaped.iter().zip(&placed) {
                assert_eq!(at.glyph, glyph.id);
                let want = -f64::from(glyph.y_offset) * scale;
                assert!(
                    (at.y - want).abs() < 1e-9,
                    "{weight:?}: glyph {} at y {} for offset {}",
                    glyph.id,
                    at.y,
                    glyph.y_offset
                );
                if glyph.y_offset > 0 {
                    assert!(
                        at.y < 0.0,
                        "{weight:?}: y grows down, so raised is negative"
                    );
                }
            }
        }
    }

    /// Spacing falls between clusters, not after the last, and never inside
    /// one (FR-84).
    #[test]
    fn spacing_falls_between_clusters() {
        let fonts = Fonts::embedded().expect("loads");
        let face = fonts.face(Weight::Regular);
        let tight = face.place("AB", 12.0, 0.0);
        let loose = face.place("AB", 12.0, 2.0);
        assert_eq!(
            tight.first().map(|g| g.x),
            Some(0.0),
            "the first glyph starts the line"
        );
        let second = |p: &[Placed]| p.get(1).expect("two glyphs").x;
        assert!(
            (second(&loose) - second(&tight) - 2.0).abs() < 1e-12,
            "one gap before B"
        );
        assert!(
            (face.measure("AB", 12.0, 2.0) - face.measure("AB", 12.0, 0.0) - 2.0).abs() < 1e-12,
            "none after B"
        );
        // e + U+0301 is one cluster: when the mark stays its own glyph, it
        // gains no gap of its own.
        if face.shape("e\u{301}x").len() == 3 {
            let marked = face.place("e\u{301}x", 12.0, 2.0);
            let plain = face.place("e\u{301}x", 12.0, 0.0);
            let mark = |p: &[Placed]| p.get(1).expect("the mark").x;
            assert!(
                (mark(&marked) - mark(&plain)).abs() < 1e-12,
                "no gap inside a cluster"
            );
        }
    }

    /// FR-104, D-93: either face answers for both, because they map the
    /// same characters to the same glyphs. If a regenerated subset ever
    /// splits them, this fails and `Fonts::missing` must ask each text's
    /// own face.
    #[test]
    fn both_faces_map_the_same_characters() {
        let map = |bytes| {
            FontRef::new(bytes)
                .expect("the vendored face parses")
                .charmap()
                .mappings()
                .map(|(c, g)| (c, g.to_u32()))
                .collect::<Vec<_>>()
        };
        let regular = map(REGULAR_TTF);
        assert!(!regular.is_empty(), "the face maps characters");
        assert_eq!(regular, map(SEMIBOLD_TTF));
    }

    /// FR-104: a character with no glyph is missing, each counted once, in
    /// the order the text first uses it.
    #[test]
    fn missing_names_each_character_the_face_lacks_once() {
        let fonts = Fonts::embedded().expect("loads");
        for weight in [Weight::Regular, Weight::SemiBold] {
            let face = fonts.face(weight);
            assert_eq!(face.missing("Kafka 中文 中"), ['中', '文'], "{weight:?}");
            assert_eq!(face.missing("عربي"), ['ع', 'ر', 'ب', 'ي'], "{weight:?}");
            assert_eq!(face.missing("Team 🙂"), ['🙂'], "{weight:?}");
            assert!(face.missing("").is_empty(), "{weight:?}");
        }
    }

    /// FR-104: what the shaper draws as a real glyph, composes, or treats
    /// as default-ignorable is not missing. Decomposed Vietnamese composes
    /// to a glyph the subset holds; the joiners and a variation selector
    /// shape to an invisible glyph.
    #[test]
    fn missing_ignores_what_the_face_draws_or_ignores() {
        let fonts = Fonts::embedded().expect("loads");
        let face = fonts.face(Weight::Regular);
        for text in [
            "Kubernetes",
            "Tie\u{302}\u{301}ng Vie\u{323}\u{302}t",
            "Tiếng Việt",
            "Cafe\u{301} Zu\u{308}rich",
            "Łódź Ærøskøbing Şişli",
            "Ελληνικά",
            "Русский язык",
            "a\u{200d}b a\u{200c}b a\u{fe0f}b a\u{ad}b a\u{2060}b",
            "\u{a0}·\u{2009}–—",
        ] {
            let missing = face.missing(text);
            assert!(missing.is_empty(), "{text:?}: {missing:?}");
        }
    }

    /// FR-104: a missing character's own mark or selector is not counted
    /// with it, and a mark the face lacks on a base it has is the one named.
    #[test]
    fn missing_names_the_character_not_its_cluster() {
        let fonts = Fonts::embedded().expect("loads");
        let face = fonts.face(Weight::Regular);
        assert_eq!(face.missing("中\u{fe0f}"), ['中']);
        assert_eq!(face.missing("q\u{1ab0}"), ['\u{1ab0}']);
        assert_eq!(face.missing("\u{1ab0}"), ['\u{1ab0}']);
    }

    /// A shaped glyph has contours; a space has none; an id the face lacks
    /// is an error.
    #[test]
    fn shaped_glyphs_outline() {
        let fonts = Fonts::embedded().expect("loads");
        let face = fonts.face(Weight::SemiBold);
        let [k, space] = face.shape("K ").try_into().expect("two glyphs");
        let ink = face.outline(k.id).expect("draws");
        assert!(matches!(ink.first(), Some(Segment::MoveTo(..))), "{ink:?}");
        assert!(matches!(ink.last(), Some(Segment::Close)), "{ink:?}");
        assert_eq!(face.outline(space.id).expect("draws"), []);
        assert!(face.outline(u32::MAX).is_err(), "no such glyph");
    }
}
