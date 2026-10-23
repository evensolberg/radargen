//! The PDF's fonts (FR-110): per weight, an embedded subset of the
//! vendored Inter TTF holding exactly the glyphs the document draws, as a
//! Type0 font over a `CIDFontType2` with Identity-H encoding, a widths
//! array and a `ToUnicode` map, so the text selects and searches.
//!
//! CIDs are the subset's glyph ids, assigned in ascending original glyph
//! id, so they depend only on which glyphs are drawn (NFR-4).

use std::collections::{BTreeMap, BTreeSet};

use pdf_writer::types::{CidFontType, FontFlags, SystemInfo, UnicodeCmap};
use pdf_writer::{Filter, Name, Pdf, Rect, Ref, Str};
use subsetter::GlyphRemapper;

use super::{EM, real};
use crate::svg::Node;
use crate::text::{Descriptor, FontError, FontMetrics, Fonts, Weight};

/// A subset tag's length: PDF 9.6.4's six capitals.
const TAG_LEN: usize = 6;
/// The most UTF-16 code units one `ToUnicode` entry maps a glyph to.
/// poppler rejects a longer `bfchar` destination ("Illegal entry in
/// bfchar block", checked at 64 units), which loses the glyph's text
/// entirely; a cluster longer than this (`x` and 63 combining acutes)
/// maps its first character instead, so it still copies as its base.
const MAX_UNICODE_UNITS: usize = 62;
/// The font stream's deflate level: the default, fixed so the bytes are.
const LEVEL: u8 = 6;

/// One weight's subset.
struct Subset {
    remapper: GlyphRemapper,
    /// Each CID's text, from the shaper's clusters.
    unicode: BTreeMap<u16, String>,
    /// Each CID's advance in thousandths of an em, by CID.
    widths: Vec<f32>,
}

/// The weights a document draws, each with its subset.
pub struct Faces {
    subsets: BTreeMap<Weight, Subset>,
}

/// The stem width the descriptor states (`/StemV`) for CSS weight `w`:
/// `BASE + PER_WEIGHT × (w − ORIGIN)`, the usual estimate for a face that
/// does not state one. The stem at the origin, in thousandths of an em.
const STEM_V_BASE: f64 = 10.0;
/// The estimate's growth per unit of CSS weight.
const STEM_V_PER_WEIGHT: f64 = 0.244;
/// The CSS weight the estimate starts from.
const STEM_V_ORIGIN: f64 = 50.0;
/// Every `CIDFont`'s character collection: Identity-H's `Adobe-Identity-0`.
const IDENTITY: SystemInfo<'static> = SystemInfo {
    registry: Str(b"Adobe"),
    ordering: Str(b"Identity"),
    supplement: 0,
};

/// A run shaped once: its weight, and each glyph with the text of the
/// cluster it starts (empty for the rest of a cluster).
type Run = (Weight, Vec<(u16, String)>);

