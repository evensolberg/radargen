//! The page's content stream (FR-110): the document's static rendering as
//! PDF operators, in document order, in document units under one
//! flip-and-scale transform. Interactive-only nodes are dropped, as the
//! static SVG drops them (FR-50).

use pdf_writer::Content;

use super::font::Faces;
use super::geometry::{self, Step};
use super::real;
use crate::palette::Palette;
use crate::palette::{Neutral, Role};
use crate::svg::{Document, Group, Node, Paint, Point};
use crate::text::{FontError, Fonts};

/// SVG's default `stroke-miterlimit`; PDF's default is 10.
const MITER_LIMIT: f32 = 4.0;
/// The page's ground (D-87): the document's own, `Document::ground`'s.
const GROUND: Paint = Paint::Role(Role::Neutral(Neutral::Surface1));
/// A colour channel's largest value, `ff`: what PDF's 1.0 stands for.
const CHANNEL_MAX: f64 = 255.0;

/// What drawing a page reads.
pub struct Canvas<'a> {
    /// The arranged document.
    pub document: &'a Document,
    /// The theme's palette, bound to literals (FR-56).
    pub palette: &'a Palette,
    /// The faces text is placed with.
    pub fonts: &'a Fonts,
    /// The embedded subsets.
    pub faces: &'a Faces,
    /// The content-to-page transform's scale: points per document unit,
    /// 0.75 × FR-57's width over the document's, divided by the page's
    /// `/UserUnit`. A paper size (PR 21) adds an offset beside it, not a
    /// second transform.
    pub scale: f64,
    /// The page box's width in the page's own units, a whole number: the
    /// `MediaBox`'s, which the ground covers.
    pub width: f64,
    /// The page box's height, likewise: what the flip translates by, so
    /// the drawing hangs from the page's top edge.
    pub height: f64,
}

/// The whole page: the ground over the page box (D-87), then the
/// transform and the document.
///
/// # Errors
///
/// [`FontError`] when a halo's glyph cannot be outlined (NFR-15).
pub fn content(canvas: &Canvas<'_>) -> Result<Vec<u8>, FontError> {
    let mut out = Content::new();
    out.save_state();
    // In page space, before the flip: the whole page box, the fraction of
    // a point past the drawing's right and bottom edges included.
    let ground = geometry::rect(Point::ORIGIN, canvas.width, canvas.height, 0.0);
    shape(
        &mut out,
        &ground,
        Paints::new(GROUND, Paint::None, 0.0),
        canvas,
    );
    // Unrounded (pdf::scale), so the drawing keeps its size; the
    // translate is the page box's own height, so the drawing hangs from
    // its top edge.
    let s = super::scale(canvas.scale);
    out.transform([s, 0.0, 0.0, -s, 0.0, real(canvas.height)])
        .set_miter_limit(MITER_LIMIT);
    node(&mut out, &canvas.document.root, canvas)?;
    out.restore_state();
    Ok(out.finish().into_vec())
}

/// One node, or nothing for an interactive-only one.
// qual:allow(complexity, max_function_lines=68) reason: "one dispatch arm per node kind, each only unpacking its fields for its writer"
fn node(out: &mut Content, node: &Node, canvas: &Canvas<'_>) -> Result<(), FontError> {
    if node.is_interactive_only() {
        return Ok(());
    }
    match node {
        Node::Group(group) => self::group(out, group, canvas)?,
        Node::Text { .. } => super::show::text(out, node, canvas)?,
        Node::Circle {
            centre,
            radius,
            fill,
            stroke,
            stroke_width,
        } => {
            shape(
                out,
                &geometry::circle(*centre, *radius),
                Paints::new(*fill, *stroke, *stroke_width),
                canvas,
            );
        }
        Node::Rect {
            origin,
            width,
            height,
            radius,
            fill,
            stroke,
            stroke_width,
        } => {
            shape(
                out,
                &geometry::rect(*origin, *width, *height, *radius),
                Paints::new(*fill, *stroke, *stroke_width),
                canvas,
            );
        }
        Node::Line {
            from,
            to,
            stroke,
            stroke_width,
        } => {
            shape(
                out,
                &[Step::Move(*from), Step::Line(*to)],
                Paints::new(Paint::None, *stroke, *stroke_width),
                canvas,
            );
        }
        Node::Path {
            commands,
            fill,
            stroke,
            stroke_width,
        } => {
            shape(
                out,
                &geometry::path(commands),
                Paints::new(*fill, *stroke, *stroke_width),
                canvas,
            );
        }
        // Interactive-only, returned above.
        Node::HitArea { .. } => {}
    }
    Ok(())
}

