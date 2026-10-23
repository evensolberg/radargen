//! The page's content stream (FR-110): the document's static rendering as
//! PDF operators, in document order, in document units under one
//! flip-and-scale transform. Interactive-only nodes are dropped, as the
//! static SVG drops them (FR-50).

use pdf_writer::Content;

use super::font::Faces;
use super::geometry::{self, Step};
use super::leaf::{self, Leaf};
use super::tag::Tagger;
use super::{Sheet, real};
use crate::palette::Palette;
use crate::svg::{Document, Extent, Group, Node, Paint, Point};
use crate::text::{FontError, Fonts};

/// SVG's default `stroke-miterlimit`; PDF's default is 10.
const MITER_LIMIT: f32 = 4.0;
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
    /// The page box and the scale the drawing is put on it at.
    pub page: Sheet,
}

/// The whole page: the ground over the page box (D-87), then the
/// transform and the document, each leaf marked into `tagger`'s tree
/// (FR-114) as the next page.
///
/// # Errors
///
/// [`FontError`] when a halo's glyph cannot be outlined (NFR-15).
pub fn content(canvas: &Canvas<'_>, tagger: &mut Tagger) -> Result<Vec<u8>, FontError> {
    tagger.begin_page();
    let mut out = Content::new();
    out.save_state();
    // The document's own ground (D-87), in page space before the flip and
    // over the whole page box: the fraction of a point past the drawing's
    // right and bottom edges included.
    let page = canvas.page;
    let ground = painted(&canvas.document.ground()).map(|(_, paints)| {
        let steps = geometry::rect(Point::ORIGIN, page.width, page.height, 0.0);
        (steps, paints)
    });
    let marked = tagger.leaf(&mut out, &Leaf::Ground);
    shape(&mut out, ground, canvas);
    Tagger::end(&mut out, marked);
    // Unrounded (pdf::scale), so the drawing keeps its size; the
    // translate is the page box's own height, so the drawing hangs from
    // its top edge.
    let s = super::scale(canvas.page.scale);
    out.transform([s, 0.0, 0.0, -s, 0.0, real(canvas.page.height)])
        .set_miter_limit(MITER_LIMIT);
    node(
        &mut out,
        &canvas.document.root,
        Point::ORIGIN,
        (canvas, tagger),
    )?;
    out.restore_state();
    Ok(out.finish().into_vec())
}