impl Faces {
    /// The subsets the static rendering of `roots`, the drawings one PDF's
    /// fonts serve, needs.
    ///
    /// # Errors
    ///
    /// [`FontError`] when a shaped glyph id does not fit 16 bits or has
    /// no advance (NFR-15).
    pub fn of<'a>(
        roots: impl IntoIterator<Item = &'a Node>,
        fonts: &Fonts,
    ) -> Result<Self, FontError> {
        let shaped = shape(roots, fonts)?;
        let mut subsets = subsets(&shaped, fonts)?;
        map_unicode(&shaped, &mut subsets, fonts);
        Ok(Self { subsets })
    }

    /// The CID `glyph` is drawn with in `weight`'s subset.
    ///
    /// # Panics
    ///
    /// If the glyph was not collected: every placed glyph is drawn from a
    /// [`Node::static_runs`](crate::svg::Node::static_runs) run, which
    /// [`Self::of`] collected; a miss is a broken invariant.
    #[must_use]
    pub fn cid(&self, weight: Weight, glyph: u32) -> u16 {
        self.subsets
            .get(&weight)
            .and_then(|s| u16::try_from(glyph).ok().and_then(|g| s.remapper.get(g)))
            .expect("every placed glyph was collected through Node::static_runs")
    }

    /// CID `cid`'s advance in thousandths of an em, as the widths array
    /// writes it, so a viewer's pen and ours agree.
    ///
    /// # Panics
    ///
    /// If `cid` is not in `weight`'s subset (a broken invariant).
    #[must_use]
    pub fn width(&self, weight: Weight, cid: u16) -> f32 {
        *self
            .subsets
            .get(&weight)
            .and_then(|s| s.widths.get(usize::from(cid)))
            .expect("a CID comes from Self::cid")
    }

    /// CID `cid`'s text, as the `ToUnicode` map gives it: what the tests
    /// read the map back with (the writer reads the subset directly).
    #[cfg(test)]
    #[must_use]
    pub fn unicode(&self, weight: Weight, cid: u16) -> Option<&str> {
        self.subsets
            .get(&weight)?
            .unicode
            .get(&cid)
            .map(String::as_str)
    }

    /// The page's resource name for `weight`'s font.
    #[must_use]
    pub const fn resource(weight: Weight) -> Name<'static> {
        match weight {
            Weight::Regular => Name(b"F0"),
            Weight::SemiBold => Name(b"F1"),
        }
    }

    /// Each used weight's Type0 font and the objects under it, numbered
    /// from `next` in a fixed order: Type0, `CIDFont`, descriptor, font
    /// file, `ToUnicode`.
    ///
    /// # Errors
    ///
    /// [`FontError`] when the subsetter or the face's metrics fail.
    pub fn write(
        &self,
        pdf: &mut Pdf,
        next: &mut Ref,
        fonts: &Fonts,
    ) -> Result<Vec<(Weight, Ref)>, FontError> {
        self.subsets
            .iter()
            .map(|(&weight, subset)| {
                let objects = FontObjects::numbered(next);
                subset.write(pdf, &objects, weight, fonts.face(weight))?;
                Ok((weight, objects.type0))
            })
            .collect()
    }
}

impl Subset {
    /// The subset of `face` holding `glyphs`, each with its advance.
    fn of(face: &FontMetrics<'_>, glyphs: BTreeSet<u16>) -> Result<Self, FontError> {
        let sorted: Vec<u16> = glyphs.into_iter().collect();
        let remapper = GlyphRemapper::new_from_glyphs_sorted(&sorted);
        let scale = EM / face.units_per_em();
        let widths = remapper
            .remapped_gids()
            .map(|g| face.advance(u32::from(g)).map(|a| real(a * scale)))
            .collect::<Result<_, _>>()?;
        Ok(Self {
            remapper,
            unicode: BTreeMap::new(),
            widths,
        })
    }

    /// `run`'s entries in the `ToUnicode` map for `pass`, first mapping
    /// wins: a glyph that is its cluster alone; then each glyph of a
    /// cluster of as many glyphs as characters, character by character, so
    /// a mark drawn as its own glyph has its own text (7.21.7); then the
    /// first glyph of any other cluster of several, with all its text; and
    /// last, each glyph still without text takes the character `reverse`
    /// says the face maps to it, or its cluster's last, so no glyph drawn
    /// is without text.
    fn map_run(&mut self, run: &[(u16, String)], (pass, reverse): (Pass, &BTreeMap<u32, char>)) {
        // A cluster is its first glyph, which holds its text, and the
        // glyphs after it, which hold none.
        for cluster in run.chunk_by(|_, (_, chars)| chars.is_empty()) {
            let Some(&(first, ref text)) = cluster.first() else {
                continue;
            };
            let even = cluster.len() == text.chars().count();
            let entries: Vec<(u16, String)> = match (pass, cluster.len()) {
                (Pass::Alone, 1) => vec![(first, text.clone())],
                (Pass::Even, n) if n > 1 && even => cluster
                    .iter()
                    .zip(text.chars())
                    .map(|((glyph, _), c)| (*glyph, c.to_string()))
                    .collect(),
                (Pass::First, n) if n > 1 && !even => vec![(first, text.clone())],
                (Pass::Rest, _) => cluster
                    .iter()
                    .filter_map(|(glyph, _)| {
                        let own = reverse.get(&u32::from(*glyph)).copied();
                        own.or_else(|| text.chars().last())
                            .map(|c| (*glyph, c.to_string()))
                    })
                    .collect(),
                _ => Vec::new(),
            };
            for (glyph, chars) in entries {
                if chars.is_empty() {
                    continue;
                }
                if let Some(cid) = self.remapper.get(glyph).filter(|&cid| cid != 0) {
                    self.unicode.entry(cid).or_insert_with(|| capped(&chars));
                }
            }
        }
    }

