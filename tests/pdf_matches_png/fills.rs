//! Where the worst-delta gate is blind to a flat colour a little off:
//! inside every filled shape, and at the ground's corners, the PDF is the
//! PNG's colour.

use radargen::palette::{Neutral, Palette, Role, Theme};
use radargen::svg::{Document, Node, Paint, Point};

use super::halos::halos_off;
use super::oracle::wide;
use super::pdf_export::{fonts, pdf};
use super::{case, cases, damaged, rendered};

/// How far a channel inside a filled shape may differ: the two
/// rasterisers agree exactly on a flat fill, so this is rounding only.
const FLAT_DELTA: u8 = 3;
/// A shape smaller than this, in document units, may have no pixel that
/// antialiasing does not touch.
const MIN_SHAPE: f64 = 4.0;
/// How many pixels around a spot must show the shape's own fill in the
/// PNG for the spot to be looked at.
const CLEAR: u32 = 2;
/// Where inside a rectangle to look, as fractions of its sides: the
/// centre first, then a ring of points around it, since a legend row's
/// text covers the centre of most rows.
const RECT_SPOTS: [(f64, f64); 9] = [
    (0.5, 0.5),
    (0.5, 0.25),
    (0.5, 0.75),
    (0.25, 0.5),
    (0.75, 0.5),
    (0.3, 0.3),
    (0.7, 0.3),
    (0.3, 0.7),
    (0.7, 0.7),
];
/// Where inside a circle to look, as fractions of its radius from the
/// centre: the centre, then eight points between a marker's number and
/// its edge.
const CIRCLE_SPOTS: [(f64, f64); 9] = [
    (0.0, 0.0),
    (0.0, -0.7),
    (0.0, 0.7),
    (-0.7, 0.0),
    (0.7, 0.0),
    (-0.5, -0.5),
    (0.5, -0.5),
    (-0.5, 0.5),
    (0.5, 0.5),
];

/// A filled shape: the points to look at inside it, in document units,
/// and its fill's channels.
struct Filled {
    spots: Vec<Point>,
    rgb: [u8; 3],
}

/// Every filled rectangle and circle wider than [`MIN_SHAPE`] that the
/// static rendering draws, through its groups' offsets.
fn filled(node: &Node, at: Point, palette: &Palette, out: &mut Vec<Filled>) {
    if node.is_interactive_only() {
        return;
    }
    let (fill, spots): (Paint, Vec<Point>) = match node {
        Node::Group(group) => {
            let at = Point {
                x: at.x + group.offset.x,
                y: at.y + group.offset.y,
            };
            for child in &group.children {
                filled(child, at, palette, out);
            }
            return;
        }
        Node::Rect {
            origin,
            width,
            height,
            fill,
            ..
        } if width.min(*height) > MIN_SHAPE => (
            *fill,
            RECT_SPOTS
                .iter()
                .map(|&(fx, fy)| Point {
                    x: at.x + origin.x + width * fx,
                    y: at.y + origin.y + height * fy,
                })
                .collect(),
        ),
        Node::Circle {
            centre,
            radius,
            fill,
            ..
        } if radius * 2.0 > MIN_SHAPE => (
            *fill,
            CIRCLE_SPOTS
                .iter()
                .map(|&(fx, fy)| Point {
                    x: at.x + centre.x + radius * fx,
                    y: at.y + centre.y + radius * fy,
                })
                .collect(),
        ),
        _ => return,
    };
    let rgb = match fill {
        Paint::Role(role) => palette.rgb(role),
        Paint::NumberOn(index) => palette.number_ink_rgb(index),
        Paint::None => return,
    };
    out.push(Filled { spots, rgb });
}

/// Whether every channel of `a` and `b` is within [`FLAT_DELTA`].
fn flat(a: &[u8], b: &[u8]) -> bool {
    a.iter().zip(b).all(|(p, q)| p.abs_diff(*q) <= FLAT_DELTA)
}

/// What [`colours_off`] found.
pub struct Colours {
    /// The filled shapes the document draws.
    shapes: usize,
    /// How many had a clear spot to look at.
    looked: usize,
    /// Each sampled pixel the two disagree on.
    pub off: Vec<String>,
}