/// A group: its children, translated by its offset, in its own saved
/// state.
fn group(out: &mut Content, group: &Group, canvas: &Canvas<'_>) -> Result<(), FontError> {
    out.save_state();
    // Decided on the rounded value, as the SVG's `shows` decides, so a
    // 1e-13 trig residue cannot write a transform on one platform and
    // none on another (NFR-4).
    let (x, y) = (real(group.offset.x), real(group.offset.y));
    if x != 0.0 || y != 0.0 {
        out.transform([1.0, 0.0, 0.0, 1.0, x, y]);
    }
    for child in &group.children {
        node(out, child, canvas)?;
    }
    out.restore_state();
    Ok(())
}

/// A shape's fill, stroke and stroke width.
#[derive(Clone, Copy)]
struct Paints {
    fill: Paint,
    stroke: Paint,
    width: f64,
}

impl Paints {
    const fn new(fill: Paint, stroke: Paint, width: f64) -> Self {
        Self {
            fill,
            stroke,
            width,
        }
    }
}

/// `steps` painted: `f`, `S` or `B` by which paints are set; nothing at
/// all when neither is, so an unpainted shape writes no path.
fn shape(out: &mut Content, steps: &[Step], paints: Paints, canvas: &Canvas<'_>) {
    let fill = colour(paints.fill, canvas.palette);
    let stroke = colour(paints.stroke, canvas.palette);
    if fill.is_none() && stroke.is_none() {
        return;
    }
    if let Some([r, g, b]) = fill {
        out.set_fill_rgb(r, g, b);
    }
    if let Some([r, g, b]) = stroke {
        out.set_stroke_rgb(r, g, b)
            .set_line_width(real(paints.width));
    }
    trace(out, steps);
    match (fill, stroke) {
        (Some(_), Some(_)) => out.fill_nonzero_and_stroke(),
        (Some(_), None) => out.fill_nonzero(),
        _ => out.stroke(),
    };
}

/// The path operators for `steps`.
pub(super) fn trace(out: &mut Content, steps: &[Step]) {
    for step in steps {
        match *step {
            Step::Move(p) => out.move_to(real(p.x), real(p.y)),
            Step::Line(p) => out.line_to(real(p.x), real(p.y)),
            Step::Cubic { c1, c2, end } => out.cubic_to(
                real(c1.x),
                real(c1.y),
                real(c2.x),
                real(c2.y),
                real(end.x),
                real(end.y),
            ),
            Step::Close => out.close_path(),
        };
    }
}