    /// This weight's five objects, numbered as `objects` gives them.
    fn write(
        &self,
        pdf: &mut Pdf,
        objects: &FontObjects,
        weight: Weight,
        face: &FontMetrics<'_>,
    ) -> Result<(), FontError> {
        let base = base_font(weight, &self.remapper);
        let base = Name(base.as_bytes());
        type0(pdf, objects, base);
        self.cid_font(pdf, objects, base);
        descriptor(pdf, objects, base, weight, face)?;
        self.font_file(pdf, objects.file, face)?;
        self.cmap(pdf, objects.cmap);
        Ok(())
    }

    /// The `CIDFontType2` and its widths array.
    fn cid_font(&self, pdf: &mut Pdf, objects: &FontObjects, base: Name<'_>) {
        let mut cid = pdf.cid_font(objects.cid_font);
        cid.subtype(CidFontType::Type2)
            .base_font(base)
            .system_info(IDENTITY)
            .font_descriptor(objects.descriptor)
            .cid_to_gid_map_predefined(Name(b"Identity"));
        cid.widths().consecutive(0, self.widths.iter().copied());
    }

    /// The subset TTF, Flate-compressed, with its uncompressed `/Length1`.
    fn font_file(&self, pdf: &mut Pdf, file: Ref, face: &FontMetrics<'_>) -> Result<(), FontError> {
        let bytes = subsetter::subset(face.bytes(), 0, &self.remapper)
            .map_err(|e| FontError::new(face.name(), format!("cannot be subset: {e}")))?;
        let length = i32::try_from(bytes.len())
            .map_err(|_| FontError::new(face.name(), "the subset is over 2 GiB".to_owned()))?;
        let packed = miniz_oxide::deflate::compress_to_vec_zlib(&bytes, LEVEL);
        pdf.stream(file, &packed)
            .filter(Filter::FlateDecode)
            .pair(Name(b"Length1"), length);
        Ok(())
    }

    /// The `ToUnicode` `CMap`, uncompressed so a golden diffs as text.
    fn cmap(&self, pdf: &mut Pdf, id: Ref) {
        let mut map = UnicodeCmap::new(Name(b"Custom"), IDENTITY);
        for (&cid, text) in &self.unicode {
            map.pair_with_multiple(cid, text.chars());
        }
        pdf.cmap(id, &map.finish());
    }
}

/// One font's object numbers, in this order: the Type0 font, its
/// `CIDFont`, the descriptor, the font file and the `ToUnicode` map.
struct FontObjects {
    type0: Ref,
    cid_font: Ref,
    descriptor: Ref,
    file: Ref,
    cmap: Ref,
}

impl FontObjects {
    /// A font's objects numbered from `next`, which moves past them.
    fn numbered(next: &mut Ref) -> Self {
        let [type0, cid_font, descriptor, file, cmap] = std::array::from_fn(|_| next.bump());
        Self {
            type0,
            cid_font,
            descriptor,
            file,
            cmap,
        }
    }
}

/// The descriptor's numbers in PDF glyph space: each a thousandth of an
/// em, but the italic angle.
struct Scaled {
    /// The face's bounding box: left, bottom, right, top; thousandths.
    bbox: [f64; 4],
    /// Degrees counter-clockwise from vertical, unscaled: 0 for Inter.
    italic_angle: f64,
    /// Above the baseline, in thousandths.
    ascent: f64,
    /// Below the baseline, negative, in thousandths.
    descent: f64,
    /// A capital's height, in thousandths.
    cap_height: f64,
    /// The vertical stem's width, in thousandths: an estimate from the CSS
    /// weight, since the face does not state one.
    stem_v: f64,
}

impl Scaled {
    /// `d`, from a face of `units_per_em`, in glyph space, with the stem
    /// width estimated for CSS weight `weight`.
    #[expect(
        clippy::suboptimal_flops,
        reason = "NFR-4: plain * and + are bit-identical on every target; mul_add falls back to a libm fma where the CPU lacks one"
    )]
    fn of(d: &Descriptor, units_per_em: f64, weight: u16) -> Self {
        let scale = EM / units_per_em;
        Self {
            bbox: d.bbox.map(|v| v * scale),
            italic_angle: d.italic_angle,
            ascent: d.ascent * scale,
            descent: d.descent * scale,
            cap_height: d.cap_height * scale,
            stem_v: STEM_V_BASE + STEM_V_PER_WEIGHT * (f64::from(weight) - STEM_V_ORIGIN),
        }
    }
}

