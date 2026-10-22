//! The PDF's fonts (FR-110): per weight, an embedded subset of the
//! vendored Inter TTF holding exactly the glyphs the document draws, as a
//! Type0 font over a `CIDFontType2` with Identity-H encoding, a widths
//! array and a `ToUnicode` map, so the text selects and searches.
//!
//! CIDs are the subset's glyph ids, assigned in ascending original glyph
//! id, so they depend only on which glyphs are drawn (NFR-4).

use std::collections::{BTreeMap, BTreeSet};

use pdf_writer::types::{CidFontType, FontFlags, SystemInfo, UnicodeCmap};
use pdf_writer::{Filter, Finish, Name, Pdf, Rect, Ref, Str};
use subsetter::GlyphRemapper;

use super::real;
use crate::svg::Node;
use crate::text::{FontError, FontMetrics, Fonts, Weight};

/// The width of a PDF glyph space em.
const EM: f64 = 1000.0;
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

impl Faces {
    /// The subsets `document`'s static rendering needs.
    ///
    /// # Errors
    ///
    /// [`FontError`] when a shaped glyph id does not fit 16 bits or has
    /// no advance (NFR-15).
    pub fn of<'a>(
        roots: impl IntoIterator<Item = &'a Node>,
        fonts: &Fonts,
    ) -> Result<Self, FontError> {
        // Each run shaped once, in document order: its weight, and each
        // glyph with the text of the cluster it starts (empty for the rest
        // of a cluster).
        let mut shaped: Vec<(Weight, Vec<(u16, String)>)> = Vec::new();
        for (style, text) in roots.into_iter().flat_map(Node::static_runs) {
            let face = fonts.face(style.weight);
            let glyphs = clusters(face, text)
                .into_iter()
                .map(|(glyph, chars)| Ok((gid(face, glyph)?, chars)))
                .collect::<Result<Vec<_>, FontError>>()?;
            shaped.push((style.weight, glyphs));
        }
        let mut used: BTreeMap<Weight, BTreeSet<u16>> = BTreeMap::new();
        for (weight, run) in &shaped {
            used.entry(*weight)
                .or_default()
                .extend(run.iter().map(|&(g, _)| g));
        }
        let mut subsets = BTreeMap::new();
        for (weight, set) in used {
            let face = fonts.face(weight);
            let sorted: Vec<u16> = set.into_iter().collect();
            let remapper = GlyphRemapper::new_from_glyphs_sorted(&sorted);
            let scale = EM / face.units_per_em();
            let widths = remapper
                .remapped_gids()
                .map(|g| face.advance(u32::from(g)).map(|a| real(a * scale)))
                .collect::<Result<_, _>>()?;
            subsets.insert(
                weight,
                Subset {
                    remapper,
                    unicode: BTreeMap::new(),
                    widths,
                },
            );
        }
        // The ToUnicode map, first mapping wins, in document order (D-103).
        // A glyph that is its cluster alone (a letter, a ligature) maps
        // first; the first glyph of a cluster of several (a base drawn
        // with a separate mark) takes the cluster's text only if nothing
        // mapped it alone, so a plain `e` never copies as `e` + U+0301.
        for alone in [true, false] {
            for (weight, run) in &shaped {
                let Some(subset) = subsets.get_mut(weight) else {
                    continue;
                };
                for (i, (glyph, chars)) in run.iter().enumerate() {
                    let single = run.get(i + 1).is_none_or(|(_, next)| !next.is_empty());
                    if single != alone || chars.is_empty() {
                        continue;
                    }
                    if let Some(cid) = subset.remapper.get(*glyph)
                        && cid != 0
                    {
                        subset.unicode.entry(cid).or_insert_with(|| chars.clone());
                    }
                }
            }
        }
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
    // qual:allow(complexity, error_handling) reason: "the documented # Panics: every placed glyph was collected"
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
    // qual:allow(complexity, error_handling) reason: "the documented # Panics: a CID comes from Self::cid"
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
    #[expect(
        clippy::suboptimal_flops,
        reason = "NFR-4: plain * and + are bit-identical on every target; mul_add falls back to a libm fma where the CPU lacks one"
    )]
    pub fn write(
        &self,
        pdf: &mut Pdf,
        next: &mut Ref,
        fonts: &Fonts,
    ) -> Result<Vec<(Weight, Ref)>, FontError> {
        let mut out = Vec::new();
        for (&weight, subset) in &self.subsets {
            let face = fonts.face(weight);
            let [type0, cid_font, descriptor, file, cmap] = [(); 5].map(|()| next.bump());
            let gids: Vec<u16> = subset.remapper.remapped_gids().collect();
            let base = format!(
                "{}+{}",
                tag(weight.postscript(), &gids),
                weight.postscript()
            );
            let base = Name(base.as_bytes());
            let info = SystemInfo {
                registry: Str(b"Adobe"),
                ordering: Str(b"Identity"),
                supplement: 0,
            };

            pdf.type0_font(type0)
                .base_font(base)
                .encoding_predefined(Name(b"Identity-H"))
                .descendant_font(cid_font)
                .to_unicode(cmap);

            let mut cid = pdf.cid_font(cid_font);
            cid.subtype(CidFontType::Type2)
                .base_font(base)
                .system_info(info)
                .font_descriptor(descriptor)
                .cid_to_gid_map_predefined(Name(b"Identity"));
            cid.widths().consecutive(0, subset.widths.iter().copied());
            cid.finish();

            let d = face.descriptor()?;
            let scale = EM / face.units_per_em();
            let [left, bottom, right, top] = d.bbox.map(|v| real(v * scale));
            pdf.font_descriptor(descriptor)
                .name(base)
                // SYMBOLIC: an Identity-H CIDFont has no standard Latin
                // encoding, and preflight tools flag NON_SYMBOLIC here.
                .flags(FontFlags::SYMBOLIC)
                .bbox(Rect::new(left, bottom, right, top))
                .italic_angle(real(d.italic_angle))
                .ascent(real(d.ascent * scale))
                .descent(real(d.descent * scale))
                .cap_height(real(d.cap_height * scale))
                .stem_v(real(10.0 + 0.244 * (f64::from(weight.css()) - 50.0)))
                .font_file2(file);

            let bytes = subsetter::subset(face.bytes(), 0, &subset.remapper)
                .map_err(|e| FontError::new(face.name(), format!("cannot be subset: {e}")))?;
            let packed = miniz_oxide::deflate::compress_to_vec_zlib(&bytes, LEVEL);
            pdf.stream(file, &packed).filter(Filter::FlateDecode).pair(
                Name(b"Length1"),
                i32::try_from(bytes.len()).map_err(|_| {
                    FontError::new(face.name(), "the subset is over 2 GiB".to_owned())
                })?,
            );

            let mut map = UnicodeCmap::new(Name(b"Custom"), info);
            for (&cid, text) in &subset.unicode {
                map.pair_with_multiple(cid, text.chars());
            }
            pdf.cmap(cmap, &map.finish());
            out.push((weight, type0));
        }
        Ok(out)
    }
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