/// Each sampled pixel where hayro's drawing of `pdf` and the PNG differ
/// by more than [`FLAT_DELTA`] in a channel: in every filled shape, the
/// first spot where the PNG shows the shape's own fill (nothing drawn
/// over or near it), and the ground at two opposite corners.
pub fn colours_off(doc: &Document, pdf: &[u8], palette: &Palette) -> Colours {
    let (size, png, drawn) = rendered(doc, pdf, palette, None);
    let (w, h) = (size.width(), size.height());
    let pixel = |image: &[u8], (x, y): (u32, u32)| {
        let at = usize::try_from((y * w + x) * 4).expect("fits");
        image.get(at..at + 4).map(<[u8]>::to_vec)
    };
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::as_conversions,
        reason = "a test's pixel index: every spot is inside the page, so its floor fits u32"
    )]
    let to_pixel = |p: &Point| {
        (
            (p.x * f64::from(w) / doc.width).floor() as u32,
            (p.y * f64::from(h) / doc.height).floor() as u32,
        )
    };
    let mut shapes = Vec::new();
    filled(&doc.root, Point::ORIGIN, palette, &mut shapes);
    let mut seen = vec![(0, 0), (w - 1, h - 1)];
    // A spot is clear when the PNG shows the shape's own fill all around
    // it, so neither rasteriser's antialiasing of a nearby glyph reaches it.
    let clear = |(x, y): (u32, u32), own: &[u8]| {
        (x.saturating_sub(CLEAR)..=x + CLEAR).all(|x| {
            (y.saturating_sub(CLEAR)..=y + CLEAR)
                .all(|y| pixel(png.data(), (x, y)).is_some_and(|p| flat(&p, own)))
        })
    };
    seen.extend(shapes.iter().filter_map(|shape| {
        let own = [shape.rgb[0], shape.rgb[1], shape.rgb[2], 255];
        shape.spots.iter().map(to_pixel).find(|&at| clear(at, &own))
    }));
    let looked = seen.len() - 2;
    let mut off = Vec::new();
    for at in seen {
        match (pixel(png.data(), at), pixel(&drawn, at)) {
            (Some(a), Some(b)) if flat(&a, &b) => {}
            (a, b) => off.push(format!("{at:?}: PNG {a:?}, PDF {b:?}")),
        }
    }
    Colours {
        shapes: shapes.len(),
        looked,
        off,
    }
}

/// Inside every filled shape, and at the ground's corners, the PDF is the
/// PNG's colour, in both themes. At least half the shapes have a clear
/// spot, so the check is never vacuous; a marker's number or a row's text
/// covers the rest.
#[test]
fn every_filled_shape_has_its_colour_inside() {
    for (name, csv, title) in cases() {
        let (doc, desc) = wide(csv, title, fonts());
        for theme in [Theme::Light, Theme::Dark] {
            let palette = Palette::built_in(theme);
            let pdf = pdf(&doc, &desc, &palette, title, None);
            let colours = colours_off(&doc, &pdf, &palette);
            eprintln!(
                "{name} {theme:?}: looked inside {} of {} shapes",
                colours.looked, colours.shapes
            );
            assert!(colours.looked * 2 >= colours.shapes, "{name} {theme:?}");
            assert!(
                colours.off.is_empty(),
                "{name} {theme:?}: {:?}",
                colours.off
            );
        }
    }
}

/// The legend's and key's `Surface2` panels left unfilled fails, in
/// both themes, though no pixel moves past [`MAX_DELTA`].
#[test]
fn unfilled_panels_fail() {
    for theme in [Theme::Light, Theme::Dark] {
        let (doc, pdf, palette) = case(theme);
        let surface = palette
            .rgb(Role::Neutral(Neutral::Surface2))
            .map(|c| f32::from(c) / 255.0);
        let broken = damaged(&pdf, |ops| {
            let mut panel = false;
            for op in ops.iter_mut() {
                match op.operator.as_str() {
                    "rg" => {
                        panel = op
                            .operands
                            .iter()
                            .zip(surface)
                            .all(|(o, c)| o.as_float().is_ok_and(|v| (v - c).abs() < 0.001));
                    }
                    "f" if panel => op.operator = "n".to_owned(),
                    _ => {}
                }
            }
        });
        assert!(
            !halos_off(&doc, &broken, &palette).is_empty()
                || !colours_off(&doc, &broken, &palette).off.is_empty(),
            "{theme:?}: unfilled panels must fail"
        );
    }
}
