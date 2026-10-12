//! Circles, rectangles, lines, paths and hit areas.

use super::{Context, DATA_HIT, Paints, attr, coord, paint, point, shows, stroke};
use crate::num::fmt_coord;
use crate::svg::{ItemKey, PathCommand, Point};

/// A rectangle's geometry.
#[derive(Debug, Clone, Copy)]
pub(super) struct Frame {
    pub(super) origin: Point,
    pub(super) width: f64,
    pub(super) height: f64,
    pub(super) radius: f64,
}

/// ` fill=".." stroke=".." [stroke-width=".."]`.
fn painted(out: &mut String, paints: Paints, context: Context<'_>) {
    attr(out, "fill", &paint(paints.fill, context.target));
    stroke(out, paints, context.target);
}

pub(super) fn circle(centre: Point, radius: f64, paints: Paints, context: Context<'_>) -> String {
    let mut out = String::from("<circle");
    coord(&mut out, "cx", centre.x);
    coord(&mut out, "cy", centre.y);
    coord(&mut out, "r", radius);
    painted(&mut out, paints, context);
    out.push_str("/>\n");
    out
}

pub(super) fn rect(frame: &Frame, paints: Paints, context: Context<'_>) -> String {
    let mut out = String::from("<rect");
    coord(&mut out, "x", frame.origin.x);
    coord(&mut out, "y", frame.origin.y);
    coord(&mut out, "width", frame.width);
    coord(&mut out, "height", frame.height);
    if shows(frame.radius) {
        coord(&mut out, "rx", frame.radius);
    }
    painted(&mut out, paints, context);
    out.push_str("/>\n");
    out
}

pub(super) fn line(from: Point, to: Point, paints: Paints, context: Context<'_>) -> String {
    let mut out = String::from("<line");
    coord(&mut out, "x1", from.x);
    coord(&mut out, "y1", from.y);
    coord(&mut out, "x2", to.x);
    coord(&mut out, "y2", to.y);
    stroke(&mut out, paints, context.target);
    out.push_str("/>\n");
    out
}

pub(super) fn path(commands: &[PathCommand], paints: Paints, context: Context<'_>) -> String {
    let mut out = String::from("<path");
    attr(&mut out, "d", &outline(commands));
    painted(&mut out, paints, context);
    out.push_str("/>\n");
    out
}

/// A path's `d`: `M x y` and `A r r 0 large sweep x y`.
fn outline(commands: &[PathCommand]) -> String {
    commands
        .iter()
        .map(|command| match *command {
            PathCommand::MoveTo(p) => ["M".to_owned(), point(p)].join(" "),
            PathCommand::Arc {
                radius,
                large_arc,
                sweep,
                end,
            } => {
                let r = fmt_coord(radius);
                let flag = |on: bool| if on { "1" } else { "0" };
                ["A", &r, &r, "0", flag(large_arc), flag(sweep), &point(end)].join(" ")
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// An invisible pointer target (FR-94); only interactive output reaches here.
pub(super) fn hit_area(centre: Point, radius: f64, key: ItemKey) -> String {
    let mut out = String::from("<circle");
    coord(&mut out, "cx", centre.x);
    coord(&mut out, "cy", centre.y);
    coord(&mut out, "r", radius);
    attr(&mut out, "fill", "none");
    attr(&mut out, "pointer-events", "all");
    attr(&mut out, DATA_HIT, &key.0.to_string());
    out.push_str("/>\n");
    out
}
