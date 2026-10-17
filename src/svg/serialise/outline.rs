//! Text as glyph outlines, for the exported SVG (architecture §11, D-88).
//!
//! Each glyph a static document draws is defined once per face and size
//! in `<defs>`, already at its size, and placed by `<use>`. Ids are safe
//! here, though §5 bans them in the page: this file holds one
//! arrangement, not two. Collecting and writing read text through one
//! function, [`runs`], so the table holds every glyph the writer places.

use std::collections::{BTreeMap, BTreeSet};

use super::text::{Run, ink};
use super::{Context, attr, coord, paint, point};
use crate::style::TextStyle;
use crate::svg::{Anchor, Document, Node, Point, Tail};
use crate::text::{FontError, Fonts, Segment, Weight};

/// A glyph in one face at one size: one definition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Key {
    weight: Weight,
    /// The size's bits: sizes are positive and finite, so their bits
    /// order as they do.
    size: u64,
    glyph: u32,
}

impl Key {
    const fn new(style: TextStyle, glyph: u32) -> Self {
        Self {
            weight: style.weight,
            size: style.size.to_bits(),
            glyph,
        }
    }
}

/// What a collected glyph draws.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Entry {
    /// The definition `gN`.
    Ink(usize),
    /// Nothing: a space.
    Blank,
}

/// Every glyph a document draws, the inked ones numbered in key order.
pub(super) struct Glyphs {
    entries: BTreeMap<Key, Entry>,
    paths: Vec<String>,
}

/// What outlined text reads: the faces to place with, and the table.
#[derive(Clone, Copy)]
pub(super) struct Outlined<'a> {
    pub(super) fonts: &'a Fonts,
    pub(super) glyphs: &'a Glyphs,
}

/// A text's runs, in the order they are drawn: each line in the text's
/// style, then the tail in its own. Collecting and writing both read text
/// through this, so they cannot disagree about what is drawn.
fn runs<'a>(
    style: TextStyle,
    lines: &'a [String],
    tail: Option<&'a Tail>,
) -> impl Iterator<Item = (TextStyle, &'a str)> {
    lines
        .iter()
        .map(move |line| (style, line.as_str()))
        .chain(tail.map(|t| (t.style, t.text.as_str())))
}

impl Glyphs {
    /// The glyphs `document`'s static rendering draws: every run of every
    /// text outside an interactive-only node.
    ///
    /// # Errors
    ///
    /// [`FontError`] when a shaped glyph cannot be outlined (NFR-15).
    pub(super) fn of(document: &Document, fonts: &Fonts) -> Result<Self, FontError> {
        let outlines = keys(&document.root, fonts)
            .into_iter()
            .map(|key| outlined(key, fonts))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self::numbered(outlines))
    }

    /// The table from each glyph's `d`: an empty one is blank, the rest
    /// numbered in key order.
    fn numbered(outlines: Vec<(Key, String)>) -> Self {
        let mut entries = BTreeMap::new();
        let mut paths = Vec::new();
        for (key, d) in outlines {
            let entry = if d.is_empty() {
                Entry::Blank
            } else {
                paths.push(d);
                Entry::Ink(paths.len() - 1)
            };
            entries.insert(key, entry);
        }
        Self { entries, paths }
    }

    /// `<defs>` with one path per inked glyph, or nothing when none inks.
    pub(super) fn defs(&self) -> String {
        if self.paths.is_empty() {
            return String::new();
        }
        let mut out = String::from("<defs>\n");
        for (id, d) in self.paths.iter().enumerate() {
            out.push_str("<path");
            attr(&mut out, "id", &format!("g{id}"));
            attr(&mut out, "d", d);
            out.push_str("/>\n");
        }
        out.push_str("</defs>\n");
        out
    }

    /// The entry for a glyph the writer places. Every placed glyph was
    /// collected through [`runs`]; a miss is a broken invariant, and
    /// silently dropping the glyph would hide it.
    fn entry(&self, style: TextStyle, glyph: u32) -> Entry {
        *self
            .entries
            .get(&Key::new(style, glyph))
            .expect("every placed glyph was collected through `runs`")
    }
}