/// Each run of `roots` shaped once, in document order.
fn shape<'a>(
    roots: impl IntoIterator<Item = &'a Node>,
    fonts: &Fonts,
) -> Result<Vec<Run>, FontError> {
    roots
        .into_iter()
        .flat_map(Node::static_runs)
        .map(|(style, text)| {
            let face = fonts.face(style.weight);
            let glyphs = clusters(face, text)
                .into_iter()
                .map(|(glyph, chars)| Ok((gid(face, glyph)?, chars)))
                .collect::<Result<Vec<_>, FontError>>()?;
            Ok((style.weight, glyphs))
        })
        .collect()
}

/// One subset per weight `shaped` draws, of the glyphs it draws.
fn subsets(shaped: &[Run], fonts: &Fonts) -> Result<BTreeMap<Weight, Subset>, FontError> {
    used(shaped)
        .into_iter()
        .map(|(weight, glyphs)| Ok((weight, Subset::of(fonts.face(weight), glyphs)?)))
        .collect()
}

/// The glyphs `shaped` draws, by weight.
fn used(shaped: &[Run]) -> BTreeMap<Weight, BTreeSet<u16>> {
    let mut used: BTreeMap<Weight, BTreeSet<u16>> = BTreeMap::new();
    for (weight, run) in shaped {
        used.entry(*weight)
            .or_default()
            .extend(run.iter().map(|&(g, _)| g));
    }
    used
}

/// The `ToUnicode` map, first mapping wins, in document order (D-103).
/// A glyph that is its cluster alone (a letter, a ligature) maps first;
/// the first glyph of a cluster of several (a base drawn with a separate
/// mark) takes the cluster's text only if nothing mapped it alone, so a
/// plain `e` never copies as `e` + U+0301.
fn map_unicode(shaped: &[Run], subsets: &mut BTreeMap<Weight, Subset>, fonts: &Fonts) {
    let reverse: BTreeMap<Weight, BTreeMap<u32, char>> = subsets
        .keys()
        .map(|&weight| (weight, fonts.face(weight).characters()))
        .collect();
    let none = BTreeMap::new();
    for pass in [Pass::Alone, Pass::Even, Pass::First, Pass::Rest] {
        for (weight, run) in shaped {
            if let Some(subset) = subsets.get_mut(weight) {
                let reverse = reverse.get(weight).unwrap_or(&none);
                subset.map_run(run, (pass, reverse));
            }
        }
    }
}

/// `chars` as a `ToUnicode` entry: whole, or its first character when it
/// is longer than [`MAX_UNICODE_UNITS`].
fn capped(chars: &str) -> String {
    if chars.encode_utf16().count() > MAX_UNICODE_UNITS {
        chars.chars().take(1).collect()
    } else {
        chars.to_owned()
    }
}

/// The subset's `BaseFont`: its tag, then the face's PostScript name.
fn base_font(weight: Weight, remapper: &GlyphRemapper) -> String {
    format!(
        "{}+{}",
        tag(weight.postscript(), remapper.remapped_gids()),
        weight.postscript()
    )
}

/// The Type0 font: Identity-H over its `CIDFont`, with its `ToUnicode`.
fn type0(pdf: &mut Pdf, objects: &FontObjects, base: Name<'_>) {
    pdf.type0_font(objects.type0)
        .base_font(base)
        .encoding_predefined(Name(b"Identity-H"))
        .descendant_font(objects.cid_font)
        .to_unicode(objects.cmap);
}

/// The font descriptor: `face`'s metrics in glyph space, and its file.
fn descriptor(
    pdf: &mut Pdf,
    objects: &FontObjects,
    base: Name<'_>,
    weight: Weight,
    face: &FontMetrics<'_>,
) -> Result<(), FontError> {
    let d = Scaled::of(&face.descriptor()?, face.units_per_em(), weight.css());
    let [left, bottom, right, top] = d.bbox.map(real);
    pdf.font_descriptor(objects.descriptor)
        .name(base)
        // SYMBOLIC: an Identity-H CIDFont has no standard Latin
        // encoding, and preflight tools flag NON_SYMBOLIC here.
        .flags(FontFlags::SYMBOLIC)
        .bbox(Rect::new(left, bottom, right, top))
        .italic_angle(real(d.italic_angle))
        .ascent(real(d.ascent))
        .descent(real(d.descent))
        .cap_height(real(d.cap_height))
        .stem_v(real(d.stem_v))
        .font_file2(objects.file);
    Ok(())
}