/// PDF 9.6.4's subset tag: six capitals, here from FNV-1a over the face's
/// name and its glyph ids, so a subset's tag depends only on what it
/// holds (NFR-4) and two different subsets rarely share one.
fn tag(postscript: &str, gids: &[u16]) -> String {
    const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0100_0000_01b3;
    let bytes = postscript
        .bytes()
        .chain(gids.iter().flat_map(|g| g.to_le_bytes()));
    let mut hash = bytes.fold(OFFSET, |h, b| (h ^ u64::from(b)).wrapping_mul(PRIME));
    (0..6)
        .map(|_| {
            let letter = b'A' + u8::try_from(hash % 26).expect("below 26");
            hash /= 26;
            char::from(letter)
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
        for text in ["e\u{301}", "x\u{301}", "\u{344}"] {
            let shaped = face.shape(text);
            let faces = Faces::of([&run(text)], fonts()).expect("subsets");
            let joined: String = shaped
                .iter()
                .filter_map(|g| faces.unicode(Weight::Regular, faces.cid(Weight::Regular, g.id)))
                .collect();
            assert_eq!(joined, text);
        }
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
        let a = tag("Inter-Regular", &[0, 3, 9]);
        assert_eq!(a.len(), 6);
        assert!(a.bytes().all(|b| b.is_ascii_uppercase()), "{a}");
        assert_eq!(a, tag("Inter-Regular", &[0, 3, 9]));
        assert_ne!(a, tag("Inter-Regular", &[0, 3, 10]));
        assert_ne!(a, tag("Inter-SemiBold", &[0, 3, 9]));
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