/// `paint` as an RGB triple of PDF reals from `palette`, or `None` for
/// [`Paint::None`]: the PDF binds roles as the static SVG does
/// (architecture §5).
pub(super) fn colour(paint: Paint, palette: &Palette) -> Option<[f32; 3]> {
    let rgb = match paint {
        Paint::Role(role) => palette.rgb(role),
        Paint::NumberOn(index) => palette.number_ink_rgb(index),
        Paint::None => return None,
    };
    Some(rgb.map(|c| real(f64::from(c) / CHANNEL_MAX)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::block::fonts;
    use crate::palette::{Neutral, Role, Theme};
    use crate::svg::{Group, ItemKey, Point, Semantics};

    fn stream(root: Node, palette: &Palette) -> String {
        let document = Document {
            width: 200.0,
            height: 100.0,
            root,
        };
        let faces = Faces::of([&document.root], fonts()).expect("subsets");
        let canvas = Canvas {
            document: &document,
            palette,
            fonts: fonts(),
            faces: &faces,
            scale: 0.75,
            width: 151.0,
            height: 76.0,
        };
        String::from_utf8(content(&canvas).expect("draws")).expect("ASCII")
    }

    fn rgb(palette: &Palette, role: Role) -> String {
        let [r, g, b] = colour(Paint::Role(role), palette).expect("a role is painted");
        format!("{r} {g} {b}")
    }

    /// The page opens with the theme's ground over the whole page box in
    /// page space (D-87), then one flip into PDF's y-up space at the
    /// export scale, translated by the page's height so the drawing hangs
    /// from its top edge, and SVG's miter limit; it closes the state it
    /// saved.
    #[test]
    fn the_page_is_grounded_then_flipped_and_scaled() {
        let palette = Palette::built_in(Theme::Dark);
        let s = stream(Node::Group(Group::new(Point::ORIGIN, Vec::new())), &palette);
        let want = format!(
            "q\n{} rg\n0 0 m\n151 0 l\n151 76 l\n0 76 l\nh\nf\n0.75 0 0 -0.75 0 76 cm\n4 M\n",
            rgb(&palette, Role::Neutral(Neutral::Surface1))
        );
        assert!(s.starts_with(&want), "{s}");
        assert_eq!(s.matches("\nf\n").count(), 1, "one ground: {s}");
        // Content::finish drops the last newline.
        assert!(s.ends_with('Q'), "{s}");
    }

    /// At a scale thousandths cannot hold (`--width 320` on a 1264-unit
    /// document), the scale is written unrounded, so the drawing meets the
    /// `MediaBox`'s edge (Review Focus 3).
    #[test]
    fn an_awkward_scale_is_written_unrounded() {
        let palette = Palette::built_in(Theme::Dark);
        let document = Document {
            width: 1264.0,
            height: 1004.0,
            root: Node::Group(Group::new(Point::ORIGIN, Vec::new())),
        };
        let faces = Faces::of([&document.root], fonts()).expect("subsets");
        let scale = 0.75 * 320.0 / 1264.0;
        let canvas = Canvas {
            document: &document,
            palette: &palette,
            fonts: fonts(),
            faces: &faces,
            scale,
            width: 240.0,
            height: 191.0,
        };
        let s = String::from_utf8(content(&canvas).expect("draws")).expect("ASCII");
        let first = s.lines().find(|l| l.ends_with(" cm")).expect("the cm line");
        let written: f64 = first
            .split(' ')
            .next()
            .expect("a")
            .parse()
            .expect("a number");
        assert!((written - scale).abs() * 1004.0 < 0.01, "{first}");
        assert!(
            !first.starts_with("0.19 "),
            "not rounded to thousandths: {first}"
        );
    }

    /// A group translates its children inside its own saved state; an
    /// interactive-only node draws nothing (FR-50, FR-94).
    #[test]
    fn groups_translate_and_interactive_nodes_vanish() {
        let palette = Palette::built_in(Theme::Light);
        let dot = Node::Circle {
            centre: Point::ORIGIN,
            radius: 2.0,
            fill: Paint::Role(Role::Series(0)),
            stroke: Paint::None,
            stroke_width: 0.0,
        };
        let hidden = Node::Group(
            Group::new(Point::ORIGIN, vec![dot.clone()]).with(Semantics::Indicator(ItemKey(2))),
        );
        let hit = Node::HitArea {
            centre: Point::ORIGIN,
            radius: 9.0,
            key: ItemKey(2),
        };
        let root = Node::Group(Group::new(
            Point { x: 10.0, y: 5.0 },
            vec![dot, hidden, hit],
        ));
        let s = stream(root, &palette);
        assert!(s.contains("q\n1 0 0 1 10 5 cm\n"), "{s}");
        assert_eq!(
            s.matches(" c\n").count(),
            4,
            "one circle's four cubics: {s}"
        );
    }

    /// Fill only is `f`, stroke only `S` with its width, both `B`, neither
    /// nothing; a line is stroked.
    #[test]
    fn paint_picks_the_operator() {
        let palette = Palette::built_in(Theme::Light);
        let rect = |fill, stroke| Node::Rect {
            origin: Point::ORIGIN,
            width: 4.0,
            height: 4.0,
            radius: 0.0,
            fill,
            stroke,
            stroke_width: 1.5,
        };
        let ink = Paint::Role(Role::Neutral(Neutral::TextPrimary));
        let root = Node::Group(Group::new(
            Point::ORIGIN,
            vec![
                rect(ink, Paint::None),
                rect(Paint::None, ink),
                rect(ink, ink),
                rect(Paint::None, Paint::None),
                Node::Line {
                    from: Point::ORIGIN,
                    to: Point { x: 3.0, y: 0.0 },
                    stroke: ink,
                    stroke_width: 0.5,
                },
            ],
        ));
        let s = stream(root, &palette);
        // The ground's `f`, then the first rect's.
        assert_eq!(s.matches("\nf\n").count(), 2, "{s}");
        assert_eq!(
            s.matches("\nS\n").count(),
            2,
            "the stroked rect and the line: {s}"
        );
        assert_eq!(s.matches("\nB\n").count(), 1, "{s}");
        assert!(s.contains("1.5 w\n") && s.contains("0.5 w\n"), "{s}");
        // Every rect here starts at the origin and a square rect is m l l l h,
        // so each painted one opens with `0 0 m`, as does the line; the
        // unpainted rect writes no path at all.
        assert_eq!(
            s.matches("\n0 0 m\n").count(),
            5,
            "the ground, the three painted rects and the line: {s}"
        );
    }

    /// Colours are the palette's literals, a number's ink included.
    #[test]
    fn colours_are_the_palettes_literals() {
        let palette = Palette::built_in(Theme::Light);
        let [r, g, b] = crate::palette::Hex::parse(palette.value(Role::Series(1)))
            .expect("hex")
            .rgb();
        let want = [r, g, b].map(|c| real(f64::from(c) / CHANNEL_MAX));
        assert_eq!(colour(Paint::Role(Role::Series(1)), &palette), Some(want));
        assert!(colour(Paint::NumberOn(0), &palette).is_some());
        assert_eq!(colour(Paint::None, &palette), None);
    }
}