/// One node, marked as [`leaf::of`] says (FR-114), or nothing for an
/// interactive-only one or one that paints nothing; `at` is where its
/// parent's origin sits in the document.
fn node(
    out: &mut Content,
    node: &Node,
    at: Point,
    (canvas, tagger): (&Canvas<'_>, &mut Tagger),
) -> Result<(), FontError> {
    if node.is_interactive_only() {
        return Ok(());
    }
    if let Node::Group(group) = node {
        return self::group(out, group, at, (canvas, tagger));
    }
    let Some(leaf) = leaf::of(node, canvas) else {
        return Ok(());
    };
    let marked = tagger.leaf(out, &leaf);
    let drawn = if let Node::Text { .. } = node {
        super::show::text(out, node, canvas)
    } else {
        shape(out, painted(node), canvas);
        Ok(())
    };
    Tagger::end(out, marked);
    drawn
}

/// A shape node's path and paints; `None` for a group, a text or a hit
/// area, which are not one path.
pub(super) fn painted(node: &Node) -> Option<(Vec<Step>, Paints)> {
    let (steps, fill, stroke, width) = match *node {
        Node::Circle {
            centre,
            radius,
            fill,
            stroke,
            stroke_width,
        } => (geometry::circle(centre, radius), fill, stroke, stroke_width),
        Node::Rect {
            origin,
            width,
            height,
            radius,
            fill,
            stroke,
            stroke_width,
        } => (
            geometry::rect(origin, width, height, radius),
            fill,
            stroke,
            stroke_width,
        ),
        Node::Line {
            from,
            to,
            stroke,
            stroke_width,
        } => (
            vec![Step::Move(from), Step::Line(to)],
            Paint::None,
            stroke,
            stroke_width,
        ),
        Node::Path {
            ref commands,
            fill,
            stroke,
            stroke_width,
        } => (geometry::path(commands), fill, stroke, stroke_width),
        Node::Group(_) | Node::Text { .. } | Node::HitArea { .. } => return None,
    };
    Some((
        steps,
        Paints {
            fill,
            stroke,
            width,
        },
    ))
}

/// A group: its children, translated by its offset, in its own saved
/// state; `at` is where its parent's origin sits in the document.
fn group(
    out: &mut Content,
    group: &Group,
    at: Point,
    (canvas, tagger): (&Canvas<'_>, &mut Tagger),
) -> Result<(), FontError> {
    // An artifact's mark wraps its saved state, never the reverse.
    let opened = tagger.open(out, &group.semantics);
    let origin = Point {
        x: at.x + group.offset.x,
        y: at.y + group.offset.y,
    };
    if let Some(extent) = group.extent {
        tagger.extent(bbox(extent, origin, canvas.page));
    }
    out.save_state();
    // Decided on the rounded value, as the SVG's `shows` decides, so a
    // 1e-13 trig residue cannot write a transform on one platform and
    // none on another (NFR-4).
    let (x, y) = (real(group.offset.x), real(group.offset.y));
    if x != 0.0 || y != 0.0 {
        out.transform([1.0, 0.0, 0.0, 1.0, x, y]);
    }
    for child in &group.children {
        node(out, child, origin, (canvas, &mut *tagger))?;
    }
    out.restore_state();
    tagger.close(out, opened);
    Ok(())
}

/// `extent`, drawn from `origin`, as a box on `page` (rg-2qs): left,
/// bottom, right, top in page space, as the drawing's flip puts it.
fn bbox(extent: Extent, origin: Point, page: Sheet) -> [f32; 4] {
    let (s, height) = (page.scale, page.height);
    let x = |v: f64| real(s * (origin.x + v));
    let y = |v: f64| real(height - s * (origin.y + v));
    [
        x(extent.min.x),
        y(extent.max.y),
        x(extent.max.x),
        y(extent.min.y),
    ]
}

/// A shape's fill, stroke and stroke width.
#[derive(Clone, Copy)]
pub(super) struct Paints {
    fill: Paint,
    stroke: Paint,
    width: f64,
}

impl Paints {
    /// Whether these paints put anything on the page: the one test both
    /// [`shape`] and [`leaf::of`] use, so what is marked is what is drawn.
    pub(super) fn show(self, palette: &Palette) -> bool {
        colour(self.fill, palette).is_some() || colour(self.stroke, palette).is_some()
    }
}

/// A path painted: `f`, `S` or `B` by which paints are set; nothing at
/// all when neither is, or for no path, so an unpainted shape writes no
/// path.
fn shape(out: &mut Content, painted: Option<(Vec<Step>, Paints)>, canvas: &Canvas<'_>) {
    let Some((steps, paints)) = painted else {
        return;
    };
    if !paints.show(canvas.palette) {
        return;
    }
    let fill = colour(paints.fill, canvas.palette);
    let stroke = colour(paints.stroke, canvas.palette);
    if let Some([r, g, b]) = fill {
        out.set_fill_rgb(r, g, b);
    }
    if let Some([r, g, b]) = stroke {
        out.set_stroke_rgb(r, g, b)
            .set_line_width(real(paints.width));
    }
    trace(out, &steps);
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

    /// rg-2qs: a box drawn from a group's origin lands in page space as
    /// the drawing's flip puts it: scaled, its y measured from the bottom.
    #[test]
    fn a_bbox_is_flipped_into_page_space() {
        let extent = Extent {
            min: Point { x: -10.0, y: -20.0 },
            max: Point { x: 10.0, y: 20.0 },
        };
        let page = Sheet {
            unit: 1.0,
            scale: 0.5,
            width: 100.0,
            height: 200.0,
        };
        // Drawn from (50, 60): x 40..60, y 40..80, then halved and flipped.
        let got = bbox(extent, Point { x: 50.0, y: 60.0 }, page);
        assert_eq!(got, [20.0, 160.0, 30.0, 180.0]);
    }

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
            page: Sheet {
                unit: 1.0,
                scale: 0.75,
                width: 151.0,
                height: 76.0,
            },
        };
        String::from_utf8(content(&canvas, &mut Tagger::new()).expect("draws")).expect("ASCII")
    }

    fn rgb(palette: &Palette, role: Role) -> String {
        let [r, g, b] = colour(Paint::Role(role), palette).expect("a role is painted");
        format!("{r} {g} {b}")
    }

    /// The page opens with the theme's ground over the whole page box in
    /// page space (D-87), an artifact (FR-114), then one flip into PDF's y-up space at the
    /// export scale, translated by the page's height so the drawing hangs
    /// from its top edge, and SVG's miter limit; it closes the state it
    /// saved.
    #[test]
    fn the_page_is_grounded_then_flipped_and_scaled() {
        let palette = Palette::built_in(Theme::Dark);
        let s = stream(Node::Group(Group::new(Point::ORIGIN, Vec::new())), &palette);
        let want = format!(
            "q\n/Artifact BMC\n{} rg\n0 0 m\n151 0 l\n151 76 l\n0 76 l\nh\nf\nEMC\n0.75 0 0 -0.75 0 76 cm\n4 M\n",
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
            page: Sheet {
                unit: 1.0,
                scale,
                width: 240.0,
                height: 191.0,
            },
        };
        let s =
            String::from_utf8(content(&canvas, &mut Tagger::new()).expect("draws")).expect("ASCII");
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
