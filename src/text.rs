//! Text measurement and FR-79 wrapping, over FR-85's two faces.
//!
//! Retires the risk that the generator cannot measure rendered text without a
//! browser. `mockup.html` sizes its canvas with `measureCtx.measureText(...)`,
//! a Canvas API with no Rust equivalent; FR-46 and FR-79 need the same numbers
//! from real font metrics, and SC-7 requires the wrap points baked into the
//! HTML to match the ones resvg rasterises.

use harfrust::{BufferClusterLevel, Direction, FontRef, ShapeOptions, ShaperData, UnicodeBuffer};

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

    /// The face's PostScript name, as its `name` table gives it: the PDF's
    /// `BaseFont` after the subset tag (FR-110).
    #[must_use]
    pub const fn postscript(self) -> &'static str {
        match self {
            Self::Regular => "Inter-Regular",
            Self::SemiBold => "Inter-SemiBold",
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

impl FontError {
    /// A failure in `face` (`Inter Regular`), for a writer outside this
    /// module that reads the face's bytes, such as the PDF's subsetter.
    #[must_use]
    pub const fn new(face: &'static str, reason: String) -> Self {
        Self { face, reason }
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

    /// The distinct characters of `text` that a PNG, SVG or PDF draws as boxes,
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
    bytes: &'a [u8],
    font: FontRef<'a>,
    data: ShaperData,
    face: &'static str,
    upem: f64,
}

/// A face's metrics as a PDF font descriptor states them, in font units,
/// y up: the descent is negative (FR-110).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Descriptor {
    /// The face's bounding box: left, bottom, right, top.
    pub bbox: [f64; 4],
    /// Above the baseline.
    pub ascent: f64,
    /// Below the baseline, negative.
    pub descent: f64,
    /// The height of a capital letter.
    pub cap_height: f64,
    /// Zero for an upright face.
    pub italic_angle: f64,
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
        let data_bytes = data;
        let data = ShaperData::new(&font);
        let upem = f64::from(data.shaper(&font).build().units_per_em());
        Ok(Self {
            bytes: data_bytes,
            font,
            data,
            face,
            upem,
        })
    }

    /// The face's TTF, for a writer that embeds it (FR-110).
    #[must_use]
    pub const fn bytes(&self) -> &'a [u8] {
        self.bytes
    }

    /// The face as an error names it: `Inter Regular`.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        self.face
    }

    /// Glyph `glyph`'s advance in font units, from `hmtx`: what a PDF's
    /// widths array holds for it (FR-110).
    ///
    /// # Errors
    ///
    /// [`FontError`] when the face has no such glyph.
    pub fn advance(&self, glyph: u32) -> Result<f64, FontError> {
        use skrifa::MetadataProvider as _;
        use skrifa::instance::{LocationRef, Size};
        self.font
            .glyph_metrics(Size::unscaled(), LocationRef::default())
            .advance_width(skrifa::GlyphId::new(glyph))
            .map(f64::from)
            .ok_or_else(|| FontError::new(self.face, format!("glyph {glyph} is not in the face")))
    }

    /// What a PDF font descriptor states about the face, in font units.
    ///
    /// # Errors
    ///
    /// [`FontError`] when the face has no bounding box (`head`).
    pub fn descriptor(&self) -> Result<Descriptor, FontError> {
        use skrifa::MetadataProvider as _;
        use skrifa::instance::{LocationRef, Size};
        let m = self.font.metrics(Size::unscaled(), LocationRef::default());
        let b = m
            .bounds
            .ok_or_else(|| FontError::new(self.face, "the face has no bounding box".to_owned()))?;
        Ok(Descriptor {
            bbox: [b.x_min, b.y_min, b.x_max, b.y_max].map(f64::from),
            ascent: f64::from(m.ascent),
            descent: f64::from(m.descent),
            cap_height: m.cap_height.map_or_else(|| f64::from(m.ascent), f64::from),
            italic_angle: f64::from(m.italic_angle),
        })
    }

    /// `text` shaped as `usvg` shapes it: left to right (A-4 keeps input out
    /// of right-to-left scripts), segment properties guessed, default
    /// options, so the two shape identically.
    #[must_use]
    pub fn shape(&self, text: &str) -> Vec<ShapedGlyph> {
        self.shape_at(text, BufferClusterLevel::default())
    }

    /// [`Self::shape`] with clusters at `level`: `usvg`'s default for
    /// drawing, or one per character for [`Self::missing`].
    fn shape_at(&self, text: &str, level: BufferClusterLevel) -> Vec<ShapedGlyph> {
        // A built shaper borrows the font and its data, so it is built per
        // call rather than stored; `usvg` does the same.
        let shaper = self.data.shaper(&self.font).build();
        let mut buffer = UnicodeBuffer::new();
        buffer.push_str(text);
        buffer.set_cluster_level(level);
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
    /// first appearance: what a PNG, SVG or PDF draws as a missing-glyph box
    /// (FR-104, D-93).
    ///
    /// Shaped, not looked up in the cmap, because the shaper composes and
    /// decomposes: `e` + U+0302 + U+0301 draws as the precomposed glyph, and
    /// U+2126 OHM SIGN as the omega the subset has. Clusters are one per
    /// character, so each `.notdef` glyph's cluster is the byte offset of
    /// the character it stands for. The shaper hides a default-ignorable
    /// character it cannot draw rather than drawing `.notdef`, so none is
    /// named.
    ///
    /// # Panics
    ///
    /// If a cluster is not the offset of a character in `text`, which the
    /// shaper guarantees.
    #[must_use]
    // qual:allow(complexity, error_handling) reason: "the documented # Panics: a cluster is a character's byte offset"
    pub fn missing(&self, text: &str) -> Vec<char> {
        // Glyph order is not text order: a shaper may move a glyph before
        // the one it follows in the text, so the offsets are sorted.
        let mut offsets: Vec<u32> = self
            .shape_at(text, BufferClusterLevel::Characters)
            .into_iter()
            .filter(|glyph| glyph.id == NOTDEF)
            .map(|glyph| glyph.cluster)
            .collect();
        offsets.sort_unstable();
        let mut seen = std::collections::BTreeSet::new();
        offsets
            .into_iter()
            .map(|cluster| {
                usize::try_from(cluster)
                    .ok()
                    .and_then(|at| text.get(at..))
                    .and_then(|rest| rest.chars().next())
                    .expect("a cluster is the byte offset of a character in text")
            })
            .filter(|&c| seen.insert(c))
            .collect()
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
    ///
    /// rg-wl7: the tail is measured whole only once it is as short as the
    /// piece that would be cut from it, not on every pass: measuring it
    /// whole each time made a long word's break quadratic. Since widths
    /// grow with a prefix's length, a tail fits whole exactly when its
    /// fitting prefix is all of it, so this cuts where the plain
    /// `while self.overflows(rest)` loop did.
    // qual:allow(complexity, error_handling) reason: "fitting_prefix returns a char boundary, so split_at_checked cannot fail"
    fn break_token<'w>(&self, token: &'w str, lines: &mut Vec<String>) -> &'w str {
        let mut rest = token;
        loop {
            let cut = self.fitting_prefix(rest);
            // Only an empty `rest` has no first character: stop rather than
            // loop, whether or not it overflows a negative width.
            if cut == 0 || (cut == rest.len() && !self.overflows(rest)) {
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
    ///
    /// rg-wl7: found by galloping then bisecting over the prefixes' char
    /// ends, so a line costs a logarithmic number of measures of prefixes
    /// at most about twice its length, not one measure of every prefix.
    /// That finds the scan's answer, the last prefix before the first that
    /// overflows, because overflowing is monotone in a prefix's length: a
    /// longer prefix adds advances, which are never negative; the kern a
    /// glyph adds against the one before it is smaller than its own
    /// advance, so adding it never narrows the prefix; a combining
    /// mark adds no advance and no letter-spacing gap, as it shares its
    /// base's cluster; and letter-spacing is never negative.
    fn fitting_prefix(&self, token: &str) -> usize {
        let fits = |end: usize| token.get(..end).is_some_and(|p| !self.overflows(p));
        let mut ends = token.char_indices().map(|(i, c)| i + c.len_utf8());
        // Gallop: take the char ends up to the first of the 1st, 2nd, 4th,
        // … prefixes that overflows, or all of them.
        let mut seen: Vec<usize> = Vec::new();
        let mut probe = 1;
        loop {
            seen.extend(ends.by_ref().take(probe - seen.len()));
            if seen.len() < probe || seen.last().is_some_and(|&end| !fits(end)) {
                break;
            }
            probe *= 2;
        }
        let fitting = seen.partition_point(|&end| fits(end));
        let first = seen.first().copied().unwrap_or(0);
        fitting
            .checked_sub(1)
            .and_then(|last| seen.get(last).copied())
            .unwrap_or(first)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_timing::assert_linear;
    use skrifa::instance::{LocationRef, Size};
    use skrifa::{FontRef, MetadataProvider};

    /// A vendored face's metrics in font units.
    fn metrics(bytes: &[u8]) -> skrifa::metrics::Metrics {
        let face = FontRef::new(bytes).expect("the vendored face parses");
        face.metrics(Size::unscaled(), LocationRef::default())
    }

    /// What a PDF font object reads from a face (FR-110): its bytes, its
    /// PostScript name as the face's name table gives it, a glyph's advance
    /// as the shaper uses it, and the descriptor's metrics.
    #[test]
    fn a_face_describes_itself_for_pdf() {
        let fonts = Fonts::embedded().expect("embedded");
        for (weight, ttf) in [
            (Weight::Regular, REGULAR_TTF),
            (Weight::SemiBold, SEMIBOLD_TTF),
        ] {
            let face = fonts.face(weight);
            assert_eq!(face.bytes(), ttf, "{weight:?}: the vendored TTF");
            let named = skrifa::FontRef::new(ttf)
                .expect("parses")
                .localized_strings(skrifa::string::StringId::POSTSCRIPT_NAME)
                .english_or_first()
                .expect("a PostScript name")
                .to_string();
            assert_eq!(weight.postscript(), named);
            // `A`'s advance is what the shaper advances by for "A" alone.
            let shaped = face.shape("A");
            let [glyph] = shaped.as_slice() else {
                panic!("one glyph: {shaped:?}")
            };
            assert_eq!(
                face.advance(glyph.id).expect("advance"),
                f64::from(glyph.x_advance)
            );
            let d = face.descriptor().expect("descriptor");
            assert_eq!(d.ascent / face.units_per_em(), TEXT_ASCENT);
            assert_eq!(-d.descent / face.units_per_em(), TEXT_DESCENT);
            assert!(
                d.cap_height > 0.0 && d.bbox[2] > d.bbox[0] && d.bbox[3] > d.bbox[1],
                "{d:?}"
            );
            assert_eq!(d.italic_angle, 0.0);
        }
        let missing = fonts.face(Weight::Regular).advance(u32::from(u16::MAX));
        assert!(missing.is_err(), "a glyph the face lacks is a FontError");
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
            // Not in the cmap, but canonically equivalent to letters that
            // are: the shaper decomposes them, a cmap lookup would not.
            "\u{212b}ngstr\u{f6}m",
            "5 \u{2126}",
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
        // A cluster ends where the next begins: the ohm sign after it is
        // drawn, as omega, though the cmap lacks it.
        assert_eq!(face.missing("中\u{2126}"), ['中']);
        // A prepended concatenation mark is a visible format character, not
        // a default-ignorable one, so it is named, alone or with a mark it
        // shares a cluster with.
        assert_eq!(face.missing("\u{600}"), ['\u{600}']);
        assert_eq!(face.missing("\u{600}\u{1ab0}"), ['\u{600}', '\u{1ab0}']);
        assert_eq!(face.missing("\u{6dd}\u{1ab0}"), ['\u{6dd}', '\u{1ab0}']);
    }

    /// FR-104 on a long name is linear: no cluster's end is searched for
    /// from every glyph, and no character is compared with every other.
    #[test]
    fn missing_runs_in_linear_time() {
        let fonts = Fonts::embedded().expect("loads");
        // About 70,000 distinct characters, none in the subset: CJK
        // Extension A, the unified block and Extension B.
        let text: String = ('\u{3400}'..='\u{4dbf}')
            .chain('\u{4e00}'..='\u{9fff}')
            .chain('\u{20000}'..='\u{2a6df}')
            .collect();
        let prefix = |n: usize| text.chars().take(n).collect::<String>();
        assert_linear(1 << 10, prefix, |t| {
            std::hint::black_box(fonts.missing(t));
        });
        assert_eq!(
            fonts.missing(&text).len(),
            text.chars().count(),
            "every one is lacking"
        );
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

    /// The linear scan `Column::fitting_prefix` replaced (rg-wl7), kept as
    /// the reference its bisection must agree with.
    fn reference_fitting_prefix(column: &Column<'_>, token: &str) -> usize {
        let first = token.chars().next().map_or(0, char::len_utf8);
        token
            .char_indices()
            .map(|(i, c)| i + c.len_utf8())
            .take_while(|&end| {
                token
                    .get(..end)
                    .is_some_and(|prefix| !column.overflows(prefix))
            })
            .last()
            .unwrap_or(first)
    }

    /// The `Column::break_token` loop rg-wl7 replaced, over the reference
    /// scan.
    fn reference_break_token<'w>(
        column: &Column<'_>,
        token: &'w str,
        lines: &mut Vec<String>,
    ) -> &'w str {
        let mut rest = token;
        while column.overflows(rest) {
            let cut = reference_fitting_prefix(column, rest);
            if cut == 0 {
                break;
            }
            let (head, tail) = rest.split_at_checked(cut).expect("a char boundary");
            lines.push(head.to_string());
            rest = tail;
        }
        rest
    }

    /// rg-wl7: the bisected break cuts every token where the linear scan
    /// did, across kerned pairs, combining marks, ligature candidates,
    /// characters the subset lacks, both weights, letter-spacing, and
    /// columns from narrower than one glyph to wider than the token. The
    /// break itself measures every tail it cuts.
    #[test]
    fn breaking_a_token_matches_the_linear_scan() {
        let fonts = Fonts::embedded().expect("loads");
        let tokens = [
            "",
            "W",
            "Kubernetes",
            "AVAVAVAWAWAWTyTyTo.Wa.",
            "Cafe\u{301}Zu\u{308}richA\u{300}\u{301}e\u{301}\u{302}\u{303}",
            "->=>!=<=>>--||ffifflfi",
            "Infrastructure-as-Code/Platform_Engineering",
            "中文Kafka🙂عربي\u{a0}·–—",
            "Tie\u{302}\u{301}ngVie\u{323}\u{302}tΕλληνικάРусский",
            "llllllllllllllllllllllllllllll",
        ];
        let mut checked = 0_usize;
        for weight in [Weight::Regular, Weight::SemiBold] {
            let face = fonts.face(weight);
            for spacing in [0.0, 1.5] {
                let width = |t: &str| face.measure(t, 12.5, spacing);
                for max_width in [-1, 0, 1, 2, 3, 5, 8, 12].map(|w| f64::from(w) * 25.0) {
                    let column = Column {
                        max_width,
                        width: &width,
                    };
                    for token in tokens {
                        assert_eq!(
                            column.fitting_prefix(token),
                            reference_fitting_prefix(&column, token),
                            "{weight:?} {spacing} {max_width}: {token:?}"
                        );
                        let (mut new, mut old) = (Vec::new(), Vec::new());
                        let rest = column.break_token(token, &mut new);
                        let reference = reference_break_token(&column, token, &mut old);
                        assert_eq!(
                            (new, rest),
                            (old, reference),
                            "{weight:?} {spacing} {max_width}: {token:?}"
                        );
                        checked += 1;
                    }
                }
            }
        }
        assert_eq!(checked, 2 * 2 * 8 * 10, "every case ran");
    }

    /// rg-wl7: breaking a long word without break points measures a
    /// number of characters linear in its length: neither the remaining
    /// tail nor every prefix of a line is measured once per line. Counted,
    /// not timed, so load cannot skew it. The old scan, kept as the
    /// reference, measures far more, so the bound tells the two apart.
    #[test]
    fn breaking_a_long_word_measures_linearly_many_characters() {
        let n = 1 << 12;
        let word = "W".repeat(n);
        let measured = std::cell::Cell::new(0_usize);
        let width = |t: &str| {
            let chars = t.chars().count();
            measured.set(measured.get() + chars);
            f64::from(u32::try_from(chars).expect("a test word is short"))
        };
        let lines = wrap_by(&word, 12.0, &width);
        assert_eq!(lines.len(), n / 12 + 1, "{lines:?}");
        let new = measured.replace(0);
        let column = Column {
            max_width: 12.0,
            width: &width,
        };
        reference_break_token(&column, &word, &mut Vec::new());
        let old = measured.get();
        assert!(new < 16 * n, "{new} characters measured for {n}");
        assert!(old > 16 * n, "the old scan measured only {old} for {n}");
    }
}