/// `glyph` as a 16-bit id, which TrueType and the subsetter use.
fn gid(face: &FontMetrics<'_>, glyph: u32) -> Result<u16, FontError> {
    u16::try_from(glyph)
        .map_err(|_| FontError::new(face.name(), format!("glyph {glyph} is past 16 bits")))
}

/// Each glyph `text` shapes to, with the text its cluster stands for: the
/// first glyph of a cluster carries the cluster's characters, the rest of
/// it none, so a ligature maps to its letters and a decomposition to its
/// character once.
///
/// # Panics
///
/// If a cluster is not the byte offset of a character in `text`, which
/// the shaper guarantees.
// qual:allow(complexity, error_handling) reason: "the documented # Panics: a cluster is a character's byte offset"
fn clusters(face: &FontMetrics<'_>, text: &str) -> Vec<(u32, String)> {
    let shaped = face.shape(text);
    let starts: BTreeSet<usize> = shaped
        .iter()
        .filter_map(|g| usize::try_from(g.cluster).ok())
        .collect();
    let mut seen = BTreeSet::new();
    shaped
        .iter()
        .map(|g| {
            let start = usize::try_from(g.cluster)
                .expect("a cluster is the byte offset of a character in text");
            let chars = if seen.insert(start) {
                let end = starts
                    .range(start + 1..)
                    .next()
                    .copied()
                    .unwrap_or(text.len());
                text.get(start..end)
                    .expect("a cluster is the byte offset of a character in text")
                    .to_owned()
            } else {
                String::new()
            };
            (g.id, chars)
        })
        .collect()
}

/// Which clusters a pass of [`map_unicode`] maps, in the passes' order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Pass {
    /// A glyph that is its cluster alone.
    Alone,
    /// A cluster of as many glyphs as characters, glyph by character.
    Even,
    /// Any other cluster of several: its first glyph, with all its text.
    First,
    /// Every glyph still without text: the character the face maps to it,
    /// or its cluster's last.
    Rest,
}