/// Each glyph `node` draws, keyed by face, size and id.
fn keys(node: &Node, fonts: &Fonts) -> BTreeSet<Key> {
    match node {
        _ if node.is_interactive_only() => BTreeSet::new(),
        Node::Group(group) => group
            .children
            .iter()
            .flat_map(|child| keys(child, fonts))
            .collect(),
        Node::Text {
            style, lines, tail, ..
        } => runs(*style, lines, tail.as_ref())
            .flat_map(|(style, text)| {
                fonts
                    .face(style.weight)
                    .shape(text)
                    .into_iter()
                    .map(move |glyph| Key::new(style, glyph.id))
            })
            .collect(),
        _ => BTreeSet::new(),
    }
}

/// A glyph's key and its path at its size.
fn outlined(key: Key, fonts: &Fonts) -> Result<(Key, String), FontError> {
    let face = fonts.face(key.weight);
    let scale = f64::from_bits(key.size) / face.units_per_em();
    Ok((key, path(&face.outline(key.glyph)?, scale)))
}

/// A glyph's contours as a path's `d`: font units scaled to the size and
/// flipped into SVG's y-down space, every number through `fmt_coord`.
fn path(segments: &[Segment], scale: f64) -> String {
    let at = |&(x, y): &(f32, f32)| {
        point(Point {
            x: f64::from(x) * scale,
            y: -f64::from(y) * scale,
        })
    };
    let step = |command: &str, points: &[(f32, f32)]| {
        [
            command,
            &points.iter().map(at).collect::<Vec<_>>().join(" "),
        ]
        .concat()
    };
    segments
        .iter()
        .map(|&segment| match segment {
            Segment::MoveTo(x, y) => step("M", &[(x, y)]),
            Segment::LineTo(x, y) => step("L", &[(x, y)]),
            Segment::QuadTo(cx, cy, x, y) => step("Q", &[(cx, cy), (x, y)]),
            Segment::CurveTo(c1x, c1y, c2x, c2y, x, y) => {
                step("C", &[(c1x, c1y), (c2x, c2y), (x, y)])
            }
            Segment::Close => "Z".to_owned(),
        })
        .collect()
}

/// How much of its advance a middle-anchored line starts before its
/// origin. A power of two, so it is exactly the `/ 2.0` resvg's anchor is.
const CENTRED: f64 = 0.5;

/// A text as a group of placed glyphs: its paint on the group, each line
/// at its baseline, its tail at `tail_x` (D-78). Filled and outside a
/// control, an image named by its words (D-88).
#[expect(
    clippy::suboptimal_flops,
    reason = "NFR-4: plain * and + are bit-identical on every target; mul_add falls back to a libm fma where the CPU lacks one"
)]
pub(super) fn text(
    run: &Run<'_>,
    tail_x: Option<f64>,
    outlined: Outlined<'_>,
    context: Context<'_>,
) -> String {
    if run.lines.is_empty() {
        return String::new();
    }
    let mut out = String::from("<g");
    ink(&mut out, run.ink, context);
    if let Some(name) = run.own_name(context) {
        attr(&mut out, "role", "img");
        attr(&mut out, "aria-label", &name);
    }
    out.push_str(">\n");
    let mut baseline = run.origin.y;
    for (i, line) in (0_u32..).zip(run.lines) {
        baseline = run.origin.y + f64::from(i) * run.leading;
        let x = match run.anchor {
            Anchor::Start => run.origin.x,
            Anchor::Middle => run.origin.x - run.style.advance(line, outlined.fonts) * CENTRED,
        };
        out.push_str(&uses(line, run.style, Point { x, y: baseline }, outlined));
    }
    if let (Some(tail), Some(x)) = (run.tail, tail_x) {
        out.push_str("<g");
        attr(&mut out, "fill", &paint(tail.ink, context.target));
        out.push_str(">\n");
        out.push_str(&uses(
            &tail.text,
            tail.style,
            Point { x, y: baseline },
            outlined,
        ));
        out.push_str("</g>\n");
    }
    out.push_str("</g>\n");
    out
}

/// One `<use>` per inked glyph of `text`, from `at`.
fn uses(text: &str, style: TextStyle, at: Point, outlined: Outlined<'_>) -> String {
    let mut out = String::new();
    for placed in outlined
        .fonts
        .face(style.weight)
        .place(text, style.size, style.letter_spacing)
    {
        if let Entry::Ink(id) = outlined.glyphs.entry(style, placed.glyph) {
            out.push_str("<use");
            attr(&mut out, "xlink:href", &format!("#g{id}"));
            coord(&mut out, "x", at.x + placed.x);
            coord(&mut out, "y", at.y + placed.y);
            out.push_str("/>\n");
        }
    }
    out
}