/// PDF 9.6.4's subset tag: [`TAG_LEN`] capitals, here from FNV-1a over
/// the face's name and its glyph ids, so a subset's tag depends only on
/// what it holds (NFR-4) and two different subsets rarely share one.
fn tag(postscript: &str, gids: impl IntoIterator<Item = u16>) -> String {
    const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0100_0000_01b3;
    const LETTERS: &[u8; 26] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ";
    const RADIX: u64 = 26;
    let bytes = postscript
        .bytes()
        .chain(gids.into_iter().flat_map(u16::to_le_bytes));
    let mut hash = bytes.fold(OFFSET, |h, b| (h ^ u64::from(b)).wrapping_mul(PRIME));
    (0..TAG_LEN)
        .filter_map(|_| {
            let letter = usize::try_from(hash % RADIX)
                .ok()
                .and_then(|i| LETTERS.get(i));
            hash /= RADIX;
            letter.copied().map(char::from)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::arrange::{Arrangement, Blocks, arrange};
    use crate::layout::block::fonts;
    use crate::layout::test_support::{PRD_EXAMPLE, plotted};
    use crate::style::TextStyle;
    use crate::svg::{Anchor, Document, Paint, Point, TextPaint};

    fn prd() -> Document {
        let (catalogue, order) = plotted(PRD_EXAMPLE, None);
        let blocks = Blocks::new(&catalogue, &order, Some("Technology Radar"), None, fonts());
        arrange(Arrangement::Wide, &blocks)
    }

    /// One line of `text` in Inter Regular: a drawing that is just it.
    fn run(text: &str) -> Node {
        Node::Text {
            origin: Point::ORIGIN,
            style: TextStyle {
                weight: Weight::Regular,
                size: 12.0,
                letter_spacing: 0.0,
            },
            ink: TextPaint::Fill(Paint::None),
            anchor: Anchor::Start,
            lines: vec![text.to_owned()],
            words: text.to_owned(),
            leading: 14.0,
            tail: None,
        }
    }

    /// Every glyph the document draws has a CID; CIDs follow glyph ids, so
    /// they do not depend on where in the document a glyph first appears;
    /// .notdef is CID 0 (NFR-4).
    #[test]
    fn every_drawn_glyph_has_a_cid_in_glyph_order() {
        let doc = prd();
        let faces = Faces::of([&doc.root], fonts()).expect("subsets");
        for (style, text) in doc.root.static_runs() {
            for glyph in fonts().face(style.weight).shape(text) {
                let _ = faces.cid(style.weight, glyph.id);
            }
        }
        for weight in [Weight::Regular, Weight::SemiBold] {
            let subset = faces
                .subsets
                .get(&weight)
                .expect("the PRD uses both weights");
            assert_eq!(subset.remapper.get(0), Some(0), ".notdef first");
            let gids: Vec<u16> = subset.remapper.remapped_gids().collect();
            let mut sorted = gids.clone();
            sorted.sort_unstable();
            assert_eq!(gids, sorted, "{weight:?}: CIDs ascend with glyph ids");
        }
    }

    /// The `ToUnicode` map gives back the text each glyph was shaped from,
    /// for every run of the PRD document (Review Focus 4).
    #[test]
    fn the_unicode_map_gives_the_text_back() {
        let doc = prd();
        let faces = Faces::of([&doc.root], fonts()).expect("subsets");
        for (style, text) in doc.root.static_runs() {
            let joined: String = fonts()
                .face(style.weight)
                .shape(text)
                .into_iter()
                .filter_map(|g| faces.unicode(style.weight, faces.cid(style.weight, g.id)))
                .collect();
            assert_eq!(joined, text, "{style:?}");
        }
    }

    /// A cluster of several characters drawn as one glyph maps back to all
    /// of them, and a character drawn as several glyphs maps once.
    ///
    /// Inter, probed 2026-10-22 through `FontMetrics::shape` (both weights
    /// alike):
    /// - Ligatures are absent: `ffi`, `fi`, `fl` and `Office` each shape to
    ///   one glyph per character (`f` is glyph 642, `i` 683, `l` 728), so no
    ///   ligature class is pinned.
    /// - Many characters to one glyph occurs: `e` + U+0301 composes to the
    ///   glyph of U+00E9 (613), one glyph for two characters.
    /// - One cluster drawn as several glyphs occurs: `x` + U+0301 has no
    ///   precomposed form and draws as `x` (981) and a mark (1695), both in
    ///   cluster 0.
    /// - One character to several glyphs occurs: U+0344 COMBINING GREEK
    ///   DIALYTIKA TONOS decomposes to two glyphs (2436, 1695), cluster 0.
    #[test]
    fn clusters_round_trip_whatever_their_shape() {
        let face = fonts().face(Weight::Regular);
        let fewer = |t: &str| face.shape(t).len() < t.chars().count();
        let more = |t: &str| face.shape(t).len() > t.chars().count();
        assert!(fewer("e\u{301}"), "precondition: e + U+0301 composes");
        assert!(
            more("\u{344}"),
            "precondition: U+0344 decomposes to several glyphs"
        );
        let x = face.shape("x\u{301}");
        assert!(
            x.len() == 2 && x.iter().all(|g| g.cluster == 0),
            "precondition: x + U+0301 is one cluster of two glyphs"
        );
        let joined = |text: &str| -> String {
            let shaped = face.shape(text);
            let faces = Faces::of([&run(text)], fonts()).expect("subsets");
            shaped
                .iter()
                .filter_map(|g| faces.unicode(Weight::Regular, faces.cid(Weight::Regular, g.id)))
                .collect()
        };
        for text in ["e\u{301}", "x\u{301}"] {
            assert_eq!(joined(text), text);
        }
        // One character drawn as several glyphs: the first glyph says it,
        // and the rest take what the face maps to them, so no glyph is
        // without text (7.21.7) at the cost of a mark said twice.
        assert_eq!(joined("\u{344}"), "\u{344}\u{301}");
    }

    /// D-103: a glyph two texts reach maps to the first text's spelling, in
    /// drawing order. `Café` (U+00E9) and `Cafe` + U+0301: the shaper
    /// composes both to one glyph, which copies as whichever is drawn
    /// first.
    ///
    /// Two runs, not a CSV: the legend sorts its names, so a CSV's row
    /// order is not the drawing's (`Cafe` + U+0301 sorts before `Café`).
    #[test]
    fn a_shared_glyph_maps_to_its_first_text() {
        let face = fonts().face(Weight::Regular);
        let (nfc, nfd) = ("Caf\u{e9}", "Cafe\u{301}");
        assert_eq!(
            face.shape(nfc).iter().map(|g| g.id).collect::<Vec<_>>(),
            face.shape(nfd).iter().map(|g| g.id).collect::<Vec<_>>(),
            "precondition: the shaper draws both spellings with the same glyphs"
        );
        let e = face.shape(nfc).last().expect("a glyph").id;
        let copied = |first: &str, second: &str| {
            let faces = Faces::of([&run(first), &run(second)], fonts()).expect("subsets");
            faces
                .unicode(Weight::Regular, faces.cid(Weight::Regular, e))
                .map(str::to_owned)
        };
        assert_eq!(copied(nfc, nfd).as_deref(), Some("\u{e9}"));
        assert_eq!(copied(nfd, nfc).as_deref(), Some("e\u{301}"));
    }

    /// The two-pass rule on its own: a glyph first drawn inside a cluster
    /// (`x` + U+0301) must not claim the cluster's text for a plain `x`
    /// that comes later; the plain run's single-glyph cluster wins.
    #[test]
    fn a_lone_glyph_maps_to_its_own_text_not_a_cluster() {
        let face = fonts().face(Weight::Regular);
        let combined = face.shape("x\u{301}");
        assert_eq!(combined.len(), 2, "precondition: two glyphs");
        assert_eq!(
            combined.first().map(|g| g.cluster),
            combined.last().map(|g| g.cluster),
            "precondition: in one cluster"
        );
        let x = face.shape("x").first().expect("a glyph").id;
        let faces = Faces::of([&run("x\u{301}"), &run("x")], fonts()).expect("subsets");
        assert_eq!(
            faces
                .unicode(Weight::Regular, faces.cid(Weight::Regular, x))
                .map(str::to_owned)
                .as_deref(),
            Some("x")
        );
    }

    /// 21c-3 review: a cluster of as many glyphs as characters, a base and
    /// a mark the face has no composed glyph for, maps glyph by character,
    /// so the mark has text of its own (ISO 14289-1, 7.21.7).
    #[test]
    fn a_mark_glyph_maps_to_its_own_character() {
        let face = fonts().face(Weight::Regular);
        let combined = face.shape("x\u{301}");
        let [x, acute] = combined.as_slice() else {
            panic!("precondition: two glyphs");
        };
        let faces = Faces::of([&run("x\u{301}")], fonts()).expect("subsets");
        let text = |glyph: u32| {
            faces
                .unicode(Weight::Regular, faces.cid(Weight::Regular, glyph))
                .map(str::to_owned)
        };
        assert_eq!(text(x.id).as_deref(), Some("x"));
        assert_eq!(text(acute.id).as_deref(), Some("\u{301}"));
    }

    /// 21c-3 review: a character the face draws as several glyphs (U+0344
    /// as a diaeresis and an acute) leaves no glyph without text: each
    /// glyph a cluster does not map takes the character the face maps to
    /// it (ISO 14289-1, 7.21.7).
    #[test]
    fn every_glyph_drawn_has_text() {
        let face = fonts().face(Weight::Regular);
        // After a space, U+0344 joins the space's cluster: three glyphs,
        // two characters.
        let text = "Greek \u{344}";
        let shaped = face.shape(text);
        let faces = Faces::of([&run(text)], fonts()).expect("subsets");
        for glyph in &shaped {
            let text = faces.unicode(Weight::Regular, faces.cid(Weight::Regular, glyph.id));
            assert!(
                text.is_some_and(|t| !t.is_empty()),
                "glyph {} has no text",
                glyph.id
            );
        }
    }

    /// An uneven cluster whose text is longer than a `ToUnicode` entry may
    /// be maps its first character only; one at the limit maps whole.
    #[test]
    fn a_long_cluster_maps_its_first_character() {
        let face = fonts().face(Weight::Regular);
        let composed = face.shape("e\u{301}").first().expect("a glyph").id;
        let copied = |text: &str| {
            let faces = Faces::of([&run(text)], fonts()).expect("subsets");
            faces
                .unicode(Weight::Regular, faces.cid(Weight::Regular, composed))
                .map(str::to_owned)
        };
        // `e` and its first acute compose to one glyph, so the cluster has
        // a glyph fewer than characters and its first glyph takes all its
        // text (an even one maps glyph by character).
        assert_eq!(
            face.shape("e\u{301}\u{301}").len(),
            2,
            "precondition: uneven"
        );
        let at_limit = ["e", &"\u{301}".repeat(MAX_UNICODE_UNITS - 1)].concat();
        let past = ["e", &"\u{301}".repeat(MAX_UNICODE_UNITS)].concat();
        assert_eq!(copied(&at_limit).as_deref(), Some(at_limit.as_str()));
        assert_eq!(copied(&past).as_deref(), Some("e"));
    }

    /// A drawing in one weight writes one font, F0 only.
    #[test]
    fn one_weight_writes_one_font() {
        let faces = Faces::of([&run("Adopt")], fonts()).expect("subsets");
        let mut pdf = Pdf::new();
        let mut next = Ref::new(1);
        let refs = faces.write(&mut pdf, &mut next, fonts()).expect("writes");
        assert_eq!(
            refs.iter().map(|(w, _)| *w).collect::<Vec<_>>(),
            [Weight::Regular]
        );
    }

    /// A text the face cannot draw shapes to .notdef, which is in every
    /// subset and maps to nothing (Review Focus 2).
    #[test]
    fn notdef_is_drawn_and_maps_to_nothing() {
        let (catalogue, order) =
            plotted("name,sector,ring,description\n中文,Tools,Adopt,d\n", None);
        let blocks = Blocks::new(&catalogue, &order, None, None, fonts());
        let doc = arrange(Arrangement::Wide, &blocks);
        assert!(
            fonts()
                .face(Weight::Regular)
                .shape("中文")
                .iter()
                .all(|g| g.id == 0),
            "precondition: Inter draws 中文 as .notdef"
        );
        let faces = Faces::of([&doc.root], fonts()).expect("subsets");
        assert_eq!(faces.cid(Weight::Regular, 0), 0);
        assert_eq!(faces.unicode(Weight::Regular, 0), None);
    }

    /// The subset tag is six capitals, fixed by the face and its glyphs.
    #[test]
    fn the_tag_is_six_capitals_from_the_glyphs() {
        let a = tag("Inter-Regular", [0, 3, 9]);
        assert_eq!(a.len(), TAG_LEN);
        assert!(a.bytes().all(|b| b.is_ascii_uppercase()), "{a}");
        assert_eq!(a, tag("Inter-Regular", [0, 3, 9]));
        assert_ne!(a, tag("Inter-Regular", [0, 3, 10]));
        assert_ne!(a, tag("Inter-SemiBold", [0, 3, 9]));
    }

    /// Writing produces one Type0 font per used weight, in Weight order,
    /// and the same bytes twice.
    #[test]
    fn fonts_write_deterministically() {
        let doc = prd();
        let write = || {
            let faces = Faces::of([&doc.root], fonts()).expect("subsets");
            let mut pdf = Pdf::new();
            let mut next = Ref::new(1);
            let refs = faces.write(&mut pdf, &mut next, fonts()).expect("writes");
            (refs, pdf.finish())
        };
        let (refs, first) = write();
        let (_, second) = write();
        assert_eq!(
            refs.iter().map(|(w, _)| *w).collect::<Vec<_>>(),
            [Weight::Regular, Weight::SemiBold]
        );
        assert!(first == second, "NFR-4");
        let text = String::from_utf8_lossy(&first);
        assert!(text.contains("/Identity-H"), "Identity-H encoding");
        assert!(text.contains("+Inter-Regular"), "a tagged BaseFont");
        assert!(text.contains("/FontFile2"), "embedded TrueType");
        assert!(text.contains("beginbfchar"), "a ToUnicode map");
    }

    /// A CID's width is its glyph's advance in thousandths of an em, as
    /// the widths array writes it; `.notdef`'s included.
    #[test]
    fn a_width_is_the_advance_in_thousandths() {
        let face = fonts().face(Weight::Regular);
        let faces = Faces::of([&run("Adopt")], fonts()).expect("subsets");
        for glyph in face.shape("Adopt").into_iter().map(|g| g.id).chain([0]) {
            let want = real(face.advance(glyph).expect("an advance") * EM / face.units_per_em());
            let got = faces.width(Weight::Regular, faces.cid(Weight::Regular, glyph));
            assert_eq!(got.to_bits(), want.to_bits(), "glyph {glyph}");
        }
    }

    /// Each weight has its own page resource name.
    #[test]
    fn each_weight_names_its_resource() {
        assert_eq!(Faces::resource(Weight::Regular), Name(b"F0"));
        assert_eq!(Faces::resource(Weight::SemiBold), Name(b"F1"));
    }
}
